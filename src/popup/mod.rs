//! The popup at the caret: runs the rewrite flow and carries out its effects.
//!
//! The state machine (`flow.rs`) decides; this file only does what it asks: native capture and
//! Replace go to the worker thread, provider requests to `rune::Latest`, and the window moves to
//! the caret. Nothing here runs AX or pasteboard work on the main thread.

pub mod scenes;
pub mod view;

use crate::config::Config;
use crate::flow::{Effect, Event, Flow, State};
use crate::placement::{self, Rect};
use crate::provider::{self, Cancel, Outcome, Provider, consent};
use crate::settings::SettingsUi;
use crate::shared::Shared;
use crate::styles::Style;
use rune::machine::{Attempt, Effects};
use rune::{AppHandle, Key, KeyEvent, Latest, Tx};
use std::sync::Arc;

/// Fixed size, so placement is exact before the first frame; long text scrolls inside.
pub const SIZE: (f64, f64) = (440.0, 300.0);

pub struct Popup {
    pub flow: Flow,
    pub config: Config,
    /// The provider checked at hotkey time; the header shows its name and model.
    pub provider: Option<Provider>,
    /// Why no provider can run: none chosen, invalid settings, or an adapter turned off.
    pub provider_problem: Option<String>,
    /// Captured without Accessibility: the user copied the text, and Replace is off.
    pub clipboard_only: bool,
    /// Set after the runtime guard saw a CLI write files; shown above the kept result.
    pub adapter_warning: Option<String>,
    request: Latest<(Attempt, Outcome)>,
    cancel: Cancel,
    links: Option<Links>,
}

/// Handles and services that exist only once the app runs (none in headless scenes' setup).
pub struct Links {
    pub shared: Arc<Shared>,
    pub me: AppHandle<Popup>,
    pub settings: AppHandle<SettingsUi>,
    #[cfg(target_os = "macos")]
    pub native: Option<crate::native::Native>,
    pub env: Option<provider::Env>,
    anchor: Option<Rect>,
}

impl Links {
    pub fn new(shared: Arc<Shared>, me: AppHandle<Popup>, settings: AppHandle<SettingsUi>) -> Links {
        #[cfg(target_os = "macos")]
        return Links { shared, me, settings, native: None, env: provider::Env::system(), anchor: None };
        #[cfg(not(target_os = "macos"))]
        Links { shared, me, settings, env: provider::Env::system(), anchor: None }
    }
}

impl Default for Popup {
    fn default() -> Self {
        Popup {
            flow: Flow::new(Style::Clear),
            config: Config::default(),
            provider: None,
            provider_problem: None,
            clipboard_only: false,
            adapter_warning: None,
            request: Latest::default(),
            cancel: Cancel::default(),
            links: None,
        }
    }
}

impl Popup {
    pub fn attach(&mut self, links: Links) {
        self.config = links.shared.config();
        self.links = Some(links);
    }

    #[cfg(target_os = "macos")]
    pub fn set_native(&mut self, native: crate::native::Native) {
        if let Some(links) = self.links.as_mut() {
            links.native = Some(native);
        }
    }

    pub fn set_waker(&mut self, waker: rune::Waker) {
        self.request.set_waker(waker);
    }

    /// The user's global shortcut, or the menu item Rewrite Selection.
    pub fn on_hotkey(&mut self, tx: &mut Tx) {
        let Some(links) = self.links.as_ref() else { return };
        if links.shared.take_testing() {
            links.settings.update(|settings, _| settings.hotkey_tested());
            return;
        }
        self.config = links.shared.config();
        let checked = self.config.provider().and_then(|provider| {
            match consent::disabled_reason(&links.shared.prefs(), provider.kind()) {
                Some(reason) => Err(reason),
                None => Ok(provider),
            }
        });
        self.provider_problem = checked.as_ref().err().cloned();
        self.provider = checked.ok();
        self.adapter_warning = None;
        self.flow.default_style = self.config.style();
        let effects = self.flow.step(Event::Hotkey { has_provider: self.provider.is_some() });
        self.run(effects, tx);
    }

    pub fn step(&mut self, event: Event, tx: &mut Tx) {
        let effects = self.flow.step(event);
        self.run(effects, tx);
    }

    #[cfg(target_os = "macos")]
    pub fn on_reply(&mut self, reply: crate::native::Reply, tx: &mut Tx) {
        use crate::native::Reply;
        match reply {
            Reply::Captured { attempt, result, caret } => {
                self.clipboard_only = !crate::capture::has_permissions();
                if let Some(links) = self.links.as_mut() {
                    // `show` checks the caret against the screens before trusting it.
                    links.anchor = caret;
                }
                let consented = match (&self.provider, &self.links) {
                    (Some(provider), Some(links)) => consent::has_consent(&links.shared.prefs(), provider),
                    _ => false,
                };
                self.step(Event::Captured { attempt, result, consented }, tx);
            }
            Reply::Applied { attempt, result } => self.step(Event::Applied { attempt, result }, tx),
        }
    }

    /// Lands a provider answer; true when the view changed.
    pub fn poll(&mut self, tx: &mut Tx) -> bool {
        let Some((attempt, outcome)) = self.request.take() else { return false };
        if let (Some(folder), Some(provider), Some(links)) = (&outcome.wrote_files, &self.provider, &self.links) {
            let home = links.env.as_ref().map(|env| env.home.display().to_string()).unwrap_or_default();
            let folder = folder.display().to_string().replacen(&home, "~", 1);
            self.adapter_warning = consent::disable(&mut links.shared.prefs(), provider.kind(), &folder).ok();
        }
        self.step(Event::Answer { attempt, result: outcome.answer }, tx);
        true
    }

    pub fn open_settings(&mut self, tx: &mut Tx) {
        if let Some(links) = self.links.as_ref() {
            links.settings.open();
        }
        self.step(Event::Cancel, tx);
    }

    fn run(&mut self, effects: Effects<Effect>, tx: &mut Tx) {
        for effect in effects {
            match effect {
                Effect::Capture(attempt) => self.capture(attempt),
                Effect::ShowPopup => self.show(),
                Effect::Request { attempt, text, style } => self.request(attempt, text, style),
                Effect::CancelRequest => {
                    self.cancel.cancel();
                    self.request.cancel();
                }
                Effect::SaveConsent => {
                    if let (Some(provider), Some(links)) = (&self.provider, &self.links) {
                        consent::grant(&mut links.shared.prefs(), provider).ok();
                    }
                }
                Effect::Apply { attempt, text } => self.apply(attempt, text),
                Effect::CopyText(text) => tx.copy(text),
                Effect::ClosePopup => self.close(),
            }
        }
    }

    fn request(&mut self, attempt: Attempt, text: String, style: Style) {
        self.cancel = Cancel::default();
        let (Some(provider), Some(env)) = (self.provider.clone(), self.links.as_ref().and_then(|l| l.env.clone()))
        else {
            return;
        };
        let cancel = self.cancel.clone();
        self.request
            .request((attempt, provider, style, text, cancel, env), |(attempt, provider, style, text, cancel, env)| {
                (attempt, provider.rewrite(&style, &text, &cancel, &env))
            });
    }

    #[cfg(target_os = "macos")]
    fn capture(&mut self, attempt: Attempt) {
        if let Some(native) = self.links.as_ref().and_then(|links| links.native.as_ref()) {
            native.send(crate::native::Command::Capture(attempt));
        }
    }

    #[cfg(target_os = "macos")]
    fn apply(&mut self, attempt: Attempt, text: String) {
        if let Some(native) = self.links.as_ref().and_then(|links| links.native.as_ref()) {
            native.send(crate::native::Command::Replace { attempt, text });
        }
    }

    #[cfg(target_os = "macos")]
    fn show(&mut self) {
        let Some(links) = self.links.as_mut() else { return };
        let Some(mtm) = objc2::MainThreadMarker::new() else { return };
        let screens = crate::mac::screens::screens(mtm);
        let caret = links.anchor.and_then(|caret| placement::usable_caret(caret, &screens));
        let anchor = caret.unwrap_or_else(|| crate::mac::screens::mouse_anchor(mtm));
        let at = placement::place(anchor, SIZE, &screens);
        crate::spike_log::spike!(
            "popup caret_raw={:?} anchor={} at=({:.0},{:.0}) scale={}",
            links.anchor,
            if caret.is_some() { "caret" } else { "pointer" },
            at.x,
            at.y,
            at.scale
        );
        links.anchor = Some(anchor);
        links.me.set_position(((at.x * at.scale).round() as i32, (at.y * at.scale).round() as i32));
        links.me.show();
    }

    #[cfg(target_os = "macos")]
    fn close(&mut self) {
        let Some(links) = self.links.as_mut() else { return };
        links.anchor = None;
        links.me.hide();
        if let Some(native) = links.native.as_ref() {
            native.send(crate::native::Command::Forget);
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn capture(&mut self, _: Attempt) {}
    #[cfg(not(target_os = "macos"))]
    fn apply(&mut self, _: Attempt, _: String) {}
    #[cfg(not(target_os = "macos"))]
    fn show(&mut self) {}
    #[cfg(not(target_os = "macos"))]
    fn close(&mut self) {}
}

/// Popup keys: ⏎ confirms what the popup offers, Esc cancels, ⌘C copies, ⌘R retries, 1–5 pick
/// a style.
pub fn on_key(p: &mut Popup, ev: &KeyEvent, tx: &mut Tx) -> bool {
    let event = match &ev.key {
        Key::Escape => Event::Cancel,
        Key::Enter => match p.flow.state {
            State::AwaitingConsent { .. } => Event::ConsentGiven,
            State::Notice(_) => Event::Cancel,
            _ => Event::Replace,
        },
        Key::Command('c') => Event::Copy,
        Key::Command('r') => Event::Retry,
        Key::Text(digit) if !ev.mods.cmd => match view::styles(&p.config).into_iter().nth(digit_index(digit)) {
            Some(style) => Event::ChangeStyle(style),
            None => return false,
        },
        _ => return false,
    };
    p.step(event, tx);
    true
}

fn digit_index(text: &str) -> usize {
    text.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).unwrap_or(usize::MAX)
}
