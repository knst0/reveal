#!/usr/bin/env bash
# Tag the release commit and publish the GitHub release with everything in dist/.
# Expects TAG, DRAFT and GITHUB_TOKEN in the environment.
set -euo pipefail

: "${TAG:?tag}"

git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git tag -a "$TAG" -m "$TAG"
git push origin "$TAG"

args=(--title "$TAG" --notes-file RELEASE_NOTES.md)
[ "${DRAFT:-false}" = true ] && args+=(--draft)

gh release create "$TAG" "${args[@]}" dist/*
