#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

dir=".changeset"
manifest="Cargo.toml"
changelog="CHANGELOG.md"
package="reveal"

die() { echo "error: $*" >&2; exit 1; }

current_version() {
  sed -n '/^\[package\]/,/^\[/p' "$manifest" \
    | sed -n 's/^version = "\(.*\)"/\1/p' | head -n1
}

changeset_files() {
  find "$dir" -maxdepth 1 -name '*.md' ! -name 'README.md' | sort
}

bump_of() {
  sed -n '/^---$/,/^---$/p' "$1" \
    | sed -n "s/^[\"']\?${package}[\"']\?[[:space:]]*:[[:space:]]*//p" \
    | tr -d ' "'"'" | head -n1
}

body_of() {
  awk '
    /^---$/ { fence++; next }
    fence >= 2 { print }
  ' "$1" | sed -e '/./,$!d' | awk '{ lines[NR] = $0 } END { last = 0; for (i = 1; i <= NR; i++) if (lines[i] ~ /[^[:space:]]/) last = i; for (i = 1; i <= last; i++) print lines[i] }'
}

rank() {
  case "$1" in
    major) echo 3 ;;
    minor) echo 2 ;;
    patch) echo 1 ;;
    *) echo 0 ;;
  esac
}

collect() {
  highest=0
  major_notes=""
  minor_notes=""
  patch_notes=""
  count=0

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    bump="$(bump_of "$file")"
    [ -n "$bump" ] || die "$file: no '${package}: major|minor|patch' in front matter"
    r="$(rank "$bump")"
    [ "$r" -gt 0 ] || die "$file: invalid bump '${bump}'"

    body="$(body_of "$file")"
    [ -n "$body" ] || die "$file: empty changelog body"

    entry="$(printf '%s' "$body" | awk '
      NR == 1 { print "- " $0; next }
      /^[[:space:]]*$/ { print ""; next }
      { print "  " $0 }
    ')"

    case "$bump" in
      major) major_notes="${major_notes}${entry}"$'\n' ;;
      minor) minor_notes="${minor_notes}${entry}"$'\n' ;;
      patch) patch_notes="${patch_notes}${entry}"$'\n' ;;
    esac

    [ "$r" -le "$highest" ] || highest="$r"
    count=$((count + 1))
  done <<< "$(changeset_files)"
}

next_version() {
  local cur="$1" level="$2"
  local major minor patch
  IFS=. read -r major minor patch <<< "${cur%%-*}"
  case "$level" in
    3) echo "$((major + 1)).0.0" ;;
    2) echo "${major}.$((minor + 1)).0" ;;
    1) echo "${major}.${minor}.$((patch + 1))" ;;
    *) echo "$cur" ;;
  esac
}

render_notes() {
  local out=""
  [ -z "$major_notes" ] || out="${out}### Major Changes"$'\n\n'"${major_notes}"$'\n'
  [ -z "$minor_notes" ] || out="${out}### Minor Changes"$'\n\n'"${minor_notes}"$'\n'
  [ -z "$patch_notes" ] || out="${out}### Patch Changes"$'\n\n'"${patch_notes}"$'\n'
  printf '%s' "$out"
}

cmd_status() {
  collect
  cur="$(current_version)"
  if [ "$count" -eq 0 ]; then
    echo "No changesets. Version stays at ${cur}."
    return 0
  fi
  next="$(next_version "$cur" "$highest")"
  echo "${count} changeset(s): ${cur} -> ${next}"
  echo
  render_notes
}

cmd_version() {
  collect
  cur="$(current_version)"
  [ "$count" -gt 0 ] || die "no changesets to release"
  next="$(next_version "$cur" "$highest")"

  awk -v v="$next" '
    /^\[/ { in_pkg = ($0 == "[package]") }
    in_pkg && !done && /^version = "/ { print "version = \"" v "\""; done = 1; next }
    { print }
  ' "$manifest" > "$manifest.tmp"
  mv "$manifest.tmp" "$manifest"

  header="# reveal"
  existing=""
  if [ -f "$changelog" ]; then
    existing="$(awk 'NR > 1 || $0 != "# reveal"' "$changelog" | sed -e '/./,$!d')"
  fi

  {
    printf '%s\n\n' "$header"
    printf '## %s\n\n' "$next"
    render_notes
    [ -z "$existing" ] || printf '%s\n' "$existing"
  } > "$changelog"

  changeset_files | while IFS= read -r file; do
    [ -n "$file" ] || continue
    rm -f "$file"
  done

  if command -v cargo >/dev/null 2>&1; then
    cargo update --workspace --offline >/dev/null 2>&1 \
      || cargo update --workspace >/dev/null 2>&1 \
      || echo "warning: could not refresh Cargo.lock" >&2
  fi

  echo "$next"
}

cmd_notes() {
  collect
  [ "$count" -gt 0 ] || die "no changesets to release"
  render_notes
}

case "${1:-status}" in
  status) cmd_status ;;
  version) cmd_version ;;
  notes) cmd_notes ;;
  *) die "usage: $0 [status|version|notes]" ;;
esac
