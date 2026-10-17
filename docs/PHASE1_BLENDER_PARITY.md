# Phase 1: Full Blender modeling parity

**Policy (binding):** Phase 1 is a **complete Blender** for polygon mesh modeling
(Edit Mode and related mesh workflow). A reduced “game-asset subset” is **not** the goal.

PolyForge may differ in architecture (Bevy, Rust, GPL-3.0) and UI chrome, but
**feature coverage for mesh modeling must match Blender**, not stop at partial tools.

## Scope of Phase 1

In scope (must reach parity):

- All Edit Mode selection modes and tools (box, circle, lasso, loops, rings, similar, linked, …)
- Full transform pipeline (grab/rotate/scale, axes, snap to grid/vert/edge/face, proportional, individual origins)
- Complete mesh operators (extrude, inset, bevel, loop cut, knife interactive, bridge, fill, merge, dissolve, delete, duplicate, separate/join, solidify, triangulate, subdivide, rip, slide, spin, screw, …)
- Topology tools and modifiers workflow (mirror, array, solidify, and expandable stack with live-friendly apply)
- Multi-object scene, collections, hierarchy / outliner
- UV editing (seams, unwrap, UV editor interaction)
- Materials assignment (slots, basic PBR)
- IO: OBJ, glTF import/export, project save/load
- Undo/redo, consistent hotkeys aligned with Blender where possible

Out of scope **for later phases** (not an excuse to skip Phase 1 mesh tools):

- Full animation, compositing, geometry nodes, cycles-equivalent renderer
- Python API (Rust command/plugin API is the Phase 1 substitute; expand toward scriptability)
- Every Blender UI panel pixel-perfect

## Working rules

1. **No “intential limits” on mesh modeling** while Phase 1 is open.
2. Gap analysis rows marked partial/missing are **backlog, not accepted cuts**.
3. Prefer Blender-compatible behavior and hotkeys when adding tools.
4. Bevy version upgrades remain priority #1 for the engine layer; parity features adapt to Bevy, not the reverse.

## Tracking

See `docs/GAP_ANALYSIS_BLENDER.md`. Phase 1 mesh Edit Mode scope is feature-complete; polish tracked in Phase 2.
