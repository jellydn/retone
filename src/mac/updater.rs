//! Sparkle, loaded at run time from `Contents/Frameworks/Sparkle.framework`.
//!
//! Loading it at run time (not linking it) lets one binary run in three ways: dev builds and
//! `cargo run` have no framework, Homebrew installs must not self-update, and a release only
//! gets a working updater when the bundle also carries an `SUPublicEDKey` to verify downloads.

use objc2::msg_send;
use objc2::rc::{Allocated, Retained};
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2_foundation::{NSBundle, NSString};
use std::cell::RefCell;
use std::path::{Path, PathBuf};

thread_local! {
    /// The `SPUStandardUpdaterController`; it must stay alive for scheduled checks to run.
    static CONTROLLER: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

/// True when Homebrew installed Retone, so `brew upgrade` owns updates.
pub fn installed_by_homebrew() -> bool {
    // Apps opened from Finder do not get the shell's HOMEBREW_PREFIX, so the default prefixes
    // are checked too. A custom prefix is found only when Retone starts with that variable set.
    let custom = std::env::var_os("HOMEBREW_PREFIX").map(|prefix| Path::new(&prefix).join("Caskroom/retone"));
    let defaults = ["/opt/homebrew/Caskroom/retone", "/usr/local/Caskroom/retone"].map(PathBuf::from);
    custom.into_iter().chain(defaults).any(|path| path.is_dir())
}

/// Starts Sparkle when this bundle can verify updates. Returns whether it started.
/// Call on the main thread, once.
pub fn start() -> bool {
    let main = NSBundle::mainBundle();
    let key = main.objectForInfoDictionaryKey(&NSString::from_str("SUPublicEDKey"));
    if key.is_none() {
        return false;
    }
    let Some(frameworks) = main.privateFrameworksPath() else { return false };
    let path = NSString::from_str(&format!("{frameworks}/Sparkle.framework"));
    let Some(framework) = NSBundle::bundleWithPath(&path) else { return false };
    // SAFETY: loads the signed Sparkle framework from inside our own bundle; its initializers
    // only register Objective-C classes.
    if !unsafe { framework.load() } {
        return false;
    }
    let Some(class) = AnyClass::get(c"SPUStandardUpdaterController") else { return false };
    // SAFETY: Sparkle 2 declares `-initWithStartingUpdater:(BOOL) updaterDelegate:(id)
    // userDriverDelegate:(id)`; nil delegates are allowed, and the class was just loaded.
    // Not started here: the initializer returns a controller even when startup fails.
    let Some(controller): Option<Retained<AnyObject>> = (unsafe {
        let alloc: Allocated<AnyObject> = msg_send![class, alloc];
        msg_send![alloc, initWithStartingUpdater: Bool::NO, updaterDelegate: None::<&AnyObject>, userDriverDelegate: None::<&AnyObject>]
    }) else {
        return false;
    };
    // SAFETY: `-updater` returns the controller's non-nil SPUUpdater, and
    // `-startUpdater:(NSError **)error` accepts NULL for the error. It returns NO when Sparkle
    // refuses the bundle, for example a malformed SUPublicEDKey.
    let started = unsafe {
        let updater: Retained<AnyObject> = msg_send![&*controller, updater];
        let started: Bool = msg_send![&*updater, startUpdater: std::ptr::null_mut::<*mut AnyObject>()];
        started.as_bool()
    };
    if started {
        CONTROLLER.with(|slot| *slot.borrow_mut() = Some(controller));
    }
    started
}

/// Shows Sparkle's own "check for updates" UI. Does nothing when Sparkle did not start.
pub fn check_now() {
    CONTROLLER.with(|slot| {
        if let Some(controller) = slot.borrow().as_ref() {
            // SAFETY: `-checkForUpdates:(id)sender` is an IBAction on the controller; nil is a valid sender.
            let _: () = unsafe { msg_send![&**controller, checkForUpdates: None::<&AnyObject>] };
        }
    });
}
