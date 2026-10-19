# UI / UX chrome

## Titlebar layout (always)

```
[ Poly | CAD | Paint | Vector ]  File Edit View  …  [mode label]  [ ? ]  [ ⚙ ]   ⟨OS window controls⟩
────────────────────────────────────────────────────────────────────────────────
 RIBBON (always visible — content depends on mode)
```

| Control | Position |
|---------|----------|
| Mode selector | Titlebar, **before** menus |
| File / Edit / View | Titlebar (compact) |
| **Ribbon** | Full-width strip **under** titlebar, always on |
| **?** and **⚙** | Titlebar trailing edge, **before** OS min/max/close |

## Mode ribbons

| Mode | Ribbon content |
|------|----------------|
| **Poly** | Mesh ops hints (Blender parity) |
| **CAD** | Box / Pad / Sketch / Solve / planes (CadCraft) |
| **Paint** | Brush pipeline + Open PhotoCraft |
| **Vector** | Rect / Pen / Pathfinder / Auto G1 + Open VectorCraft |

## Mode icons (SVG)

Source files in `assets/icons/`:

| Mode | File | Motif |
|------|------|--------|
| Poly | `mode_poly.svg` | cube wireframe |
| CAD | `mode_cad.svg` | set square + arc |
| Paint | `mode_paint.svg` | brush |
| Vector | `mode_vector.svg` | Bézier pen |

The titlebar paints matching vector paths via egui (no rasterization required at runtime).
