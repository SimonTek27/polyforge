# Building the OCCT CAD kernel

PolyForge ships a mesh-parametric CAD backend by default. The real
boundary-representation (B-rep) kernel is OpenCASCADE (OCCT), enabled with the
`cad_kernel` feature. It is optional so that the normal build stays free of
heavy native dependencies.

## 1. Install OCCT 7.8+

### Windows (vcpkg, recommended)

```powershell
vcpkg install opencascade:x64-windows
$env:OCCT_ROOT = "$env:VCPKG_INSTALLATION_ROOT\installed\x64-windows"
$env:PATH = "$env:OCCT_ROOT\bin;$env:PATH"   # runtime DLLs
```

### Windows (official installer)

Install the OCCT binaries, then point `OCCT_ROOT` at the installation directory
(the one containing `inc/` and `win64/vc14/`; the build script also accepts
`include/` + `lib/`):

```powershell
$env:OCCT_ROOT = "C:\OpenCASCADE-7.8.0-vc14-64\opencascade-7.8.0"
```

### Linux / macOS

Use the distro package (`libocct-*-dev` on Debian/Ubuntu) or homebrew
(`brew install opencascade`), then set `OCCT_ROOT` to the prefix that contains
`include/opencascade` and `lib`.

## 2. Verify the installation

```powershell
pwsh -File scripts/check_occt.ps1
```

The script resolves `OCCT_ROOT`, checks for `Standard_Version.hxx` and
`TKernel`, and prints the build commands. If it fails, the reason is reported
with the paths it probed.

## 3. Build and test

```powershell
cargo check -p polyforge-cad --features cad_kernel
cargo test  -p polyforge-cad --features cad_kernel   # includes tests/occt_kernel.rs
```

Without `OCCT_ROOT` the build script stops with an actionable message instead of
failing deep inside the compiler.

To run the full editor with the kernel:

```powershell
cargo run -p polyforge-app --features polyforge-cad/cad_kernel
```

In CAD mode the ribbon exposes a **Kernel** selector (Mesh / OCCT); switching it
marks every body dirty so it is re-evaluated with the new backend. The status
bar shows the active kernel. Next to it, **Quality** sets the tessellation
deflection (`KernelSettings::deflection`, 0.005-1.0, default 0.1); changing it
also marks bodies dirty.

## 4. What the feature enables

| Area | Behaviour with `cad_kernel` |
|------|-----------------------------|
| Evaluation | `evaluate_shape` builds primitives, Pad extrusions, Pocket cuts and Fillet rounds through `BRepAlgoAPI`/`BRepPrimAPI`, then tessellates via `BRepMesh` |
| Quality | `KernelSettings::deflection` (CAD ribbon *Quality*) controls `BRepMesh` linear deflection |
| Fillet | `CadFeature::Fillet` rounds every edge (`BRepFilletAPI_MakeFillet`); the mesh backend keeps the approximate vertex-offset version |
| STEP export | `write_step` writes a real ISO-10303-21 B-rep; falls back to the ASCII mesh stub only if OCCT cannot export |
| STEP import | `read_step` (CAD ribbon: *Import STEP*, file `artifacts/polyforge_import.step`) tessellates a STEP file into an editable mesh |
| Tests | `crates/polyforge-cad/tests/occt_kernel.rs` checks a unit cube, fillet, pocket cut and STEP round-trip; they skip when OCCT cannot initialise |

## 5. Packaging notes

- OCCT is LGPL-2.1 with the OpenCASCADE exception: link it dynamically and ship
  (or require) the OCCT DLLs. See `LICENSE-THIRD-PARTY.md`.
- Ship `TKernel`, `TKMath`, `TKG2d`, `TKG3d`, `TKGeomBase`, `TKBRep`,
  `TKTopAlgo`, `TKPrim`, `TKBO`, `TKFillet`, `TKMesh`, `TKSTEP`, `TKXSBase`
  (the list `crates/polyforge-occt-sys/build.rs` links).
- The CI job `.github/workflows/cad-kernel.yml` installs OCCT through vcpkg on a
  Windows runner and runs the same check/test commands.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `OCCT_ROOT is not set` | Export the variable in the same shell as `cargo` |
| `OCCT_ROOT does not exist: ...` | Path typo; use the directory containing `inc`/`include` |
| Linker errors mentioning `TK*.lib` not found | Point `OCCT_ROOT` at the install prefix, not at `bin` |
| Runtime crash / missing DLL | Prepend `%OCCT_ROOT%\bin` (or `win64\vc14\bin`) to `PATH` |
| `OCCT kernel failed to initialise` in the log | The editor stays on the mesh backend; fix the DLL path and restart |
