#!/usr/bin/env bash
# Fail the release when the collected artifacts are not the full expected set.
#   verify-artifacts.sh <dist-dir> <version>
set -euo pipefail

dist="${1:?dist dir}"
version="${2:?version}"

expected=(
  "reveal-x86_64-unknown-linux-gnu.tar.gz"
  "reveal-aarch64-unknown-linux-gnu.tar.gz"
  "reveal-aarch64-apple-darwin.tar.gz"
  "reveal-x86_64-apple-darwin.tar.gz"
  "reveal-x86_64-pc-windows-msvc.zip"
  "reveal-setup.exe"
  "reveal-${version}-macos.dmg"
)

# A draft run builds the Flatpak bundles but does not publish the repo, so the
# exported single-file bundles only exist on a real release.
if [ "${DRAFT:-false}" != true ]; then
  expected+=(
    "reveal-${version}-x86_64.flatpak"
    "reveal-${version}-aarch64.flatpak"
  )
fi

missing=()
for name in "${expected[@]}"; do
  [ -f "${dist}/${name}" ] || missing+=("$name")
done

if [ "${#missing[@]}" -gt 0 ]; then
  printf '::error::missing release artifact: %s\n' "${missing[@]}" >&2
  echo "collected:" >&2
  ls -1 "$dist" >&2
  exit 1
fi

echo "all ${#expected[@]} expected artifacts are present"
