---
reveal: minor
---

Decode AVIF and AVIF sequences with zenavif, and drop `dds` from the advertised formats. Extension support is now gated on `ImageFormat::reading_enabled()`, so the format list only advertises what actually decodes.

Note: zenavif is licensed AGPL-3.0-only OR LicenseRef-Imazen-Commercial.
