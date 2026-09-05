# reveal

## 0.3.0

### Minor Changes

- 0748ef5: Decode AVIF (stills, animations and grid images) with rav1d and zenavif-parse, and drop `dds` from the advertised formats. Extension support is now gated on `ImageFormat::reading_enabled()`, so the format list only advertises what actually decodes.
- 0b1ebc2: Add "Open Image" (Ctrl+O) and "Open Folder" (Ctrl+Shift+O) to open a file or folder from a native dialog, also available from the context menu.
- 3b321f1: Implement "Paste Image" (Ctrl+V) to display an image from the clipboard.

### Patch Changes

- dd8dcaa: Cap the memory a single animation may use so a long GIF, APNG or WebP no longer exhausts memory.
- 0abb106: Open the window immediately instead of waiting for the first decode and a full directory scan; the folder is scanned in the background and dropped or dialog-opened files no longer freeze the interface.
- 2e3d472: Turn a decoder panic on a corrupt file into a status-bar error instead of terminating the app.
- 2e3d472: Decode the image being viewed before its prefetched neighbours, and cancel requests outside the prefetch window and on opening a new folder.
- 237cd11: Replace the unmaintained `directories-next` with `directories`.
- 6a325c9: Build the display-ready image in the decode worker instead of the UI thread, and apply orientation after downscaling. Navigation no longer stalls the interface on large photos.
- 06133c0: Fix "Don't show again" on the update toast not persisting to the config.
- 06133c0: Fix Enter, `+` and `-` key bindings not firing; legacy `return`/`plus`/`minus` names in saved configs are migrated.
- 0df41e9: Prepare images at the display's physical resolution so they stay sharp on HiDPI screens.
- 3376512: Slow the frame ticker while the viewer is idle so a static image no longer wakes the CPU 60 times a second, and stop cloning the whole entry list on every random slideshow step.
- 0df41e9: Fit, zoom anchoring and double-click hit testing now use the image area rather than the whole window, so the zoom percentage and the rendered image agree.
- 06133c0: Linux: write `reveal.desktop` with the current executable path before registering associations.
- 06133c0: Fix `reveal <folder>` showing a read error instead of the first image in the folder.
- 06133c0: Fix update checks treating a stable release as not newer than the pre-release it supersedes.
- 500fd83: Show the embedded preview when browsing RAW files, falling back to a full develop when the preview is too small.
- 237cd11: Avoid copying the whole file when probing PNG, GIF and WebP for animation, stop cloning the configuration for every settings row, and run "Set defaults" and update checks off the UI thread.
- 1698938: Map the generic SVG font families (`sans-serif`, `serif`, `monospace`, `cursive`, `fantasy`) onto fonts that are actually installed, so `<text>` no longer renders blank on systems without Arial or Times New Roman.
- e5cd6f0: Render `<text>` in SVGs by loading system fonts, and resolve relative `<image href>` against the file's own directory.
- 10c3242: Rasterise SVGs above the display size so zooming into a vector image stays sharp, capped by a pixel budget.
- 06133c0: Fix a fast trackpad gesture collapsing zoom to 1%; the wheel factor is now exponential.
- 0df41e9: Report the original image size in the status bar and make "Original" show true 1:1 pixels instead of the downscaled copy.
- 3b321f1: Restore the window position, size, maximized and fullscreen state between runs, and honour the "Start fullscreen" setting.

## 0.2.1

### Patch Changes

- 8fdde9b: Fix update checks and automatic installation, which silently did nothing.
- b88e580: Show the application name as "Reveal" instead of "reveal" on Windows.

## 0.2.0

### Minor Changes

- 44f966f: Add a "Set defaults" button in Settings that registers Reveal as a handler for every supported image format. macOS releases now ship a `Reveal.app` bundle, without which Reveal could not be selected as a default application at all.

### Patch Changes

- 0702554: Release the left mouse button drag state when the pointer leaves the window.

## 0.1.0

### Minor Changes

- 62e7d0d: Initial release of reveal, a fast image viewer.
