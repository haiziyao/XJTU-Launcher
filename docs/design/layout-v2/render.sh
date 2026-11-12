#!/bin/sh
# Render every render/*.html to a 1120x760 @2x PNG next to this script.
cd "$(dirname "$0")"
for f in ${@:-render/*.html}; do
  name=$(basename "$f" .html | tr 'a-z' 'A-Z' | sed 's/^\([AB]\)-/\1-/')
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" --headless=new --disable-gpu \
    --hide-scrollbars --force-device-scale-factor=2 --window-size=1120,760 \
    --screenshot="$PWD/$(basename "$f" .html).png" "file://$PWD/$f" >/dev/null 2>&1
done
