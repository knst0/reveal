#!/usr/bin/env bash
# Regenerate flatpak/cargo-sources.json so the build can run fully offline.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
generator="$(mktemp)"
trap 'rm -f "$generator"' EXIT

curl -fsSL -o "$generator" \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py

python3 "$generator" "${root}/Cargo.lock" -o "${root}/flatpak/cargo-sources.json"
echo "wrote ${root}/flatpak/cargo-sources.json"
