#!/usr/bin/env bash
# Bump Bevy (and companion crate pins) in the PolyForge workspace.
# Usage: ./scripts/bump_bevy.sh <major.minor> [bevy_egui_version]
#
# This only rewrites version strings. You MUST still:
#   - apply migration guide fixes
#   - cargo build / fix API breaks
#   - update bevy-compat.toml verified + migration_guide URLs
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-}"
EGUI_VER="${2:-}"

if [[ -z "$TARGET" ]]; then
  echo "Usage: $0 <bevy_major.minor> [bevy_egui_version]" >&2
  echo "Example: $0 0.21 0.44" >&2
  exit 2
fi

if [[ ! "$TARGET" =~ ^[0-9]+\.[0-9]+$ ]]; then
  echo "error: expected major.minor like 0.21, got '$TARGET'" >&2
  exit 2
fi

CARGO="${ROOT}/Cargo.toml"
COMPAT="${ROOT}/bevy-compat.toml"

OLD="$(grep -E '^\s*version = "' "$COMPAT" | head -1 | sed 's/.*"\([0-9.]*\)".*/\1/' || true)"
# Prefer explicit from [bevy] section
OLD="$(awk '/^\[bevy\]/{s=1;next} /^\[/{s=0} s&&/^version =/{gsub(/"/,""); print $3; exit}' "$COMPAT")"

echo "Bumping Bevy ${OLD:-?} → ${TARGET}"

# --- Cargo.toml workspace.dependencies.bevy version -----------------------
# Match: bevy = { version = "0.20", ...
python3 - "$CARGO" "$TARGET" <<'PY'
import re, sys
path, target = sys.argv[1], sys.argv[2]
text = open(path).read()
new, n = re.subn(
    r'(bevy\s*=\s*\{\s*version\s*=\s*")[^"]+(")',
    rf'\g<1>{target}\2',
    text,
    count=1,
)
if n != 1:
    print("warning: expected 1 bevy version replacement in Cargo.toml, got", n, file=sys.stderr)
open(path, "w").write(new)
print("Cargo.toml: bevy version ->", target)
PY

# Optional bevy_egui
if [[ -n "$EGUI_VER" ]]; then
  python3 - "$CARGO" "$EGUI_VER" <<'PY'
import re, sys
path, ver = sys.argv[1], sys.argv[2]
text = open(path).read()
new, n = re.subn(
    r'(bevy_egui\s*=\s*")[^"]+(")',
    rf'\g<1>{ver}\2',
    text,
    count=1,
)
open(path, "w").write(new)
print("Cargo.toml: bevy_egui ->", ver, f"({n} replacement(s))")
PY
  python3 - "$COMPAT" "$EGUI_VER" <<'PY'
import re, sys
path, ver = sys.argv[1], sys.argv[2]
text = open(path).read()
new, n = re.subn(
    r'(bevy_egui\s*=\s*")[^"]+(")',
    rf'\g<1>{ver}\2',
    text,
    count=1,
)
open(path, "w").write(new)
print("bevy-compat.toml: bevy_egui ->", ver)
PY
fi

# --- bevy-compat.toml ------------------------------------------------------
python3 - "$COMPAT" "$TARGET" <<'PY'
import re, sys
path, target = sys.argv[1], sys.argv[2]
text = open(path).read()
# Only the [bevy] version key (first occurrence after [bevy] is enough with careful regex)
def repl_section(text, section, key, value):
    pattern = rf'(\[{re.escape(section)}\][^\[]*?{re.escape(key)}\s*=\s*")[^"]*(")'
    new, n = re.subn(pattern, rf'\g<1>{value}\2', text, count=1, flags=re.S)
    return new, n

text, n1 = repl_section(text, "bevy", "version", target)
# Reset verified to target.0 until someone confirms
text, n2 = repl_section(text, "bevy", "verified", f"{target}.0")
open(path, "w").write(text)
print(f"bevy-compat.toml: version={target}, verified={target}.0 (n={n1},{n2})")
print("NOTE: update migration_guide / release_notes / msrv manually.")
PY

echo
echo "Done. Next:"
echo "  1. Set migration_guide + release_notes in bevy-compat.toml"
echo "  2. cargo update -p bevy"
echo "  3. cargo build  # fix breaking API changes"
echo "  4. ./scripts/check_bevy_update.sh"
echo "  5. Commit: 'chore: upgrade Bevy to ${TARGET}'"
