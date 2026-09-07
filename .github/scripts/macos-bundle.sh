#!/usr/bin/env bash
# Fuse the per-arch macOS binaries into Reveal.app and emit the .dmg release
# artifacts.
#   macos-bundle.sh <version> <downloaded-tarball-dir> <dest-dir>
set -euo pipefail

version="${1:?version}"
src="${2:?tarball dir}"
dest="${3:?dest dir}"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

for arch in aarch64 x86_64; do
  tar -xzf "${src}/reveal-${arch}-apple-darwin.tar.gz" -C "$work"
  mv "${work}/reveal" "${work}/reveal-${arch}"
done

app="${work}/Reveal.app/Contents"
mkdir -p "${app}/MacOS" "${app}/Resources"

lipo -create -output "${app}/MacOS/reveal" "${work}/reveal-aarch64" "${work}/reveal-x86_64"
chmod +x "${app}/MacOS/reveal"
lipo -info "${app}/MacOS/reveal"

cp "${root}/resource/macos/reveal.icns" "${app}/Resources/reveal.icns"
cp "${root}/LICENSE" "${app}/Resources/LICENSE"
cargo run --quiet --features tools --bin info-plist -- "$version" > "${app}/Info.plist"
plutil -lint "${app}/Info.plist"

codesign --force --sign - --timestamp=none "${work}/Reveal.app"
codesign --verify --strict --verbose=2 "${work}/Reveal.app"

staging="${work}/staging"
mkdir -p "$staging" "$dest"
cp -R "${work}/Reveal.app" "$staging/"
ln -s /Applications "${staging}/Applications"

hdiutil create \
  -volname Reveal \
  -srcfolder "$staging" \
  -size "$(( $(du -ms "$staging" | cut -f1) + 64 ))m" \
  -ov -format UDZO \
  "${dest}/reveal-${version}-macos.dmg"

ls -l "$dest"
