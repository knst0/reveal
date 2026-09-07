---
reveal: patch
---

Fix the Windows installer failing with `RegCreateKeyEx failed; code 87` when
adding Reveal to the PATH. Write the system PATH to its actual location under
`Session Manager\Environment` instead of the non-existent `HKLM\Environment`,
and read the same key when checking whether the entry is already present.
