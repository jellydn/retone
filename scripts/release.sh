#!/bin/sh
# Packages the release bundle as a DMG and a ZIP, with SHA256SUMS, in target/release/assets.
# Passes the Sparkle and signing environment through to scripts/bundle.sh.
set -eu
cd "$(dirname "$0")/.."

version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
if [ "${GITHUB_REF_TYPE:-}" = tag ] && [ "${GITHUB_REF_NAME:-}" != "v$version" ]; then
  echo "Release tag must match Cargo.toml: v$version" >&2
  exit 1
fi

out=target/release/bundle
sh scripts/bundle.sh --release --out "$out"
app="$out/Retone.app"
lipo "$app/Contents/MacOS/retone" -verify_arch arm64

assets=target/release/assets
rm -rf "$assets"
mkdir -p "$assets"
staging=$(mktemp -d target/release/dmg-root.XXXXXX)
trap 'rm -rf "$staging"' EXIT
ditto "$app" "$staging/Retone.app"
ln -s /Applications "$staging/Applications"
hdiutil create -volname Retone -srcfolder "$staging" -ov -format UDZO "$assets/Retone-$version.dmg"
ditto -c -k --sequesterRsrc --keepParent "$app" "$assets/Retone-$version.zip"
(
  cd "$assets"
  shasum -a 256 "Retone-$version.dmg" "Retone-$version.zip" > SHA256SUMS
)
printf 'Release assets: %s\n' "$assets"
