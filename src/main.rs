//! Retone: select text in any app, press your hotkey, and get a rewrite in a popup at the caret.
//!
//! An accessory app with two windows: the popup (a nonactivating launcher panel, so the target
//! app stays active and keeps its selection) and Settings. Headless renders (`--snap`,
//! `--scene`) build both windows with in-memory prefs and start no native work.

#[cfg(target_os = "macos")]
mod apply;
#[cfg(target_os = "macos")]
mod capture;
mod config;
mod eval;
mod flow;
mod input;
#[cfg(target_os = "macos")]
mod mac;
#[cfg(target_os = "macos")]
mod native;
mod placement;
mod popup;
mod provider;
mod settings;
mod shared;
mod spike_log;
mod styles;
mod theme;
mod updates;

use popup::Popup;
use rune::{App, AppSpec, WindowOptions, WindowSpec};
use settings::SettingsUi;
use shared::Shared;
use std::sync::Arc;

fn main() {
    if let Some(code) = eval::run_from_args() {
        std::process::exit(code);
    }
    let headless = rune::is_headless();
    let shared = Arc::new(Shared::new(!headless));

    let popup_spec = AppSpec::<Popup> {
        scenes: popup::scenes::SCENES,
        setup: Some(Popup::set_waker),
        poll: Some(|p, _, tx| p.poll(tx)),
        on_key: Some(popup::on_key),
        ..AppSpec::new("Retone", (popup::SIZE.0 as f32, popup::SIZE.1 as f32), Popup::default, popup::view::view)
    };
    let mut app = App::new(popup_spec)
        .accessory()
        .windowless()
        .keep_running()
        .configured_options(WindowOptions::launcher_panel())
        // The panel loses key focus when the user clicks elsewhere: they moved on, so cancel.
        // Nothing changes on cancel (an in-flight Replace still finishes, see `flow.rs`).
        .on_focus(|p, focused, tx| {
            if !focused {
                p.step(flow::Event::Cancel, tx);
            }
        });

    let settings_spec = AppSpec::<SettingsUi> {
        scenes: settings::SCENES,
        setup: Some(SettingsUi::set_waker),
        poll: Some(|s, _, _| s.poll()),
        ..AppSpec::new("Retone Settings", (620.0, 520.0), SettingsUi::default, settings::view::view)
    };
    let hidden = WindowOptions { visible: false, ..Default::default() };
    let (settings, _settings_window) = app.window(WindowSpec::new("settings", settings_spec).options(hidden));

    // Headless renders keep each window's own state, so `--scene` shows exactly that scene;
    // the links, the hotkey and native work exist only in a real run.
    if !headless {
        let popup = app.handle();
        let (me, links_settings) = (popup.clone(), settings.clone());
        let links_shared = shared.clone();
        popup.update(move |p, _| p.attach(popup::Links::new(links_shared, me, links_settings)));
        let settings_popup = popup.clone();
        settings.update(move |ui, _| {
            *ui = SettingsUi::new(shared, Some(settings_popup));
            ui.register_saved();
        });
        #[cfg(target_os = "macos")]
        start_native(&popup, &settings);
    }
    app.run();
}

/// The worker that owns AX and the pasteboard, and the menu-bar item. Real runs only.
#[cfg(target_os = "macos")]
fn start_native(popup: &rune::AppHandle<Popup>, settings: &rune::AppHandle<SettingsUi>) {
    let replies = popup.clone();
    match native::Native::start(move |reply| replies.update(move |p, tx| p.on_reply(reply, tx))) {
        Ok(worker) => popup.update(move |p, _| p.set_native(worker)),
        Err(error) => eprintln!("retone: cannot start the native worker: {error}"),
    }
    let (popup, settings) = (popup.clone(), settings.clone());
    // AppKit objects must be made on the main thread; updates run there.
    popup.clone().update(move |_, _| {
        let channel = if mac::updater::installed_by_homebrew() {
            updates::UpdateChannel::Homebrew
        } else if mac::updater::start() {
            updates::UpdateChannel::Sparkle
        } else {
            updates::UpdateChannel::Manual
        };
        settings.update(move |ui, _| ui.update_channel = channel);
        mac::status::install(popup, settings, channel);
    });
}
