//! Screens, the mouse pointer and the frontmost app, in Retone's top-left global points.
//! AppKit requires the main thread for these, so they take a `MainThreadMarker`.

#![forbid(unsafe_code)]

use crate::placement::{Rect, Screen, from_appkit};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSEvent, NSScreen, NSWorkspace};
use objc2_foundation::NSRect;

fn rect(r: NSRect) -> Rect {
    Rect::new(r.origin.x, r.origin.y, r.size.width, r.size.height)
}

/// Every screen; the first is the primary one (with the menu bar), whose height sets the flip.
pub fn screens(mtm: MainThreadMarker) -> Vec<Screen> {
    let all = NSScreen::screens(mtm).to_vec();
    let Some(primary_height) = all.first().map(|screen| screen.frame().size.height) else { return Vec::new() };
    all.iter()
        .map(|screen| Screen {
            frame: from_appkit(rect(screen.frame()), primary_height),
            visible: from_appkit(rect(screen.visibleFrame()), primary_height),
            scale: screen.backingScaleFactor(),
        })
        .collect()
}

/// The mouse pointer as a zero-size anchor, for apps without usable caret bounds.
pub fn mouse_anchor(mtm: MainThreadMarker) -> Rect {
    let point = NSEvent::mouseLocation();
    let primary_height = NSScreen::screens(mtm).to_vec().first().map_or(0.0, |screen| screen.frame().size.height);
    from_appkit(Rect::new(point.x, point.y, 0.0, 0.0), primary_height)
}

/// The bundle ID of the app in front, for the spike log.
pub fn frontmost_bundle_id() -> Option<String> {
    Some(NSWorkspace::sharedWorkspace().frontmostApplication()?.bundleIdentifier()?.to_string())
}

/// The pid of the app in front, where the hotkey was pressed.
pub fn frontmost_pid() -> Option<i32> {
    Some(NSWorkspace::sharedWorkspace().frontmostApplication()?.processIdentifier())
}
