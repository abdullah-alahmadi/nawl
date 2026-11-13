#!/usr/bin/env bash
# Downloads the official Tectonic release binary and installs it as a Tauri sidecar
# (src-tauri/binaries/nawl-tectonic-<target-triple>[.exe]).
#
#   scripts/fetch-tectonic.sh                          # for this computer
#   scripts/fetch-tectonic.sh x86_64-pc-windows-msvc   # for a Windows build
set -euo pipefail
VERSION="${TECTONIC_VERSION:-0.17.0}"
TRIPLE="${1:-$(rustc -vV | sed -n 's/^host: //p')}"
EXT=""
case "$TRIPLE" in
  x86_64-unknown-linux-gnu) ASSET="tectonic-${VERSION}-x86_64-unknown-linux-musl.tar.gz" ;;
  aarch64-unknown-linux-gnu) ASSET="tectonic-${VERSION}-aarch64-unknown-linux-musl.tar.gz" ;;
  x86_64-pc-windows-msvc) ASSET="tectonic-${VERSION}-x86_64-pc-windows-msvc.zip"; EXT=".exe" ;;
  x86_64-apple-darwin) ASSET="tectonic-${VERSION}-x86_64-apple-darwin.tar.gz" ;;
  aarch64-apple-darwin) ASSET="tectonic-${VERSION}-aarch64-apple-darwin.tar.gz" ;;
  *) echo "No prebuilt mapping for $TRIPLE yet; install tectonic and copy it to src-tauri/binaries/nawl-tectonic-$TRIPLE" >&2; exit 1 ;;
esac
URL="https://github.com/tectonic-typesetting/tectonic/releases/download/tectonic%40${VERSION}/${ASSET}"
DEST="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/binaries"
TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
curl -fsSL -o "$TMP/archive" "$URL"
case "$ASSET" in
  *.zip)
    if command -v unzip >/dev/null; then unzip -q -o "$TMP/archive" "tectonic$EXT" -d "$TMP"
    else
      # Windows without unzip (Git Bash on CI): PowerShell can.
      mv "$TMP/archive" "$TMP/archive.zip"
      powershell -NoProfile -Command "Expand-Archive -Force -Path '$(cygpath -w "$TMP/archive.zip")' -DestinationPath '$(cygpath -w "$TMP")'"
    fi ;;
  *) tar xzf "$TMP/archive" -C "$TMP" tectonic ;;
esac
mkdir -p "$DEST"
install -m 755 "$TMP/tectonic$EXT" "$DEST/nawl-tectonic-$TRIPLE$EXT"
echo "installed $DEST/nawl-tectonic-$TRIPLE$EXT"
# Only a binary for this computer can be run here.
if [ "$TRIPLE" = "$(rustc -vV | sed -n 's/^host: //p')" ]; then "$DEST/nawl-tectonic-$TRIPLE$EXT" --version; fi
