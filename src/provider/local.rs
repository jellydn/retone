//! Local AI: an OpenAI-compatible endpoint (Ollama, LM Studio) on this Mac only.
//!
//! The host must be loopback, so the selected text cannot leave the Mac through this option.
//! No API key field; Retone stores nothing (the server may log on its side).

use super::{DEADLINE, answer_cap};
use crate::styles::{self, Style};
use serde_json::{Value, json};

pub const OLLAMA: &str = "http://127.0.0.1:11434/v1";
pub const LM_STUDIO: &str = "http://127.0.0.1:1234/v1";
/// Largest response body read, far above any capped answer; protects against a broken server.
const BODY_LIMIT: u64 = 1024 * 1024;

/// Checks a base URL such as `http://127.0.0.1:11434/v1` and returns it without a trailing `/`.
pub fn validate_endpoint(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    let Some(rest) = url.strip_prefix("http://") else {
        return Err("Use an http:// address on this Mac, such as http://127.0.0.1:11434/v1.".into());
    };
    let authority = rest.split('/').next().unwrap_or_default();
    // `http://127.0.0.1@example.com` would send the text to example.com.
    if authority.contains('@') {
        return Err("The address must not contain a user name.".into());
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
    if !matches!(host.to_ascii_lowercase().as_str(), "127.0.0.1" | "::1" | "localhost") || !port_ok {
        return Err("Retone accepts local addresses only: 127.0.0.1, ::1 or localhost.".into());
    }
    Ok(url.to_string())
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(DEADLINE)).http_status_as_error(false).build().into()
}

/// Model IDs from `GET /models`, for the exact-ID picker in Settings.
pub fn list_models(endpoint: &str) -> Result<Vec<String>, String> {
    let endpoint = validate_endpoint(endpoint)?;
    let mut response = agent().get(format!("{endpoint}/models")).call().map_err(|e| unreachable_message(&e))?;
    if !response.status().is_success() {
        return Err(format!("The server answered {}.", response.status()));
    }
    let body: Value = response.body_mut().with_config().limit(BODY_LIMIT).read_json().map_err(|_| bad_answer())?;
    let models = body["data"].as_array().ok_or_else(bad_answer)?;
    Ok(models.iter().filter_map(|model| model["id"].as_str().map(str::to_string)).collect())
}

pub fn rewrite(endpoint: &str, model: &str, style: &Style, text: &str) -> Result<String, String> {
    let endpoint = validate_endpoint(endpoint)?;
    let prompt = styles::build(style, text);
    let body = json!({
        "model": model,
        "stream": false,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": prompt.system },
            { "role": "user", "content": prompt.user },
        ],
    });
    let mut response =
        agent().post(format!("{endpoint}/chat/completions")).send_json(&body).map_err(|e| unreachable_message(&e))?;
    let status = response.status();
    let body: Value = response.body_mut().with_config().limit(BODY_LIMIT).read_json().map_err(|_| bad_answer())?;
    if !status.is_success() {
        // The server's own message is short and names the model problem; it never echoes the text
        // for these errors, and the caller redacts it anyway.
        let detail = body["error"]["message"].as_str().or(body["error"].as_str()).unwrap_or("no details");
        return Err(format!("The server answered {status}: {detail}"));
    }
    let answer = body["choices"][0]["message"]["content"].as_str().ok_or_else(bad_answer)?;
    let answer = styles::clean_answer(answer);
    if answer.chars().count() > answer_cap(text) {
        return Err("The model's answer was far longer than the text, so Retone dropped it.".into());
    }
    Ok(answer)
}

fn unreachable_message(error: &ureq::Error) -> String {
    match error {
        ureq::Error::Timeout(_) => format!("No answer within {} s.", DEADLINE.as_secs()),
        ureq::Error::ConnectionFailed | ureq::Error::Io(_) => {
            "Cannot reach the local server. Is Ollama or LM Studio running?".into()
        }
        _ => "The request to the local server failed.".into(),
    }
}

fn bad_answer() -> String {
    "The local server sent an answer Retone does not understand.".into()
}

#[cfg(test)]
#[path = "local_tests.rs"]
mod tests;
