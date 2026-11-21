#!/bin/sh
# Downloads the pinned Sparkle release, checks its SHA-256, and prints the folder that holds
# Sparkle.framework and bin/sign_update. Bump SPARKLE_VERSION and SPARKLE_SHA256 together.
set -eu
SPARKLE_VERSION=2.10.1
SPARKLE_SHA256=1a856d2a9edb8e628de88121d7be04cf0e2167f8641650b78fca00b14244cf27

root=$(cd "$(dirname "$0")/.." && pwd)
dir="$root/target/sparkle-$SPARKLE_VERSION"
if [ ! -x "$dir/bin/sign_update" ]; then
  rm -rf "$dir"
  mkdir -p "$dir"
  archive="$dir/Sparkle-$SPARKLE_VERSION.tar.xz"
  curl -fsSL -o "$archive" \
    "https://github.com/sparkle-project/Sparkle/releases/download/$SPARKLE_VERSION/Sparkle-$SPARKLE_VERSION.tar.xz"
  echo "$SPARKLE_SHA256  $archive" | shasum -a 256 -c - >&2
  tar -xf "$archive" -C "$dir"
fi
echo "$dir"
