//! Synthetic ⌘C / ⌘V for the clipboard fallback, and the checks around them.

use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGKeyCode, CGPreflightPostEventAccess,
    CGRequestPostEventAccess,
};
use std::thread::sleep;
use std::time::Duration;

/// ANSI key positions of C and V. Apps match ⌘ shortcuts by the layout's character, so a layout
/// that moves C or V (Dvorak) needs a translated key code; spike S2 covers ABC, German, French.
const KEY_C: CGKeyCode = 8;
const KEY_V: CGKeyCode = 9;
/// The user still holds the hotkey's modifiers when its handler runs; a ⌘C posted then would
/// arrive as ⌘⌥C or similar. Waiting longer than this means a stuck key, so give up.
const MODIFIER_RELEASE_LIMIT: Duration = Duration::from_millis(500);

pub fn can_post_events() -> bool {
    CGPreflightPostEventAccess()
}

pub fn request_post_events() -> bool {
    CGRequestPostEventAccess()
}

/// Waits until no modifier key is down. Returns false after the limit.
pub fn wait_for_modifier_release() -> bool {
    let modifiers = CGEventFlags::MaskCommand
        | CGEventFlags::MaskShift
        | CGEventFlags::MaskControl
        | CGEventFlags::MaskAlternate
        | CGEventFlags::MaskSecondaryFn;
    let start = rune_core::time::now();
    while start.elapsed() < MODIFIER_RELEASE_LIMIT {
        if !CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState).intersects(modifiers) {
            return true;
        }
        sleep(Duration::from_millis(10));
    }
    false
}

pub fn post_copy(pid: i32) -> bool {
    post_command_key(pid, KEY_C)
}

pub fn post_paste(pid: i32) -> bool {
    post_command_key(pid, KEY_V)
}

/// Posts ⌘key straight to the target process. Not through the HID tap: during Replace Retone's
/// popup holds key focus, and an HID-tap ⌘V would paste into the popup instead of the target.
fn post_command_key(pid: i32, key: CGKeyCode) -> bool {
    for key_down in [true, false] {
        let Some(event) = CGEvent::new_keyboard_event(None, key, key_down) else { return false };
        // Explicit flags: without them the event takes whatever modifiers are down right now.
        CGEvent::set_flags(Some(&event), CGEventFlags::MaskCommand);
        CGEvent::post_to_pid(pid, Some(&event));
    }
    true
}

/// When another app turned on Secure Event Input (a password field, some terminals), synthetic
/// keys do not arrive; the popup says so instead of failing silently.
pub fn secure_input_on() -> bool {
    #[link(name = "Carbon", kind = "framework")]
    unsafe extern "C" {
        fn IsSecureEventInputEnabled() -> u8;
    }
    // SAFETY: takes no arguments and only reads system state.
    unsafe { IsSecureEventInputEnabled() != 0 }
}
