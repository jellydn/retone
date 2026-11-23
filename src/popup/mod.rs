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
use std::sync::{Arc, Mutex};

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
    /// The last capture used the clipboard, because nothing was selected or Accessibility is off.
    pub from_clipboard: bool,
    /// The popup opened from a Services menu item, which sent the text itself.
    pub from_service: bool,
    /// The text that Service sent, until the capture takes it.
    given: Option<String>,
    /// Set after the runtime guard saw a CLI write files; shown above the kept result.
    pub adapter_warning: Option<String>,
    /// The provider list, open over the current state (⌘P or a click on the header label).
    pub picker: Option<Picker>,
    request: Latest<(Attempt, Outcome)>,
    /// Models the local endpoint lists, fetched when the provider list opens.
    local_models: Latest<Result<Vec<String>, String>>,
    cancel: Cancel,
    links: Option<Links>,
    /// The last announcement, so a screen reader hears each change once.
    last_spoken: Option<String>,
    /// The newest streamed text and its attempt, written by the request worker.
    partial: Arc<Mutex<Option<(Attempt, String)>>>,
    waker: Option<rune::Waker>,
}

pub struct Picker {
    pub choices: Vec<Provider>,
    pub selected: usize,
    /// True until the local endpoint answered with its model list.
    pub listing: bool,
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
            from_clipboard: false,
            from_service: false,
            given: None,
            adapter_warning: None,
            picker: None,
            request: Latest::default(),
            local_models: Latest::default(),
            cancel: Cancel::default(),
            links: None,
            last_spoken: None,
            partial: Arc::default(),
            waker: None,
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
        self.request.set_waker(waker.clone());
        self.local_models.set_waker(waker.clone());
        self.waker = Some(waker);
    }

    /// The user's global shortcut, or the menu item Rewrite Selection.
    pub fn on_hotkey(&mut self, tx: &mut Tx) {
        self.open(None, None, tx);
    }

    /// A style's own shortcut: the same as the main one, but the rewrite starts in that style.
    pub fn on_style_hotkey(&mut self, index: usize, tx: &mut Tx) {
        self.open(Some(index), None, tx);
    }

    /// A Services menu item: the popup opens in its style with the text the Service sent.
    pub fn on_service(&mut self, style_id: &str, text: String, tx: &mut Tx) {
        let index = crate::styles::PRESETS.iter().position(|style| style.id() == style_id);
        self.open(index, Some(text), tx);
    }

    fn open(&mut self, style: Option<usize>, given: Option<String>, tx: &mut Tx) {
        // Set first, so text from an earlier Service never reaches a later hotkey capture.
        self.from_service = given.is_some();
        self.given = given;
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
        self.picker = None;
        self.flow.default_style = style.map_or_else(|| self.config.style(), |index| self.config.style_at(index));
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
                self.from_clipboard = matches!(&result, Ok(captured) if captured.from_clipboard);
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

    /// Opens the provider list, or closes it when it is open.
    pub fn toggle_picker(&mut self) {
        if self.picker.take().is_some() {
            return;
        }
        // Only while there is something to rewrite or a question to answer.
        if !matches!(self.flow.state, State::Previewing { .. } | State::AwaitingConsent { .. }) {
            return;
        }
        // Headless scenes have no links: they use the scene's config and never touch the network.
        if let Some(links) = self.links.as_ref() {
            self.config = links.shared.config();
        }
        let choices = self.offered(&[]);
        let selected = choices.iter().position(|c| Some(c) == self.provider.as_ref()).unwrap_or(0);
        let allow_network = self.config.local_allow_network;
        let listing = self.links.is_some()
            && provider::local::validate_endpoint(&self.config.local_endpoint, allow_network).is_ok();
        if listing {
            let shared = self.links.as_ref().map(|links| links.shared.clone());
            let request = (self.config.local_endpoint.clone(), allow_network, shared);
            self.local_models.request(request, |(endpoint, allow_network, shared)| {
                let key = shared.and_then(|shared| shared.api_key()).map(provider::local::ApiKey);
                provider::local::list_models(&endpoint, allow_network, key.as_ref())
            });
        }
        self.picker = Some(Picker { choices, selected, listing });
    }

    /// The choices without adapters the runtime guard turned off.
    fn offered(&self, local_models: &[String]) -> Vec<Provider> {
        let mut choices = self.config.choices(local_models);
        if let Some(links) = self.links.as_ref() {
            let prefs = links.shared.prefs();
            choices.retain(|c| consent::disabled_reason(&prefs, c.kind()).is_none());
        }
        choices
    }

    pub fn move_selection(&mut self, down: bool) {
        if let Some(picker) = self.picker.as_mut().filter(|p| !p.choices.is_empty()) {
            let n = picker.choices.len();
            picker.selected = if down { (picker.selected + 1) % n } else { (picker.selected + n - 1) % n };
        }
    }

    /// Saves the choice for the next rewrites too, then reruns this one, or asks for consent
    /// first when this provider and model do not have it yet.
    pub fn choose(&mut self, index: usize, tx: &mut Tx) {
        // A key with no row (a letter, or 9 in a list of 3) keeps the list open.
        let Some(chosen) = self.picker.as_ref().and_then(|p| p.choices.get(index)).cloned() else { return };
        self.picker = None;
        if self.provider.as_ref() == Some(&chosen) {
            return;
        }
        self.config.choose(&chosen);
        let mut consented = false;
        if let Some(links) = self.links.as_ref() {
            // A failed write keeps the choice for this rewrite; Settings shows the saved one.
            self.config.save(&mut links.shared.prefs()).ok();
            let kind = chosen.kind();
            links.settings.update(move |settings, _| settings.reload_choice(kind));
            consented = consent::has_consent(&links.shared.prefs(), &chosen);
        }
        self.provider = Some(chosen);
        self.provider_problem = None;
        self.adapter_warning = None;
        self.step(Event::ProviderChanged { consented }, tx);
    }

    /// Adds the listed local models to the open list, keeping the highlighted row.
    fn add_local_models(&mut self, models: &[String]) -> bool {
        let choices = self.offered(models);
        let Some(picker) = self.picker.as_mut() else { return false };
        let highlighted = picker.choices.get(picker.selected).cloned();
        picker.selected = highlighted.and_then(|c| choices.iter().position(|x| *x == c)).unwrap_or(0);
        picker.choices = choices;
        picker.listing = false;
        true
    }

    /// Lands a provider answer or the local model list; true when the view changed.
    pub fn poll(&mut self, tx: &mut Tx) -> bool {
        let mut listed = false;
        if let Some(models) = self.local_models.take() {
            // An endpoint that cannot list models leaves the saved choices as they are.
            listed = self.add_local_models(&models.unwrap_or_default());
        }
        // Before the final answer, so a last partial never lands after it.
        let partial = self.partial.lock().map(|mut slot| slot.take()).unwrap_or_default();
        if let Some((attempt, text)) = &partial {
            self.step(Event::Partial { attempt: *attempt, text: text.clone() }, tx);
        }
        let Some((attempt, outcome)) = self.request.take() else { return listed || partial.is_some() };
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
        let spoken = view::spoken(self);
        let words = spoken.as_ref().map(|(words, _)| words.clone());
        if words != self.last_spoken {
            if let Some((words, how)) = spoken {
                tx.announce(words, how);
            }
            self.last_spoken = words;
        }
    }

    fn request(&mut self, attempt: Attempt, text: String, style: Style) {
        self.cancel = Cancel::default();
        let (Some(provider), Some(env)) = (self.provider.clone(), self.links.as_ref().and_then(|l| l.env.clone()))
        else {
            return;
        };
        let cancel = self.cancel.clone();
        let shared = self.links.as_ref().map(|links| links.shared.clone());
        let (partial, waker) = (self.partial.clone(), self.waker.clone());
        self.request.request(
            (attempt, provider, style, text, cancel, env, shared, partial, waker),
            |(attempt, provider, style, text, cancel, mut env, shared, partial, waker)| {
                // In the worker: the Keychain read can show a prompt.
                if provider.kind() == provider::Kind::Local {
                    env.local_api_key = shared.and_then(|shared| shared.api_key()).map(provider::local::ApiKey);
                }
                let on_partial = |so_far: &str| {
                    if let Ok(mut slot) = partial.lock() {
                        *slot = Some((attempt, so_far.to_string()));
                    }
                    if let Some(wake) = &waker {
                        wake();
                    }
                };
                (attempt, provider.rewrite(&style, &text, &cancel, &env, &on_partial))
            },
        );
    }

    #[cfg(target_os = "macos")]
    fn capture(&mut self, attempt: Attempt) {
        if let Some(native) = self.links.as_ref().and_then(|links| links.native.as_ref()) {
            native.send(crate::native::Command::Capture { attempt, given: self.given.take() });
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
    if p.picker.is_some() {
        picker_key(p, ev, tx);
        // The list holds the keys while open, so ⏎ or ⌘C cannot act on the rewrite behind it.
        return true;
    }
    let event = match &ev.key {
        Key::Command('p') => {
            p.toggle_picker();
            return true;
        }
        Key::Escape => Event::Cancel,
        Key::Enter => match &p.flow.state {
            State::AwaitingConsent { .. } => Event::ConsentGiven,
            State::Notice(_) => Event::Cancel,
            State::Previewing { style, .. } if style.changes_meaning() && !ev.mods.cmd => Event::Copy,
            _ => Event::Replace,
        },
        Key::Command('c') => Event::Copy,
        Key::Command('r') => Event::Retry,
        Key::Text(key) if !ev.mods.cmd => {
            match view::styles(&p.config).into_iter().enumerate().find(|(index, _)| view::style_key(*index) == *key) {
                Some((_, style)) => Event::ChangeStyle(style),
                None => return false,
            }
        }
        _ => return false,
    };
    p.step(event, tx);
    true
}

/// Provider list keys: ↑/↓ move, ⏎ uses the selection, 1–9 use that row, ⎋ or ⌘P close.
fn picker_key(p: &mut Popup, ev: &KeyEvent, tx: &mut Tx) {
    match &ev.key {
        Key::Up => p.move_selection(false),
        Key::Down | Key::Tab => p.move_selection(true),
        Key::Enter => {
            let selected = p.picker.as_ref().map_or(0, |picker| picker.selected);
            p.choose(selected, tx);
        }
        Key::Escape | Key::Command('p') => p.picker = None,
        Key::Text(digit) if !ev.mods.cmd => p.choose(digit_index(digit), tx),
        _ => {}
    }
}

fn digit_index(text: &str) -> usize {
    text.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).unwrap_or(usize::MAX)
}
