# Publish / sync to GitHub

Repository: https://github.com/SimonTek27/polyforge

## Full sync (recommended)

Local commits may be ahead of `origin` when HTTPS credentials are unavailable in the sandbox.

```bash
cd polyforge
git remote -v   # should be https://github.com/SimonTek27/polyforge.git
git push -u origin main
# if history diverged:
# git push -u origin main --force
```

## Partial API sync

The connected GitHub tool can push file batches without local credentials.
Prefer a single `git push` from a machine with `gh auth login` or a PAT.

## After push

- Confirm tree on GitHub includes `crates/polyforge-app/src/main.rs` and all ops modules.
- CI: `.github/workflows/bevy-update-check.yml` runs weekly.
