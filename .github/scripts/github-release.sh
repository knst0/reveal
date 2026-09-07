#!/usr/bin/env bash
# Tag the release commit and publish the GitHub release with everything in dist/.
# Expects TAG, DRAFT and GITHUB_TOKEN in the environment. Safe to re-run.
set -euo pipefail

: "${TAG:?tag}"

if ! git ls-remote --exit-code --tags origin "refs/tags/${TAG}" >/dev/null 2>&1; then
  git config user.name "github-actions[bot]"
  git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
  git tag -a "$TAG" -m "$TAG"
  git push origin "$TAG"
fi

args=(--title "$TAG" --notes-file RELEASE_NOTES.md)
[ "${DRAFT:-false}" = true ] && args+=(--draft)

if gh release view "$TAG" >/dev/null 2>&1; then
  gh release upload "$TAG" dist/* --clobber
else
  gh release create "$TAG" "${args[@]}" dist/*
fi
