# Publish / sync to GitHub

Repository: https://github.com/SimonTek27/polyforge

## Full sync (recommended)

```bash
cd polyforge
git remote -v
git push -u origin main
# if history diverged:
# git push -u origin main --force
```

Prefer `gh auth login` or a PAT on a local machine for complete tree sync (all crates).

## After push

- Confirm `crates/polyforge-app/src/main.rs` and ops modules are on GitHub
- CI: `.github/workflows/bevy-update-check.yml` runs weekly
