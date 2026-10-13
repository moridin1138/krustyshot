# Changelog

All notable changes to Rustshot are recorded here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html). See [docs/RELEASING.md](docs/RELEASING.md) for how versions are chosen.

## [Unreleased]

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

[Unreleased]: https://github.com/OWNER/rustshot/commits/main
