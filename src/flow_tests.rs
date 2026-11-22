use super::*;

fn effects(e: Effects<Effect>) -> Vec<Effect> {
    e.into_iter().collect()
}

fn captured(text: &str) -> Result<Captured, String> {
    Ok(Captured { text: text.into(), secure_field: false, can_replace: true, from_clipboard: false })
}

/// Hotkey and capture with consent already given; returns the request's attempt.
fn start(flow: &mut Flow, text: &str) -> Attempt {
    let capture = match effects(flow.step(Event::Hotkey { has_provider: true }))[..] {
        [Effect::Capture(attempt)] => attempt,
        ref other => panic!("expected Capture, got {other:?}"),
    };
    let out = effects(flow.step(Event::Captured { attempt: capture, result: captured(text), consented: true }));
    match &out[..] {
        [Effect::Request { attempt, .. }, Effect::ShowPopup] => *attempt,
        other => panic!("expected Request, got {other:?}"),
    }
}

fn applies(out: &[Effect]) -> bool {
    out.iter().any(|e| matches!(e, Effect::Apply { .. } | Effect::CopyText(_)))
}

#[test]
fn select_rewrite_replace_closes_after_the_apply_succeeds() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "  the meeting are moved  ");
    assert!(matches!(&flow.state, State::Previewing { original, .. } if original == "the meeting are moved"));
    flow.step(Event::Answer { attempt, result: Ok("The meeting has moved.".into()) });
    let out = effects(flow.step(Event::Replace));
    let apply = match &out[..] {
        [Effect::Apply { attempt, text }] if text == "The meeting has moved." => *attempt,
        other => panic!("expected Apply, got {other:?}"),
    };
    assert_eq!(effects(flow.step(Event::Applied { attempt: apply, result: Ok(()) })), vec![Effect::ClosePopup]);
    assert_eq!(flow.state, State::Idle);
}

#[test]
fn an_answer_after_escape_or_retry_changes_nothing() {
    let mut flow = Flow::new(Style::Clear);
    let first = start(&mut flow, "text");
    let out = effects(flow.step(Event::Retry));
    assert_eq!(out[0], Effect::CancelRequest, "retry during loading replaces the request");
    // The first request answers late; the preview must still wait for the second.
    flow.step(Event::Answer { attempt: first, result: Ok("stale".into()) });
    assert!(matches!(&flow.state, State::Previewing { phase: Phase::Loading, .. }));

    flow.step(Event::Cancel);
    assert_eq!(flow.state, State::Idle);
    flow.step(Event::Answer { attempt: first, result: Ok("late".into()) });
    assert_eq!(flow.state, State::Idle);
}

#[test]
fn cancel_and_failures_never_apply_or_copy() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "text");
    assert_eq!(effects(flow.step(Event::Cancel)), vec![Effect::CancelRequest, Effect::ClosePopup]);

    let attempt2 = start(&mut flow, "text");
    assert_ne!(attempt, attempt2);
    flow.step(Event::Answer { attempt: attempt2, result: Err("Timed out after 60 s".into()) });
    assert!(!applies(&effects(flow.step(Event::Replace))), "no Replace on a failed preview");
    assert!(!applies(&effects(flow.step(Event::Copy))));
    assert_eq!(effects(flow.step(Event::Cancel)), vec![Effect::ClosePopup]);

    let attempt3 = start(&mut flow, "text");
    flow.step(Event::Answer { attempt: attempt3, result: Ok("   ".into()) });
    assert!(matches!(&flow.state, State::Previewing { phase: Phase::Failed(_), .. }), "an empty answer is a failure");
}

#[test]
fn a_failed_replace_keeps_copy_and_never_offers_replace_again() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "text");
    flow.step(Event::Answer { attempt, result: Ok("Rewrite".into()) });
    let apply = match effects(flow.step(Event::Replace))[..] {
        [Effect::Apply { attempt, .. }] => attempt,
        ref other => panic!("{other:?}"),
    };
    flow.step(Event::Applied { attempt: apply, result: Err("the selection changed".into()) });
    assert_eq!(flow.apply_error.as_deref(), Some("the selection changed"));
    assert!(effects(flow.step(Event::Replace)).is_empty(), "a second paste could insert twice");
    assert_eq!(effects(flow.step(Event::Copy)), vec![Effect::CopyText("Rewrite".into()), Effect::ClosePopup]);
}

#[test]
fn escape_during_apply_waits_for_its_answer() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "text");
    flow.step(Event::Answer { attempt, result: Ok("Rewrite".into()) });
    flow.step(Event::Replace);
    assert!(effects(flow.step(Event::Cancel)).is_empty());
    assert!(matches!(flow.state, State::Applying { .. }));
}

#[test]
fn refusals_show_a_notice_and_send_nothing() {
    let cases = [
        (
            Ok(Captured { text: "secret".into(), secure_field: true, can_replace: true, from_clipboard: false }),
            Notice::Refused(Refusal::SecureField),
        ),
        (captured(" \n "), Notice::Refused(Refusal::NoText)),
        (captured(&"a".repeat(2001)), Notice::Refused(Refusal::TooLong { chars: 2001 })),
        (Err("AX timed out".into()), Notice::CaptureFailed("AX timed out".into())),
    ];
    for (result, notice) in cases {
        let mut flow = Flow::new(Style::Clear);
        let Effect::Capture(attempt) = effects(flow.step(Event::Hotkey { has_provider: true }))[0] else { panic!() };
        let out = effects(flow.step(Event::Captured { attempt, result, consented: true }));
        assert_eq!(out, vec![Effect::ShowPopup], "no Request for {notice:?}");
        assert_eq!(flow.state, State::Notice(notice));
    }
    let mut flow = Flow::new(Style::Clear);
    assert_eq!(effects(flow.step(Event::Hotkey { has_provider: false })), vec![Effect::ShowPopup]);
    assert_eq!(flow.state, State::Notice(Notice::NoProvider));
}

#[test]
fn no_request_starts_until_consent_is_given() {
    let mut flow = Flow::new(Style::Professional);
    let Effect::Capture(attempt) = effects(flow.step(Event::Hotkey { has_provider: true }))[0] else { panic!() };
    let out = effects(flow.step(Event::Captured { attempt, result: captured("text"), consented: false }));
    assert_eq!(out, vec![Effect::ShowPopup]);
    let out = effects(flow.step(Event::ConsentGiven));
    assert!(matches!(&out[..], [Effect::Request { style: Style::Professional, .. }, Effect::SaveConsent]));

    let mut declined = Flow::new(Style::Clear);
    let Effect::Capture(attempt) = effects(declined.step(Event::Hotkey { has_provider: true }))[0] else { panic!() };
    declined.step(Event::Captured { attempt, result: captured("text"), consented: false });
    assert_eq!(effects(declined.step(Event::Cancel)), vec![Effect::ClosePopup]);
    assert_eq!(declined.state, State::Idle);
}

#[test]
fn clipboard_only_capture_offers_copy_but_not_replace() {
    let mut flow = Flow::new(Style::Clear);
    let Effect::Capture(attempt) = effects(flow.step(Event::Hotkey { has_provider: true }))[0] else { panic!() };
    let result = Ok(Captured { text: "text".into(), secure_field: false, can_replace: false, from_clipboard: false });
    let Effect::Request { attempt, .. } =
        effects(flow.step(Event::Captured { attempt, result, consented: true }))[0].clone()
    else {
        panic!()
    };
    flow.step(Event::Answer { attempt, result: Ok("Rewrite".into()) });
    assert!(effects(flow.step(Event::Replace)).is_empty());
    assert!(applies(&effects(flow.step(Event::Copy))));
}

#[test]
fn a_style_change_rewrites_the_same_original_with_the_new_style() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "text");
    flow.step(Event::Answer { attempt, result: Ok("Rewrite".into()) });
    let out = effects(flow.step(Event::ChangeStyle(Style::Concise)));
    assert!(
        matches!(&out[..], [Effect::Request { text, style: Style::Concise, .. }] if text == "text"),
        "no CancelRequest when nothing is loading: {out:?}"
    );
}

#[test]
fn a_provider_change_asks_consent_first_and_drops_the_old_answer() {
    let mut flow = Flow::new(Style::Clear);
    let first = start(&mut flow, "text");
    flow.step(Event::ChangeStyle(Style::Technical));
    let loading = match &flow.state {
        State::Previewing { attempt, .. } => *attempt,
        other => panic!("expected Previewing, got {other:?}"),
    };
    // Switched while loading to a provider without consent: stop the old request, send nothing.
    let out = effects(flow.step(Event::ProviderChanged { consented: false }));
    assert_eq!(out, vec![Effect::CancelRequest]);
    assert!(matches!(&flow.state, State::AwaitingConsent { style: Style::Technical, .. }));
    flow.step(Event::Answer { attempt: loading, result: Ok("from the old provider".into()) });
    flow.step(Event::Answer { attempt: first, result: Ok("older still".into()) });
    assert!(matches!(&flow.state, State::AwaitingConsent { .. }), "late answers must not show");

    let out = effects(flow.step(Event::ConsentGiven));
    assert!(
        matches!(&out[..], [Effect::Request { text, style: Style::Technical, .. }, Effect::SaveConsent] if text == "text")
    );
}

#[test]
fn a_provider_change_with_consent_rewrites_the_same_text_and_style_again() {
    let mut flow = Flow::new(Style::Concise);
    let attempt = start(&mut flow, "text");
    flow.step(Event::Answer { attempt, result: Ok("Rewrite".into()) });
    let out = effects(flow.step(Event::ProviderChanged { consented: true }));
    assert!(matches!(&out[..], [Effect::Request { text, style: Style::Concise, .. }] if text == "text"), "{out:?}");
    assert!(matches!(&flow.state, State::Previewing { phase: Phase::Loading, .. }));
}

#[test]
fn after_a_failed_replace_no_later_rewrite_offers_replace() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "text");
    flow.step(Event::Answer { attempt, result: Ok("Rewrite".into()) });
    let Effect::Apply { attempt: apply, .. } = effects(flow.step(Event::Replace))[0].clone() else { panic!() };
    flow.step(Event::Applied { attempt: apply, result: Err("the selection changed".into()) });
    for event in [Event::Retry, Event::ChangeStyle(Style::Concise), Event::ProviderChanged { consented: true }] {
        let Effect::Request { attempt, .. } = effects(flow.step(event))[0].clone() else { panic!() };
        flow.step(Event::Answer { attempt, result: Ok("Again".into()) });
        assert!(effects(flow.step(Event::Replace)).is_empty(), "Replace must stay off after a failed apply");
    }
}

#[test]
fn a_streaming_preview_offers_no_replace_or_copy_until_the_answer_ends() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "text");
    flow.step(Event::Partial { attempt, text: "Te".into() });
    assert!(matches!(&flow.state, State::Previewing { phase: Phase::Streaming(t), .. } if t == "Te"));
    assert!(!applies(&effects(flow.step(Event::Replace))));
    assert!(!applies(&effects(flow.step(Event::Copy))));
    // A partial from an older request is ignored.
    flow.step(Event::Partial { attempt: Attempt(attempt.0 + 99), text: "stale".into() });
    assert!(matches!(&flow.state, State::Previewing { phase: Phase::Streaming(t), .. } if t == "Te"));
    flow.step(Event::Answer { attempt, result: Ok("Text.".into()) });
    // A late partial after the answer does not take the preview back to streaming.
    flow.step(Event::Partial { attempt, text: "Te".into() });
    assert!(matches!(&flow.state, State::Previewing { phase: Phase::Ready(t), .. } if t == "Text."));

    let mut flow = Flow::new(Style::Clear);
    let running = start(&mut flow, "x");
    flow.step(Event::Partial { attempt: running, text: "X".into() });
    assert_eq!(
        effects(flow.step(Event::Cancel)),
        vec![Effect::CancelRequest, Effect::ClosePopup],
        "Esc stops the stream"
    );
}
