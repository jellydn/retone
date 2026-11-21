//! The menu-bar item. Rune's tray has no macOS backend yet, so this is a small `NSStatusItem`
//! with a menu whose items forward to the app through its handles.

use crate::popup::Popup;
use crate::settings::{Page, SettingsUi};
use crate::updates::UpdateChannel;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, Sel};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem, NSVariableStatusItemLength};
use objc2_foundation::{NSData, NSSize, NSString};
use rune::AppHandle;
use std::cell::RefCell;

pub struct Handles {
    popup: AppHandle<Popup>,
    settings: AppHandle<SettingsUi>,
    channel: UpdateChannel,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and MenuTarget does not implement Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Handles]
    struct MenuTarget;

    impl MenuTarget {
        #[unsafe(method(rewrite:))]
        fn rewrite(&self, _sender: Option<&AnyObject>) {
            self.ivars().popup.update(|p, tx| p.on_hotkey(tx));
        }

        #[unsafe(method(settings:))]
        fn settings(&self, _sender: Option<&AnyObject>) {
            self.open_page(Page::Shortcut);
        }

        #[unsafe(method(permissions:))]
        fn permissions(&self, _sender: Option<&AnyObject>) {
            self.open_page(Page::Permissions);
        }

        #[unsafe(method(updates:))]
        fn updates(&self, _sender: Option<&AnyObject>) {
            // Sparkle shows its own window; other channels explain themselves on the page.
            if self.ivars().channel == UpdateChannel::Sparkle {
                crate::mac::updater::check_now();
            } else {
                self.open_page(Page::Updates);
            }
        }

        #[unsafe(method(quit:))]
        fn quit(&self, _sender: Option<&AnyObject>) {
            self.ivars().popup.quit();
        }
    }
);

impl MenuTarget {
    fn new(mtm: MainThreadMarker, handles: Handles) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(handles);
        // SAFETY: NSObject's designated initializer, called once on a freshly allocated object.
        unsafe { msg_send![super(this), init] }
    }

    fn open_page(&self, page: Page) {
        let settings = &self.ivars().settings;
        settings.update(move |ui, _| {
            ui.page = page;
            ui.refresh_disabled();
        });
        settings.open();
    }
}

thread_local! {
    /// AppKit only keeps a weak reference to menu targets, and the status item disappears when
    /// released; both live for the whole run on the main thread.
    static INSTALLED: RefCell<Option<(Retained<NSStatusItem>, Retained<MenuTarget>)>> = const { RefCell::new(None) };
}

/// The loop-and-cursor icon as a template image, so macOS tints it for light and dark menu bars.
fn menu_bar_image() -> Option<Retained<NSImage>> {
    let data = NSData::with_bytes(include_bytes!("../../assets/icon/menubar-template@2x.png"));
    let image = NSImage::initWithData(NSImage::alloc(), &data)?;
    // The PNG is 36 px for Retina; 18 pt is the menu-bar icon height.
    image.setSize(NSSize::new(18.0, 18.0));
    image.setTemplate(true);
    Some(image)
}

/// Adds the menu-bar item. Call on the main thread, once.
pub fn install(popup: AppHandle<Popup>, settings: AppHandle<SettingsUi>, channel: UpdateChannel) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let target = MenuTarget::new(mtm, Handles { popup, settings, channel });
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(NSVariableStatusItemLength);
    if let Some(button) = item.button(mtm) {
        match menu_bar_image() {
            Some(image) => button.setImage(Some(&image)),
            None => button.setTitle(&NSString::from_str("Rt")),
        }
    }
    let menu = NSMenu::new(mtm);
    let entries: [(&str, Option<Sel>); 7] = [
        ("Rewrite Selection", Some(sel!(rewrite:))),
        ("", None),
        ("Settings…", Some(sel!(settings:))),
        ("Permissions…", Some(sel!(permissions:))),
        ("Check for Updates…", Some(sel!(updates:))),
        ("", None),
        ("Quit Retone", Some(sel!(quit:))),
    ];
    for (title, action) in entries {
        let Some(action) = action else {
            menu.addItem(&NSMenuItem::separatorItem(mtm));
            continue;
        };
        // SAFETY: the selector names a method of MenuTarget taking one object argument, and the
        // target outlives the menu (both are kept in INSTALLED).
        let entry = unsafe {
            NSMenuItem::initWithTitle_action_keyEquivalent(
                NSMenuItem::alloc(mtm),
                &NSString::from_str(title),
                Some(action),
                &NSString::from_str(""),
            )
        };
        // SAFETY: as above; the target is a live object that responds to `action`.
        unsafe { entry.setTarget(Some(&target)) };
        menu.addItem(&entry);
    }
    item.setMenu(Some(&menu));
    INSTALLED.with(|installed| *installed.borrow_mut() = Some((item, target)));
}
