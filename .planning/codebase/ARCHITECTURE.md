# Architecture

**Analysis Date:** 2026-11-24

## Pattern Overview

**Overall:** Single-process macOS accessory app (`LSUIElement`, see `scripts/bundle.sh`) built on Rune. A pure state machine decides, and an effect runner carries out the decisions. Native work runs on one dedicated worker thread (`doc/adr/0001-rune-menu-bar-app.md`, `doc/adr/0002-native-worker-and-unsafe-boundary.md`).

**Key Characteristics:**
- **Pure core, impure shell:** `Flow::step` in `src/flow.rs` only changes state and returns `Effect`s. It touches no app, clipboard or network (`#![forbid(unsafe_code)]`, module doc in `src/flow.rs`). `Popup::run` in `src/popup/mod.rs` carries out each effect.
- **Attempt fencing:** each async answer (`Event::Captured`, `Partial`, `Answer`, `Applied`) carries the `rune::machine::Attempt` that started it. A late answer after Esc, Retry or a style change matches no arm and changes nothing (`src/flow.rs`, catch-all arm `(state, _) => (state, Effects::none())`).
- **One native thread:** the `retone-native` thread in `src/native.rs` owns the `NSPasteboard` (`src/mac/pasteboard.rs`) and the captured AX element (`src/mac/ax.rs`). Neither is `Send`. The app sends `Command`s and gets `Reply`s through a callback.
- **One `unsafe` boundary:** `Cargo.toml` sets `unsafe_code = "deny"`. Only the leaf modules in `src/mac/mod.rs` with `#[allow(unsafe_code)]` opt in. Pure modules add `#![forbid(unsafe_code)]` (`src/flow.rs`, `src/strategy.rs`, `src/styles.rs`, `src/provider/mod.rs`, `src/config.rs`, `src/input.rs`, `src/shared.rs`).
- **Fail closed:** on failure or cancel there is no `Effect::Apply`, so nothing changes in the target app (`src/flow.rs`, `src/apply.rs`, `doc/adr/0003-capture-and-safe-replace.md`).
- **No history:** no selected or rewritten text goes to disk, prefs or logs (`AGENTS.md` rules, `src/config.rs` module doc, `src/spike_log.rs`, `src/provider/cli.rs`).

## Layers

**Entry and wiring:**
- Purpose: Parse dev-tool flags, build the two Rune windows, link them, and start native services in real runs only.
- Location: `src/main.rs`
- Contains: `main`, `start_native`; the module list with `#[cfg(target_os = "macos")]` gates for `apply`, `capture`, `mac`, `native`.
- Depends on: `src/popup/mod.rs`, `src/settings/mod.rs`, `src/shared.rs`, `src/native.rs`, `src/mac/services.rs`, `src/mac/status.rs`, `src/mac/updater.rs`, `src/eval.rs`, `src/history_check.rs`, `src/i18n.rs`.
- Used by: the OS (binary entry).

**Pure domain logic:**
- Purpose: Decisions with no AppKit: the rewrite flow, input limits, write strategy, popup placement, prompts, config, word diff.
- Location: `src/flow.rs`, `src/input.rs`, `src/strategy.rs`, `src/placement.rs`, `src/styles.rs`, `src/config.rs`, `src/diff.rs`, `src/theme.rs`, `src/updates.rs`
- Contains: enums (`State`, `Event`, `Effect`, `Phase`, `Notice`, `Style`, `Strategy`, `Refusal`, `UpdateChannel`), pure functions (`input::check`, `strategy::for_app`, `placement::place`, `styles::build`, `styles::clean_answer`).
- Depends on: `rune::machine` (`src/flow.rs`), `rune_core::prefs` (`src/config.rs`), `t!` from `src/i18n.rs`.
- Used by: `src/popup/mod.rs`, `src/settings/mod.rs`, `src/capture.rs`, `src/provider/*`, `src/eval.rs`.

**UI (Rune windows):**
- Purpose: The popup at the caret and the Settings window. Views are pure functions of state.
- Location: `src/popup/` (`mod.rs`, `view.rs`, `scenes.rs`), `src/settings/` (`mod.rs`, `view.rs`, `rows.rs`, `icons.rs`)
- Contains: `Popup` (flow owner and effect runner), `popup::on_key`, `SettingsUi` (hotkey registration, provider setup, privacy), headless `SCENES`.
- Depends on: pure layer, `src/provider/`, `src/shared.rs`, `src/native.rs` (macOS), `rune`, `rune_kit`.
- Used by: `src/main.rs`; native callbacks in `src/mac/services.rs` and `src/mac/status.rs`.

**Native worker and capture/apply:**
- Purpose: Read the selection and write the rewrite, off the frame loop.
- Location: `src/native.rs`, `src/capture.rs`, `src/apply.rs`
- Contains: `Native::start`, `Command { Capture, Replace, Forget }`, `Reply { Captured, Applied }`, `capture::capture`, `service_capture`, `apply::replace`.
- Depends on: `src/mac/ax.rs`, `src/mac/events.rs`, `src/mac/pasteboard.rs`, `src/mac/screens.rs`, `src/strategy.rs`, `src/flow.rs` (`Captured`).
- Used by: `src/popup/mod.rs` (`capture`, `apply`, `close` send commands; `on_reply` receives replies).

**macOS FFI leaves:**
- Purpose: The only code with `unsafe`; each block has a `// SAFETY:` comment behind a safe function.
- Location: `src/mac/` (`ax.rs`, `events.rs`, `pasteboard.rs`, `screens.rs`, `services.rs`, `status.rs`, `updater.rs`, `login.rs`, `keychain.rs`, `permissions.rs`)
- Contains: AX target queries and re-check (`Target::focused`, `still_matches`, `set_selected_text` in `src/mac/ax.rs`), CGEvent ⌘C/⌘V posting (`src/mac/events.rs`), pasteboard snapshot/restore (`src/mac/pasteboard.rs`), the `NSStatusItem` menu (`src/mac/status.rs`), the Services provider (`src/mac/services.rs`), Sparkle (`src/mac/updater.rs`), `SMAppService` login item (`src/mac/login.rs`), Keychain via `security-framework` (`src/mac/keychain.rs`).
- Depends on: `objc2*` crates (`Cargo.toml`).
- Used by: `src/native.rs`, `src/capture.rs`, `src/apply.rs`, `src/popup/mod.rs`, `src/main.rs`, `src/shared.rs`, `src/settings/mod.rs`.

**Providers:**
- Purpose: Turn (style, text) into a rewrite, with no history.
- Location: `src/provider/` (`mod.rs`, `local.rs`, `cli.rs`, `pi.rs`, `opencode.rs`, `claude.rs`, `codex.rs`, `consent.rs`)
- Contains: `Kind`, `Provider`, `Env`, `Outcome`, `Cancel`, `answer_cap`, `redact` (`src/provider/mod.rs`); HTTP/SSE client with a private-address resolver (`src/provider/local.rs`); the shared CLI runner (`src/provider/cli.rs`); consent and adapter-disable records in prefs (`src/provider/consent.rs`).
- Depends on: `src/styles.rs` (prompts), `ureq`, `rune_core::prefs`.
- Used by: `src/popup/mod.rs` (`request`), `src/settings/mod.rs`, `src/eval.rs`, `src/history_check.rs`.

**Shared state and i18n:**
- Purpose: State both windows read, and UI text.
- Location: `src/shared.rs`, `src/i18n.rs`, `build.rs`, `locales/`
- Contains: `Shared { prefs: Mutex<Prefs>, memory_key, testing_hotkey }` (`src/shared.rs`); `t!`/`key!` macros and `I18N` (`src/i18n.rs`).
- Depends on: `rune_core::prefs`, `rune_i18n`.
- Used by: every module that shows text or reads settings.

## Data Flow

**One rewrite, from hotkey to Replace:**
1. **Hotkey registration.** At launch `SettingsUi::register_saved` registers the saved chord with `popup.register_shortcut(..., |p, tx| p.on_hotkey(tx))` (`src/settings/mod.rs`, `register`). There is no default hotkey (`src/config.rs`, `hotkey: Option<String>`). The menu item Rewrite Selection calls the same path (`src/mac/status.rs`, `rewrite:`).
2. **Open.** `Popup::open` (`src/popup/mod.rs`) returns early when Settings' Test row armed the press (`Shared::take_testing`, `src/shared.rs`) or when `Flow::accepts_hotkey` is false. It reloads `Config`, checks the provider against `consent::disabled_reason` (`src/provider/consent.rs`), sets `flow.default_style`, and steps `Event::Hotkey { has_provider }`.
3. **Flow → Capture.** With a provider, `Flow::step` moves `Idle` → `Capturing { attempt }` and returns `Effect::Capture(attempt)`. With no provider it returns `Notice(NoProvider)` and `ShowPopup` (`src/flow.rs`).
4. **Native capture.** `Popup::capture` sends `Command::Capture { attempt, given }` (`src/popup/mod.rs`). The `retone-native` thread calls `capture::capture(&board, given)` (`src/native.rs`):
   - No Accessibility or event-post permission: clipboard-only mode, `can_replace: false` (`src/capture.rs`, `has_permissions`).
   - Retone itself in front: error `capture.own_window` (`src/capture.rs`).
   - `ax::Target::focused()`, then `strategy::for_app(bundle_id, RETONE_APPLY_OVERRIDE)` (`src/capture.rs`, `src/strategy.rs`).
   - Secure field: refused (`secure_field: true`).
   - Selection: `Target::selected_text` (AX); else `copy_selection` posts ⌘C with a pasteboard snapshot, waits up to `COPY_WAIT` (500 ms) for `changeCount`, reads, and calls `restore_if_unchanged` (`src/capture.rs`).
   - `selection_or_clipboard`: empty selection with no selected range falls back to the clipboard (`from_clipboard: true`); a selected range that could not be read is an error, never the clipboard (`src/capture.rs`).
   - `can_replace = target.is_some() && !from_clipboard && strategy != CopyOnly` (`src/capture.rs`).
   - The worker keeps the `Capture` (target, pid, strategy) in `current` and replies `Reply::Captured { attempt, result, caret }` with `Target::caret_bounds` (`src/native.rs`).
5. **Back on the main thread.** The reply closure in `start_native` runs `popup.update(|p, tx| p.on_reply(reply, tx))` (`src/main.rs`). `Popup::on_reply` stores the caret anchor, computes `consented` with `consent::has_consent` (`src/provider/consent.rs`), and steps `Event::Captured` (`src/popup/mod.rs`).
6. **Input check and consent.** `Flow::captured` refuses secure fields, runs `input::check` (trim, 2,000-character limit, `src/input.rs`), and stores `can_replace`. Without consent it goes to `AwaitingConsent` + `ShowPopup`; ⏎ there sends `ConsentGiven`, which adds `Effect::SaveConsent` (`src/flow.rs`, `src/popup/mod.rs` `on_key`). With consent it goes to `Previewing { phase: Loading }` with `Effect::Request` + `ShowPopup`.
7. **Show.** `Popup::show` uses `placement::usable_caret`, else `mac::screens::mouse_anchor`, then `placement::place`, and shows the nonactivating launcher panel (`src/popup/mod.rs`, `src/placement.rs`, `WindowOptions::launcher_panel()` in `src/main.rs`).
8. **Provider request.** `Popup::request` runs `Provider::rewrite` in a `rune::Latest` worker. For `Local` it reads the Keychain API key in the worker (`src/popup/mod.rs`, `src/shared.rs`). `local::rewrite` streams SSE and calls `on_partial`, which writes `(attempt, text)` to the `partial` slot and wakes the loop (`src/provider/local.rs`). CLI adapters return the whole answer through `cli::execute` (`src/provider/cli.rs`). Prompts come from `styles::build` / `styles::build_single`, and answers pass `styles::clean_answer` and `answer_cap` (`src/styles.rs`, `src/provider/mod.rs`).
9. **Poll.** `Popup::poll` lands `Event::Partial` (phase `Streaming`), then `Event::Answer` (phase `Ready` or `Failed`). When `Outcome::wrote_files` is set it calls `consent::disable` and shows `adapter_warning` (`src/popup/mod.rs`, `src/provider/consent.rs`).
10. **Replace.** ⏎ (or ⌘⏎ for `Style::changes_meaning` styles, `src/styles.rs`) sends `Event::Replace` (`src/popup/mod.rs` `on_key`). Only `Previewing { phase: Ready, can_replace: true }` accepts it: the flow moves to `Applying` with a new attempt and `Effect::Apply` (`src/flow.rs`).
11. **Native apply.** `Popup::apply` sends `Command::Replace`; the worker calls `apply::replace(current, text, board)` (`src/native.rs`). `write_over` checks: pid and target present, `Target::still_matches` (same range; same element or same selected text, `src/mac/ax.rs`), not secure, then by strategy (`src/apply.rs`):
    - `CopyOnly`: error.
    - `AxSet`: frontmost pid still the same, then `set_selected_text` with read-back; a failed read-back is an error, never a paste.
    - `Paste { settle }`: secure-input check, complete snapshot, modifier release, `write_transient_text`, last frontmost-pid check, `post_paste`, sleep `settle` (400 ms, or 700 ms for Electron apps in `src/strategy.rs`), `restore_if_unchanged`.
12. **Finish.** `Reply::Applied` → `Event::Applied`. `Ok` → `Idle` + `ClosePopup`; `close` hides the panel and sends `Command::Forget`, which drops the captured element (`src/popup/mod.rs`, `src/native.rs`). `Err` → back to `Previewing { can_replace: false }` with `apply_error`; Copy still works and Replace is never offered again for this capture (`src/flow.rs`).

**Services menu entry (ADR 0008):**
1. `scripts/bundle.sh` writes four `NSServices` items to Info.plist, with `NSMessage = rewriteSelection` and `NSUserData` = style id (`doc/adr/0008-services-menu-entry.md`).
2. `ServiceProvider::rewrite_selection` reads `public.utf8-plain-text` from the service pasteboard, calls `give_focus_back` (yields activation to the menu-bar-owning app), and calls `Popup::on_service(style_id, text)` (`src/mac/services.rs`).
3. `on_service` maps the id to `styles::PRESETS` and calls `open(index, Some(text))` (`src/popup/mod.rs`). `Command::Capture { given: Some(text) }` makes capture read no selection and no clipboard (`src/native.rs`).
4. Without permissions: Copy only (`src/capture.rs`, `service-no-access`). With permissions: `service_capture` waits up to `SERVICE_FOCUS_WAIT` (500 ms) for the asking app to be in front, takes `ax::Target::focused`, and keeps Replace only when the focused field's selection matches the given text (trimmed) and the strategy is not `CopyOnly` (`src/capture.rs`). The service is send-only (no `NSReturnTypes`), so the asking app never changes from the service itself.

**Cancel paths:**
- Esc → `Event::Cancel` (`src/popup/mod.rs` `on_key`). Panel focus loss → `Event::Cancel` (`on_focus` in `src/main.rs`).
- While loading or streaming: `CancelRequest` + `ClosePopup`. `CancelRequest` sets the `Cancel` flag, which kills a CLI process group or stops the SSE loop, and cancels the `Latest` slot (`src/flow.rs`, `src/popup/mod.rs`, `src/provider/mod.rs`, `src/provider/cli.rs`).
- While `Applying`: Esc is ignored until `Applied` arrives, because ⌘V may already be posted (`src/flow.rs`).

**State Management:**
- Flow state: `Flow { state, default_style, apply_error, seq, can_replace }` owned by `Popup` (`src/flow.rs`, `src/popup/mod.rs`). Rewrites live only in memory.
- Native state: `current: Option<Capture>` on the worker thread, cleared on a new capture and on `Command::Forget` (`src/native.rs`).
- Settings: `Config` loaded from `Prefs` (`prefs.json` under `APP_ID`, separate for dev and release, `src/config.rs`). The `Shared` mutex is shared by both windows (`src/shared.rs`). Consent and disabled adapters are prefs maps `consent` and `adapter_disabled` (`src/provider/consent.rs`).
- Secrets: the Local AI API key lives in the Keychain; prefs keep only `local_api_key_saved` (`src/shared.rs`, `src/mac/keychain.rs`, `doc/adr/0006-local-ai-on-private-networks.md`).
- Async results: `rune::Latest` slots for the provider answer and local model lists, plus an `Arc<Mutex<Option<(Attempt, String)>>>` for streamed partials (`src/popup/mod.rs`, `src/settings/mod.rs`).

## Key Abstractions

**Flow state machine:**
- Purpose: The full rewrite lifecycle as one pure transition function.
- Examples: `src/flow.rs`, `src/flow_tests.rs`
- Pattern: `step(&mut self, Event) -> Effects<Effect>` over `(State, Event)` match arms. States: `Idle`, `Capturing`, `Notice(Refused | NoProvider | CaptureFailed)`, `AwaitingConsent`, `Previewing { phase: Loading | Streaming | Ready | Failed, can_replace }`, `Applying`. Effects: `Capture`, `ShowPopup`, `Request`, `CancelRequest`, `SaveConsent`, `Apply`, `CopyText`, `ClosePopup`.

**Effect runner:**
- Purpose: Map each `Effect` to real work and announce changes to screen readers.
- Examples: `Popup::run`, `request`, `capture`, `apply`, `show`, `close` in `src/popup/mod.rs`
- Pattern: Command interpreter; the macOS-only methods have no-op stubs for other targets.

**Native worker:**
- Purpose: Serialize all AX and pasteboard work on one thread that owns non-`Send` handles.
- Examples: `src/native.rs`
- Pattern: Actor with an `mpsc` command channel and a reply callback (`doc/adr/0002-native-worker-and-unsafe-boundary.md`).

**Capture / Target:**
- Purpose: The captured text plus what Replace needs to check again (AX element, range, pid, strategy).
- Examples: `Capture` in `src/capture.rs`, `Target` in `src/mac/ax.rs`, `Captured` in `src/flow.rs`
- Pattern: Value object; `Captured` is the plain-data part that crosses threads.

**Write strategy:**
- Purpose: How Replace writes into one app.
- Examples: `Strategy { Paste { settle }, AxSet, CopyOnly }`, `TABLE`, `for_app` in `src/strategy.rs`
- Pattern: Static lookup table by bundle ID with an env override (`RETONE_APPLY_OVERRIDE`).

**Provider:**
- Purpose: A closed list of rewrite back ends with an exact model ID.
- Examples: `src/provider/mod.rs`, `src/provider/local.rs`, `src/provider/cli.rs`, `src/provider/pi.rs`, `src/provider/opencode.rs`, `src/provider/claude.rs`, `src/provider/codex.rs`
- Pattern: Enum dispatch (`Provider::rewrite`). CLI adapters build a `cli::Run` (args, env, data folders, stdin prompt) and share `cli::execute`: version gate against `VERIFIED`, `0700` `WorkDir`, process-group kill, before/after data-folder snapshot (`src/provider/cli.rs`, `doc/adr/0004-providers-without-history.md`). `codex::VERIFIED` is empty, so Codex is refused until checked (`src/provider/codex.rs`).

**Style and prompt:**
- Purpose: Nine presets plus `Custom(String)`, and the prompt each sends.
- Examples: `Style`, `PRESETS`, `TONES`, `Prompt`, `build`, `build_single`, `clean_answer` in `src/styles.rs`
- Pattern: The selection goes in a `<text>…</text>` block, the system prompt says it is data, rules are filtered by `keeps_content`, and one example turn is included. Prompts stay in English (`doc/adr/0007-localization.md`). Style ids are stable prefs keys (`Style::id`).

**Consent:**
- Purpose: Ask once per provider and model (per host for network servers) before text leaves the app.
- Examples: `consent_key`, `has_consent`, `grant`, `disable`, `disabled_reason` in `src/provider/consent.rs`
- Pattern: Prefs-backed boolean map; no text stored.

**Headless scene:**
- Purpose: Render any popup or Settings state without native work, for screenshots.
- Examples: `SCENES` in `src/popup/scenes.rs` and `src/settings/mod.rs`
- Pattern: `(&'static str, fn(&mut State))` pairs given to Rune's `AppSpec::scenes` (`src/main.rs`).

## Entry Points

**App launch:**
- Location: `src/main.rs` (`main`)
- Triggers: Launching the binary or `Retone.app` (also launch at login through `src/mac/login.rs`).
- Responsibilities: Run dev tools first; set the system language unless headless (`i18n::use_system_language`); create `Shared` (persistent prefs only in real runs); build the popup `App` (accessory, windowless, keep running, launcher panel, cancel on focus loss) and the hidden Settings window; in real runs attach `Links`, build `SettingsUi::new` with `register_saved`, and call `start_native`.

**Native start:**
- Location: `src/main.rs` (`start_native`)
- Triggers: Real runs on macOS only.
- Responsibilities: Start `Native`; on the main thread, choose the `UpdateChannel` (Homebrew, else Sparkle via `mac::updater::start`, else Manual), install the Services provider (`src/mac/services.rs`) and the status item (`src/mac/status.rs`).

**Global hotkeys:**
- Location: `src/settings/mod.rs` (`register`), handlers `Popup::on_hotkey` and `Popup::on_style_hotkey` in `src/popup/mod.rs`
- Triggers: The user's recorded chord, or a per-style chord.
- Responsibilities: Start a rewrite in the default or chosen style.

**Menu-bar item:**
- Location: `src/mac/status.rs`
- Triggers: Clicks on the menu items built from `menu.rewrite`, `menu.permissions`, `menu.check_updates` and others (selectors `rewrite:`, `settings:`, `permissions:`, `updates:`, `quit:`).
- Responsibilities: Forward to the popup and Settings handles.

**Services menu:**
- Location: `src/mac/services.rs` (`ServiceProvider::rewrite_selection`)
- Triggers: The user picks "Retone: …" in another app's Services or context menu.
- Responsibilities: Pass the text and style id to `Popup::on_service`.

**Headless scenes:**
- Location: `src/popup/scenes.rs`, `src/settings/mod.rs` (`SCENES`), detected by `rune::is_headless()` in `src/main.rs`
- Triggers: `retone [--window settings] --scene NAME --snap out.png --theme dark`, `retone --list-scenes` (`AGENTS.md`).
- Responsibilities: Render one state with in-memory prefs (`Shared::new(false)`), English text, no links and no native work.

**Style eval:**
- Location: `src/eval.rs` (`run_from_args`)
- Triggers: `retone --eval-styles --model M [--provider P] [--endpoint URL] [--text FILE] [--style ID]`.
- Responsibilities: Run fixed `SAMPLES` (or a file) through every preset with the real prompt builder and provider, print answers, and check ratios (`CONCISE_MAX_RATIO`, `FIX_MAX_CHANGE`, `SUMMARY_MAX_RATIO`, `EXPAND_RATIOS`, tone distances). Exit 0/1/2. The `--endpoint` flag counts as the network opt-in; the key comes from `RETONE_API_KEY`.

**CLI history check:**
- Location: `src/history_check.rs` (`run_from_args`), driven by `scripts/cli-history-check.sh`
- Triggers: `retone --cli-check --provider P --model M [--runs N]`.
- Responsibilities: Run a fixed sample through a CLI adapter with the installed version treated as verified, so the script can compare data-folder snapshots.

## Error Handling

**Strategy:** Errors are user-facing `String`s from `t!` keys, carried in `Result<_, String>` across the worker and provider boundaries and shown in the popup. A failure never writes to the target app: it becomes a `Notice`, a `Phase::Failed`, or an `apply_error` (`src/flow.rs`).

**Patterns:**
- Capture errors → `Notice::CaptureFailed`; secure fields and input refusals → `Notice::Refused(Refusal)` (`src/flow.rs`, `src/input.rs`).
- Provider errors → `Phase::Failed`, after `redact` removes the selection and the home path and limits length to 300 characters (`src/provider/mod.rs`, `src/provider/cli.rs`).
- Apply errors → `apply_error` and `can_replace = false` permanently for that capture, to avoid a double insert (`src/flow.rs`, `src/apply.rs`).
- The clipboard is restored with `restore_if_unchanged`, which skips the restore when another app wrote meanwhile; an incomplete snapshot refuses the operation (`src/mac/pasteboard.rs`, `src/capture.rs`, `src/apply.rs`).
- Best-effort writes use `.ok()` with a comment on why (`Native::send` in `src/native.rs`; `config.save` and `consent::grant` in `src/popup/mod.rs`).
- Poisoned prefs mutex: continue with the inner value (`Shared::prefs` in `src/shared.rs`).
- Native start failure: `eprintln!` only (`src/main.rs`).
- Lints: `unused_must_use` and `let_underscore_must_use` deny; `unwrap_used` and `expect_used` warn (`Cargo.toml`).

## Cross-Cutting Concerns

**Logging:** No general logger. `spike!` lines go to stderr only when `RETONE_SPIKE_LOG=1`; they hold bundle IDs, capture paths, lengths, rects and results, never text (`src/spike_log.rs`, used in `src/capture.rs`, `src/apply.rs`, `src/popup/mod.rs`). Diagnostics stay in English (`doc/adr/0007-localization.md`).

**Validation:** Input trim and 2,000-character limit (`src/input.rs`). Endpoint validation: http/https only, no userinfo, loopback or private addresses after the opt-in, checked again in the resolver at connect time, no proxy, no redirects (`src/provider/local.rs`, `doc/adr/0006-local-ai-on-private-networks.md`). CLI model IDs must not look like a flag or contain whitespace (`cli::check_model` in `src/provider/cli.rs`). Shortcuts need a modifier and must not clash with popup chords or other Retone shortcuts (`change_shortcut` in `src/settings/mod.rs`). Answers are capped at `max(4 × input, 400)` characters (`answer_cap` in `src/provider/mod.rs`). The Replace re-check is in `src/apply.rs` and `src/mac/ax.rs`.

**Authentication:** No user accounts. Local AI uses an optional Bearer API key from the Keychain (`src/provider/local.rs`, `src/mac/keychain.rs`); its `Debug` prints `ApiKey(***)`. CLI providers use their own logins; Codex gets only a copy of `auth.json` in a temp `CODEX_HOME` (`src/provider/codex.rs`). macOS permissions (Accessibility, event posting) gate capture and Replace (`capture::has_permissions` in `src/capture.rs`, `src/mac/permissions.rs`). Per-provider consent is in `src/provider/consent.rs`.

**Localization:** `build.rs` runs `rune_i18n::codegen::generate` over `locales/` into `$OUT_DIR/i18n.rs`; `src/i18n.rs` includes it. `t!("key")` and `key!("key")` resolve keys at compile time, so a missing key fails the build. `locales/locales.json` lists layers and locales (English only now). Headless runs stay English (`src/main.rs`, `doc/adr/0007-localization.md`).

**Updates:** One owner per install: Homebrew, else Sparkle (only with `SUPublicEDKey`), else Manual (`src/updates.rs`, `src/mac/updater.rs`, `src/main.rs`, `doc/adr/0005-distribution-and-updates.md`).

**Accessibility (screen readers):** `Popup::run` announces `view::spoken` text once per change with `tx.announce` (`src/popup/mod.rs`, `src/popup/view.rs`).

---

*Architecture analysis: 2026-11-24*
