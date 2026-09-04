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

`scripts/changeset.sh` consumes them:

- `scripts/changeset.sh status` — show pending changes and the next version
- `scripts/changeset.sh version` — bump `Cargo.toml`, write `CHANGELOG.md`,
  delete the consumed changesets

The release workflow runs this automatically.
