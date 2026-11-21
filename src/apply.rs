//! Replace: paste the rewrite over the selection, then restore the clipboard.
//!
//! Before anything changes, the frontmost app, the focused element and the selection must still
//! match the capture; otherwise Replace is refused and Copy stays available. AX set (writing
//! `AXSelectedText` directly) waits for the Phase 2 app matrix, because many apps report success
//! without writing.

use crate::capture::Capture;
use crate::mac::{events, pasteboard::Board, screens};
use crate::spike_log::spike;
use std::thread::sleep;
use std::time::Duration;

/// Time for the target app to read the pasteboard after ⌘V before the restore. Slow apps can
/// still read later; then they paste the restored clipboard, which the per-app table in
/// Phase 2 tunes.
const PASTE_SETTLE: Duration = Duration::from_millis(400);

pub fn replace(capture: &Capture, rewrite: &str, board: &Board) -> Result<(), String> {
    let result = paste_over(capture, rewrite, board);
    spike!("replace app={} result={result:?}", screens::frontmost_bundle_id().unwrap_or_default());
    result
}

fn paste_over(capture: &Capture, rewrite: &str, board: &Board) -> Result<(), String> {
    let (Some(pid), Some(target)) = (capture.pid, capture.target.as_ref()) else {
        return Err("Retone cannot check where the text came from".into());
    };
    if screens::frontmost_pid() != Some(pid) {
        spike!("replace check failed: frontmost pid changed");
        return Err("the selection changed".into());
    }
    if let Err(reason) = target.still_matches(pid, &capture.captured.text) {
        spike!("replace check failed: {reason}");
        return Err("the selection changed".into());
    }
    if target.is_secure() {
        return Err("the field is a password field".into());
    }
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
    if !events::post_paste(pid) {
        board.restore_if_unchanged(&snapshot, written);
        return Err("Retone could not send ⌘V".into());
    }
    sleep(PASTE_SETTLE);
    // Skipped when the user copied something new meanwhile: never overwrite a newer copy.
    board.restore_if_unchanged(&snapshot, written);
    Ok(())
}
