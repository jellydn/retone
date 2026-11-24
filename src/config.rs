//! The user's settings in `prefs.json`. No selected or rewritten text is ever stored here.

#![forbid(unsafe_code)]

use crate::provider::{Kind, Provider, cli, local};
use crate::styles::{PRESETS, Style};
use crate::theme::Theme;
use rune_core::prefs::Prefs;
use serde_json::Value;

/// Release and dev builds keep separate prefs folders, as they have separate bundle IDs.
pub const APP_ID: &str =
    if cfg!(debug_assertions) { "io.github.jellydn.retone.dev" } else { "io.github.jellydn.retone" };

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    /// Rune accelerator string, such as `alt+super+r`. None until the user records one.
    pub hotkey: Option<String>,
    /// Optional shortcuts that open the popup in one style, by style index (the presets, then
    /// Custom), as `default_style` counts. Missing entries are None. Prefs save them by style id.
    pub style_hotkeys: Vec<Option<String>>,
    pub theme: Theme,
    /// Index into the presets, or `PRESETS.len()` for the custom style. Prefs save the style id.
    pub default_style: usize,
    pub custom_style: String,
    pub provider: Option<Kind>,
    pub local_endpoint: String,
    /// The user's opt-in for a Local AI server outside this Mac (LAN or Tailscale).
    pub local_allow_network: bool,
    pub local_model: String,
    pub pi_model: String,
    pub opencode_model: String,
    pub claude_model: String,
    pub codex_model: String,
}

impl Config {
    pub fn load(prefs: &Prefs) -> Config {
        let text = |key: &str| prefs.get::<String>(key).unwrap_or_default();
        Config {
            hotkey: prefs.get("hotkey"),
            style_hotkeys: load_style_hotkeys(prefs.get("style_hotkeys")),
            theme: prefs.get("theme").unwrap_or_default(),
            default_style: load_default_style(prefs.get("default_style")),
            custom_style: text("custom_style"),
            provider: prefs.get("provider"),
            local_endpoint: prefs.get("local_endpoint").unwrap_or_else(|| local::OLLAMA.to_string()),
            local_allow_network: prefs.get("local_allow_network").unwrap_or(false),
            local_model: text("local_model"),
            pi_model: text("pi_model"),
            opencode_model: text("opencode_model"),
            claude_model: text("claude_model"),
            codex_model: text("codex_model"),
        }
    }

    pub fn save(&self, prefs: &mut Prefs) -> Result<(), String> {
        let save = |prefs: &mut Prefs, key: &str, value: &serde_json::Value| prefs.set(key, value);
        let fields = [
            ("hotkey", serde_json::json!(self.hotkey)),
            ("style_hotkeys", save_style_hotkeys(&self.style_hotkeys)),
            ("theme", serde_json::json!(self.theme)),
            ("default_style", serde_json::json!(slot_id(self.default_style))),
            ("custom_style", serde_json::json!(self.custom_style)),
            ("provider", serde_json::json!(self.provider)),
            ("local_endpoint", serde_json::json!(self.local_endpoint)),
            ("local_allow_network", serde_json::json!(self.local_allow_network)),
            ("local_model", serde_json::json!(self.local_model)),
            ("pi_model", serde_json::json!(self.pi_model)),
            ("opencode_model", serde_json::json!(self.opencode_model)),
            ("claude_model", serde_json::json!(self.claude_model)),
            ("codex_model", serde_json::json!(self.codex_model)),
        ];
        for (key, value) in fields {
            save(prefs, key, &value).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// The style a new rewrite starts with; an empty custom style falls back to Clear.
    pub fn style(&self) -> Style {
        self.style_at(self.default_style)
    }

    /// The style at `index`: a preset, or Custom past the presets. An empty custom style falls
    /// back to Clear.
    pub fn style_at(&self, index: usize) -> Style {
        let presets = crate::styles::PRESETS;
        match presets.get(index) {
            Some(style) => style.clone(),
            None if !self.custom_style.trim().is_empty() => Style::Custom(self.custom_style.trim().to_string()),
            None => Style::Clear,
        }
    }

    /// The chosen provider, checked as Settings checks it before saving. Err names what is
    /// missing, for the popup's "Choose a provider" notice.
    pub fn provider(&self) -> Result<Provider, String> {
        match self.provider {
            None => Err(t!("popup.no_provider_body").into()),
            Some(Kind::Local) => {
                let endpoint = local::validate_endpoint(&self.local_endpoint, self.local_allow_network)?;
                let model = exact_model(&self.local_model, Kind::Local.name())?;
                Ok(Provider::Local { endpoint, model })
            }
            Some(Kind::Pi) => {
                let model = exact_model(&self.pi_model, "Pi")?;
                cli::check_model("Pi", &model)?;
                Ok(Provider::Pi { model })
            }
            Some(Kind::OpenCode) => {
                let model = exact_model(&self.opencode_model, "OpenCode")?;
                cli::check_model("OpenCode", &model)?;
                Ok(Provider::OpenCode { model })
            }
            Some(Kind::ClaudeCode) => {
                let model = exact_model(&self.claude_model, "Claude Code")?;
                cli::check_model("Claude Code", &model)?;
                Ok(Provider::ClaudeCode { model })
            }
            Some(Kind::Codex) => {
                let model = exact_model(&self.codex_model, "Codex")?;
                cli::check_model("Codex", &model)?;
                Ok(Provider::Codex { model })
            }
        }
    }
}

impl Config {
    /// What the popup's provider list offers: each provider that is valid with its saved model,
    /// and the other models the local endpoint listed next to the local one, so each provider's
    /// rows stay together. A model ID is never typed in the popup.
    pub fn choices(&self, local_models: &[String]) -> Vec<Provider> {
        let mut all: Vec<Provider> = crate::provider::KINDS
            .into_iter()
            .filter_map(|kind| Config { provider: Some(kind), ..self.clone() }.provider().ok())
            .collect();
        if let Ok(endpoint) = local::validate_endpoint(&self.local_endpoint, self.local_allow_network) {
            for model in local_models.iter().map(|m| m.trim()).filter(|m| !m.is_empty()) {
                let listed = Provider::Local { endpoint: endpoint.clone(), model: model.to_string() };
                if !all.contains(&listed) {
                    let after_local = all.iter().rposition(|p| p.kind() == Kind::Local).map_or(0, |i| i + 1);
                    all.insert(after_local, listed);
                }
            }
        }
        all
    }

    /// Makes `provider` the saved choice, with its model, as Settings would save it.
    pub fn choose(&mut self, provider: &Provider) {
        self.provider = Some(provider.kind());
        let model = provider.model().to_string();
        match provider {
            Provider::Local { .. } => self.local_model = model,
            Provider::Pi { .. } => self.pi_model = model,
            Provider::OpenCode { .. } => self.opencode_model = model,
            Provider::ClaudeCode { .. } => self.claude_model = model,
            Provider::Codex { .. } => self.codex_model = model,
        }
    }
}

fn exact_model(model: &str, name: &str) -> Result<String, String> {
    let model = model.trim();
    if model.is_empty() {
        return Err(t!("provider.exact_model", name = name));
    }
    Ok(model.to_string())
}

/// The style slots in the order 0.1.0-beta.10 and earlier saved them, by number. Frozen, so old
/// prefs keep their choices after new presets change the slot order.
const LEGACY_SLOT_IDS: [&str; 5] = ["clear", "concise", "professional", "technical", "custom"];

/// The id of the style slot at `index`: a preset, or Custom past the presets.
fn slot_id(index: usize) -> &'static str {
    PRESETS.get(index).map_or("custom", Style::id)
}

/// The slot index for a saved style id or, from older prefs, a saved slot number.
fn slot_index(saved: &Value) -> Option<usize> {
    let id = match saved {
        Value::String(id) => id.as_str(),
        Value::Number(number) => *LEGACY_SLOT_IDS.get(usize::try_from(number.as_u64()?).ok()?)?,
        _ => return None,
    };
    (0..=PRESETS.len()).find(|&index| slot_id(index) == id)
}

fn load_default_style(saved: Option<Value>) -> usize {
    saved.as_ref().and_then(slot_index).unwrap_or(0)
}

/// Shortcuts are saved as `{ "concise": "ctrl+alt+2" }`; older prefs saved a list by slot number.
fn load_style_hotkeys(saved: Option<Value>) -> Vec<Option<String>> {
    let entries: Vec<(Value, Value)> = match saved {
        Some(Value::Object(map)) => map.into_iter().map(|(id, chord)| (Value::String(id), chord)).collect(),
        Some(Value::Array(list)) => list.into_iter().enumerate().map(|(i, chord)| (Value::from(i), chord)).collect(),
        _ => Vec::new(),
    };
    let mut hotkeys = vec![None; PRESETS.len() + 1];
    for (slot, chord) in entries {
        if let (Some(index), Value::String(chord)) = (slot_index(&slot), chord) {
            hotkeys[index] = Some(chord);
        }
    }
    hotkeys
}

fn save_style_hotkeys(hotkeys: &[Option<String>]) -> Value {
    let saved = hotkeys.iter().enumerate().filter_map(|(index, chord)| Some((slot_id(index), chord.as_ref()?)));
    Value::Object(saved.map(|(id, chord)| (id.to_string(), Value::from(chord.as_str()))).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_install_has_no_hotkey_and_no_provider() {
        let config = Config::load(&Prefs::memory());
        assert_eq!(config.hotkey, None);
        assert_eq!(config.provider(), Err("Choose a provider in Settings.".into()));
        assert_eq!(config.local_endpoint, local::OLLAMA);
        assert_eq!(config.style(), Style::Clear);
    }

    #[test]
    fn settings_survive_a_save_and_load() {
        let mut prefs = Prefs::memory();
        let custom = PRESETS.len();
        let mut style_hotkeys = vec![None; custom + 1];
        style_hotkeys[1] = Some("ctrl+alt+2".into());
        style_hotkeys[custom] = Some("ctrl+alt+9".into());
        let config = Config {
            hotkey: Some("alt+super+r".into()),
            style_hotkeys,
            theme: Theme::KanagawaLotus,
            default_style: custom,
            custom_style: "Make it friendly.".into(),
            provider: Some(Kind::OpenCode),
            opencode_model: "opencode/beam-free".into(),
            ..Config::load(&prefs)
        };
        config.save(&mut prefs).expect("save");
        let loaded = Config::load(&prefs);
        assert_eq!(loaded, config);
        assert_eq!(loaded.style(), Style::Custom("Make it friendly.".into()));
        assert_eq!(loaded.style_at(1), Style::Concise);
        assert_eq!(loaded.provider(), Ok(Provider::OpenCode { model: "opencode/beam-free".into() }));
        // Saved by id, so a preset added later cannot take over the custom slot.
        assert_eq!(prefs.get::<Value>("default_style"), Some(Value::from("custom")));
        assert_eq!(
            prefs.get::<Value>("style_hotkeys"),
            Some(serde_json::json!({ "concise": "ctrl+alt+2", "custom": "ctrl+alt+9" }))
        );
    }

    #[test]
    fn prefs_from_beta_10_keep_their_style_choices() {
        let mut prefs = Prefs::memory();
        prefs.set("default_style", &4).expect("set");
        prefs.set("style_hotkeys", &serde_json::json!([null, "ctrl+alt+2", null, null, "ctrl+alt+5"])).expect("set");
        prefs.set("custom_style", "Make it friendly.").expect("set");
        let config = Config::load(&prefs);
        assert_eq!(config.style(), Style::Custom("Make it friendly.".into()));
        let chord = |id: &str| slot_index(&Value::from(id)).and_then(|i| config.style_hotkeys[i].clone());
        assert_eq!(chord("concise").as_deref(), Some("ctrl+alt+2"));
        assert_eq!(chord("custom").as_deref(), Some("ctrl+alt+5"));
        assert_eq!(config.style_hotkeys.iter().flatten().count(), 2);

        // An unknown id or a number past the old slots falls back instead of failing the load.
        prefs.set("default_style", "gone").expect("set");
        assert_eq!(Config::load(&prefs).default_style, 0);
        prefs.set("default_style", &9).expect("set");
        assert_eq!(Config::load(&prefs).default_style, 0);
    }

    #[test]
    fn the_popup_offers_valid_providers_and_listed_local_models_once() {
        let config = Config {
            provider: Some(Kind::Local),
            local_endpoint: local::OLLAMA.into(),
            local_model: "qwen2.5:0.5b".into(),
            pi_model: "".into(),
            opencode_model: "opencode/beam-free".into(),
            ..Config::default()
        };
        let listed = ["llama3.2:1b".to_string(), "qwen2.5:0.5b".to_string(), " ".to_string()];
        let choices = config.choices(&listed);
        let models: Vec<(Kind, &str)> = choices.iter().map(|p| (p.kind(), p.model())).collect();
        // Pi has no model, so it is not offered; the saved local model is not listed twice.
        assert_eq!(
            models,
            [(Kind::Local, "qwen2.5:0.5b"), (Kind::Local, "llama3.2:1b"), (Kind::OpenCode, "opencode/beam-free")]
        );
    }

    #[test]
    fn choosing_saves_the_kind_and_only_that_kinds_model() {
        let mut config =
            Config { provider: Some(Kind::Pi), pi_model: "p".into(), local_model: "a".into(), ..Config::default() };
        config.choose(&Provider::Local { endpoint: local::OLLAMA.into(), model: "b".into() });
        assert_eq!(
            (config.provider, config.local_model.as_str(), config.pi_model.as_str()),
            (Some(Kind::Local), "b", "p")
        );
    }

    #[test]
    fn a_provider_is_refused_until_it_is_valid() {
        let mut config = Config { provider: Some(Kind::Local), ..Config::default() };
        config.local_endpoint = "http://192.168.1.2:11434/v1".into();
        config.local_model = "qwen".into();
        assert!(config.provider().is_err(), "a LAN address needs the opt-in");
        config.local_allow_network = true;
        assert!(config.provider().is_ok(), "a LAN address with the opt-in");
        config.local_endpoint = "http://8.8.8.8:11434/v1".into();
        assert!(config.provider().is_err(), "a public address even with the opt-in");
        config.local_endpoint = local::OLLAMA.into();
        assert!(config.provider().is_ok());
        config.provider = Some(Kind::Pi);
        config.pi_model = "--tools".into();
        assert!(config.provider().is_err(), "a flag is not a model ID");
    }
}
