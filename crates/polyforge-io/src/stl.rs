//! Binary + ASCII STL export (Phase 29).

use polyforge_core::EditableMesh;
use std::io::{self, Write};
use std::path::Path;

pub fn write_stl_ascii(mesh: &EditableMesh, path: &Path) -> io::Result<()> {
    let mut f = std::fs::File::create(path)?;
    writeln!(f, "solid polyforge")?;
    for face in &mesh.faces {
        if face.indices.len() < 3 {
            continue;
        }
        // Fan triangulation
        let i0 = face.indices[0] as usize;
        for w in face.indices[1..].windows(2) {
            let (i1, i2) = (w[0] as usize, w[1] as usize);
            let (Some(a), Some(b), Some(c)) = (
                mesh.vertices.get(i0),
                mesh.vertices.get(i1),
                mesh.vertices.get(i2),
            ) else {
                continue;
            };
            let n = (*b - *a).cross(*c - *a).normalize_or_zero();
            writeln!(f, "  facet normal {} {} {}", n.x, n.y, n.z)?;
            writeln!(f, "    outer loop")?;
            writeln!(f, "      vertex {} {} {}", a.x, a.y, a.z)?;
            writeln!(f, "      vertex {} {} {}", b.x, b.y, b.z)?;
            writeln!(f, "      vertex {} {} {}", c.x, c.y, c.z)?;
            writeln!(f, "    endloop")?;
            writeln!(f, "  endfacet")?;
        }
    }
    writeln!(f, "endsolid polyforge")?;
    Ok(())
}

pub fn write_stl_binary(mesh: &EditableMesh, path: &Path) -> io::Result<()> {
    let mut tris: Vec<([f32; 3], [f32; 3], [f32; 3], [f32; 3])> = Vec::new();
    for face in &mesh.faces {
        if face.indices.len() < 3 {
            continue;
        }
        let i0 = face.indices[0] as usize;
        for w in face.indices[1..].windows(2) {
            let (i1, i2) = (w[0] as usize, w[1] as usize);
            let (Some(a), Some(b), Some(c)) = (
                mesh.vertices.get(i0),
                mesh.vertices.get(i1),
                mesh.vertices.get(i2),
            ) else {
                continue;
            };
            let n = (*b - *a).cross(*c - *a).normalize_or_zero();
            tris.push((
                [n.x, n.y, n.z],
                [a.x, a.y, a.z],
                [b.x, b.y, b.z],
                [c.x, c.y, c.z],
            ));
        }
    }
    let mut f = std::fs::File::create(path)?;
    let header = [0u8; 80];
    f.write_all(&header)?;
    f.write_all(&(tris.len() as u32).to_le_bytes())?;
    for (n, a, b, c) in tris {
        for v in [n, a, b, c] {
            for x in v {
                f.write_all(&x.to_le_bytes())?;
            }
        }
        f.write_all(&0u16.to_le_bytes())?; // attribute
    }
    Ok(())
}
