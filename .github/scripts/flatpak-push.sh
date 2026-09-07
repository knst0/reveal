#!/usr/bin/env bash
# Force-push the staged site as the orphan flatpak-repo branch (GitHub Pages).
#   flatpak-push.sh <site-dir>
set -euo pipefail

site="${1:?site dir}"
: "${VERSION:?version}" "${TOKEN:?token}"

cd "$site"
git init -q -b flatpak-repo
git add -A
git \
  -c user.name="github-actions[bot]" \
  -c user.email="41898282+github-actions[bot]@users.noreply.github.com" \
  commit -q -m "chore(flatpak): publish v${VERSION}"
git push --force \
  "https://x-access-token:${TOKEN}@github.com/${GITHUB_REPOSITORY}" flatpak-repo
