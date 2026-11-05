#!/usr/bin/env bash
# Downloads the official Tectonic release binary and installs it as a Tauri sidecar
# (src-tauri/binaries/tectonic-<host-triple>). Linux x86_64 only for now.
set -euo pipefail
VERSION="${TECTONIC_VERSION:-0.17.0}"
TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
case "$TRIPLE" in
  x86_64-unknown-linux-gnu) ASSET="tectonic-${VERSION}-x86_64-unknown-linux-musl.tar.gz" ;;
  *) echo "No prebuilt mapping for $TRIPLE yet; install tectonic and copy it to src-tauri/binaries/tectonic-$TRIPLE" >&2; exit 1 ;;
esac
URL="https://github.com/tectonic-typesetting/tectonic/releases/download/tectonic%40${VERSION}/${ASSET}"
DEST="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/binaries"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
curl -fsSL -o "$TMP/t.tar.gz" "$URL"
tar xzf "$TMP/t.tar.gz" -C "$TMP" tectonic
mkdir -p "$DEST"
install -m 755 "$TMP/tectonic" "$DEST/tectonic-$TRIPLE"
"$DEST/tectonic-$TRIPLE" --version
