#!/usr/bin/env bash
# Import the built bundles into the signed OSTree repo and stage the Pages site.
#   flatpak-publish.sh <bundle-dir> <site-dir> <gpg-fingerprint>
set -euo pipefail

bundles="${1:?bundle dir}"
site="${2:?site dir}"
fingerprint="${3:?gpg fingerprint}"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
sign=(--gpg-sign="$fingerprint" --gpg-homedir="$GNUPGHOME")

mkdir -p "${site}/repo/tmp" "${site}/repo/refs/remotes"
[ -f "${site}/repo/config" ] || ostree init --repo="${site}/repo" --mode=archive

for bundle in "${bundles}"/*.flatpak; do
  flatpak build-import-bundle "${sign[@]}" "${site}/repo" "$bundle"
done

flatpak build-update-repo \
  --title=Reveal \
  --generate-static-deltas \
  --prune --prune-depth=3 \
  "${sign[@]}" \
  "${site}/repo"

rm -rf "${site}/repo/tmp"
cp -R "${root}/flatpak/pages/." "$site/"

{
  echo "[Flatpak Repo]"
  echo "Title=Reveal"
  echo "Url=https://${GITHUB_REPOSITORY%%/*}.github.io/${GITHUB_REPOSITORY#*/}/repo/"
  echo "Homepage=https://github.com/${GITHUB_REPOSITORY}"
  echo "Description=A fast image viewer"
  echo "GPGKey=$(base64 -w0 "${GNUPGHOME}/pub.gpg")"
} > "${site}/reveal.flatpakrepo"
