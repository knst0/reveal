---
reveal: patch
---

Defer the full-resolution `Nearest` base until magnification actually needs it: preparing a frame no longer clones a dead `width*height*4` copy for zoom-1 views, and zooming out releases it again. Also check staged keys against the window without cloning them into a throwaway `Vec`.
