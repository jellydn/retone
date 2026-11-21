//! The user's settings in `prefs.json`. No selected or rewritten text is ever stored here.

#![forbid(unsafe_code)]

use crate::provider::{Kind, Provider, cli, local};
use crate::styles::Style;
use crate::theme::Theme;
use rune_core::prefs::Prefs;

/// Release and dev builds keep separate prefs folders, as they have separate bundle IDs.
pub const APP_ID: &str =
    if cfg!(debug_assertions) { "io.github.jellydn.retone.dev" } else { "io.github.jellydn.retone" };

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Config {
    /// Rune accelerator string, such as `alt+super+r`. None until the user records one.
    pub hotkey: Option<String>,
    pub theme: Theme,
    /// Index into the presets, or `PRESETS.len()` for the custom style.
    pub default_style: usize,
    pub custom_style: String,
    pub provider: Option<Kind>,
    pub local_endpoint: String,
    /// The user's opt-in for a Local AI server outside this Mac (LAN or Tailscale).
    pub local_allow_network: bool,
    pub local_model: String,
    pub pi_model: String,
    pub opencode_model: String,
}

impl Config {
    pub fn load(prefs: &Prefs) -> Config {
        let text = |key: &str| prefs.get::<String>(key).unwrap_or_default();
        Config {
            hotkey: prefs.get("hotkey"),
            theme: prefs.get("theme").unwrap_or_default(),
            default_style: prefs.get("default_style").unwrap_or(0),
            custom_style: text("custom_style"),
            provider: prefs.get("provider"),
            local_endpoint: prefs.get("local_endpoint").unwrap_or_else(|| local::OLLAMA.to_string()),
            local_allow_network: prefs.get("local_allow_network").unwrap_or(false),
            local_model: text("local_model"),
            pi_model: text("pi_model"),
            opencode_model: text("opencode_model"),
        }
    }

    pub fn save(&self, prefs: &mut Prefs) -> Result<(), String> {
        let save = |prefs: &mut Prefs, key: &str, value: &serde_json::Value| prefs.set(key, value);
        let fields = [
            ("hotkey", serde_json::json!(self.hotkey)),
            ("theme", serde_json::json!(self.theme)),
            ("default_style", serde_json::json!(self.default_style)),
            ("custom_style", serde_json::json!(self.custom_style)),
            ("provider", serde_json::json!(self.provider)),
            ("local_endpoint", serde_json::json!(self.local_endpoint)),
            ("local_allow_network", serde_json::json!(self.local_allow_network)),
            ("local_model", serde_json::json!(self.local_model)),
            ("pi_model", serde_json::json!(self.pi_model)),
            ("opencode_model", serde_json::json!(self.opencode_model)),
        ];
        for (key, value) in fields {
            save(prefs, key, &value).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// The style a new rewrite starts with; an empty custom style falls back to Clear.
    pub fn style(&self) -> Style {
        let presets = crate::styles::PRESETS;
        match presets.get(self.default_style) {
            Some(style) => style.clone(),
            None if !self.custom_style.trim().is_empty() => Style::Custom(self.custom_style.trim().to_string()),
            None => Style::Clear,
        }
    }

    /// The chosen provider, checked as Settings checks it before saving. Err names what is
    /// missing, for the popup's "Choose a provider" notice.
    pub fn provider(&self) -> Result<Provider, String> {
        match self.provider {
            None => Err("Choose a provider in Settings.".into()),
            Some(Kind::Local) => {
                let endpoint = local::validate_endpoint(&self.local_endpoint, self.local_allow_network)?;
                let model = exact_model(&self.local_model, "Local AI")?;
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
        }
    }
}

impl Config {
    /// What the popup's provider list offers: each provider that is valid with its saved model,
    /// and the other models the local endpoint listed next to the local one, so each provider's
    /// rows stay together. A model ID is never typed in the popup.
    pub fn choices(&self, local_models: &[String]) -> Vec<Provider> {
        let mut all: Vec<Provider> = [Kind::Local, Kind::Pi, Kind::OpenCode]
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
        }
    }
}

fn exact_model(model: &str, name: &str) -> Result<String, String> {
    let model = model.trim();
    if model.is_empty() {
        return Err(format!("Choose an exact {name} model ID in Settings."));
    }
    Ok(model.to_string())
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
        let config = Config {
            hotkey: Some("alt+super+r".into()),
            theme: Theme::KanagawaLotus,
            default_style: 4,
            custom_style: "Make it friendly.".into(),
            provider: Some(Kind::OpenCode),
            opencode_model: "opencode/beam-free".into(),
            ..Config::load(&prefs)
        };
        config.save(&mut prefs).expect("save");
        let loaded = Config::load(&prefs);
        assert_eq!(loaded, config);
        assert_eq!(loaded.style(), Style::Custom("Make it friendly.".into()));
        assert_eq!(loaded.provider(), Ok(Provider::OpenCode { model: "opencode/beam-free".into() }));
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
