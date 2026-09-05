---
reveal: patch
---

Build the display-ready image in the decode worker instead of the UI thread, and apply orientation after downscaling. Navigation no longer stalls the interface on large photos.
