# Roadmap

Rustshot aims for feature parity with Greenshot on every desktop platform. Work is grouped into phases; anything not yet scheduled sits in the backlog.

## Phase 1: core capture and editing

Done, unreleased. See [CHANGELOG.md](../CHANGELOG.md).

## Phase 2: Greenshot editor parity

- Speech bubbles and step counters
- Highlight modes: text, grayscale, magnify
- Drop shadows on shapes
- Image effects: drop shadow, torn edges, border, grayscale, invert, rotate, resize
- Include the mouse pointer in captures
- Capture delay
- Notifications after saving or copying
- Configurable tray click (for example, click captures a region, right-click shows the menu)
- Regions that span several monitors

## Phase 3: more destinations

- Print
- Open in an external app
- Email
- Upload plugins (Imgur and similar)
- OCR

## Backlog

- **Multilingual UI.** Translate every user-facing string and pick the language from the system locale, with an override in Preferences. Likely approach: [Fluent](https://projectfluent.org/) via the `fluent` / `i18n-embed` crates, with one `.ftl` file per language under `locales/`. Greenshot ships in 30+ languages, so its translations are a useful list of target languages (wording must be our own).
- Signed and notarised macOS builds (needs an Apple Developer account).
- Windows installer and embedded `.exe` icon.
- Linux packages (AppImage, .deb, Flatpak) and Wayland capture through xdg-desktop-portal.
- Automatic update check.
- Launch at login option.
