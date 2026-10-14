# Releasing

## Versioning

Rustshot follows [Semantic Versioning](https://semver.org/). There is one version for the whole project, set once in the root `Cargo.toml` under `[workspace.package]`; every crate inherits it with `version.workspace = true`.

While the version is below 1.0:

- **0.x.0** (minor): new features, or changes to settings, hotkeys or behaviour that users will notice
- **0.x.y** (patch): bug fixes only

1.0.0 marks Greenshot feature parity on macOS and Windows with a stable settings format. After 1.0 the usual rules apply: major for breaking changes, minor for features, patch for fixes.

Pre-releases use a suffix, for example `0.2.0-beta.1`. The release workflow marks every 0.x version and anything with a suffix as a pre-release on GitHub.

## Changelog

Every pull request or change that users would notice adds a line under `## [Unreleased]` in [CHANGELOG.md](../CHANGELOG.md), under one of: Added, Changed, Deprecated, Removed, Fixed, Security. Write each line for users, not developers.

## Release checklist

1. **Make sure `main` is green.** CI passes on all three platforms.
2. **Test a local release build on macOS.** Running with `cargo run` is not enough: it borrows your terminal's Screen Recording permission and hides permission and packaging bugs. Instead:
   ```bash
   cargo build --release -p rustshot-app
   scripts/bundle-macos.sh
   ```
   Quit any running Rustshot, replace `/Applications/Rustshot.app` with `target/release/bundle/Rustshot.app`, and open it **from Finder**. Then check:
   - At most one permission prompt appears at startup.
   - Without permission, a capture hotkey shows Rustshot's permission window (not a wallpaper-only screenshot). Reset permission, Open System Settings and Quit work.
   - With permission granted, region, window, full-screen and last-region capture work from both hotkeys and the tray menu, and show your real windows.
   - The overlay covers every display, including the menu bar, with nothing stretched.
   - A region on a Retina display produces the expected pixel size.
   - Each destination works: editor, clipboard, save directly, save as.
   - Every editor tool, undo and redo, crop, save, and copy (paste into Preview to check).
   - Preferences: change a hotkey, save, and confirm the new hotkey works.
   - Open Mission Control on the desktop where Rustshot started: windows must look normal size, and no Rustshot window should appear.
   - Open and close the editor, Preferences and notice windows several times, and cancel captures with Esc. Rustshot must not quit, and `~/Library/Logs/DiagnosticReports/` must have no new `rustshot-*.ips` file.
3. **Choose the version** using the rules above.
4. **Update the version** in the root `Cargo.toml`, then run `cargo build` so `Cargo.lock` picks it up.
5. **Update CHANGELOG.md:**
   - Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` and add a fresh empty `## [Unreleased]` above it.
   - Update the comparison links at the bottom.
6. **Commit** the version bump: `git commit -am "Release vX.Y.Z"`.
7. **Tag and push:**
   ```bash
   git tag -a vX.Y.Z -m "Rustshot vX.Y.Z"
   git push origin main vX.Y.Z
   ```
8. **Review the draft release.** The [release workflow](../.github/workflows/release.yml) checks that the tag matches `Cargo.toml` and that the changelog has an entry, then builds:
   - `Rustshot-vX.Y.Z-macos-arm64.zip` and `-macos-x86_64.zip` (app bundles)
   - `Rustshot-vX.Y.Z-windows-x86_64.zip`
   - `Rustshot-vX.Y.Z-linux-x86_64.tar.gz`
   - `SHA256SUMS.txt`

   It attaches them to a **draft** GitHub release, using the changelog section as the notes.
9. **Test the downloaded build before publishing.** Download the macOS zip from the draft, unzip it, replace `/Applications/Rustshot.app`, open it from Finder, and repeat the permission, capture and window open/close checks from step 2. Only then click Publish. If anything fails, delete the draft and the tag, fix it, and start again.

## If something goes wrong

- **The workflow fails before building:** the tag and `Cargo.toml` disagree, or the changelog heading is missing. Delete the tag (`git tag -d vX.Y.Z && git push origin :vX.Y.Z`), fix it, and tag again.
- **A bad release has been published:** don't reuse its version number. Fix the problem and release the next patch version.

## Not done yet

- macOS code signing and notarisation. Until then, users must right-click and choose Open the first time.
- Windows code signing and an installer.
