# Gap Analysis: PolyForge CadMode vs FreeCAD

**Status:** Phase 3 **in progress** — C0 shell done; C1–C3 MVP mesh-parametric solids in `polyforge-cad`. Full B-rep still planned.

**Goal:** when the user switches to **CadMode**, PolyForge should offer a FreeCAD-class parametric / B-rep oriented workflow, not a mesh-only editor with CAD cosmetics.

Legend: ✅ present · 🟡 partial · ❌ missing · 🔮 planned (post Phase 1–2)

---

## 0. Product shell (UI chrome) — shared across modes

These controls live on the **window chrome**, outside mode-specific toolbars.

| Element | Placement | Behaviour | Status |
|---------|-----------|-----------|--------|
| **Settings** | Window border, before main controls | Opens global preferences (theme, language, units, shortcuts, Bevy/renderer, paths) | 🔮 |
| **?** (Help) | Next to Settings | About, docs links, hotkey cheat-sheet, version / Bevy pin | 🔮 |
| **Mode selector** | **Left edge** | Exclusive modes: **PolyMode** · **CadMode** · **VectorMode** | 🔮 |

### Mode model

| Mode | Primary domain | Reference DCC | Data core |
|------|----------------|---------------|-----------|
| **PolyMode** | Polygon mesh Edit Mode | Blender | `EditableMesh` (Phase 1–2) |
| **CadMode** | Parametric solid / sketch / assembly | FreeCAD | B-rep + history graph (this doc) |
| **VectorMode** | 2D curves, paths, illustration-style vectors | Inkscape / SVG tools | Curves, fills, booleans 2D (future gap doc) |

Rules:

1. Mode switch **must not destroy** the other modes’ data; documents can hold poly + CAD + vector objects in one scene.
2. Settings / Help remain available in every mode.
3. Hotkeys that conflict across modes are **mode-scoped** (CadMode shortcuts active only when CadMode is selected).
4. Bevy remains the viewport engine for all modes; CAD geometry is tessellated for display.

---

## 1. FreeCAD workbench map → CadMode modules

| FreeCAD area | Role | PolyForge CadMode target | Priority |
|--------------|------|--------------------------|----------|
| Part | Primitive solids, booleans | Solid primitives + boolean ops | P0 |
| Part Design | Body, pad/pocket, features | Feature history on Body | P0 |
| Sketcher | 2D constrained sketches | Sketch entity + solver | P0 |
| Draft | Construction geometry, snaps | Draft tools + CAD snaps | P1 |
| Arch / BIM | Building objects | Optional later | P3 |
| Assembly (A2+/internal) | Constraints between parts | Simple assembly constraints | P2 |
| TechDraw | 2D drawing views | Deferred (VectorMode adjacency) | P3 |
| FEM / Path / Mesh | Simulation / CAM / mesh | Out of CadMode core | — |
| Spreadsheet | Parametric tables | Expression-driven dims | P2 |
| Python macros | Automation | Rust command / macro API (existing) | P1 |

---

## 2. Document model & parametric history

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Document / project file | ✅ FCStd | 🔮 CadDocument in project JSON / dedicated format | P0 |
| Body / Part container | ✅ | 🔮 `CadBody` component | P0 |
| Feature tree (history) | ✅ | 🔮 ordered feature stack + recompute | P0 |
| Recompute / dependency graph | ✅ | 🔮 dirty propagation | P0 |
| Expressions / parameters | ✅ | 🔮 named parameters + units | P1 |
| Spreadsheet-driven dims | ✅ | 🔮 | P2 |
| Rollback / edit earlier feature | ✅ | 🔮 | P0 |
| Multi-body part | ✅ | 🔮 | P1 |

---

## 3. Sketcher

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Sketch plane (XY / face / custom) | ✅ | 🔮 | P0 |
| Line / arc / circle / polyline | ✅ | 🔮 | P0 |
| Rectangle / slot / polygon | ✅ | 🔮 | P1 |
| Construction geometry | ✅ | 🔮 | P0 |
| External geometry reference | ✅ | 🔮 | P1 |
| Constraints: coincident, horizontal/vertical | ✅ | 🔮 | P0 |
| Constraints: parallel, perpendicular, equal | ✅ | 🔮 | P0 |
| Constraints: distance, radius, angle | ✅ | 🔮 | P0 |
| Constraints: symmetric, tangent | ✅ | 🔮 | P1 |
| Solver (DOF display) | ✅ | 🔮 geometric constraint solver | P0 |
| Fully constrained indicator | ✅ | 🔮 | P0 |
| Carbon copy / clone sketch | ✅ | 🔮 | P2 |

---

## 4. Part Design features

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Pad / Extrude from sketch | ✅ | 🔮 | P0 |
| Pocket / Cut | ✅ | 🔮 | P0 |
| Revolution | ✅ | 🔮 | P0 |
| Groove | ✅ | 🔮 | P1 |
| Fillet / Chamfer (solid edges) | ✅ | 🔮 | P0 |
| Draft angle | ✅ | 🔮 | P2 |
| Hole (counterbore / countersink) | ✅ | 🔮 | P1 |
| Pattern linear / polar | ✅ | 🔮 | P1 |
| Mirror feature | ✅ | 🔮 | P1 |
| Boolean Part Design | ✅ | 🔮 | P1 |
| Additive / subtractive primitives | ✅ | 🔮 | P1 |
| Thickness / shell | ✅ | 🔮 | P2 |

---

## 5. Part (CSG / boolean) primitives

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Box / Cylinder / Sphere / Cone / Torus | ✅ | 🔮 solids (not only mesh prims) | P0 |
| Boolean Union / Cut / Intersection | ✅ | 🔮 robust B-rep boolean | P0 |
| Section / slice | ✅ | 🔮 | P2 |
| Extrude / revolve face | ✅ | 🔮 | P1 |
| Loft / sweep | ✅ | 🔮 | P2 |
| Helix | ✅ | 🔮 | P2 |

---

## 6. Topology kernel & representation

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| B-rep (OpenCASCADE) | ✅ OCCT | 🔮 OCCT binding **or** pure-Rust kernel | P0 |
| Tessellation for viewport | ✅ | 🔮 mesh cache for Bevy | P0 |
| NURBS surfaces | ✅ | 🔮 | P2 |
| Exact circular arcs | ✅ | 🔮 | P0 |
| Shape healing / fix | ✅ | 🔮 | P2 |
| Import STEP / IGES | ✅ | 🔮 | P0 |
| Export STEP | ✅ | 🔮 | P0 |
| Import/export BREP | ✅ | 🔮 | P1 |
| Mesh import to solid (approx) | 🟡 | 🔮 optional | P3 |

**Decision note:** CadMode **requires** a real solid kernel. Mesh boolean approximations are not FreeCAD parity. Evaluate `opencascade-rs` / OCCT vs long-term pure Rust (e.g. truck) before implementation.

---

## 7. Draft, construction & snaps

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Work plane / grid | ✅ | 🔮 | P0 |
| Endpoint / midpoint / center snap | ✅ | 🔮 | P0 |
| Intersection / perpendicular snap | ✅ | 🔮 | P1 |
| Ortho / polar tracking | ✅ | 🔮 | P1 |
| Dimensions (linear / angular / radius) | ✅ | 🔮 | P1 |
| Construction lines | ✅ | 🔮 | P0 |
| Move / rotate / clone (Draft) | ✅ | 🔮 | P1 |

---

## 8. Assemblies

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Multi-part document | ✅ | 🔮 | P1 |
| Mate / constraint (coincident, offset, angle) | ✅ | 🔮 | P2 |
| Exploded view | 🟡 | 🔮 | P3 |
| BOM | 🟡 | 🔮 | P3 |

---

## 9. Selection, tree & UX (CadMode-specific)

| Feature | FreeCAD | PolyForge CadMode | Priority |
|---------|---------|-------------------|----------|
| Model tree (document outline) | ✅ | 🔮 Cad tree panel (extend Outliner) | P0 |
| Select face / edge / vertex (B-rep) | ✅ | 🔮 | P0 |
| Select feature in tree → highlight | ✅ | 🔮 | P0 |
| Hide / show body | ✅ | 🔮 (reuse collection visibility) | P1 |
| Edit mode sketch vs body | ✅ | 🔮 | P0 |
| Task panel for feature params | ✅ | 🔮 properties / task UI | P0 |
| Units (mm, inch, …) | ✅ | 🔮 global + display | P0 |

---

## 10. Overlap with existing PolyMode (reuse)

| PolyForge today | Reuse in CadMode |
|-----------------|------------------|
| Bevy viewport, orbit camera | Display tessellated solids |
| Outliner / collections | Base for model tree |
| MaterialTable | Appearance of solids |
| Undo / History | Extend to feature recompute journal |
| Plugin CommandRegistry / macros | Cad commands registration |
| i18n | CadMode strings |
| Project save/load | Extend schema for CadDocument |
| Settings / Help chrome | Shared (Section 0) |

**Do not** treat mesh extrude/bevel as substitutes for Pad/Fillet on B-rep.

---

## 11. Suggested CadMode implementation phases (after Phase 2)

| Stage | Deliverable |
|-------|-------------|
| **C0 — Shell** | Mode selector (Poly / CAD / Vector); Settings + Help buttons; mode-scoped UI | ✅ Phase 2–3 |
| **C1 — Kernel** | Solid kernel choice + `CadBody` + tessellation → Bevy mesh |
| **C2 — Sketcher MVP** | Plane sketch, lines/arcs, basic constraints + solver |
| **C3 — Features MVP** | Pad, Pocket, Fillet; feature tree + recompute |
| **C4 — Booleans & IO** | CSG booleans; STEP import/export |
| **C5 — Draft & params** | Snaps, dimensions, named parameters |
| **C6 — Assembly** | Multi-body mates (basic) |

---

## 12. Non-goals (CadMode)

- Full FreeCAD Python workbench compatibility  
- FEM, CAM Path, BIM/Arch completeness  
- Replacing PolyMode mesh tools  
- VectorMode feature set (separate gap analysis later)

---

## 13. Tracking

- Activate this document only when Phase 1–2 sign-off is done (`PHASE1_BLENDER_PARITY.md`, `PHASE2_ROADMAP.md`).  
- UI chrome (Settings, Help, mode selector) may be scaffolded earlier in Phase 2 **without** implementing Cad kernel.  
- Kernel decision (OCCT vs pure Rust) must be recorded in a short ADR under `docs/` before C1 coding.
