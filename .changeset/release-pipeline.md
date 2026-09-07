---
reveal: patch
---

Harden the release pipeline. Every publish now verifies the exact commit being
released with the full test, lint, feature and workflow checks before any
artifact or public channel is touched, the Flatpak repository is deployed to
GitHub Pages and checked for reachability instead of only pushed to a branch,
the Flatpak bundles are collected into the GitHub release, a draft release no
longer updates Homebrew, crates.io or the Flatpak remote, the artifact set is
checked for completeness before publishing, pull request checks cover
packaging, resources and workflow changes, and uninstalling on Windows removes
only Reveal's own PATH entry.
