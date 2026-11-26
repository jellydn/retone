//! What the Settings window shows. Pure: reads state, builds elements.
//!
//! The layout follows Hex's settings: a sidebar of pages with the version and updates at its
//! foot, and on the right a page title over one card of rows, with an "Advanced" fold below.

use super::icons::*;
use super::rows::{card, help, page_title, setting, value_link};
use super::{HotkeyTest, PAGES, Page, SettingsUi, SubPage};
use crate::i18n::I18N;
use crate::provider::{KINDS, Kind, claude, codex, local, opencode, pi};
use crate::styles::PRESETS;
use crate::theme;
use crate::updates::UpdateChannel;
use rune::{Cx, El, Icon, ScrollSpec, Shortcut, ShortcutError, Tx, col, el, icon, row, spacer, text};
use rune_kit::{
    Button, ButtonVariant, Kit, Select, ShortcutRecorder, Sidebar, SidebarRow, Switch, TextInput, glyphs,
    shortcut_error_message,
};

const RELEASES: &str = "https://github.com/jellydn/retone/releases";

pub fn view(s: &SettingsUi, cx: &mut Cx<SettingsUi>) -> El {
    let k = s.config.theme.kit(cx.system_dark());
    let mut layer = Vec::new();
    let body = match (s.page, s.sub_page) {
        (_, Some(SubPage::Address)) => address_page(s, &k),
        (_, Some(SubPage::CustomStyle)) => custom_style_page(s, &k),
        (Page::General, None) => general_page(s, &k, &mut layer),
        (Page::Provider, None) => provider_page(s, &k, &mut layer),
        (Page::Styles, None) => styles_page(s, &k, &mut layer),
        (Page::Privacy, None) => privacy_page(s, &k),
    };
    let pane = col()
        .key("pane")
        .grow(1.0)
        .h_full()
        .min_w(0.0)
        .scroll(ScrollSpec { stick_bottom: false })
        .child(col().key("page").w_full().max_w(620.0).px(32.0).pt(24.0).pb(32.0).gap(16.0).child(body));
    let base = row().key("settings").w_full().h_full().fill(k.c.page).child(sidebar(s, &k)).child(pane);
    rune_kit::root(base, layer)
}

fn page_icon(page: Page) -> &'static Icon {
    match page {
        Page::General => &GENERAL,
        Page::Provider => &PROVIDER,
        Page::Styles => &STYLES,
        Page::Privacy => &PRIVACY,
    }
}

/// macOS draws the traffic lights over the sidebar (the window has a transparent, full-size
/// titlebar), so the sidebar's content starts below them, as in Hex.
const SIDEBAR_TOP: f32 = if cfg!(target_os = "macos") { 36.0 } else { 12.0 };

static APP_ICON: Icon =
    Icon { name: "retone-app-icon", view: 1024.0, body: include_str!("../../assets/icon/icon-16.svg") };

/// The app icon and name at the top of the sidebar.
fn brand(k: &Kit) -> El {
    row()
        .key("brand")
        .h(32.0)
        .px(6.0)
        .gap(8.0)
        .center_y()
        .child(icon(&APP_ICON, 22.0).a11y_hidden())
        .child(text("Retone").font(k.ty.body_medium).color(k.c.text_base))
}

fn sidebar(s: &SettingsUi, k: &Kit) -> El {
    let mut bar =
        Sidebar::new().label(t!("settings.sidebar_label")).row_gap(4.0).header(brand(k)).header_padding(0.0, 12.0);
    for (page, label) in PAGES {
        bar = bar.child(
            SidebarRow::new(I18N.text(label))
                .icon(page_icon(page))
                .selected(s.page == page)
                .on_select(move |s: &mut SettingsUi, _: &mut Tx| s.select_page(page))
                .build(k),
        );
    }
    bar = bar.footer(footer(s, k));
    // The area under the traffic lights stands in for the titlebar: it moves the window.
    col()
        .key("sidebar-pane")
        .w(210.0)
        .h_full()
        .fill(k.c.bg_deep)
        .border(1.0, k.c.border_muted)
        .child(el().key("titlebar-space").w_full().h(SIDEBAR_TOP).window_drag())
        // The kit sidebar is full height; this box gives it only what is left under the space.
        .child(col().key("sidebar-body").w_full().grow(1.0).min_h(0.0).child(bar.build(k)))
}

/// What's new, the version, and how this copy updates, as in Hex's sidebar footer.
fn footer(s: &SettingsUi, k: &Kit) -> El {
    let version = env!("CARGO_PKG_VERSION");
    let link = |key: &str, glyph: Option<&'static Icon>, words: String, color, url: Option<String>| {
        let mut line = row()
            .key(key.to_string())
            .h(26.0)
            .px(6.0)
            .gap(8.0)
            .center_y()
            .radius(k.radius.sm)
            .children(glyph.map(|g| icon(g, 14.0).color(k.c.icon_muted).a11y_hidden()))
            .child(text(words).font(k.ty.text_small).color(color).truncate());
        if let Some(url) = url {
            line = line.role("link").focusable(true).hover(k.c.hover).press(k.c.pressed).on_press(
                move |_: &mut SettingsUi| {
                    #[cfg(target_os = "macos")]
                    crate::mac::permissions::open_url(&url);
                    #[cfg(not(target_os = "macos"))]
                    let _ = &url;
                },
            );
        }
        line
    };
    let whats_new = link(
        "whats-new",
        Some(&BOOK),
        t!("settings.footer.whats_new").into(),
        k.c.text_muted,
        Some(format!("{RELEASES}/tag/v{version}")),
    );
    let version_line = link("version", None, t!("settings.footer.version", version = version), k.c.text_faint, None);
    let updates = match s.update_channel {
        UpdateChannel::Sparkle => row()
            .key("check-updates")
            .h(26.0)
            .px(6.0)
            .center_y()
            .radius(k.radius.sm)
            .role("button")
            .focusable(true)
            .hover(k.c.hover)
            .press(k.c.pressed)
            .on_press(|_: &mut SettingsUi| {
                #[cfg(target_os = "macos")]
                crate::mac::updater::check_now();
            })
            .child(text(t!("settings.footer.check_updates")).font(k.ty.text_small).color(k.c.text_muted)),
        UpdateChannel::Homebrew => link("updates", None, t!("settings.footer.homebrew").into(), k.c.text_faint, None),
        UpdateChannel::Manual => {
            link("updates", None, t!("settings.footer.github").into(), k.c.text_muted, Some(RELEASES.into()))
        }
    };
    col().key("footer").w_full().gap(2.0).child(whats_new).child(version_line).child(updates)
}

/// A dropdown (⌄) for a row. `choose` gets the picked option's index.
fn dropdown(
    s: &SettingsUi,
    k: &Kit,
    layer: &mut Vec<El>,
    key: &'static str,
    options: &[&str],
    current: Option<usize>,
    choose: impl Fn(&mut SettingsUi, usize) + Clone + 'static,
) -> El {
    let names: Vec<String> = options.iter().map(|o| o.to_string()).collect();
    Select::new(key, options)
        .label(key)
        .width(220.0)
        .current(current.map(|i| options[i]))
        .open(s.open_select == Some(key))
        .on_toggle(move |s: &mut SettingsUi| s.toggle_select(key))
        .on_close(|s: &mut SettingsUi| s.open_select = None)
        .on_choose(move |s: &mut SettingsUi, picked: &str, _: &mut Tx| {
            s.open_select = None;
            if let Some(index) = names.iter().position(|name| name == picked) {
                choose(s, index);
            }
        })
        .build(k, layer)
}

/// The fold under a page's card, for settings most people never change. Hex's "Advanced ▸":
/// the kit's ghost Collapsible shows its caret only on hover.
fn advanced(s: &SettingsUi, k: &Kit, rows: Vec<El>) -> El {
    let open = s.advanced_open;
    let header = row()
        .key("advanced-toggle")
        .self_start()
        .role("button")
        .label(t!("settings.advanced"))
        .expanded(open)
        .focusable(true)
        .focus_ring(2.0, 0.0, k.c.border_focus)
        .h(28.0)
        .px(6.0)
        .ml(-6.0)
        .gap(4.0)
        .center_y()
        .radius(k.radius.sm)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press(|s: &mut SettingsUi| s.advanced_open = !s.advanced_open)
        .child(text(t!("settings.advanced")).font(k.ty.ui_medium).color(k.c.text_muted).a11y_hidden())
        .child(
            icon(if open { &glyphs::TRIANGLE_DOWN } else { &glyphs::TRIANGLE_RIGHT }, 14.0)
                .color(k.c.icon_muted)
                .a11y_hidden(),
        );
    col().key("advanced").w_full().gap(8.0).child(header).children(open.then(|| card(k, "advanced-card", rows)))
}

/// A row's switch. On a light card the kit's off track (`layer_04`) is nearly the card's white,
/// so light themes draw it with the border colour instead.
fn row_switch(k: &Kit, on: bool, flip: impl Fn(&mut SettingsUi) + 'static) -> El {
    let switch = Switch::new(on).on_toggle(flip);
    if k.dark {
        return switch.build(k);
    }
    let mut light = k.clone();
    light.c.layer_04 = k.c.border_strong;
    switch.build(&light)
}

fn status_line(k: &Kit, result: &Result<String, String>) -> El {
    let (body, color) = match result {
        Ok(body) => (body.clone(), k.c.success.fg),
        Err(body) => (body.clone(), k.c.danger.fg),
    };
    text(body).font(k.ty.text_small).color(color).wrap()
}

pub fn style_names() -> Vec<&'static str> {
    let mut names: Vec<&str> = PRESETS.iter().map(|style| style.name()).collect();
    names.push(t!("style.custom"));
    names
}

fn default_style_row(s: &SettingsUi, k: &Kit, layer: &mut Vec<El>) -> El {
    let current = Some(s.config.default_style.min(PRESETS.len()));
    let control = dropdown(s, k, layer, "default-style", &style_names(), current, |s, i| s.set_default_style(i));
    setting(
        k,
        "default-style",
        &glyphs::SPARKLE,
        t!("settings.default_style"),
        help(k, t!("settings.default_style_help")),
        control,
    )
}

/// The error under a recorder: who holds a conflicting chord, or why Retone refused it.
fn recorder_message(s: &SettingsUi) -> impl Fn(&ShortcutError, Option<&Shortcut>) -> String + 'static {
    let owner = s.conflict_owner.clone();
    move |error: &ShortcutError, chord: Option<&Shortcut>| match (error, chord, &owner) {
        (ShortcutError::Conflict, Some(chord), Some(owner)) => {
            t!("settings.shortcut.used_by", keys = chord.keycaps().join(" "), owner = owner)
        }
        (ShortcutError::Conflict, _, None) => t!("settings.shortcut.conflict").into(),
        (ShortcutError::InvalidAccelerator(reason), _, _) => reason.clone(),
        _ => shortcut_error_message(error, chord),
    }
}

fn general_page(s: &SettingsUi, k: &Kit, layer: &mut Vec<El>) -> El {
    let recorder = ShortcutRecorder::new("shortcut", &s.recorder, |s: &mut SettingsUi| &mut s.recorder)
        .label(t!("settings.shortcut.main_name"))
        .width(200.0)
        .on_change(|s: &mut SettingsUi, next, _: &mut Tx| s.change_hotkey(next))
        .error_message(recorder_message(s))
        .build(k);
    let shortcut_help = match s.recorder.shortcut() {
        None => t!("settings.shortcut.help_unset"),
        chord if uses_voiceover_keys(chord) => t!("settings.shortcut.help_voiceover"),
        Some(_) => t!("settings.shortcut.help_set"),
    };
    let test_help = match s.hotkey_test {
        HotkeyTest::Idle => help(k, t!("settings.test.idle")),
        HotkeyTest::Waiting => help(k, t!("settings.test.waiting")),
        HotkeyTest::Passed => Some((t!("settings.test.passed").into(), k.c.success.fg)),
    };
    let test = Button::new(t!("settings.test.button"))
        .variant(ButtonVariant::Outline)
        .disabled(s.recorder.shortcut().is_none())
        .on_press(|s: &mut SettingsUi| s.start_hotkey_test())
        .build(k);
    let themes: Vec<&str> = theme::ALL.iter().map(|t| t.name()).collect();
    let theme_index = theme::ALL.iter().position(|t| *t == s.config.theme);
    let theme_picker = dropdown(s, k, layer, "theme", &themes, theme_index, |s, i| s.set_theme(theme::ALL[i]));

    #[cfg(target_os = "macos")]
    let granted = crate::capture::has_permissions();
    #[cfg(not(target_os = "macos"))]
    let granted = false;
    let (access_help, access_control) = if granted {
        (
            help(k, t!("settings.access.on_help")),
            text(t!("settings.access.on")).font(k.ty.ui_medium).color(k.c.success.fg),
        )
    } else {
        (
            Some((t!("settings.access.off_help").into(), k.c.warning.fg)),
            Button::new(t!("settings.access.open_settings"))
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
        )
    };
    let login_help = match &s.login_error {
        Some(error) => Some((error.clone(), k.c.danger.fg)),
        None => help(k, t!("settings.login_help")),
    };
    let login = row_switch(k, s.launch_at_login, |s: &mut SettingsUi| s.set_launch_at_login(!s.launch_at_login));
    col()
        .w_full()
        .gap(16.0)
        .child(page_title(k, t!("settings.page.general")))
        .child(card(
            k,
            "general-card",
            vec![
                setting(
                    k,
                    "shortcut",
                    &glyphs::KEYBOARD,
                    t!("settings.shortcut.title"),
                    help(k, shortcut_help),
                    recorder,
                ),
                setting(k, "test", &PLAY, t!("settings.test.title"), test_help, test),
                default_style_row(s, k, layer),
                setting(k, "theme", &THEME, t!("settings.theme"), None, theme_picker),
                setting(k, "accessibility", &ACCESSIBILITY, t!("settings.access.title"), access_help, access_control),
                setting(k, "login", &POWER, t!("settings.login"), login_help, login),
            ],
        ))
        .child(advanced(
            s,
            k,
            vec![
                setting(k, "updates", &BOOK, t!("settings.updates"), help(k, s.update_channel.summary()), el()),
                setting(
                    k,
                    "after-update",
                    &RESET,
                    t!("settings.after_update"),
                    help(k, t!("settings.after_update_help")),
                    el(),
                ),
                setting(
                    k,
                    "diagnostics",
                    &NOTE,
                    t!("settings.diagnostics"),
                    help(k, t!("settings.diagnostics_help")),
                    el(),
                ),
            ],
        ))
}

fn model_field(
    k: &Kit,
    edit: &rune::TextEdit,
    placeholder: &str,
    field: fn(&mut SettingsUi) -> &mut rune::TextEdit,
) -> El {
    TextInput::new(edit)
        .label(t!("settings.provider.model"))
        .placeholder(placeholder)
        .width(Some(180.0))
        .on_edit(field)
        .build(k)
}

/// The model row of a CLI provider: how Retone runs it and which versions passed the history check.
fn cli_row(s: &SettingsUi, k: &Kit, kind: Kind, title: &str) -> El {
    type Field = fn(&mut SettingsUi) -> &mut rune::TextEdit;
    let (how, verified, placeholder, edit, field): (&str, &[&str], &str, &rune::TextEdit, Field) = match kind {
        Kind::Pi => {
            (t!("settings.provider.pi_how"), pi::VERIFIED, "ollama/qwen2.5:7b", &s.pi_model, |s| &mut s.pi_model)
        }
        Kind::OpenCode => {
            (t!("settings.provider.opencode_how"), opencode::VERIFIED, "opencode/beam-free", &s.opencode_model, |s| {
                &mut s.opencode_model
            })
        }
        Kind::ClaudeCode => {
            (t!("settings.provider.claude_how"), claude::VERIFIED, "claude-haiku-5-5", &s.claude_model, |s| {
                &mut s.claude_model
            })
        }
        // Local has no CLI row; the caller never passes it.
        Kind::Codex | Kind::Local => {
            (t!("settings.provider.codex_how"), codex::VERIFIED, "gpt-6-luna", &s.codex_model, |s| &mut s.codex_model)
        }
    };
    let checked = if verified.is_empty() {
        t!("settings.provider.not_verified").to_string()
    } else {
        t!("settings.provider.verified", verified = verified.join(", "))
    };
    let input = model_field(k, edit, placeholder, field);
    setting(k, &format!("{}-model", kind.key()), &TERMINAL, title, help(k, format!("{how} {checked}")), input)
}

/// The optional program path of the chosen CLI, for a CLI that the login shell cannot find.
fn program_row(s: &SettingsUi, k: &Kit, kind: Kind) -> El {
    let input = TextInput::new(&s.cli_program)
        .label(t!("settings.provider.program"))
        .placeholder(t!("settings.provider.program_placeholder"))
        .width(Some(240.0))
        .on_edit(|s: &mut SettingsUi| &mut s.cli_program)
        .build(k);
    let body = t!("settings.provider.program_help", name = kind.name(), program = kind.key());
    setting(k, "cli-program", &TERMINAL, t!("settings.provider.program"), help(k, body), input)
}

fn provider_page(s: &SettingsUi, k: &Kit, layer: &mut Vec<El>) -> El {
    let choices: Vec<Option<Kind>> = std::iter::once(None).chain(KINDS.map(Some)).collect();
    let names: Vec<&str> = std::iter::once(t!("settings.provider.none")).chain(KINDS.map(Kind::name)).collect();
    let current = choices.iter().position(|c| *c == s.provider_choice);
    let picker = dropdown(s, k, layer, "provider", &names, current, move |s, i| s.choose_provider(choices[i]));
    let provider_help = match s.provider_choice {
        None => t!("settings.provider.help_none"),
        Some(Kind::Local) => t!("settings.provider.help_local"),
        Some(_) => t!("settings.provider.help_cli"),
    };
    let mut rows =
        vec![setting(k, "provider", &PROVIDER, t!("settings.page.provider"), help(k, provider_help), picker)];
    match s.provider_choice {
        None => {}
        Some(Kind::Local) => rows.extend(local_rows(s, k)),
        Some(kind) => {
            rows.push(cli_row(s, k, kind, t!("settings.provider.model")));
            rows.push(program_row(s, k, kind));
        }
    }
    // The other CLIs' models, under Advanced.
    let extra = KINDS
        .into_iter()
        .filter(|kind| *kind != Kind::Local && Some(*kind) != s.provider_choice)
        .map(|kind| cli_row(s, k, kind, &t!("settings.provider.cli_model", name = kind.name())))
        .collect();
    let save = Button::new(t!("settings.save"))
        .variant(ButtonVariant::Contrast)
        .on_press(|s: &mut SettingsUi| s.save_provider())
        .build(k);
    // The status takes the space left of Save, so a long one wraps there.
    let status = match &s.provider_status {
        Some(result) => col().grow(1.0).shrink().min_w(0.0).child(status_line(k, result)),
        None => spacer(),
    };
    let actions = row().key("actions").w_full().gap(12.0).center_y().child(status);
    col()
        .w_full()
        .gap(16.0)
        .child(page_title(k, t!("settings.page.provider")))
        .child(card(k, "provider-card", rows))
        .child(actions.child(save))
        .child(advanced(s, k, extra))
}

fn local_rows(s: &SettingsUi, k: &Kit) -> Vec<El> {
    let address = value_link(k, "address-link", s.endpoint.text.trim(), |s: &mut SettingsUi| {
        s.sub_page = Some(SubPage::Address);
    });
    let network_help = if s.allow_network { t!("settings.network.on_help") } else { t!("settings.network.off_help") };
    let network = row_switch(k, s.allow_network, |s: &mut SettingsUi| s.allow_network = !s.allow_network);
    let key_placeholder = if s.api_key_saved { t!("settings.api_key.saved") } else { t!("settings.api_key.optional") };
    let mut key_control = row().gap(6.0).center_y().child(
        TextInput::new(&s.api_key)
            .secure()
            .label(t!("settings.api_key.title"))
            .placeholder(key_placeholder)
            .width(Some(200.0))
            .on_edit(|s: &mut SettingsUi| &mut s.api_key)
            .build(k),
    );
    if s.api_key_saved {
        key_control = key_control.child(
            Button::new(t!("settings.api_key.remove"))
                .variant(ButtonVariant::Ghost)
                .on_press(|s: &mut SettingsUi| s.remove_api_key())
                .build(k),
        );
    }
    let model_control =
        row().gap(6.0).center_y().child(model_field(k, &s.local_model, "qwen2.5:7b", |s| &mut s.local_model)).child(
            Button::new(t!("settings.provider.list"))
                .variant(ButtonVariant::Outline)
                .on_press(|s: &mut SettingsUi| s.list_local_models())
                .build(k),
        );
    let mut rows = vec![
        setting(k, "address", &GLOBE, t!("settings.address.title"), None, address),
        setting(k, "network", &NETWORK, t!("settings.network.title"), help(k, network_help), network),
        setting(k, "api-key", &KEY, t!("settings.api_key.title"), help(k, t!("settings.api_key.help")), key_control),
        setting(
            k,
            "model",
            &MODEL,
            t!("settings.provider.model"),
            help(k, t!("settings.provider.model_help")),
            model_control,
        ),
    ];
    match &s.listed_models {
        Some(Ok(models)) if models.is_empty() => rows.push(setting(
            k,
            "models",
            &LAYERS,
            t!("settings.models.title"),
            help(k, t!("settings.models.empty")),
            el(),
        )),
        Some(Ok(models)) => {
            let mut list = row().gap(6.0).flex_wrap().max_w(320.0);
            for model in models.iter().take(8) {
                let name = model.clone();
                list = list.child(
                    Button::new(local::short_model_name(model))
                        .variant(ButtonVariant::GhostMuted)
                        .on_press(move |s: &mut SettingsUi| s.pick_local_model(&name))
                        .build(k),
                );
            }
            rows.push(setting(
                k,
                "models",
                &LAYERS,
                t!("settings.models.title"),
                help(k, t!("settings.models.pick")),
                list,
            ));
        }
        Some(Err(error)) => rows.push(setting(
            k,
            "models",
            &LAYERS,
            t!("settings.models.title"),
            Some((error.clone(), k.c.danger.fg)),
            el(),
        )),
        None => {}
    }
    rows
}

/// The header of a sub-page: a way back to its page, then its title.
fn sub_page_header(k: &Kit, back: &str, title: &str) -> El {
    col()
        .w_full()
        .gap(4.0)
        .child(
            row()
                .key("back")
                .self_start()
                .role("button")
                .label(t!("settings.back_to", page = back))
                .focusable(true)
                .h(24.0)
                .pr(6.0)
                .gap(2.0)
                .center_y()
                .radius(k.radius.sm)
                .hover(k.c.hover)
                .press(k.c.pressed)
                .on_press(|s: &mut SettingsUi| s.sub_page = None)
                .child(icon(&CHEVRON_LEFT, 16.0).color(k.c.icon_muted).a11y_hidden())
                .child(text(back.to_string()).font(k.ty.text_small).color(k.c.text_muted)),
        )
        .child(page_title(k, title))
}

fn address_page(s: &SettingsUi, k: &Kit) -> El {
    let field = TextInput::new(&s.endpoint)
        .label(t!("settings.address.title"))
        .placeholder(local::OLLAMA)
        .width(Some(260.0))
        .on_edit(|s: &mut SettingsUi| &mut s.endpoint)
        .build(k);
    let preset = |name: &str, url: &'static str| {
        Button::new(name)
            .variant(ButtonVariant::Ghost)
            .on_press(move |s: &mut SettingsUi| s.endpoint = rune::TextEdit::new(url))
            .build(k)
    };
    let presets = row()
        .gap(6.0)
        .child(preset("Ollama", local::OLLAMA))
        .child(preset("LM Studio", local::LM_STUDIO))
        .child(preset("llama.cpp", local::LLAMA_CPP))
        .child(preset("Llama.app", local::LLAMA_APP));
    let address_help =
        if s.allow_network { t!("settings.address.help_network") } else { t!("settings.address.help_local") };
    col()
        .w_full()
        .gap(16.0)
        .child(sub_page_header(k, t!("settings.page.provider"), t!("settings.address.title")))
        .child(card(
            k,
            "address-card",
            vec![
                setting(k, "address", &GLOBE, t!("settings.address.server"), help(k, address_help), field),
                setting(
                    k,
                    "presets",
                    &PROVIDER,
                    t!("settings.address.presets"),
                    help(k, t!("settings.address.presets_help")),
                    presets,
                ),
            ],
        ))
        .child(text(t!("settings.address.save_note")).font(k.ty.text_small).color(k.c.text_muted))
}

fn styles_page(s: &SettingsUi, k: &Kit, layer: &mut Vec<El>) -> El {
    let custom = s.config.custom_style.trim();
    let custom_value = if custom.is_empty() { t!("settings.styles.not_set") } else { custom };
    let custom_link = value_link(k, "custom-link", custom_value, |s: &mut SettingsUi| {
        s.sub_page = Some(SubPage::CustomStyle);
    });
    col()
        .w_full()
        .gap(16.0)
        .child(page_title(k, t!("settings.page.styles")))
        .child(card(
            k,
            "styles-card",
            vec![
                default_style_row(s, k, layer),
                setting(
                    k,
                    "custom",
                    &STYLES,
                    t!("settings.styles.custom"),
                    help(k, t!("settings.styles.custom_help")),
                    custom_link,
                ),
                setting(
                    k,
                    "built-in",
                    &LAYERS,
                    t!("settings.styles.built_in"),
                    help(k, t!("settings.styles.built_in_help")),
                    el(),
                ),
            ],
        ))
        .child(card(k, "style-shortcuts-card", style_shortcut_rows(s, k)))
}

/// ⌃⌥ is the VoiceOver key: with VoiceOver on, such a chord reaches Retone only after VO-Tab.
fn uses_voiceover_keys(chord: Option<&Shortcut>) -> bool {
    chord.is_some_and(|chord| chord.modifiers().ctrl && chord.modifiers().alt)
}

/// One optional shortcut per style; it opens the popup in that style. Custom needs its
/// instruction first.
fn style_shortcut_rows(s: &SettingsUi, k: &Kit) -> Vec<El> {
    let custom_set = !s.config.custom_style.trim().is_empty();
    style_names()
        .into_iter()
        .enumerate()
        .map(|(index, name)| {
            let disabled = index == PRESETS.len() && !custom_set;
            let recorder = ShortcutRecorder::new(
                format!("style-shortcut-{index}"),
                &s.style_recorders[index],
                move |s: &mut SettingsUi| &mut s.style_recorders[index],
            )
            .label(t!("settings.styles.shortcut_label", name = name))
            .width(200.0)
            .disabled(disabled)
            .on_change(move |s: &mut SettingsUi, next, _: &mut Tx| s.change_style_hotkey(index, next))
            .error_message(recorder_message(s))
            .build(k);
            let note = if disabled {
                t!("settings.styles.shortcut_disabled")
            } else if uses_voiceover_keys(s.style_recorders[index].shortcut()) {
                t!("settings.styles.shortcut_help_voiceover")
            } else {
                t!("settings.styles.shortcut_help")
            };
            setting(k, &format!("style-shortcut-row-{index}"), &glyphs::KEYBOARD, name, help(k, note), recorder)
        })
        .collect()
}

fn custom_style_page(s: &SettingsUi, k: &Kit) -> El {
    let field = TextInput::new(&s.custom_style)
        .label(t!("settings.styles.custom"))
        .placeholder(t!("settings.styles.custom_placeholder"))
        .width(Some(320.0))
        .on_edit(|s: &mut SettingsUi| &mut s.custom_style)
        .on_submit(|s: &mut SettingsUi, _: &mut Tx| s.save_custom_style())
        .build(k);
    col()
        .w_full()
        .gap(16.0)
        .child(sub_page_header(k, t!("settings.page.styles"), t!("settings.styles.custom")))
        .child(card(
            k,
            "custom-card",
            vec![setting(
                k,
                "custom",
                &STYLES,
                t!("settings.styles.instruction"),
                help(k, t!("settings.styles.instruction_help")),
                field,
            )],
        ))
        .child(
            row().w_full().child(spacer()).child(
                Button::new(t!("settings.save"))
                    .variant(ButtonVariant::Contrast)
                    .on_press(|s: &mut SettingsUi| s.save_custom_style())
                    .build(k),
            ),
        )
}

fn privacy_page(s: &SettingsUi, k: &Kit) -> El {
    let consent_help = s.privacy_status.clone().unwrap_or_else(|| t!("settings.privacy.consent_help").into());
    let revoke = Button::new(t!("settings.privacy.revoke_all"))
        .variant(ButtonVariant::Outline)
        .on_press(|s: &mut SettingsUi| s.revoke_consent())
        .build(k);
    let mut rows = vec![
        setting(
            k,
            "history",
            &glyphs::EYE_OFF,
            t!("settings.privacy.history"),
            help(k, t!("settings.privacy.history_help")),
            el(),
        ),
        setting(k, "consent", &RESET, t!("settings.privacy.consent"), help(k, consent_help), revoke),
    ];
    for (kind, reason) in &s.disabled {
        let kind = *kind;
        let turn_on = Button::new(t!("settings.privacy.turn_on"))
            .variant(ButtonVariant::Warning)
            .on_press(move |s: &mut SettingsUi| s.enable_adapter(kind))
            .build(k);
        rows.push(setting(
            k,
            &format!("disabled-{}", kind.name()),
            &WARNING,
            &t!("settings.privacy.adapter_off", provider = kind.name()),
            Some((reason.clone(), k.c.warning.fg)),
            turn_on,
        ));
    }
    col().w_full().gap(16.0).child(page_title(k, t!("settings.page.privacy"))).child(card(k, "privacy-card", rows))
}
