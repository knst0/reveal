---
reveal: patch
---

Magnify from the original pixels instead of the downscaled display copy, so zooming into a large image with nearest-neighbour sampling shows true detail rather than an upscaled blur. Also drop the redundant full-resolution buffer the display copy kept alongside its render image.
