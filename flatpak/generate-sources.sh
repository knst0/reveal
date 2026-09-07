#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$root/flatpak/cargo-sources.json"

generator="$(mktemp)"
trap 'rm -f "$generator"' EXIT

curl -fsSL -o "$generator" \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py

python3 "$generator" "$root/Cargo.lock" -o "$out"
echo "wrote $out"
