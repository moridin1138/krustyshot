# Roadmap

Rustshot aims for feature parity with Greenshot on every desktop platform. Work is grouped into phases; anything not yet scheduled sits in the backlog.

## Phase 1: core capture and editing

Released in 0.1.0 and 0.1.1.

## Phase 2: Greenshot editor parity

Released in 0.2.0: speech bubbles, step counters, highlight modes, drop shadows, image effects, the mouse pointer in captures, capture delay, notifications, a configurable tray click, and regions spanning several displays.

## Settings and languages

Released in 0.3.0: the redesigned Settings window, shortcut recorder, open at login, update check, system fonts, and French, Spanish and German.

## Phase 3: more destinations

- Done (unreleased): open in another app (default or chosen), print, email, the macOS Share menu, and copy text (OCR with Vision, Windows OCR or Tesseract), from the quick menu, the after-capture settings and the editor.
- Done (unreleased): a custom uploader (any server, ShareX `.sxcu` import).
- Later: built-in uploaders for specific services, if there's demand.

## Video

- Released in 0.4.0: region, window and display recording; MP4 on macOS, GIF everywhere; quality settings; disk-space safeguards.
- Done (unreleased): MP4 on Windows (Media Foundation), the recording bar hidden from Windows recordings, microphone and system sound in MP4s, showing mouse clicks.
- Next: MP4 and sound on Linux (GStreamer, PipeWire), regions that span displays.

## Backlog

- More languages, and native-speaker review of the French, Spanish and German drafts (see [TRANSLATING.md](TRANSLATING.md)). Chinese, Japanese and Korean need CJK fallback fonts; right-to-left languages wait on support in the UI toolkit.
- Notarised macOS builds (needs an Apple Developer account; builds are already signed with Rustshot's own certificate).
- Windows installer and embedded `.exe` icon.
- Linux packages (AppImage, .deb, Flatpak) and Wayland capture through xdg-desktop-portal.
- Automatic update check (the manual one is in Settings → About).

See [CHANGELOG.md](../CHANGELOG.md) for exactly what each release contains.
