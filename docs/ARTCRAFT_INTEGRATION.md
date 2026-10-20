# ArtCraft integration (VectorCraft + PhotoCraft)

## Goal

PolyForge **tracks and updates** with upstream:

| Upstream | Repo | PolyForge role |
|----------|------|----------------|
| **VectorCraft** | https://github.com/storytold/vectorcraft | VectorMode UX target + external advanced illustration editor |
| **PhotoCraft** | https://github.com/storytold/photocraft | Raster/texture companion for materials, baking, image work |

Both are **MIT OR Apache-2.0**. PolyForge stays **GPL-3.0-or-later**. ArtCraft trademarks are **not** relicensed.

## Why not a single binary?

VectorCraft and PhotoCraft are full multi-crate editors (egui/wgpu, their own shells). Merging them into the Bevy `polyforge` process is not practical. Integration is:

1. **Vendor trees** under `vendor/` (git clone or submodule)
2. **Pins** in `vendor/pins.toml` (SHA + ref + last_checked)
3. **CI** that fails when upstream moves past the pin
4. **Bridge crate** `polyforge-bridge` (launch helpers, format affinities, pin load)
5. **UI** actions: Open VectorCraft / Open PhotoCraft

## Keep up to date (mandatory)

```bash
./scripts/sync_artcraft.sh          # pull latest main + rewrite pins
./scripts/sync_artcraft.sh --check  # exit 1 if behind (CI)
./scripts/sync_artcraft.sh --init   # optional git submodule register
```

Schedule: GitHub Action every 12h (`.github/workflows/artcraft-upstream-check.yml`).

Policy mirrors **Bevy priority**: when upstream ArtCraft apps move, PolyForge pins must follow before unrelated work.

## Exchange formats

- Vector ↔ VectorCraft: **SVG / SVGZ / PDF**
- Photo ↔ PhotoCraft: **PNG / JPEG / WebP / PSD / TIFF**

PolyForge VectorMode already does SVG; mesh materials can use textures exported from PhotoCraft.

## First-time setup

```bash
cd polyforge
./scripts/sync_artcraft.sh
# builds optional:
#   cargo run --release -p vectorcraft  # inside vendor/vectorcraft
#   cargo run --release -p photocraft   # inside vendor/photocraft
```
