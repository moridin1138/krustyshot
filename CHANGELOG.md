# Changelog

All notable changes to Rustshot are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html). See [docs/RELEASING.md](docs/RELEASING.md) for how versions are chosen.

## [Unreleased]

## [0.1.1] - 2026-10-13

### Fixed

- macOS: captures no longer silently show only the desktop background when Rustshot lacks Screen Recording permission. Rustshot now explains how to grant it, with buttons to open System Settings, reset a stale permission entry, and quit so it can be reopened.
- macOS: only one permission prompt appears at startup, instead of Apple's prompt and Rustshot's window competing.
- macOS: fixed a crash when closing windows, caused by AppKit's Touch Bar support.
- macOS: Mission Control no longer zooms all the way out on the desktop where Rustshot is running. Rustshot's hidden background window is now kept on-screen and excluded from Mission Control.
- Shortcut hints in tooltips showed empty boxes for the ⇧ and ⌥ symbols. They now read, for example, "Cmd+Shift+S" on macOS and "Ctrl+Shift+S" elsewhere.

### Changed

- Licence changed from GPL-3.0-or-later to the [PolyForm Shield License 1.0.0](LICENSE): free for any use, including commercial, except providing a product that competes with Rustshot (such as reselling it). Version 0.1.0 remains available under GPL-3.0-or-later.

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

[Unreleased]: https://github.com/moridin1138/rustshot/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/moridin1138/rustshot/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/moridin1138/rustshot/releases/tag/v0.1.0
