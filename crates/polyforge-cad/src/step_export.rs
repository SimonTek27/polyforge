
//! Minimal ASCII "STEP-like" mesh dump for CAD bodies (Phase 18).
//! Not ISO 10303 compliant — interchange stub until OCCT is linked.

use polyforge_core::EditableMesh;
use std::path::Path;

pub fn export_step_like(mesh: &EditableMesh, name: &str) -> String {
    let mut s = String::new();
    s.push_str("ISO-10303-21;\n");
    s.push_str("HEADER;\n");
    s.push_str(&format!("FILE_NAME('{}','PolyForge');\n", name));
    s.push_str("FILE_DESCRIPTION(('PolyForge mesh tessellation stub'),'2;1');\n");
    s.push_str("ENDSEC;\n");
    s.push_str("DATA;\n");
    for (i, v) in mesh.vertices.iter().enumerate() {
        s.push_str(&format!(
            "#{id}=CARTESIAN_POINT('',({x:.6},{y:.6},{z:.6}));\n",
            id = i + 1,
            x = v.x,
            y = v.y,
            z = v.z
        ));
    }
    let mut fid = mesh.vertices.len() + 1;
    for face in &mesh.faces {
        if face.indices.len() < 3 {
            continue;
        }
        let refs: String = face
            .indices
            .iter()
            .map(|i| format!("#{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");
        s.push_str(&format!("#{fid}=POLY_LOOP('',({refs}));\n"));
        fid += 1;
    }
    s.push_str("ENDSEC;\n");
    s.push_str("END-ISO-10303-21;\n");
    s
}

pub fn write_step_like(mesh: &EditableMesh, name: &str, path: &Path) -> std::io::Result<()> {
    std::fs::write(path, export_step_like(mesh, name))
}
