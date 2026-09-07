#!/usr/bin/env bash
# Publish to crates.io, treating an already-published version as success.
set -euo pipefail

if out=$(cargo publish --locked --no-verify 2>&1); then
  echo "$out"
  exit 0
fi

echo "$out"
if grep -qiE "already uploaded|already exists" <<<"$out"; then
  echo "Version already on crates.io; treating as success."
  exit 0
fi

echo "cargo publish failed." >&2
exit 1
