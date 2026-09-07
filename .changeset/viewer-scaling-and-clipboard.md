---
reveal: patch
---

Fix image scaling, presentation and clipboard defects. Downscaling no longer
enlarges extreme aspect ratios into multi-gigabyte buffers, Fit to Window and
Original Size follow viewport and DPI changes, the zoom percentage is reported
against the source image, opening a file by a bare relative name no longer
leaves an empty window, opening a folder no longer blocks the interface,
copied images keep their EXIF orientation and hold the clipboard selection, and
delete confirmation is bound to the image it was requested for.
