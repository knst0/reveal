#!/usr/bin/env bash
# Archive the freshly built binary for one target into the working directory.
#   package.sh <target>
set -euo pipefail

target="${1:?target triple}"
dir="target/${target}/release"

if [ -f "${dir}/reveal.exe" ]; then
  powershell -NoProfile -Command \
    "Compress-Archive -Force -Path '${dir}/reveal.exe' -DestinationPath 'reveal-${target}.zip'"
else
  tar -czf "reveal-${target}.tar.gz" -C "$dir" reveal
fi

ls -l reveal-"${target}".*
