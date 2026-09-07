#!/usr/bin/env bash
# Resolve the release version from Cargo.toml and guard against a stale tree.
# Emits version/tag/sha to $GITHUB_OUTPUT when running in Actions.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

if git ls-files '.changeset/*.md' | grep -qv '^.changeset/README.md$'; then
  echo "::error::unconsumed changesets on main; merge the Release PR first"
  exit 1
fi

version=$(sed -n '/^\[package\]/,/^\[/p' Cargo.toml |
  sed -n 's/^version = "\(.*\)"/\1/p' | head -n1)
[ -n "$version" ] || { echo "::error::could not read the version from Cargo.toml"; exit 1; }

tag="v${version}"
if git ls-remote --exit-code --tags origin "refs/tags/${tag}" >/dev/null 2>&1; then
  echo "::error::tag ${tag} already exists on origin"
  exit 1
fi

{
  echo "version=${version}"
  echo "tag=${tag}"
  echo "sha=$(git rev-parse HEAD)"
} >> "${GITHUB_OUTPUT:-/dev/stdout}"
