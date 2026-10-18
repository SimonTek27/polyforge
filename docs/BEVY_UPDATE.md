# Bevy update policy

> **Priority #1:** Bevy engine updates always come first.  
> PolyForge adapts to Bevy — never the other way around.

## Why

PolyForge is built on Bevy’s ECS, rendering, input, and windowing. Lagging behind a Bevy release means:

- missing engine fixes and performance work  
- growing migration debt  
- incompatible ecosystem crates (`bevy_egui`, etc.)

## Source of truth

| File | Role |
|------|------|
| `bevy-compat.toml` | Pinned Bevy line, verified build, companions, policy |
| `Cargo.toml` `[workspace.dependencies]` | Actual Cargo version requirements |
| `scripts/check_bevy_update.sh` | Compares pin vs crates.io |
| `scripts/bump_bevy.sh` | Rewrites version pins |

## Check for updates

```bash
./scripts/check_bevy_update.sh
```

- Exit `0` — on current major.minor  
- Exit `1` — newer Bevy available → **start upgrade immediately**  
- Exit `2` — network/parse error  

## Upgrade workflow

1. **Detect**  
   `./scripts/check_bevy_update.sh`

2. **Read the migration guide**  
   From the previous minor to the new one, e.g.  
   https://bevyengine.org/learn/migration-guides/

3. **Bump pins**  
   ```bash
   ./scripts/bump_bevy.sh 0.21 0.44   # bevy major.minor, optional bevy_egui
   cargo update -p bevy
   ```

4. **Fix compile errors**  
   Work top-down: `polyforge-core` → `ops` / `ui` / `io` / `uv` → `polyforge-app`.  
   Prefer Bevy’s new APIs; do not keep deprecated shims longer than one release.

5. **Verify**  
   ```bash
   cargo build --release
   cargo run --release
   ./scripts/check_bevy_update.sh
   ```

6. **Record**  
   In `bevy-compat.toml` set:
   - `verified = "x.y.z"` (exact version that built)
   - `migration_guide` / `release_notes` URLs  
   - `msrv` if Bevy raised it  
   - companion crate versions  

7. **Commit**  
   ```
   chore: upgrade Bevy to X.Y
   ```

## CI

`.github/workflows/bevy-update-check.yml` runs the check script on a schedule and on `Cargo.toml` changes.  
A failing check means an upgrade is overdue — treat it as a blocker for unrelated feature PRs when `policy.mode = "track_latest"`.

## Companion crates

When Bevy bumps a minor, always re-check:

- `bevy_egui` (must match Bevy major.minor line)
- any Bevy plugin we add later

Update `[companions]` in `bevy-compat.toml` in the same commit as the Bevy bump.

## Non-goals

- Tracking Bevy **main** / git revisions by default (too unstable for the app crate)
- Delaying upgrades for “feature freeze” — features adapt after the bump
