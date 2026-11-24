//! Headless scenes for every popup state: `retone --window popup --snap out.png --scene NAME`.
//! Each scene sets the flow state directly; no provider, AX or pasteboard work runs.

use super::Popup;
use crate::flow::{Notice, Phase, State};
use crate::input::Refusal;
use crate::provider::Provider;
use crate::styles::Style;
use crate::theme::Theme;
use rune::machine::Attempt;

const ORIGINAL: &str = "hi team, the meeting are moved to tuesday because of some reasons, pls update ur calendars";
const REWRITE: &str = "Hi team, the meeting has moved to Tuesday. Please update your calendars.";

fn with_provider(p: &mut Popup) {
    p.provider = Some(Provider::Local { endpoint: crate::provider::local::OLLAMA.into(), model: "qwen2.5:7b".into() });
    // Settings as saved, so ⌘P in a `--script` run has providers to offer.
    p.config.provider = Some(crate::provider::Kind::Local);
    p.config.local_model = "qwen2.5:7b".into();
    p.config.local_endpoint = crate::provider::local::OLLAMA.into();
    p.config.pi_model = "anthropic/claude-haiku-5-5".into();
}

fn previewing(p: &mut Popup, phase: Phase, can_replace: bool) {
    with_provider(p);
    p.flow.state =
        State::Previewing { attempt: Attempt(1), original: ORIGINAL.into(), style: Style::Clear, phase, can_replace };
}

/// A named state for `--scene`, as Rune's `AppSpec::scenes` takes it.
type Scene = (&'static str, fn(&mut Popup));

pub const SCENES: &[Scene] = &[
    ("preview-example", |p| previewing(p, Phase::Ready(REWRITE.into()), true)),
    ("preview-lotus", |p| {
        p.config.theme = Theme::KanagawaLotus;
        previewing(p, Phase::Ready(REWRITE.into()), true);
    }),
    // Summarize: ↩ copies, Replace moves to ⌘↩; Custom set, so all ten chips show.
    ("summary-example", |p| {
        previewing(p, Phase::Ready("The meeting moved to Tuesday; update your calendars.".into()), true);
        p.config.custom_style = "Make it friendly and short.".into();
        if let State::Previewing { style, .. } = &mut p.flow.state {
            *style = Style::Summarize;
        }
    }),
    ("bullets-example", |p| {
        previewing(p, Phase::Ready("- The meeting moved to Tuesday.\n- Please update your calendars.".into()), true);
        if let State::Previewing { style, .. } = &mut p.flow.state {
            *style = Style::Bullets;
        }
    }),
    ("loading-example", |p| previewing(p, Phase::Loading, true)),
    ("streaming-example", |p| previewing(p, Phase::Streaming("Hi team, the meeting has moved to Tue".into()), true)),
    ("error-example", |p| previewing(p, Phase::Failed("No answer within 60 s.".into()), true)),
    ("too-long-example", |p| {
        with_provider(p);
        p.flow.state = State::Notice(Notice::Refused(Refusal::TooLong { chars: 2431 }));
    }),
    ("secure-field-example", |p| {
        with_provider(p);
        p.flow.state = State::Notice(Notice::Refused(Refusal::SecureField));
    }),
    ("no-permission-example", |p| {
        p.clipboard_only = true;
        previewing(p, Phase::Ready(REWRITE.into()), false);
    }),
    ("clipboard-no-selection", |p| {
        p.from_clipboard = true;
        previewing(p, Phase::Ready(REWRITE.into()), false);
    }),
    // A Copy-only app (a terminal) or a field that cannot be checked again.
    ("copy-only-example", |p| previewing(p, Phase::Ready(REWRITE.into()), false)),
    ("replace-failed-example", |p| {
        previewing(p, Phase::Ready(REWRITE.into()), false);
        p.flow.apply_error = Some("the selection changed".into());
    }),
    ("no-provider-example", |p| {
        p.provider_problem = Some("Choose a provider in Settings.".into());
        p.flow.state = State::Notice(Notice::NoProvider);
    }),
    ("adapter-disabled-example", |p| {
        p.provider_problem = Some(
            "OpenCode saved files during this rewrite, so Retone turned it off. Nothing was deleted. \
             Files may be in ~/.local/share/opencode."
                .into(),
        );
        p.flow.state = State::Notice(Notice::NoProvider);
    }),
    ("provider-picker", |p| {
        previewing(p, Phase::Ready(REWRITE.into()), true);
        let local =
            |model: &str| Provider::Local { endpoint: crate::provider::local::OLLAMA.into(), model: model.into() };
        let choices = vec![
            local("qwen2.5:7b"),
            local("llama3.2:1b"),
            Provider::Pi { model: "anthropic/claude-haiku-5-5".into() },
        ];
        p.picker = Some(super::Picker { choices, selected: 2, listing: false });
    }),
    ("consent-example", |p| {
        p.provider = Some(Provider::OpenCode { model: "opencode/beam-free".into() });
        p.flow.state = State::AwaitingConsent { original: ORIGINAL.into(), style: Style::Clear };
    }),
];
