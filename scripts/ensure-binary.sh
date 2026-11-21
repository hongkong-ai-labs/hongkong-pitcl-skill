#!/usr/bin/env bash
# Print the absolute path of a working mawan binary.
# Order: $MAWAN_BIN, mawan on PATH, cargo build --release in this checkout.
set -euo pipefail

if [ -n "${MAWAN_BIN:-}" ] && [ -x "$MAWAN_BIN" ]; then
  printf '%s\n' "$MAWAN_BIN"
  exit 0
fi

if command -v mawan >/dev/null 2>&1; then
  command -v mawan
  exit 0
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
if [ -f "$ROOT/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
  (
    cd "$ROOT"
    cargo build --release --locked >/dev/null
  )
  BIN="$ROOT/target/release/mawan"
  if [ -x "$BIN" ]; then
    printf '%s\n' "$BIN"
    exit 0
  fi
fi

echo "mawan: no binary found and no Cargo build available" >&2
echo "Set MAWAN_BIN, put mawan on PATH, or build with: cargo build --release --locked" >&2
exit 1
