//! The rewrite flow as one state machine: hotkey → capture → consent → preview → apply.
//!
//! `step` only changes state and returns effects; the app carries them out (native capture,
//! provider request, paste) off the frame loop and feeds the answers back as events. Every answer
//! carries the `Attempt` that started it, so a late answer after Esc or Retry changes nothing.
//! Nothing here touches the target app or the clipboard: on cancel or failure there is simply no
//! `Apply` effect.

#![forbid(unsafe_code)]

use crate::input::{self, Refusal};
use crate::styles::Style;
use rune::machine::{Attempt, Effects, Seq};

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Idle,
    Capturing {
        attempt: Attempt,
    },
    /// Shown instead of a preview: nothing was sent and nothing changes.
    Notice(Notice),
    AwaitingConsent {
        original: String,
        style: Style,
    },
    Previewing {
        attempt: Attempt,
        original: String,
        style: Style,
        phase: Phase,
        can_replace: bool,
    },
    Applying {
        attempt: Attempt,
        original: String,
        style: Style,
        rewrite: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Loading,
    /// The answer so far, while the provider streams it. Replace and Copy wait for `Ready`.
    Streaming(String),
    Ready(String),
    Failed(String),
}

impl Phase {
    /// A request is still running: Esc, Retry or a style change must cancel it.
    pub fn is_running(&self) -> bool {
        matches!(self, Phase::Loading | Phase::Streaming(_))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Notice {
    Refused(Refusal),
    NoProvider,
    CaptureFailed(String),
}

/// What native capture found. `can_replace` is false in clipboard-only mode, when the text came
/// from the clipboard, and when the target cannot be verified again before Replace.
#[derive(Debug, Clone, PartialEq)]
pub struct Captured {
    pub text: String,
    pub secure_field: bool,
    pub can_replace: bool,
    /// The text is the clipboard's, not a selection: nothing was selected, or Accessibility is off.
    pub from_clipboard: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Hotkey {
        has_provider: bool,
    },
    Captured {
        attempt: Attempt,
        result: Result<Captured, String>,
        consented: bool,
    },
    ConsentGiven,
    /// Part of a streamed answer: all text so far.
    Partial {
        attempt: Attempt,
        text: String,
    },
    Answer {
        attempt: Attempt,
        result: Result<String, String>,
    },
    Retry,
    ChangeStyle(Style),
    /// The user picked another provider or model in the popup. `consented` says whether that
    /// provider and model already have consent.
    ProviderChanged {
        consented: bool,
    },
    Replace,
    Copy,
    Applied {
        attempt: Attempt,
        result: Result<(), String>,
    },
    /// Escape, or the popup losing its target.
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Capture(Attempt),
    ShowPopup,
    Request { attempt: Attempt, text: String, style: Style },
    CancelRequest,
    SaveConsent,
    Apply { attempt: Attempt, text: String },
    CopyText(String),
    ClosePopup,
}

pub struct Flow {
    pub state: State,
    pub default_style: Style,
    /// Why the last Replace failed, shown above the kept rewrite. Cleared by a new request.
    pub apply_error: Option<String>,
    seq: Seq,
    can_replace: bool,
}

impl Flow {
    pub fn new(default_style: Style) -> Self {
        Self { state: State::Idle, default_style, apply_error: None, seq: Seq::default(), can_replace: false }
    }

    pub fn step(&mut self, event: Event) -> Effects<Effect> {
        use Effect::*;
        let state = std::mem::replace(&mut self.state, State::Idle);
        let (next, effects) = match (state, event) {
            (State::Idle | State::Notice(_), Event::Hotkey { has_provider: false }) => {
                (State::Notice(Notice::NoProvider), Effects::one(ShowPopup))
            }
            (State::Idle | State::Notice(_), Event::Hotkey { has_provider: true }) => {
                let attempt = self.seq.next();
                (State::Capturing { attempt }, Effects::one(Capture(attempt)))
            }
            (State::Capturing { attempt }, Event::Captured { attempt: a, result, consented }) if attempt == a => {
                self.captured(result, consented)
            }
            (State::AwaitingConsent { original, style }, Event::ConsentGiven) => {
                let (state, mut effects) = self.request(original, style);
                effects.push(SaveConsent);
                (state, effects)
            }
            (
                State::Previewing { attempt, original, style, phase, can_replace },
                Event::Partial { attempt: a, text },
            ) if attempt == a && phase.is_running() => {
                let phase = Phase::Streaming(text);
                (State::Previewing { attempt, original, style, phase, can_replace }, Effects::none())
            }
            (State::Previewing { attempt, original, style, can_replace, .. }, Event::Answer { attempt: a, result })
                if attempt == a =>
            {
                let phase = match result {
                    Ok(text) if !text.trim().is_empty() => Phase::Ready(text),
                    Ok(_) => Phase::Failed(t!("flow.no_text").into()),
                    Err(error) => Phase::Failed(error),
                };
                (State::Previewing { attempt, original, style, phase, can_replace }, Effects::none())
            }
            (State::Previewing { original, style, phase, .. }, Event::Retry) => {
                let (state, effects) = self.request(original, style);
                (state, with_cancel_if_loading(&phase, effects))
            }
            (State::Previewing { original, phase, .. }, Event::ChangeStyle(style)) => {
                let (state, effects) = self.request(original, style);
                (state, with_cancel_if_loading(&phase, effects))
            }
            (State::Previewing { original, style, phase, .. }, Event::ProviderChanged { consented: true }) => {
                let (state, effects) = self.request(original, style);
                (state, with_cancel_if_loading(&phase, effects))
            }
            // Nothing is sent to a provider and model without its own consent.
            (State::Previewing { original, style, phase, .. }, Event::ProviderChanged { consented: false }) => {
                let effects = if phase.is_running() { Effects::one(CancelRequest) } else { Effects::none() };
                (State::AwaitingConsent { original, style }, effects)
            }
            (State::AwaitingConsent { original, style }, Event::ProviderChanged { consented: true }) => {
                self.request(original, style)
            }
            (
                State::Previewing { original, style, phase: Phase::Ready(rewrite), can_replace: true, .. },
                Event::Replace,
            ) => {
                let attempt = self.seq.next();
                let effects = Effects::one(Apply { attempt, text: rewrite.clone() });
                (State::Applying { attempt, original, style, rewrite }, effects)
            }
            (State::Previewing { phase: Phase::Ready(rewrite), .. }, Event::Copy) => {
                (State::Idle, Effects::two(CopyText(rewrite), ClosePopup))
            }
            (State::Applying { attempt, .. }, Event::Applied { attempt: a, result: Ok(()) }) if attempt == a => {
                (State::Idle, Effects::one(ClosePopup))
            }
            (
                State::Applying { attempt, original, style, rewrite },
                Event::Applied { attempt: a, result: Err(error) },
            ) if attempt == a => {
                // Replace refused or failed: keep the rewrite so Copy still works, but do not
                // offer Replace again, not even after Retry or a new style or provider: a second
                // try could insert the text twice. Only a new capture allows it again.
                self.apply_error = Some(error);
                self.can_replace = false;
                let phase = Phase::Ready(rewrite);
                (State::Previewing { attempt, original, style, phase, can_replace: false }, Effects::none())
            }
            (State::Previewing { phase, .. }, Event::Cancel) if phase.is_running() => {
                (State::Idle, Effects::two(CancelRequest, ClosePopup))
            }
            // Applying is short and already posted ⌘V; Esc then waits for its answer.
            (state @ State::Applying { .. }, Event::Cancel) => (state, Effects::none()),
            (State::Idle, Event::Cancel) => (State::Idle, Effects::none()),
            (_, Event::Cancel) => (State::Idle, Effects::one(ClosePopup)),
            // Late answers, and keys that do not apply in this state, change nothing.
            (state, _) => (state, Effects::none()),
        };
        self.state = next;
        effects
    }

    fn captured(&mut self, result: Result<Captured, String>, consented: bool) -> (State, Effects<Effect>) {
        let captured = match result {
            Ok(captured) => captured,
            Err(error) => return (State::Notice(Notice::CaptureFailed(error)), Effects::one(Effect::ShowPopup)),
        };
        if captured.secure_field {
            return (State::Notice(Notice::Refused(Refusal::SecureField)), Effects::one(Effect::ShowPopup));
        }
        let original = match input::check(&captured.text) {
            Ok(text) => text,
            Err(refusal) => return (State::Notice(Notice::Refused(refusal)), Effects::one(Effect::ShowPopup)),
        };
        self.can_replace = captured.can_replace;
        let style = self.default_style.clone();
        if !consented {
            return (State::AwaitingConsent { original, style }, Effects::one(Effect::ShowPopup));
        }
        let (state, mut effects) = self.request(original, style);
        effects.push(Effect::ShowPopup);
        (state, effects)
    }

    fn request(&mut self, original: String, style: Style) -> (State, Effects<Effect>) {
        self.apply_error = None;
        let attempt = self.seq.next();
        let effect = Effect::Request { attempt, text: original.clone(), style: style.clone() };
        let state =
            State::Previewing { attempt, original, style, phase: Phase::Loading, can_replace: self.can_replace };
        (state, Effects::one(effect))
    }
}

/// A Retry or style change during loading replaces the running request instead of queuing one.
fn with_cancel_if_loading(phase: &Phase, effects: Effects<Effect>) -> Effects<Effect> {
    if !phase.is_running() {
        return effects;
    }
    let mut all = Effects::one(Effect::CancelRequest);
    all.extend(effects);
    all
}

#[cfg(test)]
#[path = "flow_tests.rs"]
mod tests;
