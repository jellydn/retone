//! What the popup shows for each flow state. Pure: reads state, builds elements.

use super::Popup;
use crate::config::Config;
use crate::flow::{Event, Notice, Phase, State};
use crate::provider::consent;
use crate::styles::{PRESETS, Style};
use rune::{Cx, El, LiveRegion, ScrollSpec, Span, Tx, col, el, row, spacer, text};
use rune_kit::{Button, ButtonVariant, Kit, List, ListItem, Spinner};

/// The styles the picker offers, in key order 1–5: the presets, then Custom when it is set.
pub fn styles(config: &Config) -> Vec<Style> {
    let mut all = PRESETS.to_vec();
    if !config.custom_style.trim().is_empty() {
        all.push(Style::Custom(config.custom_style.trim().to_string()));
    }
    all
}

/// What a screen reader hears when the popup changes, and how urgently. Errors interrupt; the
/// rest waits. The rewrite itself is not read out: the user moves to it, as to any text.
pub fn spoken(p: &Popup) -> Option<(String, LiveRegion)> {
    use LiveRegion::{Assertive, Polite};
    if let Some(error) = &p.flow.apply_error {
        return Some((format!("Replace failed: {error}. Use Copy instead."), Assertive));
    }
    match &p.flow.state {
        State::Idle | State::Capturing { .. } | State::Applying { .. } => None,
        State::Notice(notice) => {
            let (title, body) = notice_text(p, notice);
            Some((format!("{title}. {body}"), Assertive))
        }
        State::AwaitingConsent { .. } => Some((p.provider.as_ref().map(consent::question).unwrap_or_default(), Polite)),
        // The same words while streaming, so each new token is not announced again.
        State::Previewing { phase: Phase::Loading | Phase::Streaming(_), .. } => Some(("Rewriting.".into(), Polite)),
        State::Previewing { phase: Phase::Failed(error), .. } => Some((format!("Rewrite failed. {error}"), Assertive)),
        State::Previewing { phase: Phase::Ready(_), can_replace, .. } => {
            let keys =
                if *can_replace { "Press Return to replace it, or ⌘C to copy it." } else { "Press ⌘C to copy it." };
            Some((format!("Rewrite ready. {keys}"), Polite))
        }
    }
}

pub fn view(p: &Popup, cx: &mut Cx<Popup>) -> El {
    let k = p.config.theme.kit(cx.system_dark());
    let body = match (&p.picker, &p.flow.state) {
        (Some(picker), _) => picker_body(p, &k, picker),
        (None, state) => state_body(p, &k, state),
    };
    col()
        .key("popup")
        .w_full()
        .h_full()
        .radius(12.0)
        .clip_rounded(12.0)
        .fill(k.c.surface_raised)
        .border(1.0, k.c.border_base)
        .padding(14.0)
        .gap(10.0)
        .child(header(p, &k))
        .child(body)
}

fn state_body(p: &Popup, k: &Kit, state: &State) -> El {
    match state {
        State::Idle | State::Capturing { .. } => loading(k, "Reading the selection…"),
        State::Notice(notice) => notice_body(p, k, notice),
        State::AwaitingConsent { .. } => consent_body(p, k),
        State::Previewing { original, style, phase, can_replace, .. } => {
            preview_body(p, k, original, style, phase, *can_replace)
        }
        State::Applying { .. } => loading(k, "Replacing…"),
    }
}

/// Whether the provider list can open now; the header label is a button only then.
fn can_switch(p: &Popup) -> bool {
    matches!(p.flow.state, State::Previewing { .. } | State::AwaitingConsent { .. })
}

fn header(p: &Popup, k: &Kit) -> El {
    let source = match &p.provider {
        Some(provider) => {
            format!("{} · {}", provider.kind().name(), crate::provider::local::short_model_name(provider.model()))
        }
        None => "No provider".into(),
    };
    row()
        .key("header")
        .w_full()
        .gap(8.0)
        .center_y()
        .child(text("Retone").font(k.ty.ui_medium).color(k.c.text_strong).a11y_hidden())
        .child(spacer())
        .child(if can_switch(p) {
            Button::new(format!("{source} ▾"))
                .variant(ButtonVariant::GhostMuted)
                .shortcut("⌘P")
                .on_press(|p: &mut Popup| p.toggle_picker())
                .build(k)
                .label(format!("Provider: {source}"))
        } else {
            text(source.clone())
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted)
                .max_lines(1)
                .label(format!("Provider: {source}"))
        })
}

/// The picker's key hint in words: a screen reader reads ↑↓ and ⏎ as symbol names.
fn spoken_hint(listing: bool) -> &'static str {
    if listing {
        "Loading local models. Arrow keys choose, Return uses, Escape goes back."
    } else {
        "Arrow keys choose, Return uses, Escape goes back."
    }
}

fn picker_body(p: &Popup, k: &Kit, picker: &super::Picker) -> El {
    let mut list =
        List::new().active(Some(picker.selected)).empty_message("No provider is set up. Open Settings → Provider.");
    for (index, choice) in picker.choices.iter().enumerate() {
        list = list.item(
            ListItem::new(format!("{}  {}", index + 1, crate::provider::local::short_model_name(choice.model())))
                .group(choice.kind().name())
                .selected(Some(choice) == p.provider.as_ref())
                .on_select(move |p: &mut Popup, tx: &mut Tx| p.choose(index, tx)),
        );
    }
    let hint = if picker.listing {
        "Loading local models…   ↑↓ choose   ⏎ use   Esc back"
    } else {
        "↑↓ choose   ⏎ use   Esc back"
    };
    col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .gap(8.0)
        .child(
            col()
                .key("providers")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec { stick_bottom: false })
                .label("Providers")
                .child(list.build(k)),
        )
        .child(text(hint).font(k.ty.text_xsmall).color(k.c.text_muted).label(spoken_hint(picker.listing)))
}

fn loading(k: &Kit, label: &str) -> El {
    row()
        .key("loading")
        .w_full()
        .grow(1.0)
        .gap(10.0)
        .center()
        .child(Spinner::new().size(16.0).build(k))
        .child(text(label.to_string()).font(k.ty.ui).color(k.c.text_muted))
}

fn message(k: &Kit, key: &str, title: &str, body: &str, tone: rune::Color) -> El {
    col()
        .key(key.to_string())
        .w_full()
        .gap(6.0)
        .child(text(title.to_string()).font(k.ty.ui_medium).color(tone))
        .child(text(body.to_string()).font(k.ty.ui).color(k.c.text_base).wrap())
}

fn buttons(items: Vec<El>) -> El {
    let mut bar = row().key("actions").w_full().gap(8.0).center_y().child(spacer());
    for item in items {
        bar = bar.child(item);
    }
    bar
}

fn action(label: &str, shortcut: &str, variant: ButtonVariant, event: fn() -> Event, k: &Kit) -> El {
    Button::new(label)
        .variant(variant)
        .shortcut(shortcut)
        .on_press_tx(move |p: &mut Popup, tx: &mut Tx| p.step(event(), tx))
        .build(k)
}

fn notice_text(p: &Popup, notice: &Notice) -> (String, String) {
    match notice {
        Notice::Refused(refusal) => (refusal.title().to_string(), refusal.message()),
        Notice::NoProvider => {
            let body = p.provider_problem.clone().unwrap_or_else(|| "Choose a provider in Settings.".into());
            ("No provider".into(), body)
        }
        Notice::CaptureFailed(error) => ("Cannot read the selection".into(), error.clone()),
    }
}

fn notice_body(p: &Popup, k: &Kit, notice: &Notice) -> El {
    let (title, body) = notice_text(p, notice);
    let mut actions = vec![action("OK", "⎋", ButtonVariant::Neutral, || Event::Cancel, k)];
    if matches!(notice, Notice::NoProvider) {
        actions.insert(
            0,
            Button::new("Open Settings")
                .variant(ButtonVariant::Outline)
                .on_press_tx(|p: &mut Popup, tx: &mut Tx| p.open_settings(tx))
                .build(k),
        );
    }
    col()
        .w_full()
        .grow(1.0)
        .gap(10.0)
        .child(message(k, "notice", &title, &body, k.c.warning.fg))
        .child(spacer())
        .child(buttons(actions))
}

fn consent_body(p: &Popup, k: &Kit) -> El {
    let question = p.provider.as_ref().map(consent::question).unwrap_or_default();
    col()
        .w_full()
        .grow(1.0)
        .gap(10.0)
        .child(message(k, "consent", "Allow this provider?", &question, k.c.text_strong))
        .child(
            text("Retone asks once per provider and model. Revoke it in Settings → Privacy.")
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted)
                .wrap(),
        )
        .child(spacer())
        .child(buttons(vec![
            action("Cancel", "⎋", ButtonVariant::Neutral, || Event::Cancel, k),
            action("Allow", "↩", ButtonVariant::Contrast, || Event::ConsentGiven, k),
        ]))
}

fn preview_body(p: &Popup, k: &Kit, original: &str, style: &Style, phase: &Phase, can_replace: bool) -> El {
    let picker = style_picker(p, k, style);
    // Removed words in red on the original, added words on a green background in the rewrite.
    let (removed, added) = match phase {
        Phase::Ready(rewrite) => crate::diff::changed_words(original, rewrite),
        _ => (Vec::new(), Vec::new()),
    };
    let removed: Vec<Span> = removed.into_iter().map(|r| Span::new(r.start, r.end).color(k.c.danger.fg)).collect();
    let added: Vec<Span> =
        added.into_iter().map(|r| Span::new(r.start, r.end).bg(k.c.success.bg, 1.0).color(k.c.success.fg)).collect();
    let (result, ready) = match phase {
        Phase::Loading => (loading(k, "Rewriting…"), false),
        Phase::Streaming(so_far) => (scrolling(k, "rewrite", so_far, Vec::new(), true).label("Rewrite so far"), false),
        Phase::Ready(rewrite) => (scrolling(k, "rewrite", rewrite, added, false).label("Rewrite"), true),
        Phase::Failed(error) => (message(k, "error", "Rewrite failed", error, k.c.danger.fg), false),
    };
    let mut notes = Vec::new();
    if let Some(error) = &p.flow.apply_error {
        notes.push(format!("Replace failed: {error}. Use Copy instead."));
    } else if ready && !can_replace {
        notes.push(if p.clipboard_only {
            "Accessibility is off, so Retone used the clipboard, not the selection. Grant it in Settings.".to_string()
        } else if p.from_clipboard {
            "Nothing was selected, so Retone used the clipboard text. Only Copy is offered.".to_string()
        } else {
            "Retone cannot replace text in this app or field, so only Copy is offered.".to_string()
        });
    }
    if matches!(phase, Phase::Streaming(_)) {
        notes.push("Still writing… Copy and Replace turn on when the answer is complete.".to_string());
    }
    if let Some(warning) = &p.adapter_warning {
        notes.push(warning.clone());
    }
    let mut actions = vec![action("Retry", "⌘R", ButtonVariant::Ghost, || Event::Retry, k)];
    if ready {
        actions.push(action("Copy", "⌘C", ButtonVariant::Neutral, || Event::Copy, k));
    }
    if ready && can_replace && p.flow.apply_error.is_none() {
        actions.push(action("Replace", "↩", ButtonVariant::Contrast, || Event::Replace, k));
    }
    let mut body = col()
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .gap(8.0)
        .child(picker)
        .child(
            text(original.to_string())
                .key("original")
                .font(k.ty.text_small)
                .color(k.c.text_muted)
                .max_lines(2)
                .spans(removed)
                .label(format!("Original: {original}")),
        )
        .child(el().w_full().h(1.0).fill(k.c.border_weak))
        .child(result);
    for note in notes {
        body = body.child(text(note).font(k.ty.text_xsmall).color(k.c.warning.fg).wrap());
    }
    body.child(buttons(actions))
}

/// `follow` keeps the end in view while a streamed answer grows.
fn scrolling(k: &Kit, key: &str, value: &str, spans: Vec<Span>, follow: bool) -> El {
    col()
        .key(key.to_string())
        .w_full()
        .grow(1.0)
        .min_h(0.0)
        .scroll(ScrollSpec { stick_bottom: follow })
        .child(text(value.to_string()).font(k.ty.ui).color(k.c.text_strong).wrap().selectable(true).spans(spans))
}

fn style_picker(p: &Popup, k: &Kit, current: &Style) -> El {
    let mut picker = row().key("styles").w_full().gap(4.0).label("Style");
    for (index, style) in styles(&p.config).into_iter().enumerate() {
        let on = style.name() == current.name();
        let label = format!("{} {}", index + 1, style.name());
        let chosen = style.clone();
        picker = picker.child(
            Button::new(label)
                .variant(if on { ButtonVariant::Contrast } else { ButtonVariant::GhostMuted })
                .on_press_tx(move |p: &mut Popup, tx: &mut Tx| p.step(Event::ChangeStyle(chosen.clone()), tx))
                .build(k)
                // The digit in the visible label is the key; read the name, state and key apart.
                .label(format!("{} style", style.name()))
                .checked(on)
                .shortcut_hint((index + 1).to_string()),
        );
    }
    picker
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Refusal;
    use rune::machine::Attempt;

    fn previewing(phase: Phase, can_replace: bool) -> Popup {
        let mut p = Popup::default();
        p.flow.state =
            State::Previewing { attempt: Attempt(1), original: "a".into(), style: Style::Clear, phase, can_replace };
        p
    }

    #[test]
    fn errors_interrupt_and_progress_waits() {
        let (words, how) = spoken(&previewing(Phase::Failed("No answer within 60 s.".into()), true)).expect("spoken");
        assert_eq!((words.as_str(), how), ("Rewrite failed. No answer within 60 s.", LiveRegion::Assertive));
        assert_eq!(spoken(&previewing(Phase::Loading, true)), Some(("Rewriting.".into(), LiveRegion::Polite)));
        // Streaming says the same words, so a screen reader does not hear every new token.
        assert_eq!(
            spoken(&previewing(Phase::Streaming("Te".into()), true)),
            Some(("Rewriting.".into(), LiveRegion::Polite))
        );
        assert_eq!(spoken(&Popup::default()), None, "nothing while idle");

        let mut too_long = Popup::default();
        too_long.flow.state = State::Notice(Notice::Refused(Refusal::TooLong { chars: 2431 }));
        let (words, how) = spoken(&too_long).expect("spoken");
        assert!(words.starts_with("Selection too long. 2,431 characters") && how == LiveRegion::Assertive, "{words}");
    }

    #[test]
    fn a_ready_rewrite_names_only_the_keys_that_work() {
        let (words, _) = spoken(&previewing(Phase::Ready("b".into()), true)).expect("spoken");
        assert!(words.contains("Return to replace"), "{words}");
        let (words, _) = spoken(&previewing(Phase::Ready("b".into()), false)).expect("spoken");
        assert!(!words.contains("Return") && words.contains("⌘C"), "{words}");
        // A failed Replace keeps the preview on screen but is announced as the error.
        let mut failed = previewing(Phase::Ready("b".into()), true);
        failed.flow.apply_error = Some("the selection changed".into());
        let (words, how) = spoken(&failed).expect("spoken");
        assert_eq!(
            (words.as_str(), how),
            ("Replace failed: the selection changed. Use Copy instead.", LiveRegion::Assertive)
        );
    }
}
