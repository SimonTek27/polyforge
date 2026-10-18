#!/usr/bin/env bash
# Check whether PolyForge is on the latest Bevy release.
# Exit codes:
#   0  up to date
#   1  update available (or policy violation)
#   2  network / parse error
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPAT="${ROOT}/bevy-compat.toml"
CARGO="${ROOT}/Cargo.toml"

if [[ ! -f "$COMPAT" ]]; then
  echo "error: missing bevy-compat.toml" >&2
  exit 2
fi

# --- helpers ---------------------------------------------------------------

toml_get() {
  # Very small TOML reader for keys we control (no nested tables with dots in values).
  # Usage: toml_get file section key
  local file="$1" section="$2" key="$3"
  awk -v sec="[$section]" -v key="$key" '
    $0 == sec { insec=1; next }
    /^\[/ { insec=0 }
    insec && $1 == key {
      sub(/^[^=]*= */, "")
      gsub(/"/, "")
      gsub(/ /, "")
      print
      exit
    }
  ' "$file"
}

fetch_latest_bevy() {
  # crates.io API
  local json
  if ! json="$(curl -fsSL --max-time 20 -A 'polyforge-bevy-check/0.1' 'https://crates.io/api/v1/crates/bevy')"; then
    echo "error: failed to reach crates.io" >&2
    return 2
  fi
  python3 -c 'import json,sys; d=json.load(sys.stdin); print(d["crate"]["max_stable_version"] if "max_stable_version" in d["crate"] else d["crate"]["max_version"])' <<<"$json"
}

major_minor() {
  # 0.20.1 -> 0.20
  echo "$1" | awk -F. '{print $1"."$2}'
}

# --- read pins -------------------------------------------------------------

PINNED="$(toml_get "$COMPAT" bevy version)"
VERIFIED="$(toml_get "$COMPAT" bevy verified)"
POLICY="$(toml_get "$COMPAT" policy mode)"

if [[ -z "$PINNED" ]]; then
  echo "error: could not read bevy.version from bevy-compat.toml" >&2
  exit 2
fi

echo "PolyForge Bevy pin:     ${PINNED} (verified ${VERIFIED:-?})"
echo "Policy:                 ${POLICY:-track_latest}"

# Cargo.toml cross-check
if grep -q "bevy = { version = \"${PINNED}" "$CARGO" 2>/dev/null \
  || grep -qE "bevy = \\{ version = \"${PINNED}" "$CARGO" 2>/dev/null \
  || grep -q "version = \"${PINNED}\"" "$CARGO" 2>/dev/null; then
  echo "Cargo.toml:             matches pin ${PINNED}"
else
  # softer check
  if grep -q "bevy" "$CARGO"; then
    CARGO_LINE="$(grep -E 'bevy\s*=' "$CARGO" | head -1 || true)"
    echo "Cargo.toml bevy line:   ${CARGO_LINE}"
    if ! echo "$CARGO_LINE" | grep -q "$PINNED"; then
      echo "WARNING: Cargo.toml Bevy version may not match bevy-compat.toml pin"
    fi
  fi
fi

# --- remote ----------------------------------------------------------------

set +e
LATEST="$(fetch_latest_bevy)"
FETCH_RC=$?
set -e
if [[ $FETCH_RC -ne 0 || -z "$LATEST" ]]; then
  echo "error: could not determine latest Bevy version" >&2
  exit 2
fi

LATEST_MM="$(major_minor "$LATEST")"
PINNED_MM="$(major_minor "$PINNED")"

echo "crates.io latest Bevy:  ${LATEST}"
echo "latest major.minor:     ${LATEST_MM}"

if [[ "$LATEST_MM" == "$PINNED_MM" ]]; then
  echo
  echo "✓ Up to date with Bevy ${LATEST_MM} (latest ${LATEST})."
  exit 0
fi

echo
echo "✗ Bevy update available: pinned ${PINNED_MM} → latest ${LATEST_MM} (${LATEST})"
echo
echo "Next steps (priority #1 per project policy):"
echo "  1. Read migration guide for ${PINNED_MM} → ${LATEST_MM}"
echo "  2. Run:  ./scripts/bump_bevy.sh ${LATEST_MM}"
echo "  3. Fix compile errors following docs/BEVY_UPDATE.md"
echo "  4. Update bevy-compat.toml verified= and companions"
echo "  5. cargo test && cargo run --release"
exit 1
