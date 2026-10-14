#!/usr/bin/env bash
# One-time: create Rustshot's self-signed code-signing certificate.
#
#   scripts/make-signing-cert.sh [output-dir]
#
# Writes rustshot-signing.p12 (certificate + private key) and its password to
# output-dir (default ~/Documents/rustshot-signing). Keep that folder private
# and out of the repository. Every build signed with this certificate looks
# like the same app to macOS, so permissions such as Screen Recording survive
# updates. It is not an Apple Developer ID: Gatekeeper still asks users to
# confirm the first launch.
set -euo pipefail

out="${1:-$HOME/Documents/rustshot-signing}"
name="Rustshot Self-Signed"
if [ -e "$out/rustshot-signing.p12" ]; then
  echo "$out/rustshot-signing.p12 already exists; refusing to overwrite it." >&2
  echo "Replacing the certificate makes macOS forget permissions again." >&2
  exit 1
fi
mkdir -p "$out"
chmod 700 "$out"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

cat > "$tmp/cert.cnf" <<CNF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $name
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
CNF

# macOS's own LibreSSL, so `security import` can read the .p12.
/usr/bin/openssl req -x509 -newkey rsa:2048 -nodes -days 3650 \
  -keyout "$tmp/key.pem" -out "$tmp/cert.pem" -config "$tmp/cert.cnf" 2>/dev/null
password=$(/usr/bin/openssl rand -base64 24)
/usr/bin/openssl pkcs12 -export -inkey "$tmp/key.pem" -in "$tmp/cert.pem" \
  -name "$name" -out "$out/rustshot-signing.p12" -passout "pass:$password"
printf '%s' "$password" > "$out/password.txt"
cp "$tmp/cert.pem" "$out/rustshot-signing.cer.pem"
chmod 600 "$out"/*

echo "Created $out/rustshot-signing.p12"
echo "SHA-256 fingerprint: $(/usr/bin/openssl x509 -in "$tmp/cert.pem" -noout -fingerprint -sha256 | cut -d= -f2)"
echo
echo "Local builds:  export RUSTSHOT_SIGNING_P12=\"$out/rustshot-signing.p12\""
echo "               export RUSTSHOT_SIGNING_PASSWORD=\"\$(cat \"$out/password.txt\")\""
echo "GitHub secrets: MACOS_SIGNING_P12_BASE64 = base64 of the .p12, MACOS_SIGNING_PASSWORD = the password"
