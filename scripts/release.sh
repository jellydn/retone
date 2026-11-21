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
mount=""
# On failure, detach the writable image before deleting it; keep it if it cannot be detached.
cleanup() {
  if [ -n "$mount" ] && ! hdiutil detach -quiet "$mount"; then
    rm -rf "$staging"
    return
  fi
  rm -rf "$staging" "$staging.rw.dmg" "$staging.mnt"
}
trap cleanup EXIT
ditto "$app" "$staging/Retone.app"
ln -s /Applications "$staging/Applications"
cp assets/icon/AppIcon.icns "$staging/.VolumeIcon.icns"
# The volume icon needs the custom-icon flag on the volume root, which -srcfolder does not
# copy; so make a writable image, set the flag on the mounted volume, then compress it.
rw="$staging.rw.dmg"
hdiutil create -volname Retone -srcfolder "$staging" -ov -format UDRW "$rw"
# An explicit mount point: the last line of `hdiutil attach` differs between machines.
mount="$staging.mnt"
mkdir -p "$mount"
hdiutil attach -nobrowse -noautoopen -mountpoint "$mount" "$rw" >/dev/null
SetFile -a C "$mount"
hdiutil detach -quiet "$mount"
mount=""
hdiutil convert "$rw" -ov -format UDZO -o "$assets/Retone-$version.dmg"
rm -f "$rw"
ditto -c -k --sequesterRsrc --keepParent "$app" "$assets/Retone-$version.zip"
(
  cd "$assets"
  shasum -a 256 "Retone-$version.dmg" "Retone-$version.zip" > SHA256SUMS
)
printf 'Release assets: %s\n' "$assets"
