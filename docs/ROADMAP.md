# PolyForge Roadmap

Single source of truth for phased work. Phase history was consolidated here; individual PHASE*.md files were removed.

Legend: ✅ done · 🟡 partial · ⬜ todo · ⏸️ deferred (needs OCCT / external kernel / user credentials)

---

## UX targets (product design)

| Mode | Reference UX | UI cues in PolyForge |
|------|----------------|----------------------|
| **Poly** | **Blender** Edit Mode | Tool shelf, N-panel properties, V/E/F selection, mesh ops hotkeys |
| **CAD** | **CadCraft** | Command line, 3D ribbon (Box/Pad/Sketch), feature/model tree, parametric solve |
| **Vector** | **VectorCraft** | Task bar (Pen/Pathfinder), Layers & Appearance panel, SVG I/O, booleans |

---

## ArtCraft upstream

| Item | Status |
|------|--------|
| `vendor/pins.toml` + `scripts/sync_artcraft.sh` | ✅ |
| CI `artcraft-upstream-check` | ✅ |
| `polyforge-bridge` launch + pin load | ✅ |
| Vendor trees populated | ⬜ run `./scripts/sync_artcraft.sh` on a machine with network/git |
| Deep in-process embed of VC/PC | ⏸️ out of scope (separate processes) |

---

## Standing rules

| Rule | Status |
|------|--------|
| Bevy updates are always priority #1 | ✅ policy (`docs/BEVY_UPDATE.md`) — currently **0.20.0** = latest |
| Phase 1 = full Blender mesh modeling parity | ✅ |
| No ZModeler references | ✅ |
| GPL-3.0 | ✅ |

---

## Phase 1–2 — PolyMode + UX shell | ✅

Mesh edit parity, gizmos, selection, modifiers, UV, materials, outliner, mode selector, settings/help.

---

## Phase 3–7 — CadMode foundation + sketcher + scene | ✅ (B-rep ⏸️)

CadBody, features, tessellate, sketcher, constraints, scene v2, STEP-like stub, multi-mesh.

| Deferred | Status |
|----------|--------|
| Real B-rep (OCCT) | ⏸️ |
| ISO STEP I/O via kernel | ⏸️ |
| Pocket true solid boolean | ⏸️ |

---

## Phase 8–17 — VectorMode + Bezier G1 | ✅

SVG, boolean, path edit, handles, auto-G1, local joints, snap.

---

## Phase 18–24 — Martinez, holes, assembly | ✅

Martinez boolean, holes, mates, hierarchy tree.

---

## Phase 25–28 — Roadmap, modularize, reparent | ✅

| Item | Status |
|------|--------|
| Unified ROADMAP | ✅ |
| `camera` / `gizmo_sys` / `selection_sys` / `scene_ops` / `hotkeys` | ✅ |
| Assembly two-click reparent + cycle check | ✅ |

---

## Phase 29–31 — STL, constraints, residual | ✅

STL export, Parallel/Coincident, residual solve loop.

---

## Phase 32 — Gauss–Newton LSQ sketch solver | ✅

| Item | Status |
|------|--------|
| Finite-difference Jacobian + damped normal equations | ✅ |
| `solve_sketch_lsq` / `solve_body_sketches_lsq` | ✅ |
| UI button **Solve LSQ (Gauss–Newton)** | ✅ |

---

## Phase 34 - OCCT / B-rep CAD kernel (mandatory, needs native deps)

Goal: replace the mesh-parametric CAD backend with a real boundary-representation
kernel via OpenCASCADE (OCCT), behind a feature flag, without breaking the mesh
fallback.

| Step | Status | Notes |
|------|--------|-------|
| 0. Prereqs on build machine | todo | OCCT 7.8+ dev (vcpkg `opencascade` or official installer), MSVC toolchain, `OCCT_ROOT` env, DLLs on `PATH` |
| 1. `polyforge-occt-sys` shim crate | done | C++ shim (`shim.cpp`) + C ABI (primitives, fuse/cut, fillet, triangulate, STEP read/write); `build.rs` no-op without `occt` feature |
| 2. Kernel trait wiring | partial | `SolidKernel` exists; `OcctKernel` links/initialises OCCT under `--features cad_kernel`, feature mapping still TODO |
| 3. Primitives + booleans | todo | BRepPrimAPI (box/cyl/sphere/cone), BRepAlgoAPI cut/fuse; keep mesh fallback |
| 4. Fillets/chamfers | todo | BRepFilletAPI; replaces `fillet.rs` approximation |
| 5. Tessellation bridge | todo | BRepMesh_IncrementalMesh -> triangles/positions/normals -> `EditableMesh` |
| 6. STEP read/write | todo | STEPControl_Reader/Writer replaces `step_export.rs` stub; units + tolerance |
| 7. UI + CI + licensing | todo | Kernel indicator (`ActiveKernel`), mesh-quality settings; vcpkg CI job; LGPL-2.1 notice in `LICENSE-THIRD-PARTY.md` |

Verification: `cargo build -p polyforge-cad --features cad_kernel`; volume/area
tests within tolerance; STEP round-trip; boolean volume checks.

Current status: crate builds without OCCT by default; enabling `cad_kernel`
fails fast with an actionable `OCCT_ROOT` message when OCCT is absent.

Blocked here: this sandbox has no OCCT installed, so steps 3-7 must be developed
where the native dependency and packaging decisions (static vs dynamic link,
DLL redistribution) can be validated.

---

## Backlog that needs external intervention

| Item | Status | Why blocked |
|------|--------|-------------|
| OCCT / `cad_kernel` real linkage | ⏸️ | Heavy native dep, packaging, license/build on target OS — needs explicit integration project |
| ISO STEP read/write via OCCT | ⏸️ | Depends on kernel |
| Full `git push` of entire source tree | 🟡 | Sandbox lacks GitHub credentials — run `git push -u origin main` locally |
| Bevy 0.21+ | ⬜ | Not released yet (track crates.io; scripts ready) |

---

## Phase 33 - PaintMode + ArtCraft bridge | done (vendor sync pending)

| Item | Status |
|------|--------|
| `polyforge-paint` crate: layers, brush engine, PNG I/O, UV projection | done |
| Paint panel: tools, brush settings, blend mode, layers, save PNG | done |
| PBR channels: Base / Roughness / Metalness / Normal / Emissive, per-channel layers | done |
| ORM packing (R=AO, G=roughness, B=metalness) + normal/emissive export | done |
| Material binding: base color, metallic-roughness, normal map, emissive | done |
| Live composited canvas preview (egui texture, rebuilt on `revision`) | done |
| Paint texture bound to active material in Paint mode (restored on exit) | done |
| "Box project UVs" button so the active mesh is paintable | done |
| Viewport raycast -> hit face -> barycentric UV -> stroke stamping | done |
| Layer masks: per-layer mask, mask-paint mode, add/fill/clear + compositing | done |
| Layer stack ops: duplicate, rename, visibility, opacity, blend, reorder — undoable | done |
| Layer merge down (bakes opacity/blend/mask) + flatten, undoable | done |
| Brush shapes (circle / square / diamond) with soft falloff, persisted in `.pfp` | done |
| Clone tool: Alt+click source picking (offset from first dab) | done |
| Undo/redo: bounded history, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y, dedupe on drags | done |
| Flood fill tool with colour tolerance, fill-region API | done |
| X/Y symmetry painting (mirrored dabs and segments) | done |
| Alt+click colour picker sampling the composited channel | done |
| Project persistence `.pfp` (encode/decode/save/load, all channels + masks) | done |
| glTF 2.0 PBR export with embedded base/ORM/normal/emissive maps | done |
| `polyforge-bridge`: VectorCraft / PhotoCraft launch + vendor pin check | done |
| Vendor trees populated | todo (`./scripts/sync_artcraft.sh` with network) |

---

## Mode map

| Mode | State |
|------|--------|
| **PolyMode** | Mesh tools production-oriented |
| **CadMode** | Parametric **mesh** kernel + sketcher + LSQ constraints + assembly UI; B-rep ⏸️ |
| **VectorMode** | Paths, SVG, Martinez boolean, Bezier G1 |
| **PaintMode** | Raster layers, brush engine, UV-mapped viewport painting, masks, undo, flood fill, symmetry, PNG output |
| **PaintMode PBR** | Base/Rough/Metal/Normal/Emit channels, ORM + normal + glTF export, material binding, `.pfp` projects |
| **PrintMode** | Mesh slicer, supports, layer preview |

---

*Last updated: Paint Phase 33 — masks, undo/redo, flood fill, symmetry, picker, `.pfp` persistence, glTF PBR export* — 2026-10-21 — remaining work needs OCCT installed (native validation) or is external (git credentials / future Bevy)*
