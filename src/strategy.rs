//! How Replace writes into each app, chosen by the target's bundle ID.
//!
//! Paste with clipboard restore is the default, because it works wherever ⌘V works and the app
//! keeps its own undo. AX set (writing `AXSelectedText`) is used only for apps that the app
//! matrix proves: many apps report success without writing. Terminals are Copy only.

#![forbid(unsafe_code)]

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// ⌘V, then the clipboard restore after `settle`, the time the app needs to read the pasteboard.
    Paste { settle: Duration },
    /// Write `AXSelectedText`, then read it back. A failed read-back is an error, never a paste.
    AxSet,
    /// Replace is not offered.
    CopyOnly,
}

const PASTE: Strategy = Strategy::Paste { settle: Duration::from_millis(400) };
/// Electron apps read the pasteboard later than AppKit apps; a late read would paste the
/// restored clipboard. A longer wait only delays the restore. The app matrix tunes it.
const PASTE_SLOW: Strategy = Strategy::Paste { settle: Duration::from_millis(700) };

/// Apps that differ from the default. AX set entries come only from the app matrix.
const TABLE: &[(&str, Strategy)] = &[
    // Terminals: a pasted newline runs a command, and the selection is usually output, not input.
    ("com.apple.Terminal", Strategy::CopyOnly),
    ("com.googlecode.iterm2", Strategy::CopyOnly),
    ("com.mitchellh.ghostty", Strategy::CopyOnly),
    ("com.github.wez.wezterm", Strategy::CopyOnly),
    ("net.kovidgoyal.kitty", Strategy::CopyOnly),
    ("org.alacritty", Strategy::CopyOnly),
    ("dev.warp.Warp-Stable", Strategy::CopyOnly),
    ("com.tinyspeck.slackmacgap", PASTE_SLOW),
    ("com.hnc.Discord", PASTE_SLOW),
    ("notion.id", PASTE_SLOW),
    ("com.microsoft.VSCode", PASTE_SLOW),
];

/// The strategy for an app. `overrides` is `RETONE_APPLY_OVERRIDE`, a comma-separated list of
/// `bundle.id=paste|ax-set|copy-only`. It lets the app-matrix session try a strategy in one app
/// without a rebuild, which would need a new Accessibility grant. Unknown entries are ignored.
pub fn for_app(bundle_id: Option<&str>, overrides: &str) -> Strategy {
    let Some(bundle_id) = bundle_id else { return PASTE };
    let overridden = overrides
        .split(',')
        .filter_map(|entry| entry.trim().split_once('='))
        .filter(|(id, _)| *id == bundle_id)
        .find_map(|(_, name)| match name {
            "paste" => Some(PASTE),
            "ax-set" => Some(Strategy::AxSet),
            "copy-only" => Some(Strategy::CopyOnly),
            _ => None,
        });
    overridden.or_else(|| TABLE.iter().find(|(id, _)| *id == bundle_id).map(|(_, strategy)| *strategy)).unwrap_or(PASTE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_and_missing_apps_paste_and_terminals_are_copy_only() {
        assert_eq!(for_app(Some("com.apple.TextEdit"), ""), PASTE);
        assert_eq!(for_app(None, ""), PASTE);
        assert_eq!(for_app(Some("com.apple.Terminal"), ""), Strategy::CopyOnly);
        // Exact match only: a prefix of a listed ID is another app.
        assert_eq!(for_app(Some("com.apple.Term"), ""), PASTE);
    }

    #[test]
    fn an_override_wins_over_the_table_for_its_app_only() {
        let overrides = "com.apple.TextEdit=ax-set, com.apple.Terminal=paste";
        assert_eq!(for_app(Some("com.apple.TextEdit"), overrides), Strategy::AxSet);
        assert_eq!(for_app(Some("com.apple.Terminal"), overrides), PASTE);
        assert_eq!(for_app(Some("com.apple.Notes"), overrides), PASTE);
        assert_eq!(for_app(Some("com.tinyspeck.slackmacgap"), overrides), PASTE_SLOW);
    }

    #[test]
    fn a_malformed_override_falls_back_to_the_table() {
        assert_eq!(for_app(Some("com.apple.Terminal"), "com.apple.Terminal=typo"), Strategy::CopyOnly);
        assert_eq!(for_app(Some("com.apple.Terminal"), "com.apple.Terminal,=,"), Strategy::CopyOnly);
    }
}
