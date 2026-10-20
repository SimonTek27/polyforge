//! Minimal glTF 2.0 JSON import (embedded base64 buffer, single mesh).

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Face};
use crate::{IoError, IoResult};
use std::path::Path;

pub fn read_gltf_file(path: &Path) -> IoResult<EditableMesh> {
    let text = std::fs::read_to_string(path)?;
    import_gltf(&text)
}

pub fn import_gltf(text: &str) -> IoResult<EditableMesh> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| IoError::Parse(format!("json: {e}")))?;

    let buffers = v["buffers"]
        .as_array()
        .ok_or_else(|| IoError::Parse("no buffers".into()))?;
    let bin = decode_buffer(&buffers[0])?;

    let accessors = v["accessors"]
        .as_array()
        .ok_or_else(|| IoError::Parse("no accessors".into()))?;
    let views = v["bufferViews"]
        .as_array()
        .ok_or_else(|| IoError::Parse("no bufferViews".into()))?;

    let prim = &v["meshes"][0]["primitives"][0];
    let pos_acc = prim["attributes"]["POSITION"]
        .as_u64()
        .ok_or_else(|| IoError::Parse("POSITION".into()))? as usize;
    let idx_acc = prim["indices"].as_u64().map(|i| i as usize);

    let positions = read_f32_vec3(&bin, &accessors[pos_acc], &views[view_idx(&accessors[pos_acc])])?;
    let indices = if let Some(ia) = idx_acc {
        read_u32_indices(&bin, &accessors[ia], &views[view_idx(&accessors[ia])])?
    } else {
        (0..positions.len() as u32).collect()
    };

    let mut mesh = EditableMesh::default();
    for p in positions {
        mesh.vertices.push(p);
        mesh.normals.push(Vec3::Y);
        mesh.uvs.push(Vec2::ZERO);
    }
    for tri in indices.chunks(3) {
        if tri.len() == 3 {
            mesh.faces
                .push(Face::triangle(tri[0], tri[1], tri[2]));
        }
    }
    mesh.recalculate_normals();
    Ok(mesh)
}

fn view_idx(acc: &serde_json::Value) -> usize {
    acc["bufferView"].as_u64().unwrap_or(0) as usize
}

fn decode_buffer(buf: &serde_json::Value) -> IoResult<Vec<u8>> {
    let uri = buf["uri"]
        .as_str()
        .ok_or_else(|| IoError::Parse("buffer uri".into()))?;
    if let Some(b64) = uri.strip_prefix("data:application/octet-stream;base64,") {
        base64_decode(b64)
    } else if let Some(b64) = uri.strip_prefix("data:application/gltf-buffer;base64,") {
        base64_decode(b64)
    } else {
        Err(IoError::Parse("only embedded base64 buffers supported".into()))
    }
}

fn base64_decode(s: &str) -> IoResult<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|b| *b != b'=' && !b.is_ascii_whitespace()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let a = val(bytes[i]).unwrap_or(0) as u32;
        let b = val(bytes[i + 1]).unwrap_or(0) as u32;
        let c = val(bytes[i + 2]).unwrap_or(0) as u32;
        let d = val(bytes[i + 3]).unwrap_or(0) as u32;
        let n = (a << 18) | (b << 12) | (c << 6) | d;
        out.push(((n >> 16) & 255) as u8);
        out.push(((n >> 8) & 255) as u8);
        out.push((n & 255) as u8);
        i += 4;
    }
    // trim padding
    let pad = s.chars().rev().take_while(|c| *c == '=').count();
    if pad > 0 && out.len() >= pad {
        out.truncate(out.len() - pad);
    }
    Ok(out)
}

fn read_f32_vec3(
    bin: &[u8],
    acc: &serde_json::Value,
    view: &serde_json::Value,
) -> IoResult<Vec<Vec3>> {
    let count = acc["count"].as_u64().unwrap_or(0) as usize;
    let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let o = offset + i * 12;
        if o + 12 > bin.len() {
            return Err(IoError::Parse("pos OOB".into()));
        }
        let x = f32::from_le_bytes(bin[o..o + 4].try_into().unwrap());
        let y = f32::from_le_bytes(bin[o + 4..o + 8].try_into().unwrap());
        let z = f32::from_le_bytes(bin[o + 8..o + 12].try_into().unwrap());
        out.push(Vec3::new(x, y, z));
    }
    Ok(out)
}

fn read_u32_indices(
    bin: &[u8],
    acc: &serde_json::Value,
    view: &serde_json::Value,
) -> IoResult<Vec<u32>> {
    let count = acc["count"].as_u64().unwrap_or(0) as usize;
    let offset = view["byteOffset"].as_u64().unwrap_or(0) as usize
        + acc["byteOffset"].as_u64().unwrap_or(0) as usize;
    let ctype = acc["componentType"].as_u64().unwrap_or(5125);
    let mut out = Vec::with_capacity(count);
    match ctype {
        5125 => {
            // u32
            for i in 0..count {
                let o = offset + i * 4;
                if o + 4 > bin.len() {
                    break;
                }
                out.push(u32::from_le_bytes(bin[o..o + 4].try_into().unwrap()));
            }
        }
        5123 => {
            // u16
            for i in 0..count {
                let o = offset + i * 2;
                if o + 2 > bin.len() {
                    break;
                }
                out.push(u16::from_le_bytes(bin[o..o + 2].try_into().unwrap()) as u32);
            }
        }
        _ => return Err(IoError::Parse(format!("index type {ctype}"))),
    }
    Ok(out)
}
