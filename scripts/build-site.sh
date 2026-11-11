#!/usr/bin/env bash
# Assembles the project website into _site/ (or the folder given): the page in site/,
# plus the icon, the preview card and the screenshots from the docs, so they're kept
# in one place. GitHub Pages publishes the result (.github/workflows/pages.yml).
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-_site}"
rm -rf "$out"
mkdir -p "$out/images"
cp -r site/. "$out/"
cp assets/nawl-icon.png "$out/icon.png"
cp assets/social-preview.png "$out/social-preview.png"
cp docs/images/*.png "$out/images/"
echo "site assembled in $out"
