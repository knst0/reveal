---
reveal: patch
---

Fix webtoon strips rendering as blank: the display buffer kept the full
720x30000 size, which exceeds the GPU texture limit. The long edge is now
capped at 8192px with the aspect ratio preserved.
