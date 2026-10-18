//! Primitive creation (cube, plane, sphere, cylinder, etc.).

use bevy::prelude::*;
use polyforge_core::EditableMesh;

/// Create a unit cube centered at origin.
pub fn create_cube() -> EditableMesh {
    EditableMesh::cube()
}

/// Create a unit plane on XZ (Y up).
pub fn create_plane(size: f32) -> EditableMesh {
    let h = size * 0.5;
    let mut mesh = EditableMesh::default();
    mesh.vertices = vec![
        Vec3::new(-h, 0.0, -h),
        Vec3::new(h, 0.0, -h),
        Vec3::new(h, 0.0, h),
        Vec3::new(-h, 0.0, h),
    ];
    mesh.uvs = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(1.0, 1.0),
        Vec2::new(0.0, 1.0),
    ];
    mesh.faces = vec![polyforge_core::Face::quad(0, 1, 2, 3)];
    mesh.recalculate_normals();
    mesh
}

/// Create a cylinder along Y axis.
pub fn create_cylinder(segments: u32, radius: f32, height: f32) -> EditableMesh {
    let segments = segments.max(3);
    let mut mesh = EditableMesh::default();
    let half = height * 0.5;

    for ring in 0..2 {
        let y = if ring == 0 { -half } else { half };
        for i in 0..segments {
            let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
            mesh.vertices
                .push(Vec3::new(a.cos() * radius, y, a.sin() * radius));
            mesh.normals.push(Vec3::new(a.cos(), 0.0, a.sin()));
            mesh.uvs
                .push(Vec2::new(i as f32 / segments as f32, ring as f32));
        }
    }

    for i in 0..segments {
        let b0 = i;
        let b1 = (i + 1) % segments;
        let t0 = i + segments;
        let t1 = (i + 1) % segments + segments;
        mesh.faces
            .push(polyforge_core::Face::quad(b0, b1, t1, t0));
    }

    let bottom_center = mesh.vertices.len() as u32;
    mesh.vertices.push(Vec3::new(0.0, -half, 0.0));
    mesh.normals.push(-Vec3::Y);
    mesh.uvs.push(Vec2::new(0.5, 0.5));
    let top_center = mesh.vertices.len() as u32;
    mesh.vertices.push(Vec3::new(0.0, half, 0.0));
    mesh.normals.push(Vec3::Y);
    mesh.uvs.push(Vec2::new(0.5, 0.5));

    for i in 0..segments {
        let b0 = i;
        let b1 = (i + 1) % segments;
        mesh.faces
            .push(polyforge_core::Face::triangle(bottom_center, b1, b0));
        let t0 = i + segments;
        let t1 = (i + 1) % segments + segments;
        mesh.faces
            .push(polyforge_core::Face::triangle(top_center, t0, t1));
    }

    mesh.recalculate_normals();
    mesh
}

/// Create a simple UV sphere.
pub fn create_uv_sphere(segments: u32, rings: u32, radius: f32) -> EditableMesh {
    let mut mesh = EditableMesh::default();

    for y in 0..=rings {
        let v = y as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        for x in 0..=segments {
            let u = x as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;

            let px = radius * phi.sin() * theta.cos();
            let py = radius * phi.cos();
            let pz = radius * phi.sin() * theta.sin();

            mesh.vertices.push(Vec3::new(px, py, pz));
            mesh.normals.push(Vec3::new(px, py, pz).normalize_or_zero());
            mesh.uvs.push(Vec2::new(u, v));
        }
    }

    for y in 0..rings {
        for x in 0..segments {
            let i0 = y * (segments + 1) + x;
            let i1 = i0 + 1;
            let i2 = i0 + (segments + 1) + 1;
            let i3 = i0 + (segments + 1);

            mesh.faces.push(polyforge_core::Face::quad(i0, i1, i2, i3));
        }
    }

    mesh
}

/// Create a cone along Y.
pub fn create_cone(segments: u32, radius: f32, height: f32) -> EditableMesh {
    let segments = segments.max(3);
    let mut mesh = EditableMesh::default();
    let half = height * 0.5;
    for i in 0..segments {
        let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
        mesh.vertices.push(Vec3::new(a.cos() * radius, -half, a.sin() * radius));
        mesh.normals.push(Vec3::Y);
        mesh.uvs.push(Vec2::new(i as f32 / segments as f32, 0.0));
    }
    let tip = mesh.vertices.len() as u32;
    mesh.vertices.push(Vec3::new(0.0, half, 0.0));
    mesh.normals.push(Vec3::Y);
    mesh.uvs.push(Vec2::new(0.5, 1.0));
    let base_c = mesh.vertices.len() as u32;
    mesh.vertices.push(Vec3::new(0.0, -half, 0.0));
    mesh.normals.push(-Vec3::Y);
    mesh.uvs.push(Vec2::new(0.5, 0.5));
    for i in 0..segments {
        let i0 = i;
        let i1 = (i + 1) % segments;
        mesh.faces.push(polyforge_core::Face::triangle(i0, i1, tip));
        mesh.faces.push(polyforge_core::Face::triangle(base_c, i1, i0));
    }
    mesh.recalculate_normals();
    mesh
}

/// Create a torus in the XZ plane.
pub fn create_torus(major_segments: u32, minor_segments: u32, major_r: f32, minor_r: f32) -> EditableMesh {
    let major_segments = major_segments.max(3);
    let minor_segments = minor_segments.max(3);
    let mut mesh = EditableMesh::default();
    for i in 0..major_segments {
        let u = (i as f32 / major_segments as f32) * std::f32::consts::TAU;
        for j in 0..minor_segments {
            let v = (j as f32 / minor_segments as f32) * std::f32::consts::TAU;
            let x = (major_r + minor_r * v.cos()) * u.cos();
            let y = minor_r * v.sin();
            let z = (major_r + minor_r * v.cos()) * u.sin();
            mesh.vertices.push(Vec3::new(x, y, z));
            mesh.normals.push(Vec3::Y);
            mesh.uvs.push(Vec2::new(i as f32 / major_segments as f32, j as f32 / minor_segments as f32));
        }
    }
    for i in 0..major_segments {
        for j in 0..minor_segments {
            let i0 = i * minor_segments + j;
            let i1 = i * minor_segments + (j + 1) % minor_segments;
            let i2 = ((i + 1) % major_segments) * minor_segments + (j + 1) % minor_segments;
            let i3 = ((i + 1) % major_segments) * minor_segments + j;
            mesh.faces.push(polyforge_core::Face::quad(i0, i1, i2, i3));
        }
    }
    mesh.recalculate_normals();
    mesh
}
