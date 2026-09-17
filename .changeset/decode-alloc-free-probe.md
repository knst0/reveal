---
reveal: patch
---

Cut per-file allocations on the decode path: match extensions case-insensitively without lowering a `String`, resolve the raster format from the path, and move decoded pixels with `into_rgba8` instead of copying them with `to_rgba8`.
