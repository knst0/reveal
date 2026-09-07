#!/usr/bin/env bash
# Render the Homebrew cask from the released .dmg and push it to the tap.
#   homebrew-cask.sh <version> <dmg-dir> <tap-checkout>
set -euo pipefail

version="${1:?version}"
dist="${2:?dmg dir}"
tap="${3:?tap checkout}"

template="$(dirname "${BASH_SOURCE[0]}")/homebrew/reveal.rb.tmpl"
sha=$(sha256sum "${dist}/reveal-${version}-macos.dmg" | cut -d' ' -f1)

mkdir -p "${tap}/Casks"
sed -e "s/__VERSION__/${version}/" -e "s/__SHA256__/${sha}/" \
  "$template" > "${tap}/Casks/reveal.rb"
cat "${tap}/Casks/reveal.rb"

cd "$tap"
git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add Casks/reveal.rb

if git diff --cached --quiet; then
  echo "Cask already up to date."
  exit 0
fi

git commit -m "reveal ${version}"
git push
