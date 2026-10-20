# PolyForge

**Open-source 3D polygon modeling tool** built with the **Bevy** engine.

Licensed under **GPL-3.0-or-later**.

## Vision

PolyForge aims to provide a fast, focused polygon modeling workflow for game assets and general 3D work:

- Clean, modern ECS architecture powered by Bevy
- Fully cross-platform (Windows, Linux, macOS)
- Free and open source forever
- Extensible design ready for plugins and scripting
- Modern features: non-destructive workflows, solid UV tools, glTF-first pipeline

## Blender gap analysis

See [`docs/GAP_ANALYSIS_BLENDER.md`](docs/GAP_ANALYSIS_BLENDER.md) for a feature-by-feature comparison with Blender Edit Mode and the prioritized roadmap.

## Current Status (v0.1.0)

This is the initial working scaffold. It already includes:

- Cargo workspace with clean separation of concerns
- Core mesh representation (`EditableMesh`) with vertices / faces / UVs
- Selection system (vertex / edge / face / object)
- Modeling operations (extrude, translate, rotate, scale, primitives)
- UV projection helpers
- IO stubs (ready for glTF / OBJ and more)
- egui-based editor UI (toolbar + properties)
- **Interactive transform gizmos** (Move / Rotate / Scale)
- **Raycast selection** (vertex / edge / face / object) with visual feedback
- **Area (marquee) selection** by dragging in the viewport
- **Vertex / edge / face editing**: move, delete, extrude, inset, bevel, loop cut, merge, fill, duplicate
- Edge loop select (Alt+click), cell linked regions
- Primitives: cube, plane, cylinder, UV sphere
- Bevy 3D viewport with orbit camera and a default cube
- **i18n** with English locale (extensible catalogs)
- **Paint mode**: PBR channels (base / roughness / metalness / normal / emissive), layer stack (duplicate, rename, opacity, blend mode, per-layer greyscale masks), flood-fill tool with tolerance, X/Y symmetry, `Alt`+click colour picker, undo/redo (`Ctrl+Z` / `Ctrl+Shift+Z`), ORM + PNG + textured glTF export, UV-mapped viewport painting
- **CAD mode**: parametric feature stack with mesh kernel; optional OCCT B-rep kernel behind `cad_kernel` (Pad/Pocket/Fillet, STEP I/O, kernel + quality controls) — see `docs/BUILD_OCCT.md`
- GPL-3.0 license

### Controls

| Action              | Input                          |
|---------------------|--------------------------------|
| Select element      | Left-click                     |
| Area (marquee) select | Left-drag on empty space     |
| Add to selection    | Shift + click / drag           |
| Clear selection     | Click empty space / Escape / Alt+A |
| Select all          | `A`                            |
| Vertex mode         | `1`                            |
| Edge mode           | `2`                            |
| Face mode           | `3`                            |
| Cell mode           | `4` (regioni di facce collegate)|
| Object mode         | `5`                            |
| Linked cells        | `L`                            |
| Grow / shrink cells | `+` / `-`                      |
| Move selection      | `G` then drag (free or on axis) |
| Rotate              | `R` then drag on axis          |
| Scale               | `S` then drag on axis          |
| Delete              | `X` or `Delete`                |
| Extrude faces       | `E` (with faces selected)      |
| Inset faces         | `I` (with faces selected)      |
| Split edges         | `Y` (with edges selected)      |
| Nudge faces normal  | `N` (with faces selected)      |
| Free move           | Move tool + drag in viewport   |
| Snap to grid        | Enable Snap in properties panel |
| Orbit camera        | Right-mouse drag               |
| Undo / Redo         | `Ctrl+Z` / `Ctrl+Y`            |
| Triangulate         | `Ctrl+T`                       |
| Subdivide           | `U`                            |
| Invert selection    | `Ctrl+I`                       |
| Symmetrize X        | `Ctrl+Shift+X`                 |
| Export OBJ          | `Ctrl+Shift+E`                 |
| Bridge loops        | `Ctrl+Shift+B`                 |
| Solidify            | `Ctrl+Shift+F`                 |
| Dissolve            | `Ctrl+Delete`                  |
| Edge ring           | `Alt+R`                        |
| Select similar      | `Shift+G`                      |
| Toggle UV seam      | `Ctrl+E`                       |
| Save / Load project | `Ctrl+S` / `Ctrl+O`            |
| Paint: undo / redo  | `Ctrl+Z` / `Ctrl+Shift+Z`      |
| Paint: pick colour  | `Alt` + click (Paint mode)     |
| Paint: flood fill   | Fill tool + click              |
| Add plane           | `Shift+A`                      |
| Add torus           | `Shift+T`                      |
| Circle select       | `C` then paint LMB             |
| Proportional edit   | `O`                            |
| Separate / Join     | `P` / `Ctrl+J`                 |
| Edit ↔ Object mode  | `Tab`                          |
| Cycle object        | `Ctrl+Tab`                     |
| Subdivide           | `W` / `U`                      |
| Macro prep.export   | `F7`                           |
| Rip                 | `V`                            |
| Slide verts         | `Alt+G`                        |
| Snap to nearest     | `Alt+S`                        |
| Export glTF         | `Ctrl+Shift+G`                 |
| Import glTF         | `Ctrl+Shift+O`                 |
| Lasso select        | `Ctrl+L` then drag             |
| Knife cut           | `K`                            |
| Spin                | `Shift+W`                      |
| Mirror modifier     | `Ctrl+Shift+M` add, `Ctrl+Alt+M` apply |
| Individual origins  | `Ctrl+Alt+S`                   |
| Screw               | `Shift+Q`                      |
| Interactive knife   | `Ctrl+K` stroke drag           |
| UV pack / islands   | UV Editor buttons              |
| Collection hide     | `Shift+H`                      |
| Plugins             | `F8` list, `F9`/`F10` run      |
| Face material slot  | `Ctrl+Shift+1` / `2`           |
| Array modifier      | `Ctrl+Shift+Y` add, `Ctrl+Alt+Y` apply |
| Rip fill            | `Shift+V`                      |
| Snap mode cycle     | `Ctrl+Backquote`               |
| Apply geo snap      | `Shift+S` (snap on)            |
| Edge ring           | `Ctrl+Alt+click` on edge       |
| Outliner / plugins  | UI windows                     |
| Zoom                | Mouse wheel                    |

## Building & Running

Requirements:
- Recent stable Rust (Bevy 0.20 currently requires a recent toolchain)
- A GPU with Vulkan / Metal / DX12 support

```bash
git clone https://github.com/your-org/polyforge.git
cd polyforge
cargo run --release
```

The binary is named `polyforge`.

Optional: the real B-rep CAD kernel (OpenCASCADE) is behind the `cad_kernel`
feature. See [`docs/BUILD_OCCT.md`](docs/BUILD_OCCT.md) and verify your install
with `pwsh -File scripts/check_occt.ps1`; without it the mesh CAD backend is used.

## Bevy updates (priority #1)

PolyForge always tracks the current Bevy release. See [`docs/BEVY_UPDATE.md`](docs/BEVY_UPDATE.md) and `bevy-compat.toml`.

```bash
./scripts/check_bevy_update.sh   # compare pin vs crates.io
./scripts/bump_bevy.sh 0.21      # rewrite version pins when upgrading
```

## Project Structure


```
polyforge/
├── crates/
│   ├── polyforge-core/     # Mesh data, selection, topology, gizmo, editor state
│   ├── polyforge-ops/      # Modeling operations (extrude, transform, primitives…)
│   ├── polyforge-uv/       # UV unwrapping & editing
│   ├── polyforge-io/       # Import / export filters
│   ├── polyforge-ui/       # egui panels & toolbars
│   └── polyforge-app/      # Main binary + Bevy app setup
├── assets/                 # Future textures, icons, etc.
├── LICENSE                 # GPL-3.0
└── Cargo.toml              # Workspace root
```

## Roadmap (high level)

1. Robust selection picking (raycast on vertices / edges / faces)
2. Complete core modeling tools (Inset, Bevel, Bridge, Knife, Loop Cut)
3. Full UV editor (seams, unwrap algorithms, packing)
4. Materials & textures (PBR materials)
5. File formats – robust glTF, OBJ, and additional formats
6. Undo / Redo (command pattern)
7. Plugin / scripting system
8. Non-destructive modifiers

## Contributing

Contributions are welcome.  
Please open issues for discussion before large changes.

Because the project is GPL-3.0, any derivative work must also be released under a compatible license.

---

*Made for the indie game and 3D community.*


See [docs/ROADMAP.md](docs/ROADMAP.md) for status.
