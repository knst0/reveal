---
reveal: patch
---

Fix memory growth when the shown image is replaced: the previous image is now evicted from the GPU sprite atlas instead of being kept for the lifetime of the window. Most visible with on-disk reload, where every save of the watched file retained another full display buffer.
