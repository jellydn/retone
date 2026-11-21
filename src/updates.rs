//! Which way this copy of Retone gets updates. Exactly one owner, so Sparkle and Homebrew never
//! replace the app under each other.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateChannel {
    /// Sparkle checks the appcast and installs signed updates.
    Sparkle,
    /// `brew upgrade --cask retone` owns updates; Sparkle stays off.
    Homebrew,
    /// No updater in this build (dev builds, or a release without a Sparkle key).
    #[default]
    Manual,
}

impl UpdateChannel {
    pub fn summary(self) -> &'static str {
        match self {
            UpdateChannel::Sparkle => "Retone checks for updates automatically and asks before it installs one.",
            UpdateChannel::Homebrew => "Homebrew installed Retone. Update it with: brew upgrade --cask retone",
            UpdateChannel::Manual => {
                "This build has no automatic updates. Download new versions from github.com/jellydn/retone/releases."
            }
        }
    }
}
