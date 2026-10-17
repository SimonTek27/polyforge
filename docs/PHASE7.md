# Phase 7 — Scene load, user constraints, SVG path `d`

## Delivered

### Scene project
| Action | Hotkey |
|--------|--------|
| Save scene v2 | Ctrl+Shift+S |
| **Load scene v2** | **Ctrl+Shift+L** |
| Auto-save | Settings (120s) |

Load restores: active mesh, CAD bodies (features JSON → tessellate), vector paths.

### Sketch constraints (user-defined)
| Constraint | UI |
|------------|-----|
| Horizontal | Add H |
| Vertical | Add V |
| Equal length | Equal len |
| Fixed length | Fixed len 1.0 on #0 |
| Solve | Solve constraints |

Stored on `CadFeature::Sketch.constraints`.

### SVG
- Parse `path d="..."` with **M/L/H/V/C/Z** (absolute + relative)
- Cubics → `BezierSegment`

## Still later
- OCCT / STEP
- Full DOF with drag-driven solver
- Scene load without duplicating entities
- Constraint on selected curve index (not only #0)
