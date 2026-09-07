# reveal

## 0.4.1

### Patch Changes

- 6c893a3: Fix AVIF decoding defects. Animated files keep their AV1 decoder state across
  frames so sequences with inter-frame dependencies decode, images using the
  identity matrix coefficients keep their RGB channels, container `irot` and
  `imir` rotation is honoured when no EXIF orientation is present, and each
  decoder instance is limited to one worker thread so concurrent decodes no
  longer oversubscribe the machine.
- 6835ba7: Register every decodable format and fix Linux integration. File associations
  now cover all RAW, EXR, QOI, farbfeld, SVGZ and PNM variants instead of only
  sixteen extensions, a test keeps the association table in step with the
  decoders, the Wayland clipboard backend is enabled on Linux so copy and paste
  work inside the Flatpak sandbox, and setting default applications from a
  Flatpak now targets the host rather than the sandbox.
- b436589: Open images handed over by the desktop environment. Files opened from Finder or
  through "Open With" now reach the viewer, both when they start Reveal and when
  it is already running, instead of being dropped.
- 738fc2b: Fix missing text in SVGs on macOS. User-installed fonts from `~/Library/Fonts`
  are now loaded, macOS font families are recognised when resolving the generic
  serif, sans-serif, cursive and monospace families, and symbol-only faces are no
  longer picked as the fallback font.
- 592b06c: Harden the release pipeline. Every publish now verifies the exact commit being
  released with the full test, lint, feature and workflow checks before any
  artifact or public channel is touched, the Flatpak repository is deployed to
  GitHub Pages and checked for reachability instead of only pushed to a branch,
  the Flatpak bundles are collected into the GitHub release, a draft release no
  longer updates Homebrew, crates.io or the Flatpak remote, the artifact set is
  checked for completeness before publishing, pull request checks cover
  packaging, resources and workflow changes, and uninstalling on Windows removes
  only Reveal's own PATH entry.
- 3cda8c0: Fix image scaling, presentation and clipboard defects. Downscaling no longer
  enlarges extreme aspect ratios into multi-gigabyte buffers, Fit to Window and
  Original Size follow viewport and DPI changes, the zoom percentage is reported
  against the source image, opening a file by a bare relative name no longer
  leaves an empty window, opening a folder no longer blocks the interface,
  copied images keep their EXIF orientation and hold the clipboard selection, and
  delete confirmation is bound to the image it was requested for.
- 4219df2: Fix the Windows installer failing with `RegCreateKeyEx failed; code 87` when
  adding Reveal to the PATH. Write the system PATH to its actual location under
  `Session Manager\Environment` instead of the non-existent `HKLM\Environment`,
  and read the same key when checking whether the entry is already present.

## 0.4.0

### Minor Changes

- 697bd4a: Ship native installers and packages: `reveal-setup.exe` for Windows, a macOS
  universal `.app` and `.dmg`, a single Homebrew cask that installs the app and
  puts `reveal` on the PATH, and a Flatpak with a signed repository.

## 0.3.2

### Patch Changes

- a9403e1: Magnify from the original pixels instead of the downscaled display copy, so zooming into a large image with nearest-neighbour sampling shows true detail rather than an upscaled blur. Also drop the redundant full-resolution buffer the display copy kept alongside its render image.

## 0.3.1

### Patch Changes

- 1fa2283: Switch gpui to the zed v1.18.0 git dependency and migrate to its updated APIs (`Application::with_platform`, `paint_image` bounds, `flex_grow`, `Window::focus`, executor timers).

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
