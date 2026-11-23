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
use crate::strategy::{self, Strategy};
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
    /// How Replace writes into this app, chosen once at capture.
    pub strategy: Strategy,
}

/// True when Retone can read other apps and post keys. Without it, clipboard-only mode.
pub fn has_permissions() -> bool {
    ax::is_trusted() && events::can_post_events()
}

pub fn capture(board: &Board, given: Option<String>) -> Result<Capture, String> {
    if !has_permissions() {
        if let Some(text) = given {
            // A Service gave the text, but without Accessibility Retone cannot check the field
            // again before a write: Copy only.
            spike!("capture path=service-no-access chars={}", text.chars().count());
            let captured = Captured { text, secure_field: false, can_replace: false, from_clipboard: false };
            return Ok(Capture { captured, target: None, pid: None, strategy: Strategy::CopyOnly });
        }
        // The user copied the text themselves; Retone only reads it and never pastes.
        let text = board.text().unwrap_or_default();
        spike!("capture path=clipboard-only chars={}", text.chars().count());
        let captured = Captured { text, secure_field: false, can_replace: false, from_clipboard: true };
        return Ok(Capture { captured, target: None, pid: None, strategy: Strategy::CopyOnly });
    }
    let pid = screens::frontmost_pid();
    // The hotkey can fire while Retone's own Settings window is in front (recording, testing);
    // reading or pasting into Retone itself makes no sense.
    if pid == Some(std::process::id() as i32) {
        return Err(t!("capture.own_window").into());
    }
    let target = ax::Target::focused();
    let bundle_id = screens::frontmost_bundle_id();
    let overrides = rune_core::env::var("RETONE_APPLY_OVERRIDE").unwrap_or_default();
    let strategy = strategy::for_app(bundle_id.as_deref(), &overrides);
    spike!(
        "capture app={} strategy={strategy:?} ax_element={} range={:?}",
        bundle_id.as_deref().unwrap_or_default(),
        target.is_some(),
        target.as_ref().and_then(|t| t.range).map(|r| (r.location, r.length))
    );

    if target.as_ref().is_some_and(ax::Target::is_secure) {
        spike!("capture path=refused-secure-field");
        let captured = Captured { text: String::new(), secure_field: true, can_replace: false, from_clipboard: false };
        return Ok(Capture { captured, target: None, pid, strategy: Strategy::CopyOnly });
    }
    if let Some(text) = given {
        return Ok(service_capture(text, target, pid, strategy));
    }
    let selection = match target.as_ref().and_then(ax::Target::selected_text) {
        Some(text) => {
            spike!("capture path=ax chars={} newlines={}", text.chars().count(), text.matches('\n').count());
            text
        }
        None => {
            let copied = copy_selection(board, pid.ok_or(t!("capture.no_app"))?);
            match &copied {
                Ok(text) => {
                    spike!("capture path=cmd-c chars={} newlines={}", text.chars().count(), text.matches('\n').count())
                }
                Err(error) => spike!("capture path=cmd-c error={error}"),
            }
            copied?
        }
    };
    // A selected range that Retone could not read (AX failed, ⌘C too slow) is still a selection:
    // falling back to the clipboard there would rewrite unrelated text.
    let range_selected = target.as_ref().and_then(|t| t.range).is_some_and(|range| range.length > 0);
    let (text, from_clipboard) = selection_or_clipboard(selection, range_selected, || board.text())?;
    if from_clipboard {
        spike!("capture path=clipboard-no-selection chars={}", text.chars().count());
    }
    // Replace needs a selection, an element to check again before writing, and an app that is
    // not Copy only.
    let can_replace = target.is_some() && !from_clipboard && strategy != Strategy::CopyOnly;
    Ok(Capture { captured: Captured { text, secure_field: false, can_replace, from_clipboard }, target, pid, strategy })
}

/// The text a Service sent. Replace stays on only when the focused field still selects that same
/// text, so the check before Replace guards the write as after the hotkey; otherwise Copy only.
fn service_capture(text: String, target: Option<ax::Target>, pid: Option<i32>, strategy: Strategy) -> Capture {
    let selected = target.as_ref().and_then(ax::Target::selected_text).filter(|s| s.trim() == text.trim());
    spike!("capture path=service chars={} matches_selection={}", text.chars().count(), selected.is_some());
    match selected {
        Some(selected) => {
            let can_replace = strategy != Strategy::CopyOnly;
            let captured = Captured { text: selected, secure_field: false, can_replace, from_clipboard: false };
            Capture { captured, target, pid, strategy }
        }
        None => {
            let captured = Captured { text, secure_field: false, can_replace: false, from_clipboard: false };
            Capture { captured, target: None, pid, strategy: Strategy::CopyOnly }
        }
    }
}

/// The selection when there is one; otherwise the clipboard, read only then. True when the text
/// came from the clipboard. An error when the app reports a selected range but its text could
/// not be read.
fn selection_or_clipboard(
    selection: String,
    range_selected: bool,
    clipboard: impl FnOnce() -> Option<String>,
) -> Result<(String, bool), String> {
    if !selection.trim().is_empty() {
        return Ok((selection, false));
    }
    if range_selected {
        return Err(t!("capture.unreadable").into());
    }
    Ok((clipboard().unwrap_or_default(), true))
}

/// Posts ⌘C to the target and reads the result, restoring the user's clipboard afterwards.
/// An empty string means the app copied nothing (no selection); the clipboard is then unchanged.
fn copy_selection(board: &Board, pid: i32) -> Result<String, String> {
    if events::secure_input_on() {
        return Err(t!("capture.secure_input").into());
    }
    if !events::wait_for_modifier_release() {
        return Err(t!("capture.modifiers").into());
    }
    let snapshot = board.snapshot();
    if !snapshot.complete {
        // Restoring part of the clipboard would lose data; let the user copy instead.
        return Err(t!("capture.clipboard").into());
    }
    if !events::post_copy(pid) {
        return Err(t!("capture.copy_failed").into());
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

    fn no_read() -> Option<String> {
        panic!("the clipboard must not be read")
    }

    #[test]
    fn a_selection_wins_over_a_full_clipboard() {
        assert_eq!(selection_or_clipboard("selected".into(), true, no_read), Ok(("selected".into(), false)));
        assert_eq!(selection_or_clipboard("selected".into(), false, no_read), Ok(("selected".into(), false)));
    }

    #[test]
    fn an_unreadable_selected_range_never_uses_the_clipboard() {
        assert!(selection_or_clipboard(String::new(), true, no_read).is_err());
    }

    #[test]
    fn no_selection_uses_the_clipboard() {
        let clipboard = || Some("on the clipboard".to_string());
        assert_eq!(selection_or_clipboard(String::new(), false, clipboard), Ok(("on the clipboard".into(), true)));
        // Some apps copy a lone newline when nothing is selected.
        assert_eq!(selection_or_clipboard("\n".into(), false, clipboard), Ok(("on the clipboard".into(), true)));
        assert_eq!(selection_or_clipboard(String::new(), false, || None), Ok((String::new(), true)));
    }
}
