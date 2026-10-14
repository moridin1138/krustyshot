# Architecture

Rustshot is a Cargo workspace with four crates plus a maintenance tool. Each crate depends only on the ones above it in this table:

| Crate | Kind | Responsibility |
|---|---|---|
| `crates/rustshot-i18n` | library | Interface translations (Fluent files in `locales/`) and the `tr!` macro |
| `crates/rustshot-core` | library, no UI | Screen capture and recording, geometry, settings, filename patterns, saving and clipboard, sending to other apps, printers and email, text recognition, disk space, mouse pointer, open at login, update check |
| `crates/rustshot-editor` | library, egui | The annotation editor: document model, undo history, rendering and export |
| `crates/rustshot-app` | binary `rustshot` | Tray icon, hotkeys, capture overlay, recording bar, Settings, fonts, and wiring everything together |
| `xtask` | binary, dev only | `cargo xtask icons` regenerates the app icon |

Every crate uses `rustshot-i18n` for user-facing text (see [TRANSLATING.md](TRANSLATING.md)). `rustshot-editor` does not depend on `rustshot-core`. It receives an image and hands back requests (save, copy, close) for the app to carry out, so it could be reused elsewhere.

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
| [`gif`](https://github.com/image-rs/image-gif) | Animated GIF recordings that store only changed areas |
| `objc2` + `objc2-av-foundation`, `-app-kit`, `-core-video`, `-core-media` | macOS: H.264 and AAC recording (AVAssetWriter), ScreenCaptureKit system sound, window levels, capture exclusion, the Touch Bar guard |
| [`cpal`](https://github.com/RustAudio/cpal) | Microphone recording on macOS and Windows, and Windows system sound (loopback) |
| [`windows`](https://github.com/microsoft/windows-rs) | Windows: H.264 recording (Media Foundation), hiding the recording bar from captures |
| [`fluent-bundle`](https://github.com/projectfluent/fluent-rs) + [`sys-locale`](https://github.com/1Password/sys-locale) | Translations and the system language |
| [`fs4`](https://github.com/al8n/fs4-rs) | Free disk space |
| [`ureq`](https://github.com/algesten/ureq) | The "Check for updates" request to GitHub |

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
| `overlay.rs` | One borderless overlay per monitor while choosing what to capture or record |
| `send.rs` | Print, email and Share: AppKit panels on macOS (main thread), `rustshot_core::share` elsewhere |
| `recording_ui.rs` | The floating recording bar (timer, size, Stop, Discard) |
| `app.rs` (`show_picker`) | The "what next?" destination picker |
| `app.rs` (`show_editors`) | One editor window per open image |
| `settings_ui.rs` | Settings: sidebar sections, instant apply, shortcut recorder |
| `app.rs` (`show_notices`) | Small message windows (errors, permissions, about) |
| `app.rs` (`show_toast`) | The "Saved" / "Copied" notification near the screen corner |

User-facing text everywhere goes through `tr!` (see [TRANSLATING.md](TRANSLATING.md)). The interface uses the system's own font (`fonts.rs`): SF Pro, Segoe UI, or a common Linux font, with the bundled font as a fallback and for text typed into screenshots.

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
Fullscreen ──► Snapshot::stitch_all()   (compose() over every display)
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

Recordings start the same way: `RecordRegion` and `RecordWindow` open the overlay (with recording help text), and the chosen area becomes a display id plus a crop in that display's pixels (`Snapshot::recording_target`); `RecordFullscreen` skips the overlay and records the display under the pointer. `App::start_recording` checks free space first, then starts a `Recording` (below).

The screen is captured before any Rustshot window appears, so the overlay shows a frozen image and never captures itself, the same approach Greenshot uses. On macOS `platform::raise_overlays` lifts the overlay windows above the menu bar and Dock and lets them appear over full-screen apps.

## The editor (`rustshot-editor`)

| File | Contents |
|---|---|
| `document.rs` | `Document` (shared base image + `Vec<Element>`), `Element`, `Style`, hit testing, crop, and `History` |
| `raster.rs` | Pixel effects: pixelate, blur, highlight |
| `export.rs` | `flatten()`: composites raster effects, then draws shapes, shadows and text with tiny-skia |
| `effects.rs` | Whole-image effects (border, shadow, torn edges, greyscale, invert, rotate, resize) that also move, rotate and scale the annotations |
| `editor.rs` | The egui UI: toolbar, properties bar, canvas, tools and shortcuts |

There are two kinds of element:

- **Vector** elements (rectangle, ellipse, line, arrow, freehand, text, speech bubble, step counter) are drawn on top of the image. The canvas paints them with egui; export paints them with tiny-skia. Shadows are a blurred silhouette in the saved image and a quick offset copy on the canvas. Step-counter numbers are not stored: they follow the counters' order in the document, so deleting one renumbers the rest.
- **Raster** elements (highlight in its four styles, obfuscate) change the pixels underneath. They are baked into the canvas texture whenever the document revision changes, and into the exported image by `raster::composite`.

Undo works by snapshot. Before each change the whole `Document` is pushed onto `History`. The base image is behind an `Arc`, so a snapshot only copies the element list, except after a crop, which creates a new base image.

## Destinations (`rustshot-core::share`)

Clipboard and file destinations live in `output.rs`. The others hand the image to something else, so `share::temp_copy` first writes a PNG named with the filename pattern to a `rustshot` folder in the system temp directory (cleared of files older than a day at start-up):

- **Open in another app:** `open -a` on macOS, the program itself elsewhere, or the default app (`open`, `ShellExecute`, `xdg-open`).
- **Print:** an `NSPrintOperation` fitted to the page on macOS; Windows' Print Pictures window (`ShellExecute` "print"); `lp` on Linux.
- **Email:** the Mail compose sharing service on macOS; Simple MAPI (`MAPISendMailW`, loaded from `mapi32.dll`) on Windows; `xdg-email` on Linux.
- **Share (macOS):** `NSSharingServicePicker` next to the pointer in the active window.
- **Copy text (`share/ocr.rs`):** Vision (`VNRecognizeTextRequest`, accurate, app language first) on macOS, `Windows.Media.Ocr` on Windows, the `tesseract` command on Linux. It runs on a background thread; the app copies the result when it arrives.

## Recording (`rustshot-core::recording`)

`Recording::start` takes a display id, an area in that display's pixels, the recording settings and a path. Two threads do the work:

- **Capture:** looks the display up itself (display handles can't move between threads on Windows), streams frames with xcap's `video_recorder`, keeps them on a steady grid at the chosen frame rate, crops them, and hands them on through a 4-frame queue. If the queue is full the frame is dropped and counted. Every 2 seconds it checks free space and stops if the disk is critically low.
- **Encode:** feeds frames to a `VideoEncoder`:
  - `gif.rs` (all platforms) stores the first frame in full, then only the changed rectangle of each frame; unchanged frames extend the previous frame's delay.
  - `mp4_macos.rs` uses AVAssetWriter (hardware H.264), which also scales to the output size.
  - `mp4_windows.rs` uses the Media Foundation Sink Writer (H.264, hardware where available). It is set up lazily on the encoder thread, because COM objects belong to the thread that creates them; frames are scaled on the CPU when the output is smaller, and passed as top-down RGB32.

  In both, the bit rate is pixels × fps × a bits-per-pixel figure for the quality. Linux MP4 (GStreamer) will be another `VideoEncoder`; until then the MP4 option is hidden there and recordings fall back to GIF.

- **Sound (`audio/`):** when the microphone or system sound is on (MP4 only), a separate audio thread starts an `AudioCapture`. Nothing ever waits for that thread: audio services can block, and a stuck source must not hold up the picture, Stop or the app. The microphone is only opened when permission is already granted (`microphone_access`); Settings asks for it when the switch is turned on, because opening the microphone without permission makes coreaudiod wait on a prompt that can't be answered mid-recording. The microphone comes through `cpal` (CoreAudio, WASAPI); Windows system sound is a `cpal` input stream on the default output device (WASAPI loopback); macOS system sound uses ScreenCaptureKit (`system_macos.rs`), loaded at run time and driven by messages so the app still starts on macOS 11 and 12, where the option is greyed out. Each source converts its audio to 48 kHz stereo and places it on the recording timeline by counting samples (`SourceClock`, which resynchronises after a gap). The encoder thread mixes the sources (`Mixer`), filling silence where there is none, 250 ms behind real time so late sources still make it, and passes it to `VideoEncoder::push_audio`: an AAC track via AVAssetWriter on macOS and the Sink Writer on Windows. A source that fails to start is logged and the recording goes on without it.
- **Clicks (`clicks.rs`):** with Show mouse clicks on, a thread polls the mouse buttons every 10 ms and the capture thread draws a dot or fading ring onto each frame.

The app side (`recording_ui.rs`) shows the floating bar, which `platform::exclude_from_capture` keeps out of the recording on macOS (`NSWindow.sharingType = none`) and Windows 10 2004+ (`SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)`). `disk.rs` provides the free-space checks used before captures and in Settings.

## Settings (`rustshot-core::config`)

`Config` is serialised as TOML with `#[serde(default)]`, so older or partial files load with defaults for anything missing. The path comes from the `directories` crate (see the README for each platform).

## Testing

Unit tests sit next to the code and cover the parts that don't need a display: geometry and crop maths, Retina scaling, stitching and recording targets, filename patterns, config round-trips and recording sizes, saving every format, disk-space thresholds, GIF change detection and frame merging, sound conversion, timing and mixing, MP4s with a sound track (macOS), click drawing, hit testing, undo and redo, crop, the pixel effects, export rendering, the shortcut recorder, every translation file being complete, and the Touch Bar crash guard (which reproduces the AppKit exception). Run them with `cargo test`. Capture and UI behaviour is checked by hand (see the release checklist in [RELEASING.md](RELEASING.md)).

Debug builds have three developer switches (environment variables, ignored in release builds):

- `RUSTSHOT_IGNORE_PERMISSION=1` skips the Screen Recording check, so the overlay can be exercised under a debugger, which never has the permission.
- `RUSTSHOT_SETTINGS_TOUR=<folder>` (macOS) opens Settings, visits each section and saves a picture of just that window to the folder, for reviewing the design. Combine with the `--settings` flag.
- `RUSTSHOT_STRESS=1` opens and closes editor windows (with a focused text box) and message windows in a loop, to hunt window-lifecycle bugs without touching files or the clipboard.

In every build, `RUSTSHOT_LANG=<code>` forces the interface language, and `--settings` opens Settings at launch.

CI (`.github/workflows/ci.yml`) runs clippy with warnings as errors, plus the tests, on macOS, Windows and Linux. `cargo check-windows` runs the Windows lint locally from a Mac; Linux can only be checked in CI.

## Platform notes

- **macOS:** needs Screen Recording permission (`permissions.rs`). Captures are refused with an explanation rather than producing wallpaper-only images when it is missing. Default hotkeys avoid ⌘⇧3/4/5. `touchbar_guard.rs` catches an exception AppKit's Touch Bar code raises at random (`-[_NSTouchBarFinderObservation invalidate]`), which otherwise crashes the app; its unit test reproduces the exception. Builds are signed with the project's self-signed certificate (`scripts/bundle-macos.sh`, see [RELEASING.md](RELEASING.md)) so permissions survive updates.
- **Windows:** Greenshot's Print Screen hotkeys. Release builds use the GUI subsystem so no console window opens. Display handles can't cross threads, so recording looks the display up inside its capture thread. MP4 recordings use Media Foundation; the recording bar is excluded from captures. Monitors with different scale factors are not yet tested.
- **Linux:** works on X11. On Wayland, global hotkeys and window listing are limited by the compositor. Recordings are GIF only for now.
