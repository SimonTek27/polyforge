# Gap Analysis: PolyForge vs Blender (Edit Mode / Mesh Modeling)

Scope: **polygon mesh modeling** in Blender Edit Mode vs current PolyForge.

**Phase 1 policy:** full Blender modeling parity is the goal (see `docs/PHASE1_BLENDER_PARITY.md`).
Partial tools are backlog items, not accepted product cuts.

Legend: ✅ present · 🟡 partial · ❌ missing · 🔮 planned

---

## 1. Selection

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| Vertex / Edge / Face modes | ✅ 1/2/3 | ✅ 1/2/3 | — |
| Object mode | ✅ Tab | ✅ Tab toggle / 5 | — |
| Cell / linked face regions | 🟡 L select linked | ✅ Cell mode + L | — |
| Box (marquee) select | ✅ B | ✅ drag | — |
| Circle select | ✅ C | ✅ C + paint | — |
| Lasso select | ✅ | ✅ Ctrl+L | — |
| Edge loop (Alt+click) | ✅ | ✅ Alt+click | — |
| Edge ring | ✅ Ctrl+Alt+click | ✅ Alt+R | — |
| Select more/less | ✅ Ctrl+Num± | ✅ +/- faces | P1 |
| Select similar | ✅ Shift+G | ✅ Shift+G (faces) | — |
| Invert selection | ✅ Ctrl+I | ✅ Ctrl+I | — |

## 2. Transform

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| Grab / Rotate / Scale | ✅ G R S | ✅ G R S | — |
| Axis constraint | ✅ X Y Z | ✅ gizmo axes | — |
| Free drag (view plane) | ✅ | ✅ | — |
| Snap to grid | ✅ | ✅ optional | — |
| Snap to vertex/edge/face | ✅ | ✅ vert/edge/face/grid | — |
| Proportional editing | ✅ O | ✅ O toggle | — |
| Individual origins | ✅ | ✅ Ctrl+Alt+S | — |

## 3. Core mesh operators

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| Extrude | ✅ E | ✅ E faces/edges | — |
| Inset | ✅ I | ✅ I | — |
| Bevel | ✅ Ctrl+B | 🟡 adding | P0 |
| Loop cut | ✅ Ctrl+R | 🟡 adding | P0 |
| Knife | ✅ K | ✅ K (X mid-plane) | — |
| Bridge edge loops | ✅ | ✅ Ctrl+Shift+B | — |
| Fill / Make face | ✅ F | 🟡 adding | P0 |
| Merge (at center / distance) | ✅ M | 🟡 adding | P0 |
| Dissolve | ✅ | ✅ Ctrl+Delete | — |
| Delete / limited dissolve | ✅ X | ✅ X | — |
| Duplicate | ✅ Shift+D | 🟡 adding | P0 |
| Separate / Join | ✅ | ✅ P / Ctrl+J | — |
| Solidify | ✅ | ✅ Ctrl+Shift+F | — |
| Triangulate | ✅ Ctrl+T | ✅ Ctrl+T | — |
| Subdivide | ✅ | ✅ U | — |

## 4. Topology tools

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| Edge split | ✅ | ✅ Y (midpoint) | — |
| Rip / Rip fill | ✅ V | ✅ V / Shift+V | — |
| Slide edge/vertex | ✅ GG | ✅ Alt+G | — |
| Spin / Screw | ✅ | ✅ Shift+W spin | — |
| Symmetrize | ✅ | ✅ Ctrl+Shift+X | — |
| Mirror modifier workflow | ✅ | ✅ Ctrl+Shift+M / Ctrl+Alt+M | — |

## 5. Primitives & scene

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| Cube | ✅ | ✅ | — |
| UV Sphere | ✅ | ✅ code only | P1 UI |
| Plane / Cylinder / Cone / Torus | ✅ | ✅ + torus Shift+T | — |
| Multi-object scene | ✅ | ✅ Ctrl+Tab cycle, P separate | — |
| Collections / hierarchy | ✅ | ✅ Collection + Outliner | — |

## 6. UV / Materials / IO

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| UV editor | ✅ | ✅ interactive drag | — |
| Seam mark | ✅ | ✅ Ctrl+E toggle | — |
| Unwrap (angle/conformal) | ✅ | 🟡 planar/box/smart | P1 |
| PBR materials | ✅ | ✅ slots + Materials UI | — |
| glTF import/export | ✅ | ✅ export/import | — |
| OBJ import/export | ✅ | ✅ export Ctrl+Shift+E | P1 import |

## 7. Infrastructure

| Feature | Blender | PolyForge | Priority |
|---------|---------|-----------|----------|
| Undo / Redo | ✅ | ✅ Ctrl+Z / Ctrl+Y | — |
| Save / Load project | ✅ | ✅ Ctrl+S / Ctrl+O (JSON) | — |
| Modifiers (non-destructive) | ✅ | ✅ live GPU eval + apply | — |
| Scripting API | ✅ Python | 🟡 Rust CommandRegistry | P2 |
| Add-ons / plugins | ✅ | 🟡 command plugins | P2 |
| Cross-platform | ✅ | ✅ Bevy | — |
| Open source license | ✅ GPL | ✅ GPL-3.0 | — |

---

## Priority roadmap (what we implement next)

### P0 (immediate – this iteration)
1. **Loop cut** – insert edge loop across quad strips  
2. **Bevel** – chamfer selected edges  
3. **Merge by distance** – weld nearby vertices  
4. **Fill (F)** – create face from boundary  
5. **Duplicate** – copy selection  
6. **Edge loop select** – Alt+click edge  
7. **More primitives** – plane, cylinder  
8. **Basic undo** – command stack  

### P1 (next milestones)
- glTF/OBJ IO, multi-object, edge extrude, bridge, subdivide, triangulate, UV seams, snap to mesh

### P2+
- Knife, proportional edit, modifiers, scripting

---

## Design stance

PolyForge is **not** a Blender clone. Goals:

- Fast low/mid-poly workflow for game assets  
- Rust + Bevy, GPL-3.0, extensible ECS architecture  
- Blender-familiar hotkeys where they make sense (G/R/S, E, I, 1/2/3) without copying the entire paradigm  

Gaps above guide prioritization, not feature parity for its own sake.



---

## Phase 1 status

Policy: **complete Blender mesh modeling** before declaring Phase 1 done.
See `docs/PHASE1_BLENDER_PARITY.md`.

Progress this iteration:
- Geometric snap vert/edge/face/grid (Ctrl+Backquote cycle, Shift+S apply)
- Live modifier GPU evaluation
- UV editor drag + select
- Materials multi-slot UI + assign
- Rip fill (Shift+V)
- Edge ring (Ctrl+Alt+click)

Closed this iteration:
- Knife freehand stroke (Ctrl+K, drag, multi-segment cut)
- Collection visibility (Shift+H toggle, Visibility enforcement)
- UV island pack / average / select island
- Hotkey parity table (`docs/HOTKEY_PARITY_BLENDER.md`)
- Expanded plugins (F8 list, F9 clear sel, F10 recalc normals)

Closed this iteration (polish):
- UV follow-active-quads + unwrap smart (UV Editor)
- Tab = Edit ↔ Object mode; Ctrl+Tab = cycle object
- Subdivide on W (and U)
- ObjectParent + ObjectLocal hierarchy transforms
- MacroRegistry + F7 run `prep.export` batch
- Alt+V rip fill (Blender parity)

Phase 1 mesh-modeling core is feature-complete for editor parity.
Optional follow-ups (not blocking Phase 1 close unless product requires):
- Geometry Nodes / modifiers beyond mirror-array-solidify
- Python scripting (Rust macros remain the substitute)
- Pixel-perfect Blender UI

Phase 1 may be closed for mesh Edit Mode scope when the team signs off on HOTKEY_PARITY and GAP tables.
