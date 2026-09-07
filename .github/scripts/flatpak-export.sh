#!/usr/bin/env bash
# Export single-file bundles out of the OSTree repo for the GitHub release.
#   flatpak-export.sh <repo-dir> <version> <dest-dir>
set -euo pipefail

repo="${1:?repo dir}"
version="${2:?version}"
dest="${3:?dest dir}"

mkdir -p "$dest"

for arch in x86_64 aarch64; do
  flatpak build-bundle "$repo" \
    "${dest}/reveal-${version}-${arch}.flatpak" \
    io.github.knst0.reveal --arch="$arch"
done

ls -l "$dest"
