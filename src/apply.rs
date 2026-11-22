//! Replace: write the rewrite over the selection with the app's strategy (see `strategy.rs`).
//!
//! Before anything changes, the frontmost app, the focused element and the selection must still
//! match the capture; otherwise Replace is refused and Copy stays available. Paste restores the
//! clipboard afterwards. AX set reads the text back, and a failed read-back is an error, never
//! a paste: the write may have landed, and a paste would insert the text twice.

use crate::capture::Capture;
use crate::mac::{events, pasteboard::Board, screens};
use crate::spike_log::spike;
use crate::strategy::Strategy;
use std::thread::sleep;
use std::time::Duration;

pub fn replace(capture: &Capture, rewrite: &str, board: &Board) -> Result<(), String> {
    let result = write_over(capture, rewrite, board);
    spike!(
        "replace app={} strategy={:?} result={result:?}",
        screens::frontmost_bundle_id().unwrap_or_default(),
        capture.strategy
    );
    result
}

fn write_over(capture: &Capture, rewrite: &str, board: &Board) -> Result<(), String> {
    let (Some(pid), Some(target)) = (capture.pid, capture.target.as_ref()) else {
        return Err("Retone cannot check where the text came from".into());
    };
    if let Err(reason) = target.still_matches(&capture.captured.text) {
        spike!("replace check failed: {reason}");
        return Err("the selection changed".into());
    }
    if target.is_secure() {
        return Err("the field is a password field".into());
    }
    match capture.strategy {
        Strategy::CopyOnly => Err("this app is Copy only".into()),
        Strategy::AxSet => {
            if screens::frontmost_pid() != Some(pid) {
                spike!("replace check failed: frontmost pid changed");
                return Err("the selection changed".into());
            }
            target.set_selected_text(rewrite).map_err(String::from)
        }
        Strategy::Paste { settle } => paste_over(pid, rewrite, board, settle),
    }
}

/// `settle` is the time for the target app to read the pasteboard after ⌘V before the restore.
/// A slower app pastes the restored clipboard instead; the strategy table tunes it per app.
fn paste_over(pid: i32, rewrite: &str, board: &Board, settle: Duration) -> Result<(), String> {
    if events::secure_input_on() {
        return Err("another app has secure keyboard input on".into());
    }
    let snapshot = board.snapshot();
    if !snapshot.complete {
        return Err("the clipboard holds data Retone cannot restore".into());
    }
    if !events::wait_for_modifier_release() {
        return Err("a modifier key is held down".into());
    }
    let written = board.write_transient_text(rewrite);
    // Last check before ⌘V, so an app switch during the AX re-check or the modifier wait is
    // still caught. It narrows the gap; it cannot make the check and the paste atomic.
    if screens::frontmost_pid() != Some(pid) {
        board.restore_if_unchanged(&snapshot, written);
        spike!("replace check failed: frontmost pid changed");
        return Err("the selection changed".into());
    }
    if !events::post_paste(pid) {
        board.restore_if_unchanged(&snapshot, written);
        return Err("Retone could not send ⌘V".into());
    }
    sleep(settle);
    // Skipped when the user copied something new meanwhile: never overwrite a newer copy.
    board.restore_if_unchanged(&snapshot, written);
    Ok(())
}
