#!/usr/bin/env bash
set -euo pipefail

BIN="${1:?usage: bundle-macos.sh <path-to-reveal-binary> [output-dir]}"
OUT="${2:-target/macos}"
APP="$OUT/Reveal.app"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n1)"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/reveal"
chmod +x "$APP/Contents/MacOS/reveal"

if [ -f resource/macos/reveal.icns ]; then
  cp resource/macos/reveal.icns "$APP/Contents/Resources/reveal.icns"
fi

cargo run --quiet --bin info-plist -- "$VERSION" > "$APP/Contents/Info.plist"

printf 'APPL????' > "$APP/Contents/PkgInfo"

echo "built $APP"
