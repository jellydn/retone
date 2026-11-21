#!/bin/sh
# Rebuilds the icon files from the SVG sources in assets/icon (needs rsvg-convert and iconutil).
# The outputs are committed, so builds and CI do not need rsvg-convert.
#
#   python3 scripts/icon-svgs.py assets/icon   # only after changing the shapes or colours
#   sh scripts/make-icons.sh
set -eu
cd "$(dirname "$0")/../assets/icon"
set_dir=$(mktemp -d)/AppIcon.iconset
mkdir -p "$set_dir"
for size in 16 32 128 256 512; do
  # 16 px uses the simpler drawing: the text lines blur into noise at that size.
  small=icon.svg
  [ "$size" = 16 ] && small=icon-16.svg
  rsvg-convert -w "$size" "$small" -o "$set_dir/icon_${size}x${size}.png"
  rsvg-convert -w $((size * 2)) icon.svg -o "$set_dir/icon_${size}x${size}@2x.png"
done
iconutil -c icns "$set_dir" -o AppIcon.icns
# Menu bar: 18 pt tall, 1x and 2x, black with alpha (a template image).
rsvg-convert -h 18 menubar-template.svg -o menubar-template.png
rsvg-convert -h 36 menubar-template.svg -o menubar-template@2x.png
rm -rf "$(dirname "$set_dir")"
echo "assets/icon: AppIcon.icns, menubar-template.png, menubar-template@2x.png"
