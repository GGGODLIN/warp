#!/bin/bash
#
# First-time setup: create a stable self-signed code-signing identity in the
# login keychain so subsequent `./script/run` invocations sign every rebuild
# with the *same* identity. macOS TCC matches (bundle_id, signing_identity)
# rather than cdhash, so the OS stops re-prompting for Accessibility / Full
# Disk Access / etc. after every rebundle.
#
# Idempotent — re-running is safe.
#
# After this script you can:
#   1. ./script/run --dont-open
#   2. open target/debug/bundle/osx/WarpOss.app
#   3. Grant any system permissions ONCE.
#   4. Iterate; permissions stick across rebuilds.
#
# macOS only.

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "This script is macOS-only (you are on $(uname -s))." >&2
    exit 1
fi

CERT_NAME="Apple Development Local"
KEYCHAIN="$HOME/Library/Keychains/login.keychain-db"

if security find-identity -p codesigning -v 2>/dev/null | grep -q "$CERT_NAME"; then
    echo "✓ '$CERT_NAME' already in keychain — nothing to do."
    echo
    echo "If you want to re-create it, delete the existing entry first:"
    echo "  security delete-identity -c \"$CERT_NAME\" \"$KEYCHAIN\""
    exit 0
fi

if ! command -v openssl >/dev/null 2>&1; then
    echo "openssl not found in PATH; install it (e.g. brew install openssl) then re-run." >&2
    exit 1
fi

echo "Creating self-signed code-signing identity '$CERT_NAME'..."
echo "(macOS may prompt for your login keychain password during import.)"
echo

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

CONFIG="$TMPDIR/cert.conf"
cat > "$CONFIG" <<EOF
[req]
distinguished_name = req_dn
prompt = no
x509_extensions = v3_req

[req_dn]
CN = $CERT_NAME

[v3_req]
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
basicConstraints = critical, CA:false
EOF

openssl genrsa -out "$TMPDIR/key.pem" 2048 >/dev/null 2>&1
openssl req -new -x509 \
    -key "$TMPDIR/key.pem" \
    -out "$TMPDIR/cert.pem" \
    -days 3650 \
    -config "$CONFIG" \
    -extensions v3_req >/dev/null 2>&1
# Apple `security import` rejects empty PKCS#12 passwords on some macOS
# versions ("MAC verification failed"); use a fixed throwaway password
# only for the import step — it never leaves this script.
P12_PASS="warp-fork-codesign-local"

# OpenSSL 3.x defaults use AES-256-CBC + PBKDF2 for PKCS#12, which Apple
# `security import` (older PKCS#12 reader) fails to parse with "MAC
# verification failed". Fall back to legacy 3DES + SHA-1 PBE so macOS
# can read the bundle.
openssl pkcs12 -export \
    -out "$TMPDIR/cert.p12" \
    -inkey "$TMPDIR/key.pem" \
    -in "$TMPDIR/cert.pem" \
    -name "$CERT_NAME" \
    -passout pass:"$P12_PASS" \
    -keypbe PBE-SHA1-3DES \
    -certpbe PBE-SHA1-3DES \
    -macalg sha1 \
    -legacy >/dev/null 2>&1 \
 || openssl pkcs12 -export \
    -out "$TMPDIR/cert.p12" \
    -inkey "$TMPDIR/key.pem" \
    -in "$TMPDIR/cert.pem" \
    -name "$CERT_NAME" \
    -passout pass:"$P12_PASS" \
    -keypbe PBE-SHA1-3DES \
    -certpbe PBE-SHA1-3DES \
    -macalg sha1 >/dev/null 2>&1

security import "$TMPDIR/cert.p12" \
    -k "$KEYCHAIN" \
    -P "$P12_PASS" \
    -T /usr/bin/codesign \
    >/dev/null

if security set-key-partition-list \
    -S "apple-tool:,apple:,codesign:" \
    -s \
    -k "" \
    "$KEYCHAIN" >/dev/null 2>&1; then
    echo "  partition list updated (no per-build keychain prompt)"
else
    echo "  partition list step skipped (may prompt on first sign — choose Always Allow)"
fi

# Warp's bundle script filters with `find-identity -p codesigning -v`, where
# `-v` requires a trusted root. Self-signed certs don't satisfy that by
# default, so add the cert as a trusted code-signing root in the System
# keychain. This is the step that needs sudo (admin password prompt below);
# without it the bundle script can't see our identity and falls back to
# ad-hoc signing again.
echo
echo "Adding cert as trusted code-signing root in System keychain (admin password required)..."
sudo security add-trusted-cert \
    -d \
    -r trustRoot \
    -p codeSign \
    -k /Library/Keychains/System.keychain \
    "$TMPDIR/cert.pem" \
    >/dev/null 2>&1 \
 && echo "  trust added" \
 || echo "  trust step skipped or failed; you may need to run with sudo -S manually"

echo
echo "✓ Imported '$CERT_NAME' into login keychain."
echo
echo "Verify with:"
echo "  security find-identity -p codesigning -v"
echo
echo "Now run ./script/run --dont-open and open target/debug/bundle/osx/WarpOss.app."
echo "macOS should ask for system permissions ONCE; subsequent rebuilds reuse them."
