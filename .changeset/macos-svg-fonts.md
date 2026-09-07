---
reveal: patch
---

Fix missing text in SVGs on macOS. User-installed fonts from `~/Library/Fonts`
are now loaded, macOS font families are recognised when resolving the generic
serif, sans-serif, cursive and monospace families, and symbol-only faces are no
longer picked as the fallback font.
