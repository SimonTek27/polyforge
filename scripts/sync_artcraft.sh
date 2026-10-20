#!/usr/bin/env bash
# Sync VectorCraft + PhotoCraft upstream into vendor/ and refresh pins.toml.
# Usage:
#   ./scripts/sync_artcraft.sh           # fetch + update pins (clone if missing)
#   ./scripts/sync_artcraft.sh --check   # exit 1 if upstream ahead of pin
#   ./scripts/sync_artcraft.sh --init    # git submodule add (once)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PINS="${ROOT}/vendor/pins.toml"
MODE="${1:-sync}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: need $1" >&2; exit 2; }; }
need git
need curl
need python3

github_sha() {
  local repo_path="$1" ref="${2:-main}"
  # repo_path like storytold/vectorcraft
  local url="https://api.github.com/repos/${repo_path}/commits/${ref}"
  local json
  if ! json="$(curl -fsSL --max-time 30 -A 'polyforge-artcraft-sync/0.1' \
      -H 'Accept: application/vnd.github+json' "$url")"; then
    echo "error: failed to query $url" >&2
    return 2
  fi
  python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])' <<<"$json"
}

github_latest_tag() {
  local repo_path="$1"
  local url="https://api.github.com/repos/${repo_path}/releases/latest"
  local json
  if ! json="$(curl -fsSL --max-time 30 -A 'polyforge-artcraft-sync/0.1' \
      -H 'Accept: application/vnd.github+json' "$url" 2>/dev/null)"; then
    echo ""
    return 0
  fi
  python3 -c 'import json,sys; d=json.load(sys.stdin); print(d.get("tag_name") or "")' <<<"$json" 2>/dev/null || echo ""
}

update_pin() {
  local section="$1" sha="$2" ref="$3"
  local today
  today="$(date -u +%Y-%m-%d)"
  python3 - "$PINS" "$section" "$sha" "$ref" "$today" <<'PY'
import re, sys
path, section, sha, ref, day = sys.argv[1:6]
text = open(path).read()
def set_key(text, section, key, value):
    # Replace key inside [section] block only
    pattern = rf'(\[{re.escape(section)}\][^\[]*?{re.escape(key)}\s*=\s*")[^"]*(")'
    new, n = re.subn(pattern, rf'\g<1>{value}\g<2>', text, count=1, flags=re.S)
    if n == 0:
        print(f"warning: could not set {section}.{key}", file=sys.stderr)
    return new
text = set_key(text, section, "pin_sha", sha)
text = set_key(text, section, "pin_ref", ref)
text = set_key(text, section, "last_checked", day)
open(path, "w").write(text)
print(f"pins.toml: [{section}] pin_sha={sha[:12]}… ref={ref} checked={day}")
PY
}

clone_or_update() {
  local name="$1" url="$2" path="$3" ref="$4"
  if [[ ! -d "$path/.git" ]]; then
    echo "Cloning $name → $path"
    mkdir -p "$(dirname "$path")"
    git clone --depth 1 --branch "$ref" "$url" "$path" 2>/dev/null \
      || git clone --depth 1 "$url" "$path"
  else
    echo "Updating $name in $path"
    git -C "$path" fetch --tags --depth 1 origin "$ref" 2>/dev/null || git -C "$path" fetch --depth 1 origin
    git -C "$path" checkout "$ref" 2>/dev/null || true
    git -C "$path" pull --ff-only origin "$ref" 2>/dev/null || true
  fi
  git -C "$path" rev-parse HEAD
}

# --- init submodules once ---------------------------------------------------
if [[ "$MODE" == "--init" ]]; then
  cd "$ROOT"
  if [[ ! -d vendor/vectorcraft/.git ]]; then
    git submodule add -f https://github.com/storytold/vectorcraft.git vendor/vectorcraft || true
  fi
  if [[ ! -d vendor/photocraft/.git ]]; then
    git submodule add -f https://github.com/storytold/photocraft.git vendor/photocraft || true
  fi
  echo "Submodules registered. Run without --init to sync."
  exit 0
fi

echo "=== ArtCraft upstream sync ==="
VC_TAG="$(github_latest_tag storytold/vectorcraft || true)"
PC_TAG="$(github_latest_tag storytold/photocraft || true)"
VC_REF="${VC_TAG:-main}"
PC_REF="${PC_TAG:-main}"
# Prefer main tip for track_latest policy when tags lag
VC_REF="main"
PC_REF="main"

VC_REMOTE="$(github_sha storytold/vectorcraft "$VC_REF" 2>/dev/null || true)"
PC_REMOTE="$(github_sha storytold/photocraft "$PC_REF" 2>/dev/null || true)"
if [[ -z "$VC_REMOTE" ]]; then
  VC_REMOTE="$(git ls-remote https://github.com/storytold/vectorcraft.git "refs/heads/$VC_REF" 2>/dev/null | awk '{print $1}')"
fi
if [[ -z "$PC_REMOTE" ]]; then
  PC_REMOTE="$(git ls-remote https://github.com/storytold/photocraft.git "refs/heads/$PC_REF" 2>/dev/null | awk '{print $1}')"
fi
VC_REMOTE="${VC_REMOTE:-unknown}"
PC_REMOTE="${PC_REMOTE:-unknown}"
echo "VectorCraft remote ($VC_REF): ${VC_REMOTE:0:12}"
echo "PhotoCraft  remote ($PC_REF): ${PC_REMOTE:0:12}"

VC_PIN="$(python3 -c "import re; t=open('$PINS').read(); m=re.search(r'\[vectorcraft\][^\[]*?pin_sha\s*=\s*\"([^\"]*)\"', t, re.S); print(m.group(1) if m else '')")"
PC_PIN="$(python3 -c "import re; t=open('$PINS').read(); m=re.search(r'\[photocraft\][^\[]*?pin_sha\s*=\s*\"([^\"]*)\"', t, re.S); print(m.group(1) if m else '')")"

if [[ "$MODE" == "--check" ]]; then
  drift=0
  if [[ -n "$VC_PIN" && "$VC_PIN" != "$VC_REMOTE" ]]; then
    echo "✗ VectorCraft pin outdated: ${VC_PIN:0:12} → ${VC_REMOTE:0:12}"
    drift=1
  else
    echo "✓ VectorCraft pin current"
  fi
  if [[ -n "$PC_PIN" && "$PC_PIN" != "$PC_REMOTE" ]]; then
    echo "✗ PhotoCraft pin outdated: ${PC_PIN:0:12} → ${PC_REMOTE:0:12}"
    drift=1
  else
    echo "✓ PhotoCraft pin current"
  fi
  if [[ "$drift" -ne 0 ]]; then
    echo
    echo "Run: ./scripts/sync_artcraft.sh"
    exit 1
  fi
  exit 0
fi

# Full sync: clone/update trees + write pins
VC_HEAD="$(clone_or_update VectorCraft https://github.com/storytold/vectorcraft.git "$ROOT/vendor/vectorcraft" "$VC_REF")"
PC_HEAD="$(clone_or_update PhotoCraft https://github.com/storytold/photocraft.git "$ROOT/vendor/photocraft" "$PC_REF")"
update_pin vectorcraft "$VC_HEAD" "$VC_REF"
update_pin photocraft "$PC_HEAD" "$PC_REF"

# NOTICE files for license compliance
mkdir -p "$ROOT/vendor"
cat > "$ROOT/vendor/NOTICE" <<NOTICE
Third-party upstream trees under vendor/:

  vectorcraft/  — https://github.com/storytold/vectorcraft
                  License: MIT OR Apache-2.0 (see upstream LICENSE files)
  photocraft/   — https://github.com/storytold/photocraft
                  License: MIT OR Apache-2.0 (see upstream LICENSE files)

ArtCraft, VectorCraft and PhotoCraft names and logos are trademarks of their
respective owners and are not open-sourced by this notice.

PolyForge itself remains GPL-3.0-or-later.
NOTICE

echo
echo "Done. vendor/ updated and pins.toml refreshed."
echo "Commit pins + submodule SHAs when ready."
