//! Multi-mode scene project (v2): poly mesh + CAD features + vector paths.

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use crate::{IoError, IoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SCENE_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneProject {
    pub version: u32,
    pub name: String,
    /// Legacy single active mesh.
    pub mesh: Option<MeshDataV2>,
    /// Phase 9: all editable meshes in the scene.
    #[serde(default)]
    pub meshes: Vec<NamedMeshData>,
    pub cad_bodies: Vec<CadBodyData>,
    pub vector_paths: Vec<VectorPathData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedMeshData {
    pub name: String,
    pub translation: [f32; 3],
    pub mesh: MeshDataV2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshDataV2 {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub faces: Vec<Vec<u32>>,
    #[serde(default)]
    pub seams: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CadBodyData {
    pub name: String,
    pub translation: [f32; 3],
    pub features_json: String, // opaque serialized features via serde_json value list
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorPathData {
    pub name: String,
    pub points: Vec<[f32; 2]>,
    pub closed: bool,
}

impl From<&EditableMesh> for MeshDataV2 {
    fn from(m: &EditableMesh) -> Self {
        Self {
            vertices: m.vertices.iter().map(|v| [v.x, v.y, v.z]).collect(),
            normals: m.normals.iter().map(|v| [v.x, v.y, v.z]).collect(),
            uvs: m.uvs.iter().map(|v| [v.x, v.y]).collect(),
            faces: m.faces.iter().map(|f| f.indices.clone()).collect(),
            seams: m.seams.iter().copied().collect(),
        }
    }
}

impl MeshDataV2 {
    pub fn to_editable(&self) -> EditableMesh {
        let mut mesh = EditableMesh::default();
        mesh.vertices = self
            .vertices
            .iter()
            .map(|v| Vec3::new(v[0], v[1], v[2]))
            .collect();
        mesh.normals = self
            .normals
            .iter()
            .map(|v| Vec3::new(v[0], v[1], v[2]))
            .collect();
        mesh.uvs = self.uvs.iter().map(|v| Vec2::new(v[0], v[1])).collect();
        mesh.faces = self
            .faces
            .iter()
            .map(|idx| polyforge_core::Face {
                indices: idx.clone(),
            })
            .collect();
        mesh.seams = self.seams.iter().copied().collect();
        if mesh.normals.len() != mesh.vertices.len() {
            mesh.recalculate_normals();
        }
        mesh
    }
}

pub fn save_scene_project(project: &SceneProject, path: &Path) -> IoResult<()> {
    let text = serde_json::to_string_pretty(project)
        .map_err(|e| IoError::Parse(format!("serialize scene: {e}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn load_scene_project(path: &Path) -> IoResult<SceneProject> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| IoError::Parse(format!("scene json: {e}")))
}
