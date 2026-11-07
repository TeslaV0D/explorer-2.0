#!/bin/bash
# Regenerates PNG sizes + Windows .ico from the SVG sources.
# Needs: rsvg-convert (librsvg2-bin), ImageMagick (convert).
set -euo pipefail
cd "$(dirname "$0")/../crates/app/res/icon"
for s in 256 128 64 48; do rsvg-convert -w $s -h $s explorer2.svg -o icon-$s.png; done
for s in 32 24 16; do rsvg-convert -w $s -h $s explorer2-small.svg -o icon-$s.png; done   # simplified small variant
convert icon-16.png icon-24.png icon-32.png icon-48.png icon-64.png icon-128.png icon-256.png explorer2.ico
identify explorer2.ico
