# Rustshot

A cross-platform screenshot tool modelled on [Greenshot](https://getgreenshot.org/), written in Rust with [egui](https://github.com/emilk/egui). It runs on macOS, Windows and Linux.

<img src="assets/icons/icon_256.png" width="96" alt="Rustshot icon">

## Features

- Menu-bar / tray icon with capture and recording menus; optionally make a left click start a capture
- Global hotkeys for region, window, full screen and last region captures
- Frozen-screen region selection with crosshair, magnifier and a pixel-size readout
- Window capture: hover to highlight, click to capture (Space switches between region and window mode, like Greenshot)
- Regions that span several displays, and full-screen capture of all displays
- Capture last region, capture delay, and optionally the mouse pointer
- Destination picker after each capture: open in editor, copy, save directly, save as, open in another app, print, email, Share (macOS), copy the text in the image (OCR), and upload to your own server or image host (ShareX `.sxcu` uploaders can be imported)
- Image editor with Phosphor icons: select, move and resize, rectangle, ellipse, line, arrow, freehand, text, speech bubble, step counter, highlight (marker, spotlight, grey out the rest, magnify), obfuscate (pixelate or blur), crop, drop shadows, undo and redo
- Effects: drop shadow, torn edges, border, greyscale, invert, rotate, resize
- Screen recording of a region, a window or a whole display, to MP4 (macOS and Windows) or GIF (all platforms), with quality, resolution and frame-rate settings and a size estimate; MP4s can include the microphone and system sound, and clicks can be shown
- Low disk space warnings for the screenshots folder, and recordings that stop safely before the disk fills
- Notifications after saving or copying (click to show the file)
- English, French, Spanish and German interface, following the system language
- Open an image from the clipboard or from a file
- Settings: language, after-capture actions, capture and recording options, shortcut recorder, output folder with free space, Greenshot-style filename pattern, PNG/JPG/BMP/GIF, open at login, check for updates

See [CHANGELOG.md](CHANGELOG.md) for what changed in each version and [docs/ROADMAP.md](docs/ROADMAP.md) for what's planned.

## Default hotkeys

| Action | macOS | Windows / Linux |
|---|---|---|
| Capture region | ⌥⇧R | Print Screen |
| Capture window | ⌥⇧W | Alt+Print Screen |
| Capture full screen | ⌥⇧F | Ctrl+Print Screen |
| Capture last region | ⌥⇧L | Shift+Print Screen |
| Start or stop recording | ⌥⇧V | Ctrl+Shift+Print Screen |

Change them in Settings (click a shortcut and press the new keys). macOS keeps ⌘⇧3/4/5 for its own screenshot tool.

## Recording

Choose **Record region…**, **Record window…** or **Record full screen** from the menu-bar icon, or press the recording shortcut (⌥⇧V on macOS) and drag a region. A small bar shows the time and file size, with **Stop** and **Discard** (which asks before deleting). When the recording has sound, microphone and speaker icons blink next to the timer; click one to mute or unmute it. Press the shortcut again, or choose **Stop recording** from the menu, to finish. The file is saved to your output folder, and a notification lets you show it.

Settings → Recording chooses MP4 or GIF, the quality, full or half resolution on Retina displays, and the frame rate, and estimates how much space a minute takes. For MP4s you can also record the **microphone** (pick which one) and **system sound** (macOS 13 or later, and Windows), and turn on **Show mouse clicks**. GIFs have no sound. MP4 recording works on macOS and Windows; Linux records GIFs for now. Recordings use one display at a time.

### Editor shortcuts

| Key | Action |
|---|---|
| `R` `E` `L` `A` `F` `T` `S` `I` `H` `O` `C` | Rectangle, ellipse, line, arrow, freehand, text, speech bubble, step counter, highlight, obfuscate, crop |
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

- **macOS:** unzip and move `Rustshot.app` to Applications, then open it. macOS warns the first time; see [Security warnings when installing](#security-warnings-when-installing). Updates keep the Screen Recording and Microphone permissions.
- **Windows:** unzip and run `rustshot.exe`. Windows may warn the first time; see below.
- **Linux:** extract and run `rustshot`. It needs GTK 3 and an appindicator library.

### Security warnings when installing

Rustshot is free, made by one developer, and its downloads aren't signed with a paid certificate (Apple charges US$99 a year, and Windows signing about US$120 a year). The free signing programmes for open-source projects don't apply either: they require an OSI-approved licence, and Rustshot's [licence](#license) isn't one. So macOS and Windows can't confirm who made Rustshot, and warn you the first time. That doesn't mean anything is wrong with the app. Here's what to expect, and how to check what you're running.

**Mac:** the first time you open it, macOS says it can't verify the developer.
- On macOS 15 (Sequoia) and later: choose **Done**, then go to System Settings → Privacy & Security, scroll down, choose **Open Anyway** next to Rustshot, and confirm.
- On macOS 11 to 14: right-click (or Control-click) the app in Finder, choose **Open**, then **Open** again.

You do this once per download. Every release is signed with the same Rustshot certificate, so macOS treats updates as the same app and keeps your Screen Recording and Microphone permissions.

**Windows:** SmartScreen may show "Windows protected your PC". Choose **More info**, then **Run anyway**. On PCs with **Smart App Control** turned on (Windows Security → App & browser control), Windows blocks unsigned apps outright and offers no way to run them. Turning Smart App Control off is possible, but Windows only lets you turn it back on by resetting or reinstalling Windows, so decide that for yourself.

**Linux:** no warnings.

**How to know what you're getting:**

- Every download on the [releases page](https://github.com/moridin1138/rustshot/releases) is built by this repository's GitHub Actions workflows from that release's public source code, not on anyone's own computer.
- Each release includes `SHA256SUMS.txt`, and GitHub shows each file's checksum on the releases page. To check a download, run `shasum -a 256 Rustshot-*.zip` (macOS and Linux) or `Get-FileHash Rustshot-*.zip` (Windows PowerShell), and compare.
- You can read every line of the source here, and [build Rustshot yourself](#build-from-source) if you'd rather.

**What Rustshot sends over the internet:**

- **Update check:** a request to GitHub's public API for the latest version number, shortly after start-up and once a day. Turn it off in Settings → General. **Check for updates** in Settings → About asks only when you click it.
- **Uploads:** only to an uploader you set up yourself (Settings → Uploads), and only when you choose **Upload**.
- **Nothing else.** There's no analytics or tracking. Text recognition (Copy text) runs on your computer. Email, Share and Print hand the image to your own apps.

### macOS permissions

Screenshots and recordings need **Screen Recording** permission. macOS asks the first time Rustshot runs. If it's missing, Rustshot shows a window explaining how to turn it on (System Settings → Privacy & Security → Screen Recording, then restart Rustshot); when running with `cargo run`, give the permission to your terminal app instead. Recording the microphone also needs **Microphone** permission, which macOS asks for when you turn the Microphone switch on in Settings → Recording.

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
| `cargo run -p rustshot-editor --example sample -- out.png` | Render every annotation type into a sample image, to check how they look |
| `cargo run --release -p rustshot-core --example record -- out.mp4 5 medium` | Record the main display for 5 seconds without the app (`.gif` for GIF; add `full` for full resolution, `mic` and `system` for sound; `list` instead of a path lists microphones) |
| `cargo check-windows` | Lint the Windows build from macOS (once: `rustup target add x86_64-pc-windows-gnu`) |
| `RUSTSHOT_LANG=fr cargo run -p rustshot-app` | Run in a particular language |

## Project documents

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): how the code is organised and how a capture flows through it
- [docs/RELEASING.md](docs/RELEASING.md): versioning rules and the release checklist
- [docs/ROADMAP.md](docs/ROADMAP.md): planned phases and the backlog
- [docs/TRANSLATING.md](docs/TRANSLATING.md): fixing translations and adding languages
- [CHANGELOG.md](CHANGELOG.md): release history

## Settings

Settings live in `rustshot.toml` in the platform config folder:

- macOS: `~/Library/Application Support/org.Rustshot.Rustshot/`
- Windows: `%APPDATA%\Rustshot\Rustshot\config\`
- Linux: `~/.config/rustshot/`

## Credits

- Inspired by [Greenshot](https://getgreenshot.org/) (GPL-3.0). Rustshot is a new implementation and contains no Greenshot code.
- Icons: [Phosphor Icons](https://phosphoricons.com/) (MIT), via [egui-phosphor](https://github.com/amPerl/egui-phosphor).

## License

Rustshot is source-available under the [PolyForm Shield License 1.0.0](LICENSE).

In plain terms (the licence text is what counts):

- You can use Rustshot for free for any purpose, including at work and for commercial projects.
- You can read, modify and share the code, as long as you pass on the licence.
- You can't sell Rustshot, or offer a product or service that competes with it, including a repackaged or rebranded version.

PolyForm Shield is a "source-available" licence, not an open-source one in the [OSI](https://opensource.org/licenses) sense, because of that last point. For most people it makes no difference: Rustshot is free to download, use and change.

Version 0.1.0 and earlier commits were published under GPL-3.0-or-later and remain available under those terms. From 0.1.1 onward, Rustshot uses the PolyForm Shield licence.

Rustshot's dependencies keep their own licences, all permissive: mostly MIT and Apache-2.0, with some BSD, Zlib, ISC and Unicode, one MPL-2.0 crate, and Mozilla's root certificate list (CDLA-Permissive-2.0). Greenshot inspired the design, but Rustshot contains no Greenshot code.
