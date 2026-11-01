# Changelog

All notable changes to Krustyshot are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html). See [docs/RELEASING.md](docs/RELEASING.md) for how versions are chosen.

## [Unreleased]

### Fixed

- Windows: Krustyshot failed to start on a PC without Microsoft's Visual C++ Redistributable ("VCRUNTIME140.dll was not found"). It now includes that runtime.
- In Settings, a long description (such as the one under **Full resolution** in Recording) ran underneath its switch; it now wraps beside it.

## [0.6.0] - 2026-10-29

Rustshot is now **Krustyshot**.

### Changed

- New name: Rustshot is now Krustyshot, because another screenshot app is called Rustshot. The app, its menus, the downloads (`Krustyshot.app`, `krustyshot.exe`) and the GitHub address (github.com/moridin1138/krustyshot) use the new name; the old address redirects. Settings, shortcuts and macOS permissions carry over. Delete the old Rustshot app after installing; Open at login switches to Krustyshot the first time it runs.

## [0.5.1] - 2026-10-29

Sound in recordings, scrolling screenshots, and more places to send captures: other apps, the printer, email, Share, text recognition and your own upload server. (0.5.0 was tagged but never published; its changes are included here.)

### Added

- **Sound in MP4 recordings:** the microphone (with a choice of device) and system sound, each turned on separately in Settings → Recording and mixed into one track. System sound needs macOS 13 or later; on Windows it records what the default speakers or headphones play. Both are off by default. While recording, the bar shows a microphone and a speaker icon that blink with the red dot; click one to mute or unmute that source. On macOS, Krustyshot asks for microphone access when you turn the microphone on in Settings.
- **Show mouse clicks** (Settings → Recording): a dot while the button is held and a fading ring where you click.
- While recording a region or a window, the rest of the display is dimmed and a thin red line marks the recorded area. Clicks go straight through, and none of it appears in the recording.
- MP4 recording on Windows, using Windows' own H.264 encoder (Media Foundation), with the same quality, resolution and frame-rate settings as on macOS.
- On Windows 10 (version 2004) and later, the recording bar is hidden from the recording, as on macOS.
- New destinations, in the quick menu, the after-capture settings and the editor's new send menu: **Open in another app** (the default image app, or one chosen in Settings), **Print** (⌘P / Ctrl+P in the editor), **Email** (a new message with the image attached), **Share** on macOS (AirDrop, Messages, Notes and more) and **Copy text**, which recognises the text in the image (Vision on macOS, Windows' built-in OCR, Tesseract on Linux) and copies it.
- **Custom uploader** (Settings → Uploads): send captures to your own server or an image host and copy the link it returns. Set the address, POST or PUT, a form field or the raw file, headers (such as an API key), extra form fields, and where the link is in the reply (`$json:data.link$`). ShareX uploader files (`.sxcu`) can be imported, and **Test upload** checks the settings. Once set up, "Upload to …" appears in the quick menu, the after-capture settings and the editor.
- Automatic update check (Settings → General, on by default): shortly after start-up and once a day Krustyshot asks GitHub for the latest version number. A new version is announced once with a Download button, and the menu-bar menu offers it until you update.
- Settings → About has **View on GitHub**, **Release notes** and **Report a problem** buttons right under the version.
- **Capture scrolling region** (menu, or ⌥⇧S on macOS and Ctrl+Alt+Print Screen elsewhere): drag over something taller than the screen, such as a web page or document, and Krustyshot scrolls it for you and joins the screenshots into one tall image. It stops at the bottom, when you click, or at 25,000 pixels. Fixed headers and footers appear once. The image goes to your after-capture destinations like any screenshot. On macOS it needs the Accessibility permission, which it asks for the first time; on Linux it needs X11.
- Krustyshot keeps a log file (Settings → About → Log file; `~/Library/Logs/Rustshot/` on macOS), with the previous run's log kept alongside, to help when reporting a problem.

### Changed

- **About Krustyshot** in the menu-bar menu opens Settings → About instead of a separate window.
- The recording bar keeps every item in a fixed place, so the timer, size and icons don't shift as they change.
- Releases are published as the latest release rather than as pre-releases; only versions with a suffix such as `-beta.1` are pre-releases.

### Fixed

- On recent macOS, screenshots included the mouse pointer even with **Include mouse pointer** turned off, because the system's older capture call adds it. macOS 14 and later now capture with ScreenCaptureKit, which leaves it out; the setting draws the pointer when it's on, as before.

## [0.4.0] - 2026-10-14

Screen recording.

### Added

- Screen recording from the menu, matching the screenshot options: **Record region…** (drag an area), **Record window…** (click a window) and **Record full screen** (the display the pointer is on). The record shortcut (⌥⇧V on macOS, Ctrl+Shift+Print Screen elsewhere) starts a region recording or stops the current one. A small bar shows the time and size so far, with Stop and Discard (Discard asks for confirmation, since the video is deleted). On macOS the bar is hidden from the recording, and the menu-bar icon turns red while recording.
- MP4 recordings on macOS (the system's own H.264 encoder) and GIF recordings on every platform. GIFs store only what changed between frames, so screen recordings stay small.
- Recording settings: format, quality (Low, Medium, High), full or half resolution on Retina displays, frame rate, and an estimate of the size per minute.
- Low disk space warning: when the disk holding your screenshots folder has 5% or less free, Krustyshot warns before recording (and before screenshots, at most every 30 minutes), with options to continue, choose another folder, or cancel. Settings > Saving shows the free space.
- A recording in progress stops and keeps what it has if the disk gets critically low (under 1% or 512 MB), and quitting Krustyshot saves an active recording first.

## [0.3.0] - 2026-10-14

Settings redesign and multilingual interface.

### Added

- Redesigned Settings window (was Preferences): a sidebar with General, Capture, Shortcuts, Saving and About sections, and every change applies immediately.
- Shortcut recorder: click a shortcut and press the new keys, instead of typing names like `alt+shift+KeyR`.
- Filename buttons that insert the date, time, window title or user name, with a live example.
- Open at login.
- Check for updates (About section), which asks GitHub only when you click it.
- `--settings` launch option to open Settings straight away.
- French, Spanish and German translations (machine-drafted). The language follows the system and can be changed in Settings > General. See [docs/TRANSLATING.md](docs/TRANSLATING.md) to improve a translation or add a language.

### Changed

- The interface uses each system's own font (SF Pro on macOS, Segoe UI on Windows, Cantarell, Noto Sans or DejaVu Sans on Linux) at slightly larger sizes, with higher-contrast secondary text. Text typed into screenshots keeps the bundled font so it looks the same everywhere.

## [0.2.0] - 2026-10-14

Phase 2: Greenshot editor parity.

### Added

- Speech bubbles (`S`): a rounded box with text and a tail you can drag to point at anything.
- Step counters (`I`): numbered circles that renumber themselves when one is deleted.
- Highlight styles: marker (as before), spotlight (darken everything else), grey out the rest, and magnify.
- Soft drop shadows on shapes, on by default and switchable per annotation.
- Effects menu in the editor: drop shadow, torn edges, border, greyscale, invert colours, rotate left and right, and resize. Annotations move, rotate and scale with the image.
- Region captures can span several displays; the result keeps the sharpest display's resolution.
- Option to include the mouse pointer in captures.
- Capture delay of up to 30 seconds.
- A small notification after saving or copying; click it to show the file in Finder or Explorer.
- Choose what a left click on the menu-bar icon does: open the menu, or start a capture (right click then opens the menu).

### Changed

- Full-screen captures of several displays are assembled the same way as spanning regions.
- JPG and BMP files put transparent areas (from drop shadow or torn edges) on white instead of black.

## [0.1.1] - 2026-10-13

### Fixed

- macOS: captures no longer silently show only the desktop background when Krustyshot lacks Screen Recording permission. Krustyshot now explains how to grant it, with buttons to open System Settings, reset a stale permission entry, and quit so it can be reopened.
- macOS: only one permission prompt appears at startup, instead of Apple's prompt and Krustyshot's window competing.
- macOS: fixed a crash after a few minutes to an hour of use, caused by an error inside AppKit's Touch Bar support (even on Macs without a Touch Bar). Krustyshot now catches that error instead of quitting.
- The capture overlay could take a screenshot by itself if a mouse button was released just after it opened. A capture now needs a click or drag that starts inside the overlay.
- macOS: Mission Control no longer zooms all the way out on the desktop where Krustyshot is running. Krustyshot's hidden background window is now kept on-screen and excluded from Mission Control.
- Esc now closes Krustyshot's message windows.
- Shortcut hints in tooltips showed empty boxes for the ⇧ and ⌥ symbols. They now read, for example, "Cmd+Shift+S" on macOS and "Ctrl+Shift+S" elsewhere.

### Changed

- macOS builds are signed with Krustyshot's own certificate, so macOS keeps the Screen Recording permission when you update. You'll need to grant it once more for 0.1.1. Builds are still not notarised by Apple, so the first launch needs right-click and Open.
- Licence changed from GPL-3.0-or-later to the [PolyForm Shield License 1.0.0](LICENSE): free for any use, including commercial, except providing a product that competes with Krustyshot (such as reselling it). Version 0.1.0 remains available under GPL-3.0-or-later.

## [0.1.0] - 2026-10-13

First release, licensed under GPL-3.0-or-later.

### Added

- Menu-bar / tray app with capture menu, open from clipboard or file, open output folder, preferences and about.
- Global hotkeys for region, window, full screen and last region captures, with platform-specific defaults.
- Frozen-screen capture overlay on every monitor, with crosshair, magnifier, pixel-size readout, window highlighting and Space to switch between region and window mode.
- Full-screen capture that stitches all monitors, including mixed Retina and standard displays.
- Destination picker plus configurable destinations: editor, clipboard, save directly, save as.
- Image editor: select, move and resize, rectangle, ellipse, line, arrow, freehand, text, highlight, pixelate and blur obfuscation, crop, undo and redo, colours, thickness, fill, zoom.
- Phosphor icons for the editor toolbar.
- Greenshot-compatible filename patterns and PNG, JPG, BMP and GIF output.
- Preferences window with hotkey validation and filename preview.
- macOS Screen Recording permission check with a shortcut to System Settings.
- App icon, `cargo xtask icons` to regenerate it, and macOS bundle metadata.
- CI on macOS, Windows and Linux, and a tag-driven release workflow.

[Unreleased]: https://github.com/moridin1138/krustyshot/compare/v0.6.0...HEAD
[0.6.0]: https://github.com/moridin1138/krustyshot/compare/v0.5.1...v0.6.0
[0.5.1]: https://github.com/moridin1138/krustyshot/compare/v0.4.0...v0.5.1
[0.4.0]: https://github.com/moridin1138/krustyshot/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/moridin1138/krustyshot/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/moridin1138/krustyshot/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/moridin1138/krustyshot/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/moridin1138/krustyshot/releases/tag/v0.1.0
