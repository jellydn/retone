#!/bin/sh
# Builds Retone.app. Local builds (the default) use the dev bundle ID and an ad-hoc signature,
# so they never share an Accessibility grant or prefs with a release.
#
#   scripts/bundle.sh                 # dev: "Retone Dev.app", io.github.jellydn.retone.dev
#   scripts/bundle.sh --release       # release: "Retone.app", io.github.jellydn.retone
#   scripts/bundle.sh --out DIR       # where the .app goes (default: target/bundle)
#
# Release options, from the environment:
#   SPARKLE_FRAMEWORK   path to Sparkle.framework; embedded only together with SPARKLE_PUBLIC_KEY
#   SPARKLE_PUBLIC_KEY  the EdDSA public key; without it the app has no automatic updates,
#                       because Sparkle must never install an update it cannot verify
#   SIGNING_IDENTITY    codesign identity; default "Retone Release Signing" when the keychain
#                       has it, otherwise ad hoc
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/target/bundle"
release=false
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out=$2; shift 2 ;;
    --release) release=true; shift ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)

if $release; then
  name="Retone"
  bundle_id="io.github.jellydn.retone"
  cargo build --locked --release --manifest-path "$root/Cargo.toml"
  binary="$root/target/release/retone"
else
  name="Retone Dev"
  bundle_id="io.github.jellydn.retone.dev"
  # A debug build: it keeps Rune's headless tooling and the dev prefs folder, and the spikes
  # need its diagnostics more than its speed.
  cargo build --manifest-path "$root/Cargo.toml"
  binary="$root/target/debug/retone"
fi

sparkle_keys=""
if $release && [ -n "${SPARKLE_PUBLIC_KEY:-}" ]; then
  if [ ! -d "${SPARKLE_FRAMEWORK:-}" ]; then
    echo "SPARKLE_PUBLIC_KEY is set, but SPARKLE_FRAMEWORK is not a Sparkle.framework folder" >&2
    exit 1
  fi
  sparkle_keys="  <key>SUFeedURL</key><string>https://raw.githubusercontent.com/jellydn/retone/main/appcast.xml</string>
  <key>SUPublicEDKey</key><string>$SPARKLE_PUBLIC_KEY</string>
  <key>SUEnableAutomaticChecks</key><true/>
  <key>SUScheduledCheckInterval</key><integer>86400</integer>"
fi

# Services menu items (src/mac/services.rs): each sends the selection to Retone, which opens its
# popup in that style. No NSReturnTypes, so the asking app never changes from here, and no
# NSKeyEquivalent: Retone has no default shortcut. NSRequiredContext must be present, or macOS
# registers the item but does not show it.
service() {
  cat <<SERVICE
    <dict>
      <key>NSMenuItem</key><dict><key>default</key><string>Retone: $1</string></dict>
      <key>NSMessage</key><string>rewriteSelection</string>
      <key>NSUserData</key><string>$2</string>
      <key>NSPortName</key><string>$name</string>
      <key>NSSendTypes</key><array><string>NSStringPboardType</string><string>public.utf8-plain-text</string></array>
      <key>NSRequiredContext</key><dict/>
    </dict>
SERVICE
}
services="  <key>NSServices</key>
  <array>
$(service "Fix Spelling and Grammar" fix)
$(service "Improve Writing" clear)
$(service "Summarize" summarize)
$(service "Bullet Points" bullets)
  </array>"

app="$out/$name.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$binary" "$app/Contents/MacOS/retone"
cp "$root/assets/icon/AppIcon.icns" "$app/Contents/Resources/AppIcon.icns"
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
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>14.0</string>
  <!-- Menu-bar app: no Dock icon, no menu bar of its own. -->
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
$services
$sparkle_keys
</dict>
</plist>
EOF

identity=${SIGNING_IDENTITY:-}
if $release && [ -z "$identity" ] && security find-identity -v -p codesigning | grep -q '"Retone Release Signing"'; then
  identity="Retone Release Signing"
fi
identity=${identity:--}

if [ -n "$sparkle_keys" ]; then
  mkdir -p "$app/Contents/Frameworks"
  ditto "$SPARKLE_FRAMEWORK" "$app/Contents/Frameworks/Sparkle.framework"
  # Sign the framework's helpers inside out; --deep is not reliable for Sparkle's XPC services.
  fw="$app/Contents/Frameworks/Sparkle.framework/Versions/B"
  for part in "$fw/XPCServices/Installer.xpc" "$fw/XPCServices/Downloader.xpc" "$fw/Autoupdate" "$fw/Updater.app"; do
    codesign --force --options runtime --sign "$identity" "$part"
  done
  codesign --force --options runtime --sign "$identity" "$app/Contents/Frameworks/Sparkle.framework"
fi

# Ad hoc: macOS ties the Accessibility grant to this exact build, so each new build asks again.
# A stable certificate keeps the grant across updates.
codesign --force --sign "$identity" --identifier "$bundle_id" "$app"
codesign --verify --deep --strict "$app"
codesign -dr - "$app" 2>&1 | sed -n 's/^# designated => /designated requirement: /p'
echo "$app"
