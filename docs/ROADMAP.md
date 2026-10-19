# PolyForge Roadmap

Single source of truth for phased work. Phase history was consolidated here; individual PHASE*.md files were removed.

Legend: ✅ done · 🟡 partial · ⬜ todo · ⏸️ deferred (needs OCCT / external kernel / user credentials)

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

## Phase 33 — 3D print module | ✅

| Item | Status |
|------|--------|
| `polyforge-print` crate (types, profiles, geometry, slicer, G-code) | ✅ |
| 9 built-in printer profiles (Prusa, Bambu, Creality, Voron, generic) | ✅ |
| `mesh_source` bridge: `EditableMesh` → slicer triangles, Y-up → Z-up | ✅ |
| Bed placement (centre XY, drop to Z=0) + scale in mm/unit | ✅ |
| Bevy plugin (`PrintState`) + **3D Print** panel (printer, quality, temps, adhesion) | ✅ |
| Slice → stats → **Export G-code** to `artifacts/` | ✅ |
| 18 tests (14 unit + 3 integration + 1 doc) | ✅ |

### Phase 33b — Slicer depth | ✅

| Item | Status |
|------|--------|
| Overhang support generation (raster occupancy + angle reach, propagated to bed) | ✅ |
| Support moves emitted as `--- Support ---` at `support_speed` | ✅ |
| Multi-layer top/bottom solid shells (`top_solid_layers` / `bottom_solid_layers`) | ✅ |
| Support controls in the 3D Print panel (enable + overhang angle) | ✅ |
| Support time/filament accounted in estimates | ✅ |
| 21 tests (17 unit + 4 integration + 1 doc) | ✅ |

---

## Backlog that needs external intervention

| Item | Status | Why blocked |
|------|--------|-------------|
| OCCT / `cad_kernel` real linkage | ⏸️ | Heavy native dep, packaging, license/build on target OS — needs explicit integration project |
| ISO STEP read/write via OCCT | ⏸️ | Depends on kernel |
| Full `git push` of entire source tree | 🟡 | Sandbox lacks GitHub credentials — run `git push -u origin main` locally |
| Bevy 0.21+ | ⬜ | Not released yet (track crates.io; scripts ready) |

---

## Mode map

| Mode | State |
|------|--------|
| **PolyMode** | Mesh tools production-oriented |
| **CadMode** | Parametric **mesh** kernel + sketcher + LSQ constraints + assembly UI; B-rep ⏸️ |
| **VectorMode** | Paths, SVG, Martinez boolean, Bezier G1 |

---

*Last updated: Phase 32 — 2026-10-18 — remaining work is OCCT (deferred) or external (git credentials / future Bevy)*
