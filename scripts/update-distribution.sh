#!/bin/sh
# After a release is published: writes Casks/retone.rb for the new DMG, and appcast.xml when the
# DMG was signed for Sparkle. Run from the repo root with the assets in target/release/assets.
#
#   scripts/update-distribution.sh VERSION [SPARKLE_TOOLS_DIR]
#
# SPARKLE_PRIVATE_KEY (environment) signs the appcast item; without it appcast.xml is left as is.
set -eu
version=$1
tools=${2:-}
dmg="target/release/assets/Retone-$version.dmg"
url="https://github.com/jellydn/retone/releases/download/v$version/Retone-$version.dmg"
sha=$(shasum -a 256 "$dmg" | cut -d' ' -f1)

# Never let an older tag that finishes late replace a newer cask or appcast.
current=$(sed -n 's/^  version "\(.*\)"/\1/p' Casks/retone.rb 2>/dev/null || true)
if [ -n "$current" ] && ! python3 - "$current" "$version" <<'PY'
import re, sys

def key(v):
    core, _, pre = v.partition("-")
    nums = tuple(int(n) for n in core.split("."))
    # SemVer: a pre-release sorts before its release; numeric parts compare as numbers.
    parts = tuple((0, int(p), "") if p.isdigit() else (1, 0, p) for p in pre.split(".")) if pre else ()
    return nums + ((0, parts) if pre else (1, ()),)

current, new = sys.argv[1:]
sys.exit(0 if key(new) >= key(current) else 1)
PY
then
  echo "Casks/retone.rb has $current, newer than $version: not changed." >&2
  exit 0
fi

cat > Casks/retone.rb <<EOF
cask "retone" do
  version "$version"
  sha256 "$sha"

  url "https://github.com/jellydn/retone/releases/download/v#{version}/Retone-#{version}.dmg"
  name "Retone"
  desc "Rewrite the selected text in any app with your own hotkey"
  homepage "https://github.com/jellydn/retone"

  # Betas are GitHub pre-releases, which :github_latest and the default
  # :github_releases skip; keep them, but not drafts.
  livecheck do
    url :url
    regex(/^v?(\d+(?:\.\d+)+(?:-[\w.]+)?)$/i)
    strategy :github_releases do |json, regex|
      json.map do |release|
        next if release["draft"]

        match = release["tag_name"]&.match(regex)
        next if match.blank?

        match[1]
      end
    end
  end

  depends_on arch: :arm64
  depends_on macos: ">= :sonoma"

  app "Retone.app"

  zap trash: [
    "~/Library/Application Support/io.github.jellydn.retone",
    "~/Library/Preferences/io.github.jellydn.retone.plist",
  ]
end
EOF

if [ -z "${SPARKLE_PRIVATE_KEY:-}" ] || [ -z "$tools" ]; then
  echo "No Sparkle key: appcast.xml not changed." >&2
  exit 0
fi

# sign_update prints: sparkle:edSignature="..." length="..."
attrs=$(printf '%s' "$SPARKLE_PRIVATE_KEY" | "$tools/bin/sign_update" --ed-key-file - "$dmg")
date=$(LC_ALL=C date -u '+%a, %d %b %Y %H:%M:%S +0000')
# One item: Sparkle only needs the newest release, and a short feed is easy to review.
cat > appcast.xml <<EOF
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>Retone Releases</title>
    <link>https://github.com/jellydn/retone</link>
    <language>en</language>
    <item>
      <title>Version $version</title>
      <link>https://github.com/jellydn/retone/releases/tag/v$version</link>
      <sparkle:version>$version</sparkle:version>
      <sparkle:shortVersionString>$version</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>14.0</sparkle:minimumSystemVersion>
      <pubDate>$date</pubDate>
      <enclosure url="$url" $attrs type="application/octet-stream" />
    </item>
  </channel>
</rss>
EOF
echo "appcast.xml updated for $version" >&2
