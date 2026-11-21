//! The Settings window: shortcut, provider, style and theme, privacy, permissions, updates.
//!
//! Every change is validated before it is saved: a hotkey only after it registers, a provider
//! only when its endpoint is loopback and its model ID exact.

pub mod view;

use crate::config::Config;
use crate::popup::Popup;
use crate::provider::{Kind, consent, local};
use crate::shared::Shared;
use rune::{AppHandle, Latest, Shortcut, ShortcutError, ShortcutId, TextEdit, Tx};
use rune_kit::ShortcutRecorderState;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Shortcut,
    Provider,
    Style,
    Privacy,
    Permissions,
    Updates,
}

pub const PAGES: [(Page, &str); 6] = [
    (Page::Shortcut, "Shortcut"),
    (Page::Provider, "Provider"),
    (Page::Style, "Style"),
    (Page::Privacy, "Privacy"),
    (Page::Permissions, "Permissions"),
    (Page::Updates, "Updates"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HotkeyTest {
    #[default]
    Idle,
    Waiting,
    Passed,
}

/// Popup keys a global shortcut must not take, or the popup could not use them.
const POPUP_CHORDS: [&str; 2] = ["super+c", "super+r"];

pub struct SettingsUi {
    pub page: Page,
    pub config: Config,
    pub recorder: ShortcutRecorderState,
    pub hotkey_test: HotkeyTest,
    /// Who holds the chord the last conflict refused, when Rune can tell.
    pub conflict_owner: Option<String>,
    pub endpoint: TextEdit,
    pub local_model: TextEdit,
    pub pi_model: TextEdit,
    pub opencode_model: TextEdit,
    pub custom_style: TextEdit,
    pub provider_choice: Option<Kind>,
    /// The last Save result on the Provider page.
    pub provider_status: Option<Result<String, String>>,
    pub models: Latest<Result<Vec<String>, String>>,
    pub listed_models: Option<Result<Vec<String>, String>>,
    pub disabled: Vec<(Kind, String)>,
    pub privacy_status: Option<String>,
    shared: Arc<Shared>,
    popup: Option<AppHandle<Popup>>,
    registration: Option<ShortcutId>,
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
            recorder: ShortcutRecorderState::new(config.hotkey.as_deref().and_then(|h| h.parse().ok())),
            hotkey_test: HotkeyTest::Idle,
            conflict_owner: None,
            endpoint: TextEdit::new(&config.local_endpoint),
            local_model: TextEdit::new(&config.local_model),
            pi_model: TextEdit::new(&config.pi_model),
            opencode_model: TextEdit::new(&config.opencode_model),
            custom_style: TextEdit::new(&config.custom_style),
            provider_choice: config.provider,
            provider_status: None,
            models: Latest::default(),
            listed_models: None,
            disabled: Vec::new(),
            privacy_status: None,
            config,
            shared,
            popup,
            registration: None,
        };
        ui.refresh_disabled();
        ui
    }

    pub fn set_waker(&mut self, waker: rune::Waker) {
        self.models.set_waker(waker);
    }

    pub fn poll(&mut self) -> bool {
        let Some(models) = self.models.take() else { return false };
        self.listed_models = Some(models);
        true
    }

    /// At launch: registers the saved hotkey. A failure leaves the chord shown with its error.
    pub fn register_saved(&mut self) {
        let Some(shortcut) = self.recorder.shortcut().cloned() else { return };
        if self.register(Some(&shortcut)).is_err() {
            self.recorder.mark_unregistered();
        }
    }

    /// The recorder's change: refuse popup chords, register the new chord, then save it.
    pub fn change_hotkey(&mut self, next: Option<&Shortcut>) -> Result<(), ShortcutError> {
        if let Some(chord) = next {
            let name = chord.to_string();
            if !["ctrl+", "alt+", "super+"].iter().any(|m| name.contains(m)) {
                return Err(ShortcutError::InvalidAccelerator("Use ⌘, ⌃ or ⌥ in the shortcut.".into()));
            }
            if POPUP_CHORDS.contains(&name.as_str()) {
                return Err(ShortcutError::InvalidAccelerator("The popup uses this shortcut.".into()));
            }
        }
        let result = self.register(next);
        self.conflict_owner = match (&result, next, &self.popup) {
            (Err(ShortcutError::Conflict), Some(chord), Some(popup)) => popup.shortcut_owner(chord),
            _ => None,
        };
        result?;
        self.config.hotkey = next.map(|chord| chord.to_string());
        self.save();
        Ok(())
    }

    /// Registers `next` before dropping the old chord, so a refused chord keeps the old one.
    fn register(&mut self, next: Option<&Shortcut>) -> Result<(), ShortcutError> {
        let popup = self.popup.as_ref().ok_or(ShortcutError::NotRunning)?;
        let new = next
            .map(|chord| popup.register_shortcut(chord.clone(), |p: &mut Popup, tx: &mut Tx| p.on_hotkey(tx)))
            .transpose()?;
        if let Some(old) = self.registration
            && let Err(error) = popup.unregister_shortcut(old)
        {
            if let Some(new) = new {
                popup.unregister_shortcut(new).ok();
            }
            return Err(error);
        }
        self.registration = new;
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
        next.local_model = self.local_model.text.trim().to_string();
        next.pi_model = self.pi_model.text.trim().to_string();
        next.opencode_model = self.opencode_model.text.trim().to_string();
        let checked = match next.provider {
            None => Ok("No provider: the shortcut shows \"Choose a provider\".".to_string()),
            Some(_) => next.provider().map(|p| format!("Saved: {} · {}", p.kind().name(), p.model())),
        };
        if checked.is_ok() {
            self.config = next;
            self.save();
        }
        self.provider_status = Some(checked);
    }

    pub fn list_local_models(&mut self) {
        let endpoint = self.endpoint.text.clone();
        self.listed_models = None;
        self.models.request(endpoint, |endpoint| local::list_models(&endpoint));
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
    ("shortcut-recorded", |s| s.recorder = ShortcutRecorderState::new("alt+super+r".parse().ok())),
    ("provider-local", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::Local);
        s.local_model = TextEdit::new("qwen2.5:7b");
        s.listed_models = Some(Ok(vec!["qwen2.5:0.5b".into(), "qwen2.5:7b".into()]));
    }),
    ("provider-opencode", |s| {
        s.page = Page::Provider;
        s.provider_choice = Some(Kind::OpenCode);
        s.opencode_model = TextEdit::new("opencode/beam-free");
        s.provider_status = Some(Ok("Saved: OpenCode · opencode/beam-free".into()));
    }),
    ("style", |s| s.page = Page::Style),
    ("style-lotus", |s| {
        s.page = Page::Style;
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
    ("permissions", |s| s.page = Page::Permissions),
    ("updates", |s| s.page = Page::Updates),
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
