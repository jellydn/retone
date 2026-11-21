#!/bin/sh
# Builds Retone.app. Local builds (the default) use the dev bundle ID and an ad-hoc signature,
# so they never share an Accessibility grant or prefs with a release.
#
#   scripts/bundle.sh                 # dev: "Retone Dev.app", io.github.jellydn.retone.dev
#   scripts/bundle.sh --out DIR       # where the .app goes (default: target/bundle)
#
# Release signing with the "Retone Release Signing" certificate and Sparkle embedding come
# with the release workflow; this script refuses to pretend it does them.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/target/bundle"
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out=$2; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

name="Retone Dev"
bundle_id="io.github.jellydn.retone.dev"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)

# A debug build: it keeps Rune's headless tooling and the dev prefs folder, and the spikes
# need its diagnostics more than its speed.
cargo build --manifest-path "$root/Cargo.toml"

app="$out/$name.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "$root/target/debug/retone" "$app/Contents/MacOS/retone"
cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$bundle_id</string>
  <key>CFBundleName</key><string>$name</string>
  <key>CFBundleDisplayName</key><string>$name</string>
  <key>CFBundleExecutable</key><string>retone</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <!-- Menu-bar app: no Dock icon, no menu bar of its own. -->
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
EOF

# Ad hoc: macOS ties the Accessibility grant to this exact build, so each rebuild asks again.
codesign --force --sign - --identifier "$bundle_id" "$app"
codesign --verify --strict "$app"
codesign -dr - "$app" 2>&1 | sed -n 's/^# designated => /designated requirement: /p'
echo "$app"
