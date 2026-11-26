//! Local AI: an OpenAI-compatible endpoint (Ollama, LM Studio, llama.cpp's llama-server) on this
//! Mac, or, after the user
//! turns it on, on a private network address (LAN or Tailscale).
//!
//! Only loopback and private addresses are accepted. The check runs in the HTTP client's
//! resolver, on the addresses it then connects to, so a host name cannot resolve to a public
//! address after the check. No proxy and no redirects, so the text cannot be sent on elsewhere.
//! An optional API key (llama-server `--api-key`) is sent as a Bearer token; it lives in the
//! Keychain, never in prefs. Retone stores no text (the server may log on its side).

use super::{Cancel, DEADLINE, answer_cap};
use crate::styles::{self, Style};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader};
use std::net::IpAddr;
use ureq::config::Config;
use ureq::http::Uri;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::unversioned::resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};

pub const OLLAMA: &str = "http://127.0.0.1:11434/v1";
pub const LM_STUDIO: &str = "http://127.0.0.1:1234/v1";
/// llama.cpp's `llama-server`: one loaded model, whose ID is often the GGUF file path; it
/// ignores the model name in requests.
pub const LLAMA_CPP: &str = "http://127.0.0.1:8080/v1";
/// Llama.app (llama.app), the menu-bar app from the llama.cpp team: its llama.cpp router lists
/// every installed model and loads the one a request names.
pub const LLAMA_APP: &str = "http://127.0.0.1:9931/v1";
/// Largest response body read, far above any capped answer; protects against a broken server.
const BODY_LIMIT: u64 = 1024 * 1024;

/// The API key for one request. Debug never prints it, so it cannot end up in a log line.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(pub String);

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ApiKey(***)")
    }
}

/// What a host is before any name lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Host {
    Loopback,
    Private,
    Public,
    /// A host name other than `localhost`: decided when the resolver looks it up.
    Name,
}

/// Splits `http(s)://host[:port]/path` and returns the authority and the host (IPv6 without
/// brackets). Err for anything Retone does not send to.
fn authority(url: &str) -> Result<(&str, &str), String> {
    let Some(rest) = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")) else {
        return Err(t!("local.scheme").into());
    };
    let authority = rest.split('/').next().unwrap_or_default();
    // `http://127.0.0.1@example.com` would send the text to example.com.
    if authority.contains('@') {
        return Err(t!("local.user_name").into());
    }
    // IPv6 hosts are bracketed: `[::1]:8080`.
    let (host, port) = match authority.strip_prefix('[') {
        Some(v6) => match v6.split_once(']') {
            Some((host, after)) => (host, after.strip_prefix(':')),
            None => ("", None),
        },
        None => match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        },
    };
    let port_ok = port.is_none_or(|port| port.parse::<u16>().is_ok());
    let name_ok = !host.is_empty()
        && !host.starts_with(['-', '.'])
        && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'));
    if !port_ok || !name_ok {
        return Err(t!("local.invalid").into());
    }
    Ok((authority, host))
}

fn classify(host: &str) -> Host {
    if host.eq_ignore_ascii_case("localhost") {
        return Host::Loopback;
    }
    match host.parse::<IpAddr>() {
        Ok(ip) if ip.to_canonical().is_loopback() => Host::Loopback,
        Ok(ip) if is_private(ip) => Host::Private,
        Ok(_) => Host::Public,
        Err(_) if host.contains(':') => Host::Public,
        Err(_) => Host::Name,
    }
}

/// Private network ranges: RFC 1918, link-local, the CGNAT range Tailscale uses (100.64/10),
/// and IPv6 unique local (includes Tailscale's fd7a:115c:a1e0::/48) and link-local.
fn is_private(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            v4.is_private() || v4.is_link_local() || (a == 100 && (b & 0xc0) == 64)
        }
        IpAddr::V6(v6) => v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

/// Whether the resolver may connect to `ip` for an endpoint whose host is loopback (only
/// loopback then) or not (loopback or private).
fn allowed(ip: IpAddr, loopback_only: bool) -> bool {
    let ip = ip.to_canonical();
    ip.is_loopback() || (!loopback_only && is_private(ip))
}

/// Checks a base URL such as `http://127.0.0.1:11434/v1` and returns it without a trailing `/`.
/// Addresses outside this Mac need `allow_network`, the user's opt-in.
pub fn validate_endpoint(url: &str, allow_network: bool) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    let (_, host) = authority(url)?;
    match classify(host) {
        Host::Loopback => Ok(url.to_string()),
        Host::Public => Err(t!("local.public").into()),
        Host::Private | Host::Name if allow_network => Ok(url.to_string()),
        Host::Private | Host::Name => Err(t!("local.needs_network").into()),
    }
}

/// The `host:port` of an endpoint outside this Mac, for the consent key and question; None for
/// loopback.
pub fn network_host(endpoint: &str) -> Option<&str> {
    let (authority, host) = authority(endpoint).ok()?;
    (classify(host) != Host::Loopback).then_some(authority)
}

/// Refuses a lookup unless every address it returns is allowed, so the connection can only go
/// to an address that passed the check.
#[derive(Debug)]
struct PrivateOnly {
    loopback_only: bool,
}

impl Resolver for PrivateOnly {
    fn resolve(&self, uri: &Uri, config: &Config, timeout: NextTimeout) -> Result<ResolvedSocketAddrs, ureq::Error> {
        let addrs = DefaultResolver::default().resolve(uri, config, timeout)?;
        if addrs.iter().all(|addr| allowed(addr.ip(), self.loopback_only)) {
            Ok(addrs)
        } else {
            Err(ureq::Error::Other(t!("local.not_private").into()))
        }
    }
}

fn agent(endpoint: &str) -> ureq::Agent {
    let loopback_only = network_host(endpoint).is_none();
    // The macOS trust store, so a LAN server with a certificate from the user's own CA works.
    let tls = TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build();
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(DEADLINE))
        .http_status_as_error(false)
        .proxy(None)
        .max_redirects(0)
        .tls_config(tls)
        .build();
    ureq::Agent::with_parts(config, DefaultConnector::default(), PrivateOnly { loopback_only })
}

/// Model IDs from `GET /models`, for the exact-ID picker in Settings.
pub fn list_models(endpoint: &str, allow_network: bool, key: Option<&ApiKey>) -> Result<Vec<String>, String> {
    let endpoint = validate_endpoint(endpoint, allow_network)?;
    let mut request = agent(&endpoint).get(format!("{endpoint}/models"));
    if let Some(ApiKey(key)) = key {
        request = request.header("Authorization", format!("Bearer {key}"));
    }
    let mut response = request.call().map_err(|e| unreachable_message(&e))?;
    let status = response.status();
    let body: Value = response.body_mut().with_config().limit(BODY_LIMIT).read_json().map_err(|_| bad_answer())?;
    if !status.is_success() {
        return Err(status_message(status, &body));
    }
    let models = body["data"].as_array().ok_or_else(bad_answer)?;
    Ok(models.iter().filter_map(|model| model["id"].as_str().map(str::to_string)).collect())
}

/// One rewrite, streamed: `on_partial` gets all text so far after each chunk, and a cancel stops
/// reading at the next chunk. A server that ignores `stream` and sends one JSON answer still works.
pub fn rewrite(
    endpoint: &str,
    model: &str,
    key: Option<&ApiKey>,
    style: &Style,
    text: &str,
    cancel: &Cancel,
    on_partial: &dyn Fn(&str),
) -> Result<String, String> {
    // The opt-in was checked when the provider was built from the saved settings; this check
    // still refuses public addresses, and the resolver checks the addresses themselves.
    let endpoint = validate_endpoint(endpoint, true)?;
    let prompt = styles::build(style, text);
    let body = json!({
        "model": model,
        "stream": true,
        "temperature": 0.2,
        "messages": messages(&prompt),
    });
    let mut request = agent(&endpoint).post(format!("{endpoint}/chat/completions"));
    if let Some(ApiKey(key)) = key {
        request = request.header("Authorization", format!("Bearer {key}"));
    }
    let mut response = request.send_json(&body).map_err(|e| unreachable_message(&e))?;
    let status = response.status();
    let streamed = response.body().mime_type() == Some("text/event-stream");
    if !status.is_success() || !streamed {
        let body: Value = response.body_mut().with_config().limit(BODY_LIMIT).read_json().map_err(|_| bad_answer())?;
        if !status.is_success() {
            return Err(status_message(status, &body));
        }
        let answer = body["choices"][0]["message"]["content"].as_str().ok_or_else(bad_answer)?;
        return finish(answer, text);
    }
    let reader = BufReader::new(response.body_mut().with_config().limit(BODY_LIMIT).reader());
    let mut answer = String::new();
    for line in reader.lines() {
        if cancel.is_cancelled() {
            return Err(t!("provider.cancelled").into());
        }
        let line = line.map_err(|_| t!("local.stopped").to_string())?;
        // Other lines are SSE comments, event names or blank separators.
        let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
        if data == "[DONE]" {
            break;
        }
        let chunk: Value = serde_json::from_str(data).map_err(|_| bad_answer())?;
        if chunk.get("error").is_some() {
            return Err(status_message(status, &chunk));
        }
        if let Some(delta) = chunk["choices"][0]["delta"]["content"].as_str() {
            answer.push_str(delta);
            // Stop a runaway model early instead of after the whole answer arrived.
            if answer.chars().count() > answer_cap(text) {
                return Err(t!("provider.too_long").into());
            }
            on_partial(&answer);
        }
    }
    finish(&answer, text)
}

fn finish(answer: &str, text: &str) -> Result<String, String> {
    let answer = styles::clean_answer(answer);
    if answer.chars().count() > answer_cap(text) {
        return Err(t!("provider.too_long").into());
    }
    Ok(answer)
}

/// System, then the style's example as an earlier user/assistant turn, then the real text. Ollama
/// and llama-server both accept this order.
fn messages(prompt: &styles::Prompt) -> Value {
    let mut messages = vec![json!({ "role": "system", "content": prompt.system })];
    if let Some((input, answer)) = &prompt.example {
        messages.push(json!({ "role": "user", "content": input }));
        messages.push(json!({ "role": "assistant", "content": answer }));
    }
    messages.push(json!({ "role": "user", "content": prompt.user }));
    Value::Array(messages)
}

/// The server's own message is short and names the model problem (llama-server: "Loading model"
/// while it starts); it should not echo the text, and `Provider::rewrite` redacts it anyway.
fn status_message(status: ureq::http::StatusCode, body: &Value) -> String {
    if status.as_u16() == 401 {
        return t!("local.unauthorized").into();
    }
    let detail = body["error"]["message"].as_str().or(body["error"].as_str()).unwrap_or(t!("provider.no_details"));
    t!("local.status", status = status, detail = detail)
}

/// A model ID for display: llama-server reports the GGUF file path, so only its file name is
/// shown. Other IDs stay whole, as `hf.co/user/model:q4` (Ollama) names a different model.
pub fn short_model_name(model: &str) -> &str {
    if !model.starts_with('/') {
        return model;
    }
    model.rsplit('/').next().filter(|name| !name.is_empty()).unwrap_or(model)
}

fn unreachable_message(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Timeout(_) => t!("provider.timeout", seconds = DEADLINE.as_secs()),
        ureq::Error::ConnectionFailed | ureq::Error::Io(_) | ureq::Error::HostNotFound => {
            t!("local.unreachable").into()
        }
        ureq::Error::Other(_) => t!("local.not_private").into(),
        ureq::Error::NativeTls(_) | ureq::Error::Tls(_) => t!("local.tls").into(),
        _ => t!("local.failed").into(),
    }
}

fn bad_answer() -> String {
    t!("local.bad_answer").into()
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod tests;
