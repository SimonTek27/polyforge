# vendor/ — ArtCraft upstream trees

| Directory | Upstream | Role in PolyForge |
|-----------|----------|-------------------|
| `vectorcraft/` | [storytold/vectorcraft](https://github.com/storytold/vectorcraft) | **VectorMode** reference / external illustration editor |
| `photocraft/` | [storytold/photocraft](https://github.com/storytold/photocraft) | **Photo** companion for textures, raster, materials |

## Sync (required on every upstream change)

```bash
./scripts/sync_artcraft.sh          # pull latest + update pins.toml
./scripts/sync_artcraft.sh --check  # CI: fail if pin is behind
./scripts/sync_artcraft.sh --init   # register git submodules once
```

Pins live in [`pins.toml`](pins.toml). CI workflow: `.github/workflows/artcraft-upstream-check.yml`.

## License

Upstream code: **MIT OR Apache-2.0**. See each tree’s LICENSE files and [`NOTICE`](NOTICE).  
PolyForge remains **GPL-3.0-or-later**.
