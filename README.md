# PolyForge

Open-source **3D polygon modeling** tool written in Rust, powered by the [Bevy](https://bevyengine.org/) game engine.

Licensed under **GPL-3.0-or-later**.

## Vision

PolyForge aims to be a modern, fast, and extensible alternative for low/mid-poly modeling workflows, with a focus on:

- Clean topology tools (vertices, edges, faces, cells)
- Interactive gizmos and Blender-familiar hotkeys
- GPU-friendly mesh pipeline via Bevy
- Modular crate architecture for ops, UV, IO, and UI

## Blender gap analysis

See [`docs/GAP_ANALYSIS_BLENDER.md`](docs/GAP_ANALYSIS_BLENDER.md) for a feature-by-feature comparison with Blender Edit Mode and the prioritized roadmap.

## Current Status (v0.1.0)

- Workspace crates: `polyforge-core`, `polyforge-ops`, `polyforge-uv`, `polyforge-io`, `polyforge-ui`, `polyforge-app`
- Interactive **Move / Rotate / Scale** gizmos
- **Raycast selection** (vertex / edge / face / object) with visual feedback
- **Area (marquee) selection** by dragging in the viewport
- **Cell mode**: linked face regions (`4`, `L`, `+/-`)
- **Vertex / edge / face editing**: move, delete, extrude, inset, bevel, loop cut, merge, fill, duplicate
- Edge loop select (Alt+click)
- Primitives: cube, plane, cylinder, UV sphere
- Bevy 3D viewport with orbit camera and a default cube
- GPL-3.0 license

### Controls

| Action | Input |
|--------|-------|
| Select element | Left-click |
| Area (marquee) select | Left-drag on empty space |
| Add to selection | Shift + click / drag |
| Clear selection | Click empty space / Escape / Alt+A |
| Select all | `A` |
| Vertex mode | `1` |
| Edge mode | `2` |
| Face mode | `3` |
| Cell mode | `4` (regioni di facce collegate) |
| Object mode | `5` |
| Linked cells | `L` |
| Grow / shrink cells | `+` / `-` |
| Move selection | `G` then drag (free or on axis) |
| Rotate | `R` then drag on axis |
| Scale | `S` then drag on axis |
| Delete | `X` or `Delete` |
| Extrude faces | `E` (with faces selected) |
| Inset faces | `I` (with faces selected) |
| Split edges | `Y` (with edges selected) |
| Nudge faces normal | `N` (with faces selected) |
| Loop cut | `Ctrl+R` (edge selected) |
| Bevel | `Ctrl+B` (edges selected) |
| Merge by distance | `M` |
| Fill face | `F` (3+ vertices) |
| Duplicate | `Shift+D` |
| Edge loop select | `Alt` + click edge |
| Free move | Move tool + drag in viewport |
| Snap to grid | Enable Snap in properties panel |
| Orbit camera | Right-mouse drag |
| Zoom | Mouse wheel |

## Building & Running

Requirements:
- Recent stable Rust (Bevy 0.20 currently requires a recent toolchain)
- A GPU with Vulkan / Metal / DX12 support

```bash
git clone https://github.com/SimonTek27/polyforge.git
cd polyforge
cargo run --release
```

The binary is named `polyforge`.

## Project Structure

```
polyforge/
├── crates/
│   ├── polyforge-core/   # Mesh data, selection, picking, gizmos
│   ├── polyforge-ops/    # Modeling operations
│   ├── polyforge-uv/     # UV mapping (stubs)
│   ├── polyforge-io/     # Import/export (stubs)
│   ├── polyforge-ui/     # egui panels
│   └── polyforge-app/    # Main binary
├── docs/
│   ├── GAP_ANALYSIS_BLENDER.md
│   └── PUBLISH_GITHUB.md
├── LICENSE
└── README.md
```

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).
