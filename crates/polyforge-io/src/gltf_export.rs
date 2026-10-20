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

/// PNG payloads for a glTF PBR material. Each map is embedded in the file as
/// a base64 `data:` URI, so the exported `.gltf` is self-contained.
#[derive(Default, Clone, Copy)]
pub struct GltfTextures<'a> {
    /// sRGB base-colour map.
    pub base_color: Option<&'a [u8]>,
    /// Linear ORM map: `R` = AO, `G` = roughness, `B` = metalness.
    pub metallic_roughness: Option<&'a [u8]>,
    /// Linear tangent-space normal map.
    pub normal: Option<&'a [u8]>,
    /// sRGB emissive map.
    pub emissive: Option<&'a [u8]>,
}

impl GltfTextures<'_> {
    fn is_empty(&self) -> bool {
        self.base_color.is_none()
            && self.metallic_roughness.is_none()
            && self.normal.is_none()
            && self.emissive.is_none()
    }
}

fn data_uri(mime: &str, bytes: &[u8]) -> String {
    format!("data:{mime};base64,{}", base64_encode(bytes))
}

/// Export mesh as minimal glTF 2.0 with base64 buffer.
pub fn export_gltf(mesh: &EditableMesh) -> IoResult<String> {
    export_gltf_with_textures(mesh, &GltfTextures::default())
}

/// Export mesh as glTF 2.0 with a single PBR material whose maps are embedded
/// as base64 `data:` URIs.
pub fn export_gltf_with_textures(
    mesh: &EditableMesh,
    textures: &GltfTextures,
) -> IoResult<String> {
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

    let mut primitive = serde_json::json!({
        "attributes": { "POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2 },
        "indices": 3,
        "mode": 4
    });

    let mut images: Vec<serde_json::Value> = Vec::new();
    let mut slots: Vec<(&str, &[u8])> = Vec::new();
    if let Some(bytes) = textures.base_color {
        slots.push(("baseColorTexture", bytes));
    }
    if let Some(bytes) = textures.metallic_roughness {
        slots.push(("metallicRoughnessTexture", bytes));
    }
    if let Some(bytes) = textures.normal {
        slots.push(("normalTexture", bytes));
    }
    if let Some(bytes) = textures.emissive {
        slots.push(("emissiveTexture", bytes));
    }

    let mut pbr = serde_json::Map::new();
    let mut material = serde_json::Map::new();
    for (slot, bytes) in &slots {
        let index = images.len();
        images.push(serde_json::json!({
            "uri": data_uri("image/png", bytes),
            "name": slot.trim_end_matches("Texture"),
        }));
        let reference = serde_json::json!({ "index": index, "texCoord": 0 });
        if *slot == "normalTexture" {
            material.insert((*slot).into(), reference);
        } else if *slot == "emissiveTexture" {
            material.insert((*slot).into(), reference);
            material.insert("emissiveFactor".into(), serde_json::json!([1.0, 1.0, 1.0]));
        } else {
            pbr.insert((*slot).into(), reference);
        }
    }

    if !slots.is_empty() {
        primitive["material"] = serde_json::json!(0);
    }

    let mut gltf = serde_json::json!({
        "asset": { "version": "2.0", "generator": "PolyForge" },
        "scenes": [{ "nodes": [0] }],
        "scene": 0,
        "nodes": [{ "mesh": 0 }],
        "meshes": [{
            "primitives": [primitive]
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

    if !slots.is_empty() {
        let textures_json: Vec<serde_json::Value> = (0..images.len())
            .map(|source| serde_json::json!({ "source": source, "sampler": 0 }))
            .collect();
        material.insert("name".into(), serde_json::json!("PolyForgePBR"));
        material.insert("pbrMetallicRoughness".into(), serde_json::Value::Object(pbr));

        let object = gltf
            .as_object_mut()
            .ok_or_else(|| IoError::Parse("gltf root must be an object".into()))?;
        object.insert("images".into(), serde_json::Value::Array(images));
        object.insert("textures".into(), serde_json::Value::Array(textures_json));
        object.insert(
            "samplers".into(),
            serde_json::json!([{ "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 }]),
        );
        object.insert(
            "materials".into(),
            serde_json::Value::Array(vec![serde_json::Value::Object(material)]),
        );
    }

    serde_json::to_string_pretty(&gltf).map_err(|e| IoError::Parse(format!("gltf json: {e}")))
}

pub fn write_gltf_file(mesh: &EditableMesh, path: &Path) -> IoResult<()> {
    let text = export_gltf(mesh)?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Write a glTF 2.0 file with an embedded PBR material to `path`.
pub fn write_gltf_textured_file(
    mesh: &EditableMesh,
    textures: &GltfTextures,
    path: &Path,
) -> IoResult<()> {
    let text = export_gltf_with_textures(mesh, textures)?;
    std::fs::write(path, text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_base64(text: &str) -> Vec<u8> {
        const T: &[u8] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        let mut acc: u32 = 0;
        let mut bits = 0;
        for b in text.bytes() {
            if b == b'=' {
                break;
            }
            let v = T.iter().position(|&c| c == b).expect("valid base64") as u32;
            acc = (acc << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        out
    }

    #[test]
    fn base64_matches_rfc4648_examples() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(
            decode_base64(&base64_encode(b"polyforge")),
            b"polyforge".to_vec()
        );
    }

    #[test]
    fn plain_export_has_no_material() {
        let mesh = EditableMesh::cube();
        let json: serde_json::Value =
            serde_json::from_str(&export_gltf(&mesh).unwrap()).unwrap();
        assert!(json.get("materials").is_none());
        assert!(json["meshes"][0]["primitives"][0].get("material").is_none());
    }

    #[test]
    fn textured_export_embeds_pbr_material() {
        let mesh = EditableMesh::cube();
        let base: Vec<u8> = vec![1, 2, 3, 4];
        let orm: Vec<u8> = vec![5, 6, 7, 8];
        let normal: Vec<u8> = vec![9, 10];
        let emissive: Vec<u8> = vec![11];

        let textures = GltfTextures {
            base_color: Some(&base),
            metallic_roughness: Some(&orm),
            normal: Some(&normal),
            emissive: Some(&emissive),
        };
        let json: serde_json::Value =
            serde_json::from_str(&export_gltf_with_textures(&mesh, &textures).unwrap()).unwrap();

        assert_eq!(json["images"].as_array().unwrap().len(), 4);
        assert_eq!(json["textures"][0]["source"], 0);
        assert_eq!(json["meshes"][0]["primitives"][0]["material"], 0);

        let material = &json["materials"][0];
        assert_eq!(material["pbrMetallicRoughness"]["baseColorTexture"]["index"], 0);
        assert_eq!(
            material["pbrMetallicRoughness"]["metallicRoughnessTexture"]["index"],
            1
        );
        assert_eq!(material["normalTexture"]["index"], 2);
        assert_eq!(material["emissiveTexture"]["index"], 3);
        assert_eq!(material["emissiveFactor"], serde_json::json!([1.0, 1.0, 1.0]));
        assert_eq!(json["samplers"][0]["wrapS"], 10497);

        let uri = json["images"][0]["uri"].as_str().unwrap();
        assert!(uri.starts_with("data:image/png;base64,"));
        let payload = &uri["data:image/png;base64,".len()..];
        assert_eq!(decode_base64(payload), base);
    }

    #[test]
    fn textured_export_only_uses_provided_maps() {
        let mesh = EditableMesh::cube();
        let base: Vec<u8> = vec![42];
        let textures = GltfTextures {
            base_color: Some(&base),
            ..Default::default()
        };
        let json: serde_json::Value =
            serde_json::from_str(&export_gltf_with_textures(&mesh, &textures).unwrap()).unwrap();

        assert_eq!(json["images"].as_array().unwrap().len(), 1);
        let material = &json["materials"][0];
        assert!(material.get("normalTexture").is_none());
        assert!(material.get("emissiveTexture").is_none());
        assert_eq!(material["pbrMetallicRoughness"]["baseColorTexture"]["index"], 0);
    }

    #[test]
    fn empty_mesh_is_rejected() {
        let mesh = EditableMesh::default();
        assert!(export_gltf(&mesh).is_err());
    }
}
