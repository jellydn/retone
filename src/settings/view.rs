//! What the Settings window shows. Pure: reads state, builds elements.

use super::{HotkeyTest, PAGES, Page, SettingsUi};
use crate::provider::{Kind, local, opencode, pi};
use crate::styles::PRESETS;
use crate::theme;
use crate::updates::UpdateChannel;
use rune::{Cx, El, ScrollSpec, Shortcut, ShortcutError, Tx, col, row, spacer, text};
use rune_kit::{
    Button, ButtonVariant, Kit, SegmentedControl, ShortcutRecorder, Switch, TextInput, shortcut_error_message,
};

pub fn view(s: &SettingsUi, cx: &mut Cx<SettingsUi>) -> El {
    let k = s.config.theme.kit(cx.system_dark());
    let labels: Vec<&str> = PAGES.iter().map(|(_, label)| *label).collect();
    let selected = PAGES.iter().position(|(page, _)| *page == s.page);
    let tabs = SegmentedControl::new(&labels, selected)
        .fit()
        .on_pick(|s: &mut SettingsUi, i, _: &mut Tx| s.page = PAGES[i].0)
        .build(&k);
    let page = match s.page {
        Page::Shortcut => shortcut_page(s, &k),
        Page::Provider => provider_page(s, &k),
        Page::Style => style_page(s, &k),
        Page::Privacy => privacy_page(s, &k),
        Page::Permissions => permissions_page(&k),
        Page::Updates => updates_page(s, &k),
    };
    col()
        .key("settings")
        .w_full()
        .h_full()
        .fill(k.c.page)
        .padding(20.0)
        .gap(16.0)
        .child(row().w_full().center().child(tabs))
        .child(
            col()
                .key("page")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .gap(14.0)
                .scroll(ScrollSpec { stick_bottom: false })
                .child(page),
        )
}

fn heading(k: &Kit, title: &str) -> El {
    text(title.to_string()).font(k.ty.ui_medium).color(k.c.text_strong)
}

fn note(k: &Kit, body: impl Into<String>) -> El {
    text(body.into()).font(k.ty.text_small).color(k.c.text_muted).wrap()
}

/// A visible label above a field; the kit's own label is only read by screen readers.
fn labelled(k: &Kit, label: &str, field: El) -> El {
    col().w_full().gap(4.0).child(text(label.to_string()).font(k.ty.text_small).color(k.c.text_muted)).child(field)
}

fn status(k: &Kit, result: &Result<String, String>) -> El {
    let (body, color) = match result {
        Ok(body) => (body.clone(), k.c.success.fg),
        Err(body) => (body.clone(), k.c.danger.fg),
    };
    text(body).font(k.ty.text_small).color(color).wrap()
}

fn shortcut_page(s: &SettingsUi, k: &Kit) -> El {
    let owner = s.conflict_owner.clone();
    let message = move |error: &ShortcutError, chord: Option<&Shortcut>| match (error, chord, &owner) {
        (ShortcutError::Conflict, Some(chord), Some(owner)) => {
            format!("{} is used by {owner}.", chord.keycaps().join(" "))
        }
        (ShortcutError::Conflict, _, None) => "Another app holds this shortcut.".into(),
        (ShortcutError::InvalidAccelerator(reason), _, _) => reason.clone(),
        _ => shortcut_error_message(error, chord),
    };
    let recorder = ShortcutRecorder::new("shortcut", &s.recorder, |s: &mut SettingsUi| &mut s.recorder)
        .label("Rewrite selection")
        .on_change(|s: &mut SettingsUi, next, _: &mut Tx| s.change_hotkey(next))
        .error_message(message)
        .build(k);
    let test = match s.hotkey_test {
        HotkeyTest::Idle => note(k, "Check that the shortcut reaches Retone."),
        HotkeyTest::Waiting => note(k, "Press your shortcut now."),
        HotkeyTest::Passed => text("It works.").font(k.ty.text_small).color(k.c.success.fg),
    };
    let mut page = col().w_full().gap(10.0).child(heading(k, "Global shortcut"));
    if s.recorder.shortcut().is_none() {
        page = page.child(note(k, "Record a shortcut to start. Until then, use Rewrite Selection in the menu bar."));
    }
    page.child(recorder)
        .child(
            row()
                .key("test")
                .w_full()
                .gap(10.0)
                .center_y()
                .child(
                    Button::new("Test")
                        .variant(ButtonVariant::Outline)
                        .disabled(s.recorder.shortcut().is_none())
                        .on_press(|s: &mut SettingsUi| s.start_hotkey_test())
                        .build(k),
                )
                .child(test),
        )
        .child(note(k, "This shortcut works in every app and replaces that app's own use of it."))
}

fn provider_page(s: &SettingsUi, k: &Kit) -> El {
    let choices = [None, Some(Kind::Local), Some(Kind::Pi), Some(Kind::OpenCode)];
    let selected = choices.iter().position(|c| *c == s.provider_choice);
    let picker = SegmentedControl::new(&["None", "Local AI", "Pi", "OpenCode"], selected)
        .fit()
        .on_pick(move |s: &mut SettingsUi, i, _: &mut Tx| s.choose_provider(choices[i]))
        .build(k);
    let mut page = col().w_full().gap(10.0).child(heading(k, "Provider")).child(picker);
    page = match s.provider_choice {
        None => page.child(note(k, "Retone sends nothing until you choose a provider.")),
        Some(Kind::Local) => local_fields(s, k, page),
        Some(Kind::Pi) => page
            .child(labelled(
                k,
                "Model",
                TextInput::new(&s.pi_model)
                    .label("Model")
                    .placeholder("ollama/qwen2.5:7b")
                    .on_edit(|s: &mut SettingsUi| &mut s.pi_model)
                    .build(k),
            ))
            .child(note(
                k,
                format!(
                    "Runs pi -p --no-session with tools off, in a private temp folder. Verified versions: {}.",
                    pi::VERIFIED.join(", ")
                ),
            )),
        Some(Kind::OpenCode) => page
            .child(labelled(
                k,
                "Model",
                TextInput::new(&s.opencode_model)
                    .label("Model")
                    .placeholder("opencode/beam-free")
                    .on_edit(|s: &mut SettingsUi| &mut s.opencode_model)
                    .build(k),
            ))
            .child(note(
                k,
                format!(
                    "Runs opencode run with its database in memory and tools off, in a private temp folder. \
                 Free models can have different privacy terms and limits. Verified versions: {}.",
                    opencode::VERIFIED.join(", ")
                ),
            )),
    };
    page = page.child(row().w_full().child(spacer()).child(
        Button::new("Save").variant(ButtonVariant::Contrast).on_press(|s: &mut SettingsUi| s.save_provider()).build(k),
    ));
    if let Some(result) = &s.provider_status {
        page = page.child(status(k, result));
    }
    page
}

/// Optional, for llama-server `--api-key`. The saved key is never shown again.
fn api_key_row(s: &SettingsUi, k: &Kit) -> El {
    let placeholder = if s.api_key_saved { "Saved in the Keychain" } else { "Optional, for llama-server --api-key" };
    let field = TextInput::new(&s.api_key)
        .secure()
        .label("API key")
        .placeholder(placeholder)
        .on_edit(|s: &mut SettingsUi| &mut s.api_key)
        .build(k);
    let mut block = col().w_full().gap(4.0).child(labelled(k, "API key", field));
    if s.api_key_saved {
        block = block.child(
            row().child(
                Button::new("Remove key")
                    .variant(ButtonVariant::Ghost)
                    .on_press(|s: &mut SettingsUi| s.remove_api_key())
                    .build(k),
            ),
        );
    }
    block
}

fn local_fields(s: &SettingsUi, k: &Kit, page: El) -> El {
    let presets = row()
        .gap(8.0)
        .child(
            Button::new("Ollama")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut SettingsUi| s.endpoint = rune::TextEdit::new(local::OLLAMA))
                .build(k),
        )
        .child(
            Button::new("LM Studio")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut SettingsUi| s.endpoint = rune::TextEdit::new(local::LM_STUDIO))
                .build(k),
        )
        .child(
            Button::new("llama.cpp")
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut SettingsUi| s.endpoint = rune::TextEdit::new(local::LLAMA_CPP))
                .build(k),
        )
        .child(spacer())
        .child(
            Button::new("List models")
                .variant(ButtonVariant::Outline)
                .on_press(|s: &mut SettingsUi| s.list_local_models())
                .build(k),
        );
    let mut page = page
        .child(labelled(
            k,
            "Address",
            TextInput::new(&s.endpoint)
                .label("Address")
                .placeholder(local::OLLAMA)
                .on_edit(|s: &mut SettingsUi| &mut s.endpoint)
                .build(k),
        ))
        .child(presets)
        .child(
            Switch::new(s.allow_network)
                .label("Allow LAN and Tailscale servers")
                .on_toggle(|s: &mut SettingsUi| s.allow_network = !s.allow_network)
                .build(k),
        )
        .child(api_key_row(s, k))
        .child(labelled(
            k,
            "Model",
            TextInput::new(&s.local_model)
                .label("Model")
                .placeholder("qwen2.5:7b")
                .on_edit(|s: &mut SettingsUi| &mut s.local_model)
                .build(k),
        ));
    match &s.listed_models {
        Some(Ok(models)) if models.is_empty() => {
            page = page.child(note(k, "The server has no models. Pull one first."))
        }
        Some(Ok(models)) => {
            let mut list = row().w_full().gap(6.0);
            for model in models.iter().take(8) {
                let name = model.clone();
                list = list.child(
                    Button::new(local::short_model_name(model))
                        .variant(ButtonVariant::GhostMuted)
                        .on_press(move |s: &mut SettingsUi| s.pick_local_model(&name))
                        .build(k),
                );
            }
            page = page.child(list);
        }
        Some(Err(error)) => page = page.child(status(k, &Err(error.clone()))),
        None => {}
    }
    let privacy = if s.allow_network {
        "Private addresses only, such as 192.168.1.20, 100.101.1.2 or box.tailnet.ts.net. The text goes over \
         your network: plain http is not encrypted, so use Tailscale or https on a shared network. The server \
         may log requests on its side."
    } else {
        "Only addresses on this Mac are accepted. The server may log requests on its side. Use a 3B model or \
         larger, such as qwen2.5:3b: smaller models often ignore the style."
    };
    page.child(note(k, privacy))
}

fn style_page(s: &SettingsUi, k: &Kit) -> El {
    let mut names: Vec<&str> = PRESETS.iter().map(|style| style.name()).collect();
    names.push("Custom");
    let styles = SegmentedControl::new(&names, Some(s.config.default_style.min(PRESETS.len())))
        .fit()
        .on_pick(|s: &mut SettingsUi, i, _: &mut Tx| s.set_default_style(i))
        .build(k);
    let themes: Vec<&str> = theme::ALL.iter().map(|t| t.name()).collect();
    let theme_index = theme::ALL.iter().position(|t| *t == s.config.theme);
    let theme_picker = SegmentedControl::new(&themes, theme_index)
        .fit()
        .on_pick(|s: &mut SettingsUi, i, _: &mut Tx| s.set_theme(theme::ALL[i]))
        .build(k);
    col()
        .w_full()
        .gap(10.0)
        .child(heading(k, "Default style"))
        .child(styles)
        .child(labelled(
            k,
            "Custom style",
            TextInput::new(&s.custom_style)
                .label("Custom style")
                .placeholder("Make it friendly and short.")
                .on_edit(|s: &mut SettingsUi| &mut s.custom_style)
                .on_submit(|s: &mut SettingsUi, _: &mut Tx| s.save_custom_style())
                .build(k),
        ))
        .child(
            row().w_full().child(spacer()).child(
                Button::new("Save custom style")
                    .variant(ButtonVariant::Outline)
                    .on_press(|s: &mut SettingsUi| s.save_custom_style())
                    .build(k),
            ),
        )
        .child(heading(k, "Theme"))
        .child(theme_picker)
}

fn privacy_page(s: &SettingsUi, k: &Kit) -> El {
    let mut page = col()
        .w_full()
        .gap(10.0)
        .child(heading(k, "Privacy"))
        .child(note(
            k,
            "Retone keeps no writing history. Selected and rewritten text stays in memory and is never saved.",
        ))
        .child(
            row().w_full().child(
                Button::new("Revoke all consent")
                    .variant(ButtonVariant::Outline)
                    .on_press(|s: &mut SettingsUi| s.revoke_consent())
                    .build(k),
            ),
        );
    if let Some(body) = &s.privacy_status {
        page = page.child(note(k, body.clone()));
    }
    for (kind, reason) in &s.disabled {
        let kind = *kind;
        page = page.child(text(reason.clone()).font(k.ty.text_small).color(k.c.warning.fg).wrap()).child(
            Button::new(format!("I understand, turn {} on again", kind.name()))
                .variant(ButtonVariant::Warning)
                .on_press(move |s: &mut SettingsUi| s.enable_adapter(kind))
                .build(k),
        );
    }
    page
}

fn permissions_page(k: &Kit) -> El {
    #[cfg(target_os = "macos")]
    let granted = crate::capture::has_permissions();
    #[cfg(not(target_os = "macos"))]
    let granted = false;
    let state = if granted {
        "Accessibility is on: Retone reads the selection and can replace it."
    } else {
        "Accessibility is off: clipboard-only mode. Copy the text with ⌘C, press your shortcut, and use Copy."
    };
    col()
        .w_full()
        .gap(10.0)
        .child(heading(k, "Permissions"))
        .child(note(k, state))
        .child(
            row().w_full().gap(8.0).child(
                Button::new("Open Accessibility Settings")
                    .variant(ButtonVariant::Outline)
                    .on_press(|_: &mut SettingsUi| {
                        // The requests add Retone to the Accessibility list (with the system prompt
                        // the first time); the pane is where the user turns it on.
                        #[cfg(target_os = "macos")]
                        {
                            crate::mac::ax::request_trust();
                            crate::mac::events::request_post_events();
                            crate::mac::permissions::open_accessibility_settings();
                        }
                    })
                    .build(k),
            ),
        )
        .child(note(
            k,
            "If an update reset access, turn Retone off and on in that list, or remove it and add it again.",
        ))
}

fn updates_page(s: &SettingsUi, k: &Kit) -> El {
    let page = col()
        .w_full()
        .gap(10.0)
        .child(heading(k, "Updates"))
        .child(note(k, format!("Retone {}.", env!("CARGO_PKG_VERSION"))))
        .child(note(k, s.update_channel.summary()));
    if s.update_channel != UpdateChannel::Sparkle {
        return page;
    }
    page.child(
        Button::new("Check Now")
            .variant(ButtonVariant::Outline)
            .on_press(|_: &mut SettingsUi| {
                #[cfg(target_os = "macos")]
                crate::mac::updater::check_now();
            })
            .build(k),
    )
}
