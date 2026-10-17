# PolyForge

Open-source 3D polygon modeling tool built with the **Bevy** engine.
Licensed under **GPL-3.0-or-later**.

## Modes

- **PolyMode** — mesh Edit Mode (Blender-oriented Phase 1–2)
- **CadMode** — parametric solids / sketcher (Phases 3–11, mesh kernel MVP)
- **VectorMode** — 2D paths, SVG, boolean 2D

## Quick start

```bash
cargo run -p polyforge-app --release
```

## Docs

- `docs/PHASE1_BLENDER_PARITY.md` … `docs/PHASE11.md`
- `docs/GAP_ANALYSIS_BLENDER.md` / `docs/GAP_ANALYSIS_FREECAD.md`
- `docs/BEVY_UPDATE.md` — Bevy upgrades are always priority #1

## GitHub

https://github.com/SimonTek27/polyforge

Full source push may require local `git push` with credentials when the API only syncs partial trees.
