# Changesets

Release notes for `reveal` come from changeset files in this directory, not from
commit messages.

## Adding a changeset

Create a markdown file here (any name, e.g. `fix-zoom-jitter.md`):

```markdown
---
reveal: patch
---

Fix zoom jitter when scrolling with a trackpad.
```

The bump type is `major`, `minor`, or `patch`. The body is the changelog entry;
it may span multiple paragraphs.

Add one changeset per user-visible change. Changes that do not affect users
(refactors, CI, tests) need no changeset.

## Releasing

A maintainer runs the **Release** workflow (`workflow_dispatch`). One run does
everything:

1. Computes the next version from the pending changesets, applies the bump to
   `Cargo.toml`/`Cargo.lock`, rewrites `CHANGELOG.md`, deletes the consumed
   changesets, and commits that on a fresh `release/vX.Y.Z` branch.
2. Builds every target from that branch, then tags it and publishes the GitHub
   Release, the Homebrew cask, the crates.io release, and the Flatpak repo.
3. Opens a `chore(release): vX.Y.Z` PR from `release/vX.Y.Z` into `main`.

Merge that PR to finish — use a **merge commit, not a squash**, so the tagged
commit stays reachable from `main`. Until it is merged, `main` keeps the old
version, so nothing is bumped unless the release actually succeeded.

`.github/scripts/changeset.sh` does the version work and can be run locally:

- `changeset.sh status` — show pending changes and the next version
- `changeset.sh version` — bump `Cargo.toml`, write `CHANGELOG.md`, delete the
  consumed changesets
