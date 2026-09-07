#!/usr/bin/env bash
# Copy the freshly imported single-file bundles out of the OSTree repo so the
# GitHub release can carry them next to the other artifacts.
#   export-bundles.sh <repo-dir> <version> <dest-dir>
set -euo pipefail

repo="${1:?repo dir}"
version="${2:?version}"
dest="${3:?dest dir}"

mkdir -p "$dest"

flatpak build-bundle \
  "$repo" \
  "$dest/reveal-${version}-x86_64.flatpak" \
  io.github.knst0.reveal \
  --arch=x86_64

flatpak build-bundle \
  "$repo" \
  "$dest/reveal-${version}-aarch64.flatpak" \
  io.github.knst0.reveal \
  --arch=aarch64

ls -l "$dest"
