//! The macOS Services entries ("Retone: Fix Spelling and Grammar" and others) in the context
//! menu of any app that supports Services. `scripts/bundle.sh` lists them in Info.plist.
//!
//! A service only sends the selected text to Retone and returns nothing, so the app that asked
//! never changes here: Retone opens its popup with a preview, and Replace runs the same checks
//! as after the hotkey.

use crate::popup::Popup;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSPasteboard, NSWorkspace};
use objc2_foundation::NSString;
use rune::AppHandle;
use std::cell::RefCell;
use std::ffi::c_void;

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and ServiceProvider does not implement Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppHandle<Popup>]
    struct ServiceProvider;

    impl ServiceProvider {
        /// `NSMessage` in Info.plist; `NSUserData` is the style id.
        #[unsafe(method(rewriteSelection:userData:error:))]
        fn rewrite_selection(&self, board: &NSPasteboard, style_id: Option<&NSString>, _error: *mut c_void) {
            let text = board.stringForType(&NSString::from_str("public.utf8-plain-text")).map(|t| t.to_string());
            give_focus_back(MainThreadMarker::from(self));
            let (Some(text), Some(id)) = (text, style_id.map(|id| id.to_string())) else { return };
            self.ivars().update(move |p, tx| p.on_service(&id, text, tx));
        }
    }
);

/// macOS makes Retone the frontmost app while it handles a Service. The app that asked still owns
/// the menu bar (Retone has none), so it gets activation back: then it keeps its selection, and
/// the checks before Replace find its field in front.
fn give_focus_back(mtm: MainThreadMarker) {
    let Some(asking) = NSWorkspace::sharedWorkspace().menuBarOwningApplication() else { return };
    if asking.processIdentifier() == std::process::id() as i32 {
        return;
    }
    NSApplication::sharedApplication(mtm).yieldActivationToApplication(&asking);
    asking.activateWithOptions(NSApplicationActivationOptions::empty());
}

thread_local! {
    /// AppKit does not retain the services provider; it lives for the whole run.
    static PROVIDER: RefCell<Option<Retained<ServiceProvider>>> = const { RefCell::new(None) };
}

/// Registers the provider. Call on the main thread, once.
pub fn install(popup: AppHandle<Popup>) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let this = ServiceProvider::alloc(mtm).set_ivars(popup);
    // SAFETY: NSObject's designated initializer, called once on a freshly allocated object.
    let provider: Retained<ServiceProvider> = unsafe { msg_send![super(this), init] };
    // SAFETY: the provider responds to the `NSMessage` selector in Info.plist and is kept alive
    // in PROVIDER for the whole run.
    unsafe { NSApplication::sharedApplication(mtm).setServicesProvider(Some(&provider)) };
    PROVIDER.with(|slot| *slot.borrow_mut() = Some(provider));
}
