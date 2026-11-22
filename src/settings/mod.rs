//! The Settings window: a sidebar of pages (General, Provider, Styles, Privacy), each a card of
//! rows, with the version and updates in the sidebar footer.
//!
//! Every change is validated before it is saved: a hotkey only after it registers, a provider
//! only when its endpoint is allowed (this Mac, or a private network after the opt-in) and its
//! model ID exact.

mod icons;
mod rows;
pub mod view;

use crate::config::Config;
use crate::popup::Popup;
use crate::provider::{Kind, consent, local};
use crate::shared::Shared;
use crate::styles::PRESETS;
use crate::updates::UpdateChannel;
use rune::{AppHandle, Latest, Shortcut, ShortcutError, ShortcutId, TextEdit, Tx};
use rune_kit::ShortcutRecorderState;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    General,
    Provider,
    Styles,
    Privacy,
}

pub const PAGES: [(Page, &str); 4] =
    [(Page::General, "General"), (Page::Provider, "Provider"), (Page::Styles, "Styles"), (Page::Privacy, "Privacy")];

/// A page under a row with ›: only fields too long for a row get one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubPage {
    Address,
    CustomStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HotkeyTest {
    #[default]
    Idle,
    Waiting,
    Passed,
}

/// One optional shortcut per style: the presets, then Custom.
pub const STYLE_SLOTS: usize = PRESETS.len() + 1;

/// Which shortcut a recorder sets: the main one, or the one for the style at this index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Main,
    Style(usize),
}

/// Popup keys a global shortcut must not take, or the popup could not use them.
const POPUP_CHORDS: [&str; 3] = ["super+c", "super+r", "super+p"];

pub struct SettingsUi {
    pub page: Page,
    pub sub_page: Option<SubPage>,
    /// The "Advanced" fold under the page's card.
    pub advanced_open: bool,
    /// The key of the dropdown whose list is open, if any.
    pub open_select: Option<&'static str>,
    /// What macOS says, read when the window opens; Retone does not keep its own copy.
    pub launch_at_login: bool,
    pub login_error: Option<String>,
    pub config: Config,
    pub recorder: ShortcutRecorderState,
    pub style_recorders: Vec<ShortcutRecorderState>,
    pub hotkey_test: HotkeyTest,
    /// Who holds the chord the last conflict refused, when Rune can tell.
    pub conflict_owner: Option<String>,
    pub endpoint: TextEdit,
    /// The "Allow LAN and Tailscale servers" switch; saved with the rest of the Provider page.
    pub allow_network: bool,
    /// A new API key typed in; Save moves it to the Keychain and clears the field.
    pub api_key: TextEdit,
    /// Whether a key is in the Keychain, from prefs, so the Keychain is not read here.
    pub api_key_saved: bool,
    pub local_model: TextEdit,
    pub pi_model: TextEdit,
    pub opencode_model: TextEdit,
    pub custom_style: TextEdit,
    pub provider_choice: Option<Kind>,
    pub update_channel: UpdateChannel,
    /// The last Save result on the Provider page.
    pub provider_status: Option<Result<String, String>>,
    pub models: Latest<Result<Vec<String>, String>>,
    pub listed_models: Option<Result<Vec<String>, String>>,
    pub disabled: Vec<(Kind, String)>,
    pub privacy_status: Option<String>,
    shared: Arc<Shared>,
    popup: Option<AppHandle<Popup>>,
    registration: Option<ShortcutId>,
    style_registrations: Vec<Option<ShortcutId>>,
}

impl Default for SettingsUi {
    /// Headless renders and tests: an in-memory store, no running app.
    fn default() -> Self {
        SettingsUi::new(Arc::new(Shared::new(false)), None)
    }
}

impl SettingsUi {
    pub fn new(shared: Arc<Shared>, popup: Option<AppHandle<Popup>>) -> SettingsUi {
        let config = shared.config();
        let mut ui = SettingsUi {
            page: Page::default(),
            sub_page: None,
            advanced_open: false,
            open_select: None,
            launch_at_login: false,
            login_error: None,
            recorder: ShortcutRecorderState::new(config.hotkey.as_deref().and_then(|h| h.parse().ok())),
            style_recorders: (0..STYLE_SLOTS)
                .map(|i| {
                    let saved = config.style_hotkeys.get(i).cloned().flatten();
                    ShortcutRecorderState::new(saved.and_then(|h| h.parse().ok()))
                })
                .collect(),
            hotkey_test: HotkeyTest::Idle,
            conflict_owner: None,
            endpoint: TextEdit::new(&config.local_endpoint),
            allow_network: config.local_allow_network,
            api_key: TextEdit::default(),
            api_key_saved: shared.api_key_saved(),
            local_model: TextEdit::new(&config.local_model),
            pi_model: TextEdit::new(&config.pi_model),
            opencode_model: TextEdit::new(&config.opencode_model),
            custom_style: TextEdit::new(&config.custom_style),
            provider_choice: config.provider,
            update_channel: UpdateChannel::default(),
            provider_status: None,
            models: Latest::default(),
            listed_models: None,
            disabled: Vec::new(),
            privacy_status: None,
            config,
            shared,
            popup,
            registration: None,
            style_registrations: vec![None; STYLE_SLOTS],
        };
        ui.refresh_disabled();
        ui.refresh_launch_at_login();
        ui
    }

    /// A sidebar press: the new page opens at its top, with no fold or list left open.
    pub fn select_page(&mut self, page: Page) {
        self.page = page;
        self.sub_page = None;
        self.advanced_open = false;
        self.open_select = None;
    }

    pub fn toggle_select(&mut self, key: &'static str) {
        self.open_select = if self.open_select == Some(key) { None } else { Some(key) };
    }

    /// Only a running app asks macOS: headless renders and tests have no bundle to register.
    pub fn refresh_launch_at_login(&mut self) {
        #[cfg(target_os = "macos")]
        if self.popup.is_some() {
            self.launch_at_login = crate::mac::login::is_enabled();
        }
    }

    /// In headless renders and tests the switch only flips: there is no bundle to register.
    pub fn set_launch_at_login(&mut self, on: bool) {
        #[cfg(target_os = "macos")]
        let result = if self.popup.is_some() { crate::mac::login::set_enabled(on) } else { Ok(()) };
        #[cfg(not(target_os = "macos"))]
        let result: Result<(), String> = Ok(());
        match result {
            Ok(()) => {
                self.launch_at_login = on;
                self.login_error = None;
            }
            Err(error) => self.login_error = Some(error),
        }
    }

    pub fn set_waker(&mut self, waker: rune::Waker) {
        self.models.set_waker(waker);
    }

    pub fn poll(&mut self) -> bool {
        let Some(models) = self.models.take() else { return false };
        self.listed_models = Some(models);
        true
    }

    /// At launch: registers the saved hotkeys. A failure leaves the chord shown with its error.
    pub fn register_saved(&mut self) {
        let slots = std::iter::once(Slot::Main).chain((0..STYLE_SLOTS).map(Slot::Style));
        for slot in slots {
            let Some(shortcut) = self.recorder_mut(slot).shortcut().cloned() else { continue };
            if self.register(slot, Some(&shortcut)).is_err() {
                self.recorder_mut(slot).mark_unregistered();
            }
        }
    }

    pub fn change_hotkey(&mut self, next: Option<&Shortcut>) -> Result<(), ShortcutError> {
        self.change_shortcut(Slot::Main, next)
    }

    pub fn change_style_hotkey(&mut self, index: usize, next: Option<&Shortcut>) -> Result<(), ShortcutError> {
        self.change_shortcut(Slot::Style(index), next)
    }

    /// A recorder's change: refuse popup chords and chords another Retone shortcut uses,
    /// register the new chord, then save it.
    fn change_shortcut(&mut self, slot: Slot, next: Option<&Shortcut>) -> Result<(), ShortcutError> {
        if let Some(chord) = next {
            let name = chord.to_string();
            if !["ctrl+", "alt+", "super+"].iter().any(|m| name.contains(m)) {
                return Err(ShortcutError::InvalidAccelerator("Use ⌘, ⌃ or ⌥ in the shortcut.".into()));
            }
            if POPUP_CHORDS.contains(&name.as_str()) {
                return Err(ShortcutError::InvalidAccelerator("The popup uses this shortcut.".into()));
            }
            if let Some(owner) = self.retone_owner(&name, slot) {
                return Err(ShortcutError::InvalidAccelerator(format!("Retone uses this shortcut for {owner}.")));
            }
        }
        let result = self.register(slot, next);
        self.conflict_owner = match (&result, next, &self.popup) {
            (Err(ShortcutError::Conflict), Some(chord), Some(popup)) => popup.shortcut_owner(chord),
            _ => None,
        };
        result?;
        let saved = next.map(|chord| chord.to_string());
        match slot {
            Slot::Main => self.config.hotkey = saved,
            Slot::Style(index) => {
                self.config.style_hotkeys.resize(STYLE_SLOTS, None);
                self.config.style_hotkeys[index] = saved;
            }
        }
        self.save();
        Ok(())
    }

    /// The other Retone shortcut that already uses `chord`, named for the error message.
    fn retone_owner(&self, chord: &str, slot: Slot) -> Option<String> {
        if slot != Slot::Main && self.config.hotkey.as_deref() == Some(chord) {
            return Some("Rewrite selection".into());
        }
        let names = view::style_names();
        self.config.style_hotkeys.iter().enumerate().find_map(|(index, saved)| {
            (slot != Slot::Style(index) && saved.as_deref() == Some(chord))
                .then(|| format!("the {} style", names.get(index).copied().unwrap_or("Custom")))
        })
    }

    fn recorder_mut(&mut self, slot: Slot) -> &mut ShortcutRecorderState {
        match slot {
            Slot::Main => &mut self.recorder,
            Slot::Style(index) => &mut self.style_recorders[index],
        }
    }

    /// Registers `next` before dropping the old chord, so a refused chord keeps the old one.
    fn register(&mut self, slot: Slot, next: Option<&Shortcut>) -> Result<(), ShortcutError> {
        let popup = self.popup.as_ref().ok_or(ShortcutError::NotRunning)?;
        let new = next
            .map(|chord| match slot {
                Slot::Main => popup.register_shortcut(chord.clone(), |p: &mut Popup, tx: &mut Tx| p.on_hotkey(tx)),
                Slot::Style(index) => popup
                    .register_shortcut(chord.clone(), move |p: &mut Popup, tx: &mut Tx| p.on_style_hotkey(index, tx)),
            })
            .transpose()?;
        let current = match slot {
            Slot::Main => &mut self.registration,
            Slot::Style(index) => &mut self.style_registrations[index],
        };
        if let Some(old) = *current
            && let Err(error) = popup.unregister_shortcut(old)
        {
            if let Some(new) = new {
                popup.unregister_shortcut(new).ok();
            }
            return Err(error);
        }
        *current = new;
        Ok(())
    }

    pub fn start_hotkey_test(&mut self) {
        self.shared.set_testing(true);
        self.hotkey_test = HotkeyTest::Waiting;
    }

    /// Called from the popup when the armed test press arrives.
    pub fn hotkey_tested(&mut self) {
        self.hotkey_test = HotkeyTest::Passed;
    }

    pub fn choose_provider(&mut self, choice: Option<Kind>) {
        self.provider_choice = choice;
        self.provider_status = None;
    }

    /// Validates the Provider page and saves it only when valid.
    pub fn save_provider(&mut self) {
        let mut next = self.config.clone();
        next.provider = self.provider_choice;
        next.local_endpoint = self.endpoint.text.trim().to_string();
        next.local_allow_network = self.allow_network;
        next.local_model = self.local_model.text.trim().to_string();
        next.pi_model = self.pi_model.text.trim().to_string();
        next.opencode_model = self.opencode_model.text.trim().to_string();
        let checked = match next.provider {
            None => Ok("No provider: the shortcut shows \"Choose a provider\".".to_string()),
            Some(_) => next.provider().map(|p| format!("Saved: {} · {}", p.kind().name(), p.model())),
        };
        let new_key = self.api_key.text.trim().to_string();
        let checked = checked.and_then(|saved| {
            if !new_key.is_empty() {
                self.shared.set_api_key(Some(&new_key))?;
                self.api_key = TextEdit::default();
                self.api_key_saved = true;
            }
            Ok(saved)
        });
        if checked.is_ok() {
            self.config = next;
            self.save();
        }
        self.provider_status = Some(checked);
    }

    /// The popup saved another provider: show the saved choice and its model. Only that
    /// provider's model field changes, so unsaved edits in the others stay.
    pub fn reload_choice(&mut self, kind: Kind) {
        self.config = self.shared.config();
        self.provider_choice = self.config.provider;
        let (field, model) = match kind {
            Kind::Local => (&mut self.local_model, &self.config.local_model),
            Kind::Pi => (&mut self.pi_model, &self.config.pi_model),
            Kind::OpenCode => (&mut self.opencode_model, &self.config.opencode_model),
        };
        *field = TextEdit::new(model);
    }

    pub fn remove_api_key(&mut self) {
        let result = self.shared.set_api_key(None).map(|()| "API key removed from the Keychain.".to_string());
        if result.is_ok() {
            self.api_key_saved = false;
        }
        self.provider_status = Some(result);
    }

    pub fn list_local_models(&mut self) {
        // A key typed but not saved yet is used as is; otherwise the saved one, read in the worker.
        let typed = Some(self.api_key.text.trim().to_string()).filter(|key| !key.is_empty());
        let request = (self.endpoint.text.clone(), self.allow_network, typed, self.shared.clone());
        self.listed_models = None;
        self.models.request(request, |(endpoint, allow_network, typed, shared)| {
            let key = typed.or_else(|| shared.api_key()).map(local::ApiKey);
            local::list_models(&endpoint, allow_network, key.as_ref())
        });
    }

    pub fn pick_local_model(&mut self, model: &str) {
        self.local_model = TextEdit::new(model);
    }

    pub fn set_default_style(&mut self, index: usize) {
        self.config.default_style = index;
        self.save();
    }

    pub fn save_custom_style(&mut self) {
        self.config.custom_style = self.custom_style.text.trim().to_string();
        self.save();
        self.sub_page = None;
    }

    pub fn set_theme(&mut self, theme: crate::theme::Theme) {
        self.config.theme = theme;
        self.save();
    }

    pub fn revoke_consent(&mut self) {
        let result = consent::revoke_all(&mut self.shared.prefs());
        self.privacy_status = Some(match result {
            Ok(()) => "Retone will ask again before the next rewrite with each provider.".into(),
            Err(error) => format!("Could not save: {error}"),
        });
    }

    /// The user read the warning naming the folder and turns the adapter on again.
    pub fn enable_adapter(&mut self, kind: Kind) {
        consent::enable(&mut self.shared.prefs(), kind).ok();
        self.refresh_disabled();
    }

    pub fn refresh_disabled(&mut self) {
        let prefs = self.shared.prefs();
        self.disabled = [Kind::Pi, Kind::OpenCode]
            .into_iter()
            .filter_map(|kind| consent::disabled_reason(&prefs, kind).map(|reason| (kind, reason)))
            .collect();
    }

    fn save(&mut self) {
        // A failed write keeps the change for this session; the next save tries again.
        self.config.save(&mut self.shared.prefs()).ok();
    }
}

/// A named state for `--window settings --scene NAME`.
type Scene = (&'static str, fn(&mut SettingsUi));

pub const SCENES: &[Scene] = &[
    ("general", |s| s.recorder = ShortcutRecorderState::new("alt+super+r".parse().ok())),
    ("general-advanced", |s| s.advanced_open = true),
    ("general-theme-open", |s| s.open_select = Some("theme")),
    ("general-light", |s| s.config.theme = crate::theme::Theme::Light),
    ("provider-local", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::Local);
        s.local_model = TextEdit::new("qwen2.5:7b");
        s.listed_models = Some(Ok(vec!["qwen2.5:0.5b".into(), "qwen2.5:7b".into()]));
    }),
    ("provider-local-network", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::Local);
        s.allow_network = true;
        s.endpoint = TextEdit::new("http://box.tailnet.ts.net:11434/v1");
        s.local_model = TextEdit::new("qwen2.5:7b");
    }),
    ("provider-llama-cpp", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::Local);
        s.endpoint = TextEdit::new(local::LLAMA_CPP);
        s.api_key_saved = true;
        s.local_model = TextEdit::new("/Users/me/models/qwen2.5-3b-instruct-q4_k_m.gguf");
        s.listed_models = Some(Ok(vec!["/Users/me/models/qwen2.5-3b-instruct-q4_k_m.gguf".into()]));
    }),
    ("provider-address", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::Local);
        s.sub_page = Some(SubPage::Address);
    }),
    ("provider-opencode", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::OpenCode);
        s.opencode_model = TextEdit::new("opencode/beam-free");
        s.provider_status = Some(Ok("Saved: OpenCode · opencode/beam-free".into()));
        s.advanced_open = true;
    }),
    ("styles", |s| s.page = Page::Styles),
    ("styles-shortcuts", |s| {
        s.page = Page::Styles;
        s.style_recorders[1] = ShortcutRecorderState::new("ctrl+alt+2".parse().ok());
    }),
    ("styles-custom", |s| {
        s.page = Page::Styles;
        s.sub_page = Some(SubPage::CustomStyle);
        s.custom_style = TextEdit::new("Make it friendly and short.");
    }),
    ("styles-lotus", |s| {
        s.page = Page::Styles;
        s.config.theme = crate::theme::Theme::KanagawaLotus;
    }),
    ("privacy-adapter-disabled", |s| {
        s.page = Page::Privacy;
        s.disabled = vec![(
            Kind::OpenCode,
            "OpenCode saved files during this rewrite, so Retone turned it off. Nothing was deleted. \
             Files may be in ~/.local/share/opencode."
                .into(),
        )];
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_provider_is_saved_only_when_valid() {
        let mut ui = SettingsUi::default();
        ui.choose_provider(Some(Kind::Local));
        ui.endpoint = TextEdit::new("http://10.0.0.2:11434/v1");
        ui.local_model = TextEdit::new("qwen2.5:7b");
        ui.save_provider();
        assert!(matches!(ui.provider_status, Some(Err(_))));
        assert_eq!(ui.shared.config().provider, None, "nothing saved");

        ui.allow_network = true;
        ui.save_provider();
        assert!(matches!(ui.provider_status, Some(Ok(_))), "the LAN address with the opt-in");
        assert!(ui.shared.config().local_allow_network, "the opt-in is saved with the page");

        ui.api_key = TextEdit::new(" key-1 ");
        ui.save_provider();
        assert_eq!(ui.shared.api_key().as_deref(), Some("key-1"), "the key goes to the key store");
        assert!(ui.api_key.text.is_empty() && ui.api_key_saved, "the field clears once saved");
        assert!(!ui.shared.config().local_endpoint.contains("key-1"));
        ui.remove_api_key();
        assert_eq!(ui.shared.api_key(), None);

        ui.endpoint = TextEdit::new(local::OLLAMA);
        ui.save_provider();
        assert!(matches!(ui.provider_status, Some(Ok(_))));
        assert_eq!(ui.shared.config().provider, Some(Kind::Local));
        assert_eq!(ui.shared.config().local_model, "qwen2.5:7b");
    }

    #[test]
    fn chords_without_a_modifier_or_used_by_the_popup_are_refused() {
        let mut ui = SettingsUi::default();
        for chord in ["shift+r", "super+c", "super+r"] {
            let shortcut: Shortcut = chord.parse().expect("parse");
            assert!(matches!(ui.change_hotkey(Some(&shortcut)), Err(ShortcutError::InvalidAccelerator(_))), "{chord}");
        }
        assert_eq!(ui.shared.config().hotkey, None);
    }

    #[test]
    fn a_chord_used_by_another_retone_shortcut_is_refused() {
        let mut ui = SettingsUi::default();
        ui.config.hotkey = Some("alt+super+r".into());
        ui.config.style_hotkeys = vec![None, Some("ctrl+alt+2".into())];
        let main: Shortcut = "alt+super+r".parse().expect("parse");
        let concise: Shortcut = "ctrl+alt+2".parse().expect("parse");
        let refused =
            |result| matches!(result, Err(ShortcutError::InvalidAccelerator(reason)) if reason.contains("Retone uses"));
        assert!(refused(ui.change_style_hotkey(0, Some(&main))), "a style cannot take the main chord");
        assert!(refused(ui.change_style_hotkey(0, Some(&concise))), "nor another style's chord");
        assert!(refused(ui.change_hotkey(Some(&concise))), "the main shortcut cannot take a style's chord");
        // Its own chord is not a duplicate: it reaches registration (no running app here).
        assert!(matches!(ui.change_style_hotkey(1, Some(&concise)), Err(ShortcutError::NotRunning)));
    }

    #[test]
    fn the_test_row_turns_green_on_the_next_press() {
        let mut ui = SettingsUi::default();
        ui.start_hotkey_test();
        assert_eq!(ui.hotkey_test, HotkeyTest::Waiting);
        assert!(ui.shared.take_testing(), "the next press is used up by the test");
        assert!(!ui.shared.take_testing());
        ui.hotkey_tested();
        assert_eq!(ui.hotkey_test, HotkeyTest::Passed);
    }
}
