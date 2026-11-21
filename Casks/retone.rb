cask "retone" do
  version "0.1.0-beta.1"
  sha256 "2159cabbf9e88f24aaae9cfe5fcf43e724561d02cbd6fc24c8b9c15ceb625af4"

  url "https://github.com/jellydn/retone/releases/download/v#{version}/Retone-#{version}.dmg"
  name "Retone"
  desc "Rewrite the selected text in any app with your own hotkey"
  homepage "https://github.com/jellydn/retone"

  # Betas are GitHub pre-releases, which :github_latest skips.
  livecheck do
    url :url
    strategy :github_releases
  end

  depends_on arch: :arm64
  depends_on macos: ">= :sonoma"

  app "Retone.app"

  zap trash: [
    "~/Library/Application Support/io.github.jellydn.retone",
    "~/Library/Preferences/io.github.jellydn.retone.plist",
  ]
end
