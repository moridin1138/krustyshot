# Roadmap

Rustshot aims for feature parity with Greenshot on every desktop platform. Work is grouped into phases; anything not yet scheduled sits in the backlog.

## Phase 1: core capture and editing

Released in 0.1.0 and 0.1.1.

## Phase 2: Greenshot editor parity

Done, to be released as 0.2.0: speech bubbles, step counters, highlight modes, drop shadows, image effects, the mouse pointer in captures, capture delay, notifications, a configurable tray click, and regions spanning several displays. See [CHANGELOG.md](../CHANGELOG.md).

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
