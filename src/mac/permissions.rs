//! Opens the System Settings pane where the user grants Accessibility. Retone never runs
//! `tccutil` and never scripts System Settings; the user turns the switch on.

#![forbid(unsafe_code)]

use objc2_app_kit::NSWorkspace;
use objc2_foundation::{NSString, NSURL};

const ACCESSIBILITY_PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility";

pub fn open_accessibility_settings() -> bool {
    let Some(url) = NSURL::URLWithString(&NSString::from_str(ACCESSIBILITY_PANE)) else { return false };
    NSWorkspace::sharedWorkspace().openURL(&url)
}
