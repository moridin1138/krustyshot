# Architecture

Rustshot is a Cargo workspace with three crates plus a maintenance tool. Each crate depends only on the ones above it in this table:

| Crate | Kind | Responsibility |
|---|---|---|
| `crates/rustshot-core` | library, no UI | Screen capture, geometry, settings, filename patterns, saving and clipboard |
| `crates/rustshot-editor` | library, egui | The annotation editor: document model, undo history, rendering and export |
| `crates/rustshot-app` | binary `rustshot` | Tray icon, hotkeys, capture overlay, windows, and wiring everything together |
| `xtask` | binary, dev only | `cargo xtask icons` regenerates the app icon |

`rustshot-editor` does not depend on `rustshot-core`. It receives an image and hands back requests (save, copy, close) for the app to carry out, so it could be reused elsewhere.

## Key libraries

| Library | Used for |
|---|---|
| [`eframe`/`egui`](https://github.com/emilk/egui) | All windows and UI (one OS window per egui viewport) |
| [`xcap`](https://github.com/nashaofu/xcap) | Capturing monitors and listing windows on macOS, Windows and Linux |
| [`tray-icon`](https://github.com/tauri-apps/tray-icon) + `muda` | Tray / menu-bar icon and menu |
| [`global-hotkey`](https://github.com/tauri-apps/global-hotkey) | System-wide hotkeys |
| [`arboard`](https://github.com/1Password/arboard) | Clipboard images and text |
| [`tiny-skia`](https://github.com/RazrFalcon/tiny-skia) + `ab_glyph` | Anti-aliased rendering of annotations when exporting |
| [`image`](https://github.com/image-rs/image) | Encoding PNG, JPG, BMP and GIF; pixel effects |
| [`rfd`](https://github.com/PolyMeilex/rfd) | Native open, save and message dialogs |
| [`egui-phosphor`](https://github.com/amPerl/egui-phosphor) | Phosphor icon font for the toolbar |

## Coordinates

There are three coordinate spaces, and mixing them up is the most likely source of bugs:

1. **Global logical points:** the whole desktop, origin at the top-left of the primary monitor. `Region`, `MonitorShot::rect`, `WindowInfo::rect` and the saved "last region" use this. On macOS xcap already reports points; on Windows and Linux `capture.rs` divides xcap's physical pixels by each monitor's scale factor.
2. **Image pixels:** the captured bitmap. A Retina monitor has 2 image pixels per point (`MonitorShot::scale()`). Editor elements are stored in image pixels.
3. **egui screen points:** positions inside a particular window. The overlay maps these to global points from the window's actual size; the editor maps them to image pixels through its `View` (origin + scale).

## The app shell (`rustshot-app`)

eframe always has a root window. Rustshot keeps it as a 1×1, transparent, click-through window in the top-left corner (`platform::hide_background_window` also hides it from Mission Control and window cycling), and opens everything else as child viewports. It must stay on-screen: an off-screen window makes Mission Control zoom out to fit it. The root can't simply be hidden: eframe stops running the UI of a hidden root, and child windows can only be opened from that UI.

On macOS the event loop is set to the Accessory activation policy, so there is no Dock icon or app menu (`platform.rs`). The bundle's `Info.plist` (`crates/rustshot-app/bundle/Info.plist`, filled in by `scripts/bundle-macos.sh`) sets `LSUIElement` for the same effect when launched as an `.app`.

Tray menu clicks and hotkey presses arrive on callbacks. These push an `AppEvent` onto a channel and call `request_repaint()`; `App::logic` drains the channel on the next frame.

`App::ui` runs every frame and shows whichever windows are active, each as an immediate viewport:

| Module | Window |
|---|---|
| `overlay.rs` | One borderless overlay per monitor during a capture |
| `app.rs` (`show_picker`) | The "what next?" destination picker |
| `app.rs` (`show_editors`) | One editor window per open image |
| `settings_ui.rs` | Preferences |
| `app.rs` (`show_notices`) | Small message windows (errors, permissions, about) |

## How a capture flows

```
hotkey / tray click
   │  AppEvent on channel (tray clicks wait 250 ms so the menu has closed)
   ▼
App::start_capture(mode)
   │  capture::take_snapshot(): grab every monitor (+ window list) at once
   ▼
Region / Window ─────► Overlay (frozen screenshot on each monitor)
   │                     user drags a region or clicks a window
   │                     Snapshot::crop() → RgbaImage
Fullscreen ──► Snapshot::stitch_all()
Last region ─► Snapshot::crop(config.last_region)
   │
   ▼
App::deliver(Capture)
   ├─ Picker      → destination picker window → one of the below
   ├─ Editor      → new editor window
   ├─ Clipboard   → output::copy_image_to_clipboard
   ├─ File        → output::auto_path + save_to
   └─ FileAs      → save dialog + save_to
```

The screen is captured before any Rustshot window appears, so the overlay shows a frozen image and never captures itself, the same approach Greenshot uses. On macOS `platform::raise_overlays` lifts the overlay windows above the menu bar and Dock and lets them appear over full-screen apps.

## The editor (`rustshot-editor`)

| File | Contents |
|---|---|
| `document.rs` | `Document` (shared base image + `Vec<Element>`), `Element`, `Style`, hit testing, crop, and `History` |
| `raster.rs` | Pixel effects: pixelate, blur, highlight |
| `export.rs` | `flatten()`: composites raster effects, then draws shapes and text with tiny-skia |
| `editor.rs` | The egui UI: toolbar, properties bar, canvas, tools and shortcuts |

There are two kinds of element:

- **Vector** elements (rectangle, ellipse, line, arrow, freehand, text) are drawn on top of the image. The canvas paints them with egui; export paints them with tiny-skia.
- **Raster** elements (highlight, obfuscate) change the pixels underneath. They are baked into the canvas texture whenever the document revision changes, and into the exported image by `raster::composite`.

Undo works by snapshot. Before each change the whole `Document` is pushed onto `History`. The base image is behind an `Arc`, so a snapshot only copies the element list, except after a crop, which creates a new base image.

## Settings (`rustshot-core::config`)

`Config` is serialised as TOML with `#[serde(default)]`, so older or partial files load with defaults for anything missing. The path comes from the `directories` crate (see the README for each platform).

## Testing

Unit tests sit next to the code and cover the parts that don't need a display: geometry and crop maths, Retina scaling, stitching, filename patterns, config round-trips, saving every format, hit testing, undo and redo, crop, the pixel effects, and export rendering. Run them with `cargo test`. Capture and UI behaviour is checked by hand (see the release checklist in [RELEASING.md](RELEASING.md)).

CI (`.github/workflows/ci.yml`) runs clippy with warnings as errors, plus the tests, on macOS, Windows and Linux.

## Platform notes

- **macOS:** needs Screen Recording permission (`permissions.rs`). Default hotkeys avoid ⌘⇧3/4/5.
- **Windows:** Greenshot's Print Screen hotkeys. Release builds use the GUI subsystem so no console window opens. Monitors with different scale factors are not yet tested.
- **Linux:** works on X11. On Wayland, global hotkeys and window listing are limited by the compositor.
