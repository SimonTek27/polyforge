# Publishing PolyForge to GitHub

Repository: https://github.com/SimonTek27/polyforge

## Full source push (recommended on your machine)

The sandbox cannot use `git push` (no credentials). From a clone with auth:

```bash
# Option A — replace remote tree with local workspace
cd polyforge
git add -A
git commit -m "sync: full PolyForge source"
git push -u origin main

# Option B — unpack the full archive then push
tar xzf polyforge-full-src.tar.gz
cd polyforge   # or the extracted folder
git init
git remote add origin https://github.com/SimonTek27/polyforge.git
git add -A
git commit -m "Import full PolyForge source"
git branch -M main
git push -u origin main
```

## What the API sync already has

Partial updates via GitHub API: workspace Cargo.toml, all crate Cargo.toml, docs/ROADMAP, some modules.

For a **complete** tree matching the sandbox, use Option A/B above or the `polyforge-full-src.tar.gz` artifact.
