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

# Sign. With RUSTSHOT_SIGNING_P12 and RUSTSHOT_SIGNING_PASSWORD set (see
# scripts/make-signing-cert.sh), use Rustshot's self-signed certificate so
# every build has the same identity and macOS keeps its permissions.
# Otherwise fall back to an ad-hoc signature, which changes every build.
if [ -n "${RUSTSHOT_SIGNING_P12:-}" ]; then
  : "${RUSTSHOT_SIGNING_PASSWORD:?set RUSTSHOT_SIGNING_PASSWORD too}"
  keychain=$(mktemp -d)/rustshot-signing.keychain-db
  kc_password=$(/usr/bin/openssl rand -hex 16)
  # codesign only finds identities in keychains on the search list, so add a
  # throwaway keychain for the duration and always put the list back.
  original_keychains=$(security list-keychains -d user | tr -d '"')
  cleanup() {
    # shellcheck disable=SC2086
    security list-keychains -d user -s $original_keychains
    security delete-keychain "$keychain" 2>/dev/null || true
  }
  trap cleanup EXIT
  security create-keychain -p "$kc_password" "$keychain"
  security set-keychain-settings "$keychain"
  security unlock-keychain -p "$kc_password" "$keychain"
  security import "$RUSTSHOT_SIGNING_P12" -k "$keychain" -P "$RUSTSHOT_SIGNING_PASSWORD" -T /usr/bin/codesign >/dev/null
  security set-key-partition-list -S apple-tool:,apple: -s -k "$kc_password" "$keychain" >/dev/null
  # shellcheck disable=SC2086
  security list-keychains -d user -s "$keychain" $original_keychains
  identity=$(security find-certificate -a -c "Rustshot Self-Signed" -Z "$keychain" | awk '/SHA-1 hash:/ {print $3; exit}')
  [ -n "$identity" ] || { echo "no Rustshot Self-Signed certificate in $RUSTSHOT_SIGNING_P12" >&2; exit 1; }
  codesign --force --deep --keychain "$keychain" --sign "$identity" \
    --identifier org.rustshot.Rustshot "$app"
  echo "signed with Rustshot Self-Signed ($identity)"
else
  echo "RUSTSHOT_SIGNING_P12 not set: ad-hoc signing (macOS permissions reset every build)" >&2
  codesign --force --deep --sign - "$app"
fi
echo "built $app ($version)"
