---
reveal: patch
---

Fix windows freezing after launch on Wayland: recreate the Vulkan swapchain when surface acquisition fails with `ERROR_OUT_OF_DATE_KHR` instead of rendering frames that are never presented.
