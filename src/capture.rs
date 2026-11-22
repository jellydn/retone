//! Reads the text to rewrite when the hotkey fires. The selection always wins: Accessibility
//! first, then a ⌘C with clipboard snapshot and restore. Only when nothing is selected does Retone
//! use what is already on the clipboard. Without permissions Retone cannot read the selection at
//! all, so it uses the clipboard (clipboard-only mode).
//!
//! Runs on a worker thread, never in the frame loop: AX calls and the ⌘C wait can take up to
//! about a second in a slow app.

use crate::flow::Captured;
use crate::mac::{ax, events, pasteboard::Board, screens};
use crate::spike_log::spike;
use std::thread::sleep;
use std::time::Duration;

/// How long a ⌘C may take to reach the pasteboard before Retone decides nothing was selected.
/// Generous, because a slow copy would otherwise fall back to the old clipboard text.
const COPY_WAIT: Duration = Duration::from_millis(500);

pub struct Capture {
    pub captured: Captured,
    /// The element to check again before Replace; None in clipboard-only mode.
    pub target: Option<ax::Target>,
    pub pid: Option<i32>,
}

/// True when Retone can read other apps and post keys. Without it, clipboard-only mode.
pub fn has_permissions() -> bool {
    ax::is_trusted() && events::can_post_events()
}

pub fn capture(board: &Board) -> Result<Capture, String> {
    if !has_permissions() {
        // The user copied the text themselves; Retone only reads it and never pastes.
        let text = board.text().unwrap_or_default();
        spike!("capture path=clipboard-only chars={}", text.chars().count());
        let captured = Captured { text, secure_field: false, can_replace: false, from_clipboard: true };
        return Ok(Capture { captured, target: None, pid: None });
    }
    let pid = screens::frontmost_pid();
    // The hotkey can fire while Retone's own Settings window is in front (recording, testing);
    // reading or pasting into Retone itself makes no sense.
    if pid == Some(std::process::id() as i32) {
        return Err("Select text in another app, then press your shortcut.".into());
    }
    let target = ax::Target::focused();
    spike!(
        "capture app={} ax_element={} range={:?}",
        screens::frontmost_bundle_id().unwrap_or_default(),
        target.is_some(),
        target.as_ref().and_then(|t| t.range).map(|r| (r.location, r.length))
    );

    if target.as_ref().is_some_and(ax::Target::is_secure) {
        spike!("capture path=refused-secure-field");
        let captured = Captured { text: String::new(), secure_field: true, can_replace: false, from_clipboard: false };
        return Ok(Capture { captured, target: None, pid });
    }
    let selection = match target.as_ref().and_then(ax::Target::selected_text) {
        Some(text) => {
            spike!("capture path=ax chars={} newlines={}", text.chars().count(), text.matches('\n').count());
            text
        }
        None => {
            let copied = copy_selection(board, pid.ok_or("No app is in front.")?);
            match &copied {
                Ok(text) => {
                    spike!("capture path=cmd-c chars={} newlines={}", text.chars().count(), text.matches('\n').count())
                }
                Err(error) => spike!("capture path=cmd-c error={error}"),
            }
            copied?
        }
    };
    let (text, from_clipboard) = selection_or_clipboard(selection, || board.text());
    if from_clipboard {
        spike!("capture path=clipboard-no-selection chars={}", text.chars().count());
    }
    // Replace needs a selection and an element to check again before pasting; otherwise Copy only.
    let can_replace = target.is_some() && !from_clipboard;
    Ok(Capture { captured: Captured { text, secure_field: false, can_replace, from_clipboard }, target, pid })
}

/// The selection when there is one; otherwise the clipboard, read only then. True when the text
/// came from the clipboard.
fn selection_or_clipboard(selection: String, clipboard: impl FnOnce() -> Option<String>) -> (String, bool) {
    if !selection.trim().is_empty() {
        return (selection, false);
    }
    (clipboard().unwrap_or_default(), true)
}

/// Posts ⌘C to the target and reads the result, restoring the user's clipboard afterwards.
/// An empty string means the app copied nothing (no selection); the clipboard is then unchanged.
fn copy_selection(board: &Board, pid: i32) -> Result<String, String> {
    if events::secure_input_on() {
        return Err("Another app has secure keyboard input on, so Retone cannot copy the selection.".into());
    }
    if !events::wait_for_modifier_release() {
        return Err("Release the shortcut keys, then try again.".into());
    }
    let snapshot = board.snapshot();
    if !snapshot.complete {
        // Restoring part of the clipboard would lose data; let the user copy instead.
        return Err(
            "The clipboard holds data Retone cannot restore. Copy the text with ⌘C, then press your shortcut.".into()
        );
    }
    if !events::post_copy(pid) {
        return Err("Retone could not send ⌘C.".into());
    }
    let start = rune_core::time::now();
    while board.change_count() == snapshot.change_count {
        if start.elapsed() > COPY_WAIT {
            return Ok(String::new());
        }
        sleep(Duration::from_millis(10));
    }
    let copied = board.change_count();
    let text = board.text().unwrap_or_default();
    board.restore_if_unchanged(&snapshot, copied);
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::selection_or_clipboard;

    #[test]
    fn a_selection_wins_over_a_full_clipboard() {
        let read = || -> Option<String> { panic!("the clipboard must not be read when text is selected") };
        assert_eq!(selection_or_clipboard("selected".into(), read), ("selected".into(), false));
    }

    #[test]
    fn no_selection_uses_the_clipboard() {
        let clipboard = || Some("on the clipboard".to_string());
        assert_eq!(selection_or_clipboard(String::new(), clipboard), ("on the clipboard".into(), true));
        // Some apps copy a lone newline when nothing is selected.
        assert_eq!(selection_or_clipboard("\n".into(), clipboard), ("on the clipboard".into(), true));
        assert_eq!(selection_or_clipboard(String::new(), || None), (String::new(), true));
    }
}
