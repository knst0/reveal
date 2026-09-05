---
reveal: minor
---

Decode AVIF (stills, animations and grid images) with rav1d and zenavif-parse, and drop `dds` from the advertised formats. Extension support is now gated on `ImageFormat::reading_enabled()`, so the format list only advertises what actually decodes.
