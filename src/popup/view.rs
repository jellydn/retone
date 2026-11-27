//! What the popup shows for each flow state. Pure: reads state, builds elements.

use super::Popup;
use crate::config::Config;
use crate::flow::{Event, Notice, Phase, State};
use crate::provider::consent;
use crate::styles::{PRESETS, Style, TONES};
use rune::{Cx, El, LiveRegion, ScrollSpec, Span, Tx, col, el, row, spacer, text};
use rune_kit::{Button, ButtonVariant, ControlSize, Kit, List, ListItem, Spinner};

/// The styles the picker offers, in key order 1–9, 0, then -: the presets, then Custom when it is set.
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
        return Some((t!("popup.replace_failed", error = error), Assertive));
    }
    match &p.flow.state {
        State::Idle | State::Capturing { .. } | State::Applying { .. } => None,
        State::Notice(notice) => {
            let (title, body) = notice_text(p, notice);
            Some((t!("popup.spoken.notice", title = title, body = body), Assertive))
        }
        State::AwaitingConsent { .. } => Some((p.provider.as_ref().map(consent::question).unwrap_or_default(), Polite)),
        // The same words while streaming, so each new token is not announced again.
        State::Previewing { phase: Phase::Loading | Phase::Streaming(_), .. } => {
            Some((t!("popup.spoken.rewriting").into(), Polite))
        }
        State::Previewing { phase: Phase::Failed(error), .. } => {
            Some((t!("popup.spoken.failed", error = error), Assertive))
        }
        State::Previewing { phase: Phase::Ready(_), can_replace, .. } => {
            let words = if *can_replace { t!("popup.spoken.ready_replace") } else { t!("popup.spoken.ready_copy") };
            Some((words.into(), Polite))
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
        State::Idle | State::Capturing { .. } => loading(k, t!("popup.reading")),
        State::Notice(notice) => notice_body(p, k, notice),
        State::AwaitingConsent { .. } => consent_body(p, k),
        State::Previewing { original, style, phase, can_replace, .. } => {
            preview_body(p, k, original, style, phase, *can_replace)
        }
        State::Applying { .. } => loading(k, t!("popup.replacing")),
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
        None => t!("popup.no_provider").into(),
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
                .label(t!("popup.provider_label", source = source))
        } else {
            text(source.clone())
                .font(k.ty.text_xsmall)
                .color(k.c.text_muted)
                .max_lines(1)
                .label(t!("popup.provider_label", source = source))
        })
}

/// The picker's key hint in words: a screen reader reads ↑↓ and ⏎ as symbol names.
fn spoken_hint(listing: bool) -> &'static str {
    if listing { t!("popup.picker.spoken_hint_loading") } else { t!("popup.picker.spoken_hint") }
}

fn picker_body(p: &Popup, k: &Kit, picker: &super::Picker) -> El {
    let mut list = List::new().active(Some(picker.selected)).empty_message(t!("popup.picker.empty"));
    for (index, choice) in picker.choices.iter().enumerate() {
        list = list.item(
            ListItem::new(format!("{}  {}", index + 1, crate::provider::local::short_model_name(choice.model())))
                .group(choice.kind().name())
                .selected(Some(choice) == p.provider.as_ref())
                .on_select(move |p: &mut Popup, tx: &mut Tx| p.choose(index, tx)),
        );
    }
    let hint = if picker.listing { t!("popup.picker.hint_loading") } else { t!("popup.picker.hint") };
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
                .label(t!("popup.picker.label"))
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
            let body = p.provider_problem.clone().unwrap_or_else(|| t!("popup.no_provider_body").into());
            (t!("popup.no_provider").into(), body)
        }
        Notice::CaptureFailed(error) => (t!("popup.capture_failed").into(), error.clone()),
    }
}

fn notice_body(p: &Popup, k: &Kit, notice: &Notice) -> El {
    let (title, body) = notice_text(p, notice);
    let mut actions = vec![action(t!("popup.ok"), "⎋", ButtonVariant::Neutral, || Event::Cancel, k)];
    if matches!(notice, Notice::NoProvider) {
        actions.insert(
            0,
            Button::new(t!("popup.open_settings"))
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
        .child(message(k, "consent", t!("popup.consent.title"), &question, k.c.text_strong))
        .child(text(t!("popup.consent.note")).font(k.ty.text_xsmall).color(k.c.text_muted).wrap())
        .child(spacer())
        .child(buttons(vec![
            action(t!("popup.cancel"), "⎋", ButtonVariant::Neutral, || Event::Cancel, k),
            action(t!("popup.allow"), "↩", ButtonVariant::Contrast, || Event::ConsentGiven, k),
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
        Phase::Loading => (loading(k, t!("popup.rewriting")), false),
        Phase::Streaming(so_far) => {
            (scrolling(k, "rewrite", so_far, Vec::new(), true).label(t!("popup.rewrite_so_far")), false)
        }
        Phase::Ready(rewrite) => (scrolling(k, "rewrite", rewrite, added, false).label(t!("popup.rewrite")), true),
        Phase::Failed(error) => (message(k, "error", t!("popup.rewrite_failed"), error, k.c.danger.fg), false),
    };
    let mut notes = Vec::new();
    if let Some(error) = &p.flow.apply_error {
        notes.push(t!("popup.replace_failed", error = error));
    } else if ready && !can_replace {
        notes.push(if p.from_service {
            t!("popup.note.from_service").to_string()
        } else if p.clipboard_only {
            t!("popup.note.clipboard_only").to_string()
        } else if p.from_clipboard {
            t!("popup.note.from_clipboard").to_string()
        } else {
            t!("popup.note.copy_only").to_string()
        });
    }
    if let Phase::Ready(rewrite) = phase
        && style.too_long(original, rewrite)
    {
        notes.push(t!("popup.note.too_long").to_string());
    }
    if matches!(phase, Phase::Streaming(_)) {
        notes.push(t!("popup.note.streaming").to_string());
    }
    if let Some(warning) = &p.adapter_warning {
        notes.push(warning.clone());
    }
    let mut actions = vec![action(t!("popup.retry"), "⌘R", ButtonVariant::Ghost, || Event::Retry, k)];
    let replace = ready && can_replace && p.flow.apply_error.is_none();
    // ↩ goes to the main button, the last one: Copy for a summary or an expansion, else Replace.
    let copy_first = style.changes_meaning();
    if replace && copy_first {
        actions.push(action(t!("popup.replace"), "⌘↩", ButtonVariant::Neutral, || Event::Replace, k));
    }
    if ready {
        let (shortcut, variant) =
            if copy_first { ("↩", ButtonVariant::Contrast) } else { ("⌘C", ButtonVariant::Neutral) };
        actions.push(action(t!("popup.copy"), shortcut, variant, || Event::Copy, k));
    }
    if replace && !copy_first {
        actions.push(action(t!("popup.replace"), "↩", ButtonVariant::Contrast, || Event::Replace, k));
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
                .label(t!("popup.original_label", original = original)),
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
        // AccessKit gives a scroll view AXUnknown on macOS; a document is text to read.
        .role("document")
        .child(text(value.to_string()).font(k.ty.ui).color(k.c.text_strong).wrap().selectable(true).spans(spans))
}

/// The key that picks the style at `index`: 1–9, 0 for the tenth, then - for the eleventh (Custom).
pub fn style_key(index: usize) -> String {
    if index >= 10 { "-".to_string() } else { ((index + 1) % 10).to_string() }
}

/// Two rows, tones then actions with Custom, so every style keeps a visible chip and one key.
fn style_picker(p: &Popup, k: &Kit, current: &Style) -> El {
    let mut chips: Vec<El> = styles(&p.config)
        .into_iter()
        .enumerate()
        .map(|(index, style)| {
            let on = style.name() == current.name();
            let key = style_key(index);
            let chosen = style.clone();
            // Small chips, so the five tones fit one row of the 440 pt popup.
            Button::new(format!("{key} {}", style.name()))
                .control_size(ControlSize::Small)
                .variant(if on { ButtonVariant::Contrast } else { ButtonVariant::GhostMuted })
                .on_press_tx(move |p: &mut Popup, tx: &mut Tx| p.step(Event::ChangeStyle(chosen.clone()), tx))
                .build(k)
                // The digit in the visible label is the key; read the name, state and key apart.
                .label(t!("popup.style_label", name = style.name()))
                .checked(on)
                .shortcut_hint(key)
        })
        .collect();
    let actions = chips.split_off(TONES);
    let chip_row = |key: &str, label: &str, chips: Vec<El>| {
        chips.into_iter().fold(row().key(key.to_string()).w_full().gap(4.0).label(label), |row, chip| row.child(chip))
    };
    col()
        .key("styles")
        .w_full()
        .gap(4.0)
        .label(t!("popup.style_picker"))
        .child(chip_row("tones", t!("popup.style_tones"), chips))
        .child(chip_row("actions", t!("popup.style_actions"), actions))
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
    fn return_copies_a_summary_and_replaces_a_rewrite() {
        let press = |style: Style, cmd: bool| {
            let mut p = previewing(Phase::Ready("b".into()), true);
            if let State::Previewing { style: current, .. } = &mut p.flow.state {
                *current = style;
            }
            let mods = rune::Mods { cmd, ..Default::default() };
            super::super::on_key(&mut p, &rune::KeyEvent { key: rune::Key::Enter, mods }, &mut Tx::default());
            p.flow.state
        };
        assert!(matches!(press(Style::Summarize, false), State::Idle), "↩ copies and closes");
        assert!(matches!(press(Style::Summarize, true), State::Applying { .. }), "⌘↩ replaces");
        assert!(matches!(press(Style::Expand, false), State::Idle));
        assert!(matches!(press(Style::Tldr, false), State::Idle), "a TL;DR copies like a summary");
        assert!(matches!(press(Style::Clear, false), State::Applying { .. }));
        assert!(matches!(press(Style::Bullets, false), State::Applying { .. }), "a list is still a rewrite");

        let mut p = previewing(Phase::Ready("b".into()), true);
        p.config.custom_style = "Rhyme.".into();
        let key = |p: &mut Popup, text: &str| {
            let ev = rune::KeyEvent { key: rune::Key::Text(text.into()), mods: rune::Mods::default() };
            super::super::on_key(p, &ev, &mut Tx::default());
        };
        key(&mut p, "0");
        assert!(matches!(&p.flow.state, State::Previewing { style: Style::Tldr, .. }), "0 picks TL;DR");
        key(&mut p, "-");
        assert!(matches!(&p.flow.state, State::Previewing { style: Style::Custom(_), .. }), "- picks Custom");
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
