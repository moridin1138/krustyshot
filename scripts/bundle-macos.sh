#!/usr/bin/env bash
# Build Rustshot.app from a compiled binary.
#
#   scripts/bundle-macos.sh [target-triple]
#
# Expects `cargo build --release -p rustshot-app [--target <triple>]` to have
# run already. Writes target/[<triple>/]release/bundle/Rustshot.app.
set -euo pipefail

cd "$(dirname "$0")/.."
target="${1:-}"
release_dir="target/${target:+$target/}release"
binary="$release_dir/rustshot"
version=$(grep -m1 '^version = ' Cargo.toml | cut -d'"' -f2)

[ -x "$binary" ] || { echo "missing $binary; run cargo build --release first" >&2; exit 1; }

app="$release_dir/bundle/Rustshot.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/rustshot"

# Icon: macOS wants an .iconset folder with fixed names, turned into .icns.
iconset=$(mktemp -d)/Rustshot.iconset
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
  cp "assets/icons/icon_${size}.png" "$iconset/icon_${size}x${size}.png"
  cp "assets/icons/icon_$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/Rustshot.icns"

sed "s/@VERSION@/$version/g" crates/rustshot-app/bundle/Info.plist > "$app/Contents/Info.plist"

# Ad-hoc signature so Gatekeeper treats it as one app (not a notarised build).
codesign --force --deep --sign - "$app"
echo "built $app ($version)"
