---
reveal: patch
---

Do not let the built-in updater overwrite a binary a package manager owns. A
Homebrew bundle, a Windows installer and a Flatpak now report the command that
upgrades them instead of replacing themselves in place, which on macOS would
have broken the app's signature. The Flatpak also builds without the updater.
