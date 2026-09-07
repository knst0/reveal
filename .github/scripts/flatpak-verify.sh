#!/usr/bin/env bash
# Confirm the published Pages site actually serves the Flatpak repository.
#   flatpak-verify.sh <page-url>
set -euo pipefail

base="${1:?page url}"
base="${base%/}"

fetch() {
  local url="$1"
  local attempt
  for attempt in 1 2 3 4 5 6; do
    if curl -fsS --max-time 30 "$url" -o /dev/null; then
      return 0
    fi
    echo "attempt ${attempt} failed for ${url}; retrying" >&2
    sleep 10
  done
  echo "::error::${url} is not reachable" >&2
  return 1
}

fetch "${base}/reveal.flatpakrepo"
fetch "${base}/repo/summary"
fetch "${base}/repo/config"

echo "the flatpak repository is reachable at ${base}"
