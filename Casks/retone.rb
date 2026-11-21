cask "retone" do
  version "0.1.0-beta.3"
  sha256 "67338085bee2157af07a68be8235cef5080c8d6eb8ba71709ef9c1b075cb7ec8"

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
  depends_on macos: :sonoma

  app "Retone.app"

  zap trash: [
    "~/Library/Application Support/io.github.jellydn.retone",
    "~/Library/Preferences/io.github.jellydn.retone.plist",
  ]
end
