#!/usr/bin/env bash
# Take the README screenshots (docs/images/) on macOS.
#
#   export RUSTSHOT_SIGNING_P12=~/Documents/rustshot-signing/rustshot-signing.p12
#   export RUSTSHOT_SIGNING_PASSWORD="$(cat ~/Documents/rustshot-signing/password.txt)"
#   scripts/readme-screenshots.sh [parts]
#
# parts: comma-separated, from editor,menu,settings,recording,scrolling (all
# by default). Quit Krustyshot first, and leave the mouse alone for about a
# minute: the scrolling capture moves the pointer.
#
# Needs Google Chrome (to render the sample page) and the signing
# certificate. The demo build runs from /Applications/Krustyshot.app, because
# macOS 15 only trusts screen capture by an app at the place it was approved
# (anywhere else it asks again, and the prompt lands in the pictures); the
# normal build is put back afterwards. Only Krustyshot's own windows are
# photographed, and a throwaway home folder keeps your settings out of them.
set -euo pipefail

cd "$(dirname "$0")/.."
work=$(mktemp -d)
out="$work/out"
mkdir -p "$out" "$work/home" "$work/chrome"

# The sample: a Wikipedia article (CC BY-SA), rendered at 2x.
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
url=https://en.wikipedia.org/wiki/Screenshot
render() {
  "$chrome" --headless=new --disable-gpu --no-first-run --user-data-dir="$work/chrome" --hide-scrollbars \
    --force-device-scale-factor=2 --window-size="$1" --screenshot="$out/$2" "$url" 2>/dev/null
}
render 1100,700 sample.png
render 900,6000 page-tall.png

cargo build --release -p rustshot-app --features demo
scripts/bundle-macos.sh
app=/Applications/Krustyshot.app
# Put the normal build back however this ends.
restore() {
  cargo build --release -p rustshot-app
  scripts/bundle-macos.sh
  rm -rf "$app"
  ditto target/release/bundle/Krustyshot.app "$app"
}
trap restore EXIT
rm -rf "$app"
ditto target/release/bundle/Krustyshot.app "$app"
open -n -W --env HOME="$work/home" --env KRUSTYSHOT_DEMO="$out" --env KRUSTYSHOT_DEMO_PARTS="${1:-editor,menu,settings,recording,scrolling}" \
  --env RUSTSHOT_SETTINGS_TOUR="$out" "$app"

# Retina pictures at most 1600 px wide; the scrolling result smaller still.
mkdir -p docs/images
for f in editor quick-menu recording-bar scrolling-bar general uploads; do
  [ -f "$out/$f.png" ] && sips -Z 1600 "$out/$f.png" --out "docs/images/$f.png" >/dev/null
done
[ -f "$out/scrolling.png" ] && sips --resampleWidth 600 "$out/scrolling.png" --out docs/images/scrolling.png >/dev/null
echo "Pictures in docs/images; full-size originals and the demo's log in $work"
