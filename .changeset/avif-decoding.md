---
reveal: patch
---

Fix AVIF decoding defects. Animated files keep their AV1 decoder state across
frames so sequences with inter-frame dependencies decode, images using the
identity matrix coefficients keep their RGB channels, container `irot` and
`imir` rotation is honoured when no EXIF orientation is present, and each
decoder instance is limited to one worker thread so concurrent decodes no
longer oversubscribe the machine.
