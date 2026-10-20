//! Simple PolyForge project save / load (JSON of editable mesh).

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use crate::{IoError, IoResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub name: String,
    pub mesh: MeshData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub faces: Vec<Vec<u32>>,
    #[serde(default)]
    pub seams: Vec<(u32, u32)>,
}

impl From<&EditableMesh> for MeshData {
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

impl MeshData {
    pub fn to_editable(&self) -> EditableMesh {
        let mut mesh = EditableMesh::default();
        mesh.vertices = self.vertices.iter().map(|v| Vec3::new(v[0], v[1], v[2])).collect();
        mesh.normals = self.normals.iter().map(|v| Vec3::new(v[0], v[1], v[2])).collect();
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

pub fn save_project(mesh: &EditableMesh, path: &Path, name: &str) -> IoResult<()> {
    let project = ProjectFile {
        version: 1,
        name: name.to_string(),
        mesh: MeshData::from(mesh),
    };
    let text = serde_json::to_string_pretty(&project)
        .map_err(|e| IoError::Parse(format!("serialize: {e}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn load_project(path: &Path) -> IoResult<(String, EditableMesh)> {
    let text = std::fs::read_to_string(path)?;
    let project: ProjectFile =
        serde_json::from_str(&text).map_err(|e| IoError::Parse(format!("json: {e}")))?;
    Ok((project.name, project.mesh.to_editable()))
}
