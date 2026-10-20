//! Builds the OCCT C ABI shim when the `occt` feature is enabled.
//!
//! Without the feature this script is a no-op, so the crate (and the whole
//! workspace) builds on machines without OpenCASCADE installed.
//!
//! Required when `OCCT_ROOT` points at an OCCT 7.8+ installation:
//! - headers: `include/opencascade` (official) or `include` (vcpkg)
//! - libs: `lib`, `win64/vc14/lib` or `win64/vc14/bin`
//! - DLLs must be on `PATH` at runtime (Windows).

use std::env;
use std::path::PathBuf;

const OCCT_LIBS: &[&str] = &[
    "TKernel",
    "TKMath",
    "TKG2d",
    "TKG3d",
    "TKGeomBase",
    "TKBRep",
    "TKTopAlgo",
    "TKPrim",
    "TKBO",
    "TKFillet",
    "TKMesh",
    "TKSTEP",
    "TKXSBase",
];

fn main() {
    println!("cargo:rerun-if-changed=src/shim.cpp");
    println!("cargo:rerun-if-env-changed=OCCT_ROOT");

    if env::var_os("CARGO_FEATURE_OCCT").is_none() {
        return;
    }

    let root = env::var("OCCT_ROOT").unwrap_or_else(|_| {
        panic!(
            "OCCT_ROOT is not set. Point it at an OCCT 7.8+ installation \
             (vcpkg: `vcpkg install opencascade`, then OCCT_ROOT=<vcpkg>/installed/x64-windows)."
        )
    });
    let root = PathBuf::from(root);
    if !root.is_dir() {
        panic!("OCCT_ROOT does not exist: {}", root.display());
    }

    let mut include_dirs = Vec::new();
    for candidate in [root.join("include/opencascade"), root.join("include")] {
        if candidate.is_dir() {
            include_dirs.push(candidate);
        }
    }
    if include_dirs.is_empty() {
        panic!(
            "OCCT headers not found under {} (expected include/opencascade or include)",
            root.display()
        );
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("src/shim.cpp")
        .warnings(false)
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-std=c++17");
    for dir in &include_dirs {
        build.include(dir);
    }
    build.compile("polyforge_occt_shim");

    let lib_dirs = [
        root.join("lib"),
        root.join("win64/vc14/lib"),
        root.join("win64/vc14/bin"),
    ];
    let mut linked_dirs = 0;
    for dir in &lib_dirs {
        if dir.is_dir() {
            println!("cargo:rustc-link-search=native={}", dir.display());
            linked_dirs += 1;
        }
    }
    if linked_dirs == 0 {
        panic!(
            "OCCT libraries not found under {} (looked in lib, win64/vc14/lib, win64/vc14/bin)",
            root.display()
        );
    }

    for lib in OCCT_LIBS {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
}
