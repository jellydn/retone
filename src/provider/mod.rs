//! Providers: a local OpenAI-compatible endpoint (on this Mac, or on a private network after
//! the user opts in), and the Pi and OpenCode CLIs.
//!
//! A closed list on purpose: Retone never guesses flags, switches tools or falls back to another
//! provider. Every request is bounded (deadline, answer cap) and its errors are redacted.

#![forbid(unsafe_code)]

pub mod cli;
pub mod consent;
pub mod local;
pub mod opencode;
pub mod pi;

use crate::styles::Style;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub const DEADLINE: Duration = Duration::from_secs(60);
/// Longest error text shown in the popup.
const ERROR_LIMIT: usize = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Local,
    Pi,
    #[serde(rename = "opencode")]
    OpenCode,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Local => "Local AI",
            Kind::Pi => "Pi",
            Kind::OpenCode => "OpenCode",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Kind::Local => "local",
            Kind::Pi => "pi",
            Kind::OpenCode => "opencode",
        }
    }
}

/// The chosen provider with its exact model ID, as saved in Settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Local { endpoint: String, model: String },
    Pi { model: String },
    OpenCode { model: String },
}

impl Provider {
    pub fn kind(&self) -> Kind {
        match self {
            Provider::Local { .. } => Kind::Local,
            Provider::Pi { .. } => Kind::Pi,
            Provider::OpenCode { .. } => Kind::OpenCode,
        }
    }

    pub fn model(&self) -> &str {
        match self {
            Provider::Local { model, .. } | Provider::Pi { model } | Provider::OpenCode { model } => model,
        }
    }

    /// Runs one rewrite. Blocking: call it from a worker, never the frame loop.
    pub fn rewrite(&self, style: &Style, text: &str, cancel: &Cancel, env: &Env) -> Outcome {
        match self {
            Provider::Local { endpoint, model } => {
                Outcome::answer(local::rewrite(endpoint, model, env.local_api_key.as_ref(), style, text))
            }
            Provider::Pi { model } => pi::rewrite(&env.pi_program, model, style, text, cancel, &env.home),
            Provider::OpenCode { model } => {
                opencode::rewrite(&env.opencode_program, model, style, text, cancel, &env.home)
            }
        }
    }
}

/// Where the CLIs and their data folders are; tests point these at stubs and a fake home.
#[derive(Debug, Clone)]
pub struct Env {
    pub home: PathBuf,
    pub pi_program: PathBuf,
    pub opencode_program: PathBuf,
    /// The Local AI API key, read from the Keychain just before the request.
    pub local_api_key: Option<local::ApiKey>,
}

impl Env {
    /// The user's home, with the CLIs found by name on `PATH` at run time.
    pub fn system() -> Option<Env> {
        let home = rune_core::env::var("HOME").map(PathBuf::from)?;
        Some(Env { home, pi_program: "pi".into(), opencode_program: "opencode".into(), local_api_key: None })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub answer: Result<String, String>,
    /// Set when a CLI changed files in its data folders during the run: the adapter must be
    /// turned off. The folder is named for the user; Retone never deletes the CLI's files.
    pub wrote_files: Option<PathBuf>,
}

impl Outcome {
    fn answer(answer: Result<String, String>) -> Self {
        Self { answer, wrote_files: None }
    }
}

/// Shared cancel flag for one request. A CLI child is killed when it is set; an HTTP request
/// cannot be interrupted, so its late answer is dropped by the flow's attempt fencing instead.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Most characters an answer may have: four times the input, so a runaway model cannot fill the
/// popup, with a floor because a two-word input can fairly become a sentence.
pub fn answer_cap(input: &str) -> usize {
    (input.chars().count() * 4).max(400)
}

/// Makes an error safe to show and log: no selected text, no home path, bounded length.
pub fn redact(error: &str, selected: &str, home: Option<&str>) -> String {
    let mut text = error.trim().to_string();
    if selected.trim().chars().count() >= 4 {
        text = text.replace(selected.trim(), "[selected text]");
    }
    if let Some(home) = home.filter(|home| home.len() > 1) {
        text = text.replace(home, "~");
    }
    if text.chars().count() > ERROR_LIMIT {
        text = text.chars().take(ERROR_LIMIT).collect::<String>() + "…";
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_lose_the_selection_and_the_home_path() {
        let error = "failed on 'the meeting are moved' reading /Users/ana/.pi/agent/x";
        assert_eq!(
            redact(error, " the meeting are moved ", Some("/Users/ana")),
            "failed on '[selected text]' reading ~/.pi/agent/x"
        );
        assert_eq!(redact(&"e".repeat(400), "", None).chars().count(), ERROR_LIMIT + 1);
    }

    #[test]
    fn the_answer_cap_is_four_times_the_input_with_a_floor() {
        assert_eq!(answer_cap("hi"), 400);
        assert_eq!(answer_cap(&"é".repeat(2000)), 8000);
    }
}
