#!/usr/bin/env bash
# Print the newest CHANGELOG.md section (the notes for the release being cut).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

awk '
  /^## / { if (++seen == 1) next; if (seen == 2) exit }
  seen == 1
' CHANGELOG.md
