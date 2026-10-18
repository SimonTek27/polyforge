//! Minimal Wavefront OBJ import / export.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Face};
use crate::{IoError, IoResult};
use std::path::Path;

/// Export mesh to OBJ text (vertices + faces only).
pub fn export_obj(mesh: &EditableMesh) -> String {
    let mut out = String::from("# PolyForge OBJ export\n");
    for v in &mesh.vertices {
        out.push_str(&format!("v {} {} {}\n", v.x, v.y, v.z));
    }
    for uv in &mesh.uvs {
        out.push_str(&format!("vt {} {}\n", uv.x, uv.y));
    }
    for n in &mesh.normals {
        out.push_str(&format!("vn {} {} {}\n", n.x, n.y, n.z));
    }
    for face in &mesh.faces {
        out.push('f');
        for &idx in &face.indices {
            let i = idx + 1; // OBJ is 1-based
            if (idx as usize) < mesh.uvs.len() && (idx as usize) < mesh.normals.len() {
                out.push_str(&format!(" {i}/{i}/{i}"));
            } else {
                out.push_str(&format!(" {i}"));
            }
        }
        out.push('\n');
    }
    out
}

/// Write OBJ to a file path.
pub fn write_obj_file(mesh: &EditableMesh, path: &Path) -> IoResult<()> {
    let text = export_obj(mesh);
    std::fs::write(path, text)?;
    Ok(())
}

/// Parse a simple OBJ (v / f lines; ignores groups/materials).
pub fn import_obj(text: &str) -> IoResult<EditableMesh> {
    let mut mesh = EditableMesh::default();
    let mut faces = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let tag = parts.next().unwrap_or("");
        match tag {
            "v" => {
                let x: f32 = parts.next().ok_or_else(|| IoError::Parse("v x".into()))?.parse()
                    .map_err(|e| IoError::Parse(format!("v x: {e}")))?;
                let y: f32 = parts.next().ok_or_else(|| IoError::Parse("v y".into()))?.parse()
                    .map_err(|e| IoError::Parse(format!("v y: {e}")))?;
                let z: f32 = parts.next().ok_or_else(|| IoError::Parse("v z".into()))?.parse()
                    .map_err(|e| IoError::Parse(format!("v z: {e}")))?;
                mesh.vertices.push(Vec3::new(x, y, z));
                mesh.normals.push(Vec3::Y);
                mesh.uvs.push(Vec2::ZERO);
            }
            "f" => {
                let mut indices = Vec::new();
                for token in parts {
                    // formats: i  |  i/j  |  i/j/k  |  i//k
                    let first = token.split('/').next().unwrap_or("0");
                    let mut idx: i32 = first
                        .parse()
                        .map_err(|e| IoError::Parse(format!("f index: {e}")))?;
                    if idx < 0 {
                        idx = mesh.vertices.len() as i32 + idx + 1;
                    }
                    if idx <= 0 {
                        return Err(IoError::Parse(format!("invalid face index {idx}")));
                    }
                    indices.push((idx - 1) as u32);
                }
                if indices.len() >= 3 {
                    faces.push(Face { indices });
                }
            }
            _ => {}
        }
    }

    if mesh.vertices.is_empty() {
        return Err(IoError::Parse("no vertices in OBJ".into()));
    }
    mesh.faces = faces;
    mesh.recalculate_normals();
    Ok(mesh)
}

pub fn read_obj_file(path: &Path) -> IoResult<EditableMesh> {
    let text = std::fs::read_to_string(path)?;
    import_obj(&text)
}
