# Phase 2 roadmap

Phase 1 delivered Blender-level **mesh Edit Mode** coverage on Bevy.

## Phase 2 goals

1. **Engine hygiene**
   - Keep Bevy on latest (`bevy-compat.toml` + CI)
   - `cargo check` / fix API drift after every Bevy bump
   - Split `polyforge-app` systems into modules (`input`, `gizmo`, `gpu`, `scene`)

2. **Viewport & UX**
   - Object mode pick / transform whole objects
   - Outliner activate object on click
   - Status bar (mode, snap, proportional, counts)
   - Contextual menus (W / right-click)

3. **Modeling depth**
   - Knife freehand improve (edge-constrained)
   - Live modifier gizmos / disable stack items in UI
   - Better merge menu (at center / at first / collapse)

4. **UV & materials**
   - Follow-active-quads polish + pack island UI
   - Per-slot StandardMaterial on GPU from MaterialTable

5. **IO & pipeline**
   - glTF import materials
   - Multi-object project save/load
   - Auto-save

6. **Extensibility**
   - Document plugin command API
   - Example external crate registering a command

## Immediate next tasks

- [x] Status bar in UI (started)
- [ ] Modularize `polyforge-app/src/main.rs`
- [ ] Object-mode transform
- [ ] MaterialTable → GPU materials
- [ ] Full `git push` when credentials available
