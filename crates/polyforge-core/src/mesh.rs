//! Editable mesh representation optimized for interactive modeling.

use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::asset::RenderAssetUsages;
use serde::{Deserialize, Serialize};
use super::selection::Selection;

/// Unique identifier for an editable mesh entity.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[reflect(Component)]
pub struct MeshId(pub u64);

/// Main editable mesh component.
/// Stores geometry in a form convenient for topology editing
/// (vertices, edges, faces) rather than pure GPU triangle buffers.
#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct EditableMesh {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    /// Faces stored as lists of vertex indices (supports quads & triangles).
    pub faces: Vec<Face>,
    /// Optional material handle (Bevy asset path or id).
    pub material: Option<String>,
    /// UV seam edges (min,max) vertex pairs.
    #[serde(default)]
    #[reflect(ignore)]
    pub seams: std::collections::HashSet<(u32, u32)>,
    /// Per-face material slot index into a scene material table (0 = default).
    #[serde(default)]
    #[reflect(ignore)]
    pub face_materials: Vec<u32>,
}



impl Default for EditableMesh {
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            normals: Vec::new(),
            uvs: Vec::new(),
            faces: Vec::new(),
            material: None,
            seams: std::collections::HashSet::new(),
            face_materials: Vec::new(),
        }
    }
}



/// A single face (polygon). Currently limited to 3 or 4 vertices for simplicity.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub struct Face {
    pub indices: Vec<u32>,
}

impl Face {
    pub fn triangle(a: u32, b: u32, c: u32) -> Self {
        Self {
            indices: vec![a, b, c],
        }
    }

    pub fn quad(a: u32, b: u32, c: u32, d: u32) -> Self {
        Self {
            indices: vec![a, b, c, d],
        }
    }

    pub fn is_triangle(&self) -> bool {
        self.indices.len() == 3
    }

    pub fn is_quad(&self) -> bool {
        self.indices.len() == 4
    }
}

impl EditableMesh {
    /// Create a simple unit cube.
    pub fn cube() -> Self {
        let vertices = vec![
            // Front
            Vec3::new(-0.5, -0.5, 0.5),
            Vec3::new(0.5, -0.5, 0.5),
            Vec3::new(0.5, 0.5, 0.5),
            Vec3::new(-0.5, 0.5, 0.5),
            // Back
            Vec3::new(-0.5, -0.5, -0.5),
            Vec3::new(0.5, -0.5, -0.5),
            Vec3::new(0.5, 0.5, -0.5),
            Vec3::new(-0.5, 0.5, -0.5),
        ];

        let faces = vec![
            Face::quad(0, 1, 2, 3), // Front
            Face::quad(5, 4, 7, 6), // Back
            Face::quad(4, 0, 3, 7), // Left
            Face::quad(1, 5, 6, 2), // Right
            Face::quad(3, 2, 6, 7), // Top
            Face::quad(4, 5, 1, 0), // Bottom
        ];

        let mut mesh = Self {
            vertices,
            normals: Vec::new(),
            uvs: vec![Vec2::ZERO; 8],
            faces,
            material: None,
            seams: std::collections::HashSet::new(),
            face_materials: Vec::new(),
        };
        mesh.recalculate_normals();
        mesh
    }

    /// Recalculate smooth vertex normals (simple average).
    pub fn recalculate_normals(&mut self) {
        self.normals = vec![Vec3::ZERO; self.vertices.len()];

        for face in &self.faces {
            if face.indices.len() < 3 {
                continue;
            }
            let v0 = self.vertices[face.indices[0] as usize];
            let v1 = self.vertices[face.indices[1] as usize];
            let v2 = self.vertices[face.indices[2] as usize];
            let normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();

            for &idx in &face.indices {
                self.normals[idx as usize] += normal;
            }
        }

        for n in &mut self.normals {
            *n = n.normalize_or_zero();
        }
    }

    /// Convert to a Bevy Mesh for rendering.
    pub fn to_bevy_mesh(&self) -> Mesh {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();

        let mut index_offset = 0u32;

        for face in &self.faces {
            // Triangulate quads / n-gons simply (fan)
            if face.indices.len() < 3 {
                continue;
            }

            for i in 1..face.indices.len() - 1 {
                let i0 = face.indices[0] as usize;
                let i1 = face.indices[i] as usize;
                let i2 = face.indices[i + 1] as usize;

                positions.push(self.vertices[i0]);
                positions.push(self.vertices[i1]);
                positions.push(self.vertices[i2]);

                normals.push(self.normals.get(i0).copied().unwrap_or(Vec3::Y));
                normals.push(self.normals.get(i1).copied().unwrap_or(Vec3::Y));
                normals.push(self.normals.get(i2).copied().unwrap_or(Vec3::Y));

                uvs.push(self.uvs.get(i0).copied().unwrap_or(Vec2::ZERO));
                uvs.push(self.uvs.get(i1).copied().unwrap_or(Vec2::ZERO));
                uvs.push(self.uvs.get(i2).copied().unwrap_or(Vec2::ZERO));

                indices.push(index_offset);
                indices.push(index_offset + 1);
                indices.push(index_offset + 2);
                index_offset += 3;
            }
        }

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_indices(Indices::U32(indices));
        mesh
    }

    /// Compute the center (average) of all vertices.
    pub fn center(&self) -> Vec3 {
        if self.vertices.is_empty() {
            return Vec3::ZERO;
        }
        let sum: Vec3 = self.vertices.iter().copied().sum();
        sum / self.vertices.len() as f32
    }

    /// Compute the center of selected vertices (or whole mesh if empty selection).
    pub fn selection_center(&self, selection: &Selection) -> Vec3 {
        if selection.vertices.is_empty() {
            return self.center();
        }
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for &idx in &selection.vertices {
            if let Some(v) = self.vertices.get(idx as usize) {
                sum += *v;
                count += 1;
            }
        }
        if count == 0 {
            self.center()
        } else {
            sum / count as f32
        }
    }
}

impl EditableMesh {
    pub fn mark_seam(&mut self, a: u32, b: u32) {
        let e = if a < b { (a, b) } else { (b, a) };
        self.seams.insert(e);
    }

    pub fn clear_seam(&mut self, a: u32, b: u32) {
        let e = if a < b { (a, b) } else { (b, a) };
        self.seams.remove(&e);
    }

    pub fn toggle_seam(&mut self, a: u32, b: u32) {
        let e = if a < b { (a, b) } else { (b, a) };
        if !self.seams.remove(&e) {
            self.seams.insert(e);
        }
    }

    pub fn ensure_face_materials(&mut self) {
        if self.face_materials.len() < self.faces.len() {
            self.face_materials.resize(self.faces.len(), 0);
        }
    }

    pub fn set_face_material(&mut self, face: u32, slot: u32) {
        self.ensure_face_materials();
        if (face as usize) < self.face_materials.len() {
            self.face_materials[face as usize] = slot;
        }
    }
}

