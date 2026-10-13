# Rustshot

A cross-platform screenshot tool modelled on [Greenshot](https://getgreenshot.org/), written in Rust with [egui](https://github.com/emilk/egui). Runs on macOS, Windows and Linux.

## Features (Phase 1)

- Menu-bar / tray icon with a capture menu
- Global hotkeys for region, window, full screen and last region
- Frozen-screen region selection with crosshair, magnifier and pixel-size readout
- Window capture: hover to highlight, click to capture (Space switches between region and window mode, like Greenshot)
- Full-screen capture of all monitors, stitched together
- "Capture last region"
- Destination picker after each capture: open in editor, copy, save directly, save as
- Image editor: select / move / resize, rectangle, ellipse, line, arrow, freehand, text, highlight, obfuscate (pixelate or blur), crop, undo/redo, line colour, thickness and fill
- Open an image from the clipboard or from a file
- Preferences: hotkeys, destinations, output folder, Greenshot-style filename pattern, PNG/JPG/BMP/GIF, JPEG quality

## Default hotkeys

| Action | macOS | Windows / Linux |
|---|---|---|
| Capture region | ⌥⇧R | Print Screen |
| Capture window | ⌥⇧W | Alt+Print Screen |
| Capture full screen | ⌥⇧F | Ctrl+Print Screen |
| Capture last region | ⌥⇧L | Shift+Print Screen |

You can change them in Preferences. macOS keeps ⌘⇧3/4/5 for itself.

### Editor shortcuts

`R` rectangle, `E` ellipse, `L` line, `A` arrow, `F` freehand, `T` text, `H` highlight, `O` obfuscate, `C` crop, `Esc` select. ⌘Z / ⌘⇧Z undo/redo, Delete removes the selection, Enter applies a crop, Shift while drawing makes squares/circles or snaps lines to 45°, ⌘S save, ⌘⇧S save as, ⌘C copy, ⌘+/⌘−/⌘0 zoom.

## Build and run

```bash
cargo run --release -p rustshot-app
```

On macOS, if `cargo` is not found, Homebrew's rustup is not on your PATH:

```bash
echo 'export PATH="/opt/homebrew/opt/rustup/bin:$PATH"' >> ~/.zshrc
```

### macOS permissions

The first capture needs **Screen Recording** permission. Enable it in System Settings → Privacy & Security → Screen Recording for Rustshot (or for your terminal when running with `cargo run`), then restart Rustshot.

### macOS app bundle

```bash
cargo install cargo-bundle
cargo bundle --release -p rustshot-app
```

This produces `target/release/bundle/osx/Rustshot.app`, which runs as a menu-bar app with no Dock icon.

### Linux

You need GTK and appindicator development packages (see `.github/workflows/ci.yml`). Global hotkeys and window picking need X11; under Wayland use the tray menu.

## Layout

| Crate | Purpose |
|---|---|
| `crates/rustshot-core` | Capture (xcap), config, filename patterns, saving, clipboard |
| `crates/rustshot-editor` | Annotation editor UI, document model, export renderer |
| `crates/rustshot-app` | Tray, hotkeys, capture overlay, windows (the `rustshot` binary) |

Settings are stored in `rustshot.toml` in the platform config folder (on macOS: `~/Library/Application Support/org.Rustshot.Rustshot/`).

## Roadmap

- **Phase 2:** speech bubbles, step counters, more highlight modes (text, grayscale, magnify), drop shadows, effects (torn edges, border, grayscale, invert, rotate, resize), mouse pointer in captures, capture delay, notifications
- **Phase 3:** printing, open in external app, email, upload plugins (Imgur etc.), OCR

## License

GPL-3.0-or-later (the same licence as Greenshot).
