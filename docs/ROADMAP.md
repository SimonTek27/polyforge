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
| Live composited canvas preview (egui texture, rebuilt on `revision`) | done |
| Paint texture bound to active material in Paint mode (restored on exit) | done |
| "Box project UVs" button so the active mesh is paintable | done |
| Viewport raycast -> hit face -> barycentric UV -> stroke stamping | done |
| `polyforge-bridge`: VectorCraft / PhotoCraft launch + vendor pin check | done |
| Vendor trees populated | todo (`./scripts/sync_artcraft.sh` with network) |

---

## Mode map

| Mode | State |
|------|--------|
| **PolyMode** | Mesh tools production-oriented |
| **CadMode** | Parametric **mesh** kernel + sketcher + LSQ constraints + assembly UI; B-rep ⏸️ |
| **VectorMode** | Paths, SVG, Martinez boolean, Bezier G1 |
| **PaintMode** | Raster layers, brush engine, UV-mapped viewport painting, PNG output |
| **PrintMode** | Mesh slicer, supports, layer preview |

---

*Last updated: UI pass — Blender / CadCraft / VectorCraft chrome* — 2026-10-18 — remaining work is OCCT (deferred) or external (git credentials / future Bevy)*
