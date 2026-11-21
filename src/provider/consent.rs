//! Consent per provider and model, and adapters turned off by the runtime guard. Both live in
//! `prefs.json`; neither holds any text.

use super::{Kind, Provider, local, opencode};
use rune_core::prefs::Prefs;
use std::collections::BTreeMap;

const CONSENT: &str = "consent";
const DISABLED: &str = "adapter_disabled";

/// A server outside this Mac gets its own key with its host, so moving the same model to
/// another machine asks again. Loopback keeps the plain key it always had.
fn consent_key(provider: &Provider) -> String {
    match provider {
        Provider::Local { endpoint, model } if let Some(host) = local::network_host(endpoint) => {
            format!("local@{host}:{model}")
        }
        _ => format!("{}:{}", provider.kind().key(), provider.model()),
    }
}

pub fn has_consent(prefs: &Prefs, provider: &Provider) -> bool {
    let map: BTreeMap<String, bool> = prefs.get(CONSENT).unwrap_or_default();
    map.get(&consent_key(provider)).copied().unwrap_or(false)
}

pub fn grant(prefs: &mut Prefs, provider: &Provider) -> Result<(), String> {
    let mut map: BTreeMap<String, bool> = prefs.get(CONSENT).unwrap_or_default();
    map.insert(consent_key(provider), true);
    prefs.set(CONSENT, &map).map_err(|e| e.to_string())
}

/// Settings → Privacy: revoking all asks again before the next request.
pub fn revoke_all(prefs: &mut Prefs) -> Result<(), String> {
    prefs.remove(CONSENT).map_err(|e| e.to_string())
}

/// The consent question shown before the first request with this provider and model.
pub fn question(provider: &Provider) -> String {
    if let Provider::Local { endpoint, model } = provider
        && let Some(host) = local::network_host(endpoint)
    {
        let mut text =
            format!("Send the selected text to Local AI on {host} / {model}? It leaves this Mac over your network.");
        if endpoint.starts_with("http://") {
            text.push_str(" Plain http is not encrypted: use Tailscale or https on a shared network.");
        }
        return text;
    }
    let mut text = format!("Send the selected text to {} / {}?", provider.kind().name(), provider.model());
    if provider.kind() == Kind::OpenCode && opencode::is_free_model(provider.model()) {
        text.push_str(" Free models can have different privacy terms and limits.");
    }
    text
}

pub fn disabled_reason(prefs: &Prefs, kind: Kind) -> Option<String> {
    let map: BTreeMap<String, String> = prefs.get(DISABLED).unwrap_or_default();
    map.get(kind.key()).cloned()
}

/// Called when the runtime guard saw the CLI write files. Nothing is deleted.
pub fn disable(prefs: &mut Prefs, kind: Kind, folder: &str) -> Result<String, String> {
    let reason = format!(
        "{} saved files during this rewrite, so Retone turned it off. Nothing was deleted. Files may be in {folder}.",
        kind.name()
    );
    let mut map: BTreeMap<String, String> = prefs.get(DISABLED).unwrap_or_default();
    map.insert(kind.key().into(), reason.clone());
    prefs.set(DISABLED, &map).map_err(|e| e.to_string())?;
    Ok(reason)
}

/// Settings: the user turns the adapter on again after reading the warning.
pub fn enable(prefs: &mut Prefs, kind: Kind) -> Result<(), String> {
    let mut map: BTreeMap<String, String> = prefs.get(DISABLED).unwrap_or_default();
    map.remove(kind.key());
    prefs.set(DISABLED, &map).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_is_per_provider_and_exact_model() {
        let mut prefs = Prefs::memory();
        let pi = Provider::Pi { model: "ollama/qwen2.5:0.5b".into() };
        assert!(!has_consent(&prefs, &pi));
        grant(&mut prefs, &pi).expect("grant");
        assert!(has_consent(&prefs, &pi));
        assert!(!has_consent(&prefs, &Provider::Pi { model: "ollama/qwen2.5:7b".into() }), "a new model asks again");
        assert!(!has_consent(&prefs, &Provider::OpenCode { model: "ollama/qwen2.5:0.5b".into() }));
        revoke_all(&mut prefs).expect("revoke");
        assert!(!has_consent(&prefs, &pi));
    }

    #[test]
    fn a_server_outside_this_mac_asks_again_and_names_its_host() {
        let mut prefs = Prefs::memory();
        let here = Provider::Local { endpoint: crate::provider::local::OLLAMA.into(), model: "qwen".into() };
        let lan = Provider::Local { endpoint: "http://192.168.1.5:11434/v1".into(), model: "qwen".into() };
        let tailnet = Provider::Local { endpoint: "https://box.tailnet.ts.net/v1".into(), model: "qwen".into() };
        grant(&mut prefs, &here).expect("grant");
        assert!(!has_consent(&prefs, &lan), "the same model on another machine asks again");
        grant(&mut prefs, &lan).expect("grant");
        assert!(!has_consent(&prefs, &tailnet));
        assert!(question(&lan).contains("on 192.168.1.5:11434 / qwen"));
        assert!(question(&lan).contains("Plain http is not encrypted"));
        assert!(!question(&tailnet).contains("Plain http"), "https needs no warning");
        assert_eq!(question(&here), "Send the selected text to Local AI / qwen?");
    }

    #[test]
    fn free_opencode_models_carry_the_privacy_note() {
        let free = Provider::OpenCode { model: "opencode/beam-free".into() };
        assert!(question(&free).ends_with("Free models can have different privacy terms and limits."));
        assert_eq!(
            question(&Provider::Local { endpoint: "x".into(), model: "qwen".into() }),
            "Send the selected text to Local AI / qwen?"
        );
    }

    #[test]
    fn a_disabled_adapter_stays_off_until_enabled() {
        let mut prefs = Prefs::memory();
        disable(&mut prefs, Kind::OpenCode, "~/.local/share/opencode").expect("disable");
        assert!(disabled_reason(&prefs, Kind::OpenCode).is_some_and(|r| r.contains("Nothing was deleted")));
        assert_eq!(disabled_reason(&prefs, Kind::Pi), None);
        enable(&mut prefs, Kind::OpenCode).expect("enable");
        assert_eq!(disabled_reason(&prefs, Kind::OpenCode), None);
    }
}
