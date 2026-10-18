//! Simple non-destructive modifier stack (mirror first).

use bevy::prelude::*;
use super::mesh::{EditableMesh, Face};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum MirrorAxis {
    X,
    Y,
    Z,
}

/// Mirror modifier: live preview is applied in a derived mesh; **Apply** bakes it.
#[derive(Component, Reflect, Debug, Clone)]
#[reflect(Component)]
pub struct MirrorModifier {
    pub axis: MirrorAxis,
    pub merge_threshold: f32,
    pub enabled: bool,
}

impl Default for MirrorModifier {
    fn default() -> Self {
        Self {
            axis: MirrorAxis::X,
            merge_threshold: 1e-4,
            enabled: true,
        }
    }
}

/// Stack of modifiers on an object (order = evaluation order).
#[derive(Debug, Clone, Reflect)]
pub struct ArrayModifier {
    pub count: u32,
    pub offset: Vec3,
    pub enabled: bool,
}

impl Default for ArrayModifier {
    fn default() -> Self {
        Self {
            count: 3,
            offset: Vec3::new(1.2, 0.0, 0.0),
            enabled: true,
        }
    }
}

#[derive(Debug, Clone, Reflect)]
pub struct SolidifyModifier {
    pub thickness: f32,
    pub enabled: bool,
}

impl Default for SolidifyModifier {
    fn default() -> Self {
        Self {
            thickness: 0.1,
            enabled: true,
        }
    }
}

#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
pub struct ModifierStack {
    pub mirror: Option<MirrorModifier>,
    pub array: Option<ArrayModifier>,
    pub solidify: Option<SolidifyModifier>,
}


/// Evaluate mirror into a new mesh (does not mutate source).
pub fn eval_mirror(source: &EditableMesh, m: &MirrorModifier) -> EditableMesh {
    if !m.enabled {
        return source.clone();
    }
    let mut out = source.clone();
    let axis = m.axis;
    let thr = m.merge_threshold;

    // Snap plane verts
    for v in &mut out.vertices {
        match axis {
            MirrorAxis::X if v.x.abs() < thr => v.x = 0.0,
            MirrorAxis::Y if v.y.abs() < thr => v.y = 0.0,
            MirrorAxis::Z if v.z.abs() < thr => v.z = 0.0,
            _ => {}
        }
    }

    let count = out.vertices.len();
    let mut map = HashMap::new();
    for i in 0..count {
        let v = out.vertices[i];
        let mirrored = match axis {
            MirrorAxis::X => Vec3::new(-v.x, v.y, v.z),
            MirrorAxis::Y => Vec3::new(v.x, -v.y, v.z),
            MirrorAxis::Z => Vec3::new(v.x, v.y, -v.z),
        };
        // Weld if on plane
        let on_plane = match axis {
            MirrorAxis::X => v.x.abs() < thr,
            MirrorAxis::Y => v.y.abs() < thr,
            MirrorAxis::Z => v.z.abs() < thr,
        };
        if on_plane {
            map.insert(i as u32, i as u32);
        } else {
            let ni = out.vertices.len() as u32;
            out.vertices.push(mirrored);
            out.normals
                .push(out.normals.get(i).copied().unwrap_or(Vec3::Y));
            out.uvs.push(out.uvs.get(i).copied().unwrap_or(Vec2::ZERO));
            map.insert(i as u32, ni);
        }
    }

    let mut extra = Vec::new();
    for face in &source.faces {
        let mut indices: Vec<u32> = face.indices.iter().map(|i| map[i]).collect();
        indices.reverse();
        extra.push(Face { indices });
    }
    out.faces.extend(extra);
    out.recalculate_normals();
    out
}

/// Bake mirror modifier into the mesh and clear it.
pub fn apply_mirror(mesh: &mut EditableMesh, stack: &mut ModifierStack) {
    if let Some(m) = stack.mirror.take() {
        *mesh = eval_mirror(mesh, &m);
    }
}

pub fn eval_array(source: &EditableMesh, a: &ArrayModifier) -> EditableMesh {
    if !a.enabled || a.count <= 1 {
        return source.clone();
    }
    let mut out = source.clone();
    for i in 1..a.count {
        let offset = a.offset * i as f32;
        let base = out.vertices.len() as u32;
        for v in &source.vertices {
            out.vertices.push(*v + offset);
        }
        for n in &source.normals {
            out.normals.push(*n);
        }
        for uv in &source.uvs {
            out.uvs.push(*uv);
        }
        for face in &source.faces {
            out.faces.push(Face {
                indices: face.indices.iter().map(|x| x + base).collect(),
            });
        }
    }
    out
}

pub fn apply_all_modifiers(mesh: &mut EditableMesh, stack: &mut ModifierStack) {
    if let Some(m) = stack.solidify.take() {
        if m.enabled {
            // inline simple solidify
            let sel = super::selection::Selection::default();
            let _ = sel;
            // bake via eval mirror-style: call ops would create dep cycle — simple thickness along normals
            let mut accum = std::collections::HashMap::<u32, (Vec3, u32)>::new();
            for face in &mesh.faces {
                let n = super::topology::face_normal(mesh, face);
                for &vi in &face.indices {
                    let e = accum.entry(vi).or_insert((Vec3::ZERO, 0));
                    e.0 += n;
                    e.1 += 1;
                }
            }
            let mut map = std::collections::HashMap::new();
            for (vi, (sum, count)) in &accum {
                let n = (*sum / *count as f32).normalize_or_zero();
                let pos = mesh.vertices[*vi as usize] + n * m.thickness;
                let ni = mesh.vertices.len() as u32;
                mesh.vertices.push(pos);
                mesh.normals.push(-n);
                mesh.uvs.push(mesh.uvs.get(*vi as usize).copied().unwrap_or(Vec2::ZERO));
                map.insert(*vi, ni);
            }
            let mut new_faces = mesh.faces.clone();
            for face in &mesh.faces {
                if face.indices.iter().all(|i| map.contains_key(i)) {
                    let mut inner: Vec<u32> = face.indices.iter().map(|i| map[i]).collect();
                    inner.reverse();
                    new_faces.push(Face { indices: inner });
                    let n = face.indices.len();
                    for i in 0..n {
                        let a = face.indices[i];
                        let b = face.indices[(i + 1) % n];
                        new_faces.push(Face::quad(a, b, map[&b], map[&a]));
                    }
                }
            }
            mesh.faces = new_faces;
            mesh.recalculate_normals();
        }
    }
    if let Some(m) = stack.mirror.take() {
        *mesh = eval_mirror(mesh, &m);
    }
    if let Some(a) = stack.array.take() {
        *mesh = eval_array(mesh, &a);
    }
}

/// Non-destructive evaluation of the full stack (does not mutate source).
pub fn eval_modifier_stack(source: &EditableMesh, stack: &ModifierStack) -> EditableMesh {
    let mut out = source.clone();
    if let Some(ref s) = stack.solidify {
        if s.enabled {
            let mut tmp_stack = ModifierStack {
                solidify: Some(s.clone()),
                mirror: None,
                array: None,
            };
            apply_all_modifiers(&mut out, &mut tmp_stack);
            // re-put solidify was taken — clone path uses take; rebuild
            out = source.clone();
            // Manual solidify eval without take
            out = eval_solidify_preview(&out, s);
        }
    }
    if let Some(ref m) = stack.mirror {
        if m.enabled {
            out = eval_mirror(&out, m);
        }
    }
    if let Some(ref a) = stack.array {
        if a.enabled {
            out = eval_array(&out, a);
        }
    }
    out
}

fn eval_solidify_preview(mesh: &EditableMesh, m: &SolidifyModifier) -> EditableMesh {
    let mut out = mesh.clone();
    let mut accum = std::collections::HashMap::<u32, (Vec3, u32)>::new();
    for face in &out.faces {
        let n = super::topology::face_normal(&out, face);
        for &vi in &face.indices {
            let e = accum.entry(vi).or_insert((Vec3::ZERO, 0));
            e.0 += n;
            e.1 += 1;
        }
    }
    let mut map = std::collections::HashMap::new();
    for (vi, (sum, count)) in &accum {
        let n = (*sum / *count as f32).normalize_or_zero();
        let pos = out.vertices[*vi as usize] + n * m.thickness;
        let ni = out.vertices.len() as u32;
        out.vertices.push(pos);
        out.normals.push(-n);
        out.uvs.push(out.uvs.get(*vi as usize).copied().unwrap_or(Vec2::ZERO));
        map.insert(*vi, ni);
    }
    let mut new_faces = out.faces.clone();
    for face in &mesh.faces {
        if face.indices.iter().all(|i| map.contains_key(i)) {
            let mut inner: Vec<u32> = face.indices.iter().map(|i| map[i]).collect();
            inner.reverse();
            new_faces.push(Face { indices: inner });
            let n = face.indices.len();
            for i in 0..n {
                let a = face.indices[i];
                let b = face.indices[(i + 1) % n];
                new_faces.push(Face::quad(a, b, map[&b], map[&a]));
            }
        }
    }
    out.faces = new_faces;
    out.recalculate_normals();
    out
}
