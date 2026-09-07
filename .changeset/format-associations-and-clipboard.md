---
reveal: patch
---

Register every decodable format and fix Linux integration. File associations
now cover all RAW, EXR, QOI, farbfeld, SVGZ and PNM variants instead of only
sixteen extensions, a test keeps the association table in step with the
decoders, the Wayland clipboard backend is enabled on Linux so copy and paste
work inside the Flatpak sandbox, and setting default applications from a
Flatpak now targets the host rather than the sandbox.
