# Notes for coding assistants

Krustyshot (called Rustshot until 0.6.0) is a cross-platform Greenshot-style screenshot and
screen-recording tool in Rust with egui. Start with [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
for how the code is organised, and [docs/RELEASING.md](docs/RELEASING.md) for versions,
the changelog and the release checklist.

## Working with the maintainer

- **Don't commit, push or tag.** The maintainer does that, in GitHub Desktop. Prepare the
  change and hand over a suggested commit message and the commands. No AI co-author lines
  in commit messages.
- **Writing style** (code comments, docs, app text): no em dashes or en dashes; metric units;
  plain words. Every user-visible change gets a line under `## [Unreleased]` in
  `CHANGELOG.md`, written for users.
- **App text** lives in `crates/rustshot-i18n/locales/<lang>/rustshot.ftl` (English,
  French, Spanish, German). Add every new key to all four; `cargo test` checks they match.

## Checks before handing over

```bash
cargo clippy --all-targets -- -D warnings
cargo test
cargo check-windows   # Windows lint from macOS; no extra flags (-p or -q break it)
```

Linux is only checked in CI. On the maintainer's Mac, rustup comes from Homebrew:
`export PATH="/opt/homebrew/opt/rustup/bin:$PATH"`.

## The name

The rename to Krustyshot changed only what people see (app and file names, menus, downloads,
GitHub address). These stay `rustshot` on purpose, so settings and macOS permissions carry
over; don't "finish" the rename: the bundle ID `org.rustshot.Rustshot`, the settings and log
folders, crate names, `RUSTSHOT_*` environment variables, and the "Rustshot Self-Signed"
signing certificate.

## Testing on macOS (important)

- Test the **installed, signed** app, not `cargo run`, which borrows the terminal's
  permissions. Build, sign and install as in step 2 of docs/RELEASING.md, then open it from
  Finder. Crash reports: `~/Library/Logs/DiagnosticReports/krustyshot-*.ips`. App log:
  `~/Library/Logs/Rustshot/rustshot.log`.
- **Never trigger a macOS permission prompt while a recording or the capture overlay is up.**
  It once froze the whole Mac. Ask for permissions in Settings, before capturing.
- macOS 15 asks again for screen access when the app runs from anywhere but where it was
  approved, so tests that capture should run the app from `/Applications`.
- Don't take full-screen screenshots of the maintainer's screen, simulate keystrokes while
  their copy runs, touch their clipboard, or save into their screenshots folder. Capture only
  Krustyshot's own windows (`screencapture -l <window number>`); the README pictures are made
  this way by `scripts/readme-screenshots.sh`.
- The signing certificate and its password (`~/Documents/rustshot-signing/`) are private.
  Never commit or print them.

## Windows

Windows builds come from GitHub Actions (MSVC, with the C runtime built in, see
`.cargo/config.toml`). Test on a real Windows PC, not over Remote Desktop.
