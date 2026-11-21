#!/usr/bin/env bash
# Build mawan from this checkout and install it into ~/.local/bin.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${MAWAN_BIN_DIR:-$HOME/.local/bin}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required: https://rustup.rs" >&2
  exit 1
fi

cargo build --release --locked --manifest-path "$ROOT/Cargo.toml"
mkdir -p "$DEST"
install -m 0755 "$ROOT/target/release/mawan" "$DEST/mawan"
printf 'installed %s\n' "$DEST/mawan"
case ":$PATH:" in
  *":$DEST:"*) ;;
  *) echo "note: add $DEST to PATH" >&2 ;;
esac
