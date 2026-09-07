---
reveal: patch
---

Fix the interface rendering without any text on macOS. `gpui_platform` was built
without the `font-kit` feature, so `gpui_macos` had no font backend and silently
skipped every glyph in the toolbar, status bar, menus and settings.
