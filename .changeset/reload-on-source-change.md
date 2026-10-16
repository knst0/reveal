---
reveal: minor
---

Reload the shown image when another program modifies it on disk: the file's size and modification time are polled every 250 ms, and once a new stamp is stable across two polls the image is re-decoded and re-rendered in place. Smooth scaling is now off by default.
