//! Minimal glTF 2.0 JSON export (single mesh, embedded buffer).

use bevy::math::{Vec2, Vec3};
use polyforge_core::EditableMesh;
use crate::{IoError, IoResult};
use std::path::Path;

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    match data.len() - i {
        1 => {
            let n = (data[i] as u32) << 16;
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

/// Export mesh as minimal glTF 2.0 with base64 buffer.
pub fn export_gltf(mesh: &EditableMesh) -> IoResult<String> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for face in &mesh.faces {
        if face.indices.len() < 3 {
            continue;
        }
        for i in 1..face.indices.len() - 1 {
            let tri = [face.indices[0], face.indices[i], face.indices[i + 1]];
            for &vi in &tri {
                let base = (positions.len() / 3) as u32;
                let v = mesh.vertices.get(vi as usize).copied().unwrap_or(Vec3::ZERO);
                let n = mesh.normals.get(vi as usize).copied().unwrap_or(Vec3::Y);
                let uv = mesh.uvs.get(vi as usize).copied().unwrap_or(Vec2::ZERO);
                positions.extend_from_slice(&[v.x, v.y, v.z]);
                normals.extend_from_slice(&[n.x, n.y, n.z]);
                uvs.extend_from_slice(&[uv.x, uv.y]);
                indices.push(base);
            }
        }
    }

    if positions.is_empty() {
        return Err(IoError::Parse("empty mesh".into()));
    }

    let mut bin = Vec::new();
    let write_f32s = |bin: &mut Vec<u8>, data: &[f32]| {
        for f in data {
            bin.extend_from_slice(&f.to_le_bytes());
        }
    };
    let pos_offset = 0usize;
    write_f32s(&mut bin, &positions);
    let norm_offset = bin.len();
    write_f32s(&mut bin, &normals);
    let uv_offset = bin.len();
    write_f32s(&mut bin, &uvs);
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let idx_offset = bin.len();
    for i in &indices {
        bin.extend_from_slice(&i.to_le_bytes());
    }

    let vert_count = positions.len() / 3;
    let idx_count = indices.len();
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for i in 0..vert_count {
        for k in 0..3 {
            let v = positions[i * 3 + k];
            min[k] = min[k].min(v);
            max[k] = max[k].max(v);
        }
    }

    let b64 = base64_encode(&bin);
    let uri = format!("data:application/octet-stream;base64,{b64}");

    let gltf = serde_json::json!({
        "asset": { "version": "2.0", "generator": "PolyForge" },
        "scenes": [{ "nodes": [0] }],
        "scene": 0,
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [{
                "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 },
                "indices": 3,
                "mode": 4
            }]
        }],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": vert_count, "type": "VEC3", "max": max, "min": min },
            { "bufferView": 1, "componentType": 5126, "count": vert_count, "type": "VEC3" },
            { "bufferView": 2, "componentType": 5126, "count": vert_count, "type": "VEC2" },
            { "bufferView": 3, "componentType": 5125, "count": idx_count, "type": "SCALAR" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": pos_offset, "byteLength": vert_count * 12, "target": 34962 },
            { "buffer": 0, "byteOffset": norm_offset, "byteLength": vert_count * 12, "target": 34962 },
            { "buffer": 0, "byteOffset": uv_offset, "byteLength": vert_count * 8, "target": 34962 },
            { "buffer": 0, "byteOffset": idx_offset, "byteLength": idx_count * 4, "target": 34963 }
        ],
        "buffers": [{ "byteLength": bin.len(), "uri": uri }]
    });

    serde_json::to_string_pretty(&gltf).map_err(|e| IoError::Parse(format!("gltf json: {e}")))
}

pub fn write_gltf_file(mesh: &EditableMesh, path: &Path) -> IoResult<()> {
    let text = export_gltf(mesh)?;
    std::fs::write(path, text)?;
    Ok(())
}
