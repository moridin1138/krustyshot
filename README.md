# Rustshot

A cross-platform screenshot tool modelled on [Greenshot](https://getgreenshot.org/), written in Rust with [egui](https://github.com/emilk/egui). It runs on macOS, Windows and Linux.

<img src="assets/icons/icon_256.png" width="96" alt="Rustshot icon">

## Features

- Menu-bar / tray icon with a capture menu
- Global hotkeys for region, window, full screen and last region captures
- Frozen-screen region selection with crosshair, magnifier and a pixel-size readout
- Window capture: hover to highlight, click to capture (Space switches between region and window mode, like Greenshot)
- Full-screen capture of all monitors, stitched together
- Capture last region
- Destination picker after each capture: open in editor, copy, save directly, save as
- Image editor with Phosphor icons: select, move and resize, rectangle, ellipse, line, arrow, freehand, text, highlight, obfuscate (pixelate or blur), crop, undo and redo, line colour, thickness and fill
- Open an image from the clipboard or from a file
- Preferences: hotkeys, destinations, output folder, Greenshot-style filename pattern, PNG/JPG/BMP/GIF, JPEG quality

See [CHANGELOG.md](CHANGELOG.md) for what changed in each version and [docs/ROADMAP.md](docs/ROADMAP.md) for what's planned.

## Default hotkeys

| Action | macOS | Windows / Linux |
|---|---|---|
| Capture region | ⌥⇧R | Print Screen |
| Capture window | ⌥⇧W | Alt+Print Screen |
| Capture full screen | ⌥⇧F | Ctrl+Print Screen |
| Capture last region | ⌥⇧L | Shift+Print Screen |

Change them in Preferences. macOS keeps ⌘⇧3/4/5 for its own screenshot tool.

### Editor shortcuts

| Key | Action |
|---|---|
| `R` `E` `L` `A` `F` `T` `H` `O` `C` | Rectangle, ellipse, line, arrow, freehand, text, highlight, obfuscate, crop |
| `Esc` | Select tool (or cancel crop / clear selection) |
| Shift while drawing | Squares and circles; lines snap to 45° |
| Delete | Remove the selected annotation |
| Enter | Apply crop |
| ⌘Z / ⌘⇧Z | Undo / redo |
| ⌘S / ⌘⇧S | Save / save as |
| ⌘C | Copy image |
| ⌘+ / ⌘− / ⌘0 | Zoom in / out / fit |

On Windows and Linux use Ctrl instead of ⌘.

## Install

Download the package for your platform from the [releases page](https://github.com/moridin1138/rustshot/releases).

- **macOS:** unzip and move `Rustshot.app` to Applications. Builds are not yet signed, so the first time right-click the app and choose Open.
- **Windows:** unzip and run `rustshot.exe`.
- **Linux:** extract and run `rustshot`. It needs GTK 3 and an appindicator library.

### macOS permissions

Captures need **Screen Recording** permission. Enable it in System Settings → Privacy & Security → Screen Recording for Rustshot (or for your terminal when running with `cargo run`), then restart Rustshot.

## Build from source

You need a stable Rust toolchain ([rustup](https://rustup.rs/)).

```bash
cargo run --release -p rustshot-app
```

On Linux, install the system libraries listed in [.github/workflows/ci.yml](.github/workflows/ci.yml) first.

Build a macOS app bundle:

```bash
cargo build --release -p rustshot-app
scripts/bundle-macos.sh
```

The bundle is written to `target/release/bundle/Rustshot.app` and runs as a menu-bar app with no Dock icon.

### Development commands

| Command | What it does |
|---|---|
| `cargo test` | Run the unit tests |
| `cargo clippy --all-targets -- -D warnings` | Lint (CI fails on any warning) |
| `cargo xtask icons` | Regenerate the app icons in `assets/icons/` |
| `scripts/bundle-macos.sh` | Package a release build as `Rustshot.app` (macOS) |

## Project documents

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): how the code is organised and how a capture flows through it
- [docs/RELEASING.md](docs/RELEASING.md): versioning rules and the release checklist
- [docs/ROADMAP.md](docs/ROADMAP.md): planned phases and the backlog
- [CHANGELOG.md](CHANGELOG.md): release history

## Settings

Settings live in `rustshot.toml` in the platform config folder:

- macOS: `~/Library/Application Support/org.Rustshot.Rustshot/`
- Windows: `%APPDATA%\Rustshot\Rustshot\config\`
- Linux: `~/.config/rustshot/`

## Credits

- Inspired by [Greenshot](https://getgreenshot.org/). Rustshot is a new implementation and contains no Greenshot code.
- Icons: [Phosphor Icons](https://phosphoricons.com/) (MIT), via [egui-phosphor](https://github.com/amPerl/egui-phosphor).

## License

Rustshot is free software under the [GNU General Public License v3.0 or later](LICENSE).
