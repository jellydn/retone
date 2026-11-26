# Codebase Structure

**Analysis Date:** 2026-11-24

## Directory Layout

```
retone/
├── src/                  # All Rust source: one binary crate (`retone`)
│   ├── main.rs           # Entry: dev-tool flags, two Rune windows, native start in real runs
│   ├── flow.rs           # Pure popup state machine (tests in flow_tests.rs)
│   ├── capture.rs        # Selection capture: AX, then ⌘C, then clipboard; service_capture
│   ├── apply.rs          # Replace: re-check, then paste or AX set
│   ├── strategy.rs       # Per-app write strategy table (pure)
│   ├── native.rs         # Worker thread that owns the pasteboard and the AX element
│   ├── placement.rs      # Popup position: caret, then pointer (pure)
│   ├── input.rs          # Trim and 2,000-character limit (pure)
│   ├── styles.rs         # Styles and the prompt builder (pure)
│   ├── config.rs         # Settings in prefs.json; APP_ID per build
│   ├── shared.rs         # Prefs mutex, API key access, hotkey-test flag
│   ├── diff.rs           # Word diff for the preview
│   ├── theme.rs          # Themes
│   ├── updates.rs        # UpdateChannel: Sparkle, Homebrew, Manual
│   ├── i18n.rs           # t!() / key!() over the build-time catalog
│   ├── spike_log.rs      # spike!() stderr diagnostics (RETONE_SPIKE_LOG=1)
│   ├── eval.rs           # --eval-styles dev tool
│   ├── history_check.rs  # --cli-check dev tool
│   ├── popup/            # Popup window: effect runner, view, scenes
│   ├── settings/         # Settings window
│   ├── provider/         # Local HTTP provider, CLI runner and adapters, consent
│   └── mac/              # macOS FFI; the only modules allowed `unsafe`
├── locales/              # UI text catalogs (rune-i18n)
├── doc/adr/              # Architecture decision records 0001–0008
├── docs/screenshots/     # README screenshots (rendered with --snap)
├── assets/icon/          # App and menu-bar icons
├── scripts/              # Bundle, release, icons, Sparkle, CLI history check
├── Casks/                # Homebrew cask written by the release workflow
├── .github/workflows/    # CI and release
├── build.rs              # Turns locales/ into $OUT_DIR/i18n.rs
├── Cargo.toml            # Crate, pinned Rune revision, objc2 features, lints
├── AGENTS.md             # Agent rules, commands and layout
└── README.md             # User documentation
```

## Directory Purposes

**`src/`:**
- Purpose: The single binary crate. Top-level files are the pure domain logic plus the capture/apply/native trio and the dev tools.
- Contains: Rust modules declared in `src/main.rs`; test files named `*_tests.rs` included with `#[path]`.
- Key files: `src/main.rs`, `src/flow.rs`, `src/native.rs`, `src/capture.rs`, `src/apply.rs`, `src/strategy.rs`, `src/styles.rs`

**`src/popup/`:**
- Purpose: The popup panel at the caret. It owns the `Flow` and runs its effects.
- Contains: `Popup`, `Links`, `Picker`, `on_key` (`src/popup/mod.rs`); the pure view and `spoken` text (`src/popup/view.rs`); headless `SCENES` (`src/popup/scenes.rs`).
- Key files: `src/popup/mod.rs`, `src/popup/view.rs`, `src/popup/scenes.rs`

**`src/settings/`:**
- Purpose: The Settings window: General, Provider, Styles and Privacy pages.
- Contains: `SettingsUi` with hotkey registration, provider save and validation, consent revoke, adapter re-enable, and `SCENES` (`src/settings/mod.rs`); the view (`src/settings/view.rs`); row and card helpers (`src/settings/rows.rs`); icons (`src/settings/icons.rs`).
- Key files: `src/settings/mod.rs`, `src/settings/view.rs`

**`src/provider/`:**
- Purpose: Rewrite back ends with no history.
- Contains: `Kind`, `Provider`, `Env`, `Outcome`, `Cancel`, `redact`, `answer_cap` (`src/provider/mod.rs`); OpenAI-compatible streaming client with a private-address resolver (`src/provider/local.rs`); the shared CLI runner (`src/provider/cli.rs`); adapters (`src/provider/pi.rs`, `src/provider/opencode.rs`, `src/provider/claude.rs`, `src/provider/codex.rs`); consent and disabled adapters (`src/provider/consent.rs`).
- Key files: `src/provider/mod.rs`, `src/provider/local.rs`, `src/provider/cli.rs`, `src/provider/consent.rs`

**`src/mac/`:**
- Purpose: All macOS FFI. `src/mac/mod.rs` sets `#[allow(unsafe_code)]` per leaf module; every `unsafe` block has a `// SAFETY:` comment.
- Contains: AX target, re-check and AX set (`src/mac/ax.rs`); CGEvent ⌘C/⌘V, modifier wait, secure input (`src/mac/events.rs`); pasteboard snapshot, transient write, restore (`src/mac/pasteboard.rs`); screens, frontmost pid and bundle ID (`src/mac/screens.rs`); Services provider (`src/mac/services.rs`); status item menu (`src/mac/status.rs`); Sparkle (`src/mac/updater.rs`); login item (`src/mac/login.rs`); Keychain (`src/mac/keychain.rs`); System Settings links (`src/mac/permissions.rs`).
- Key files: `src/mac/ax.rs`, `src/mac/pasteboard.rs`, `src/mac/events.rs`, `src/mac/services.rs`, `src/mac/status.rs`

**`locales/`:**
- Purpose: UI text. English is the source and the fallback.
- Contains: `locales/locales.json` (layers `["app"]`, locales `en`), `locales/app/en.json` (flat `"area.key": "text"` map, keys such as `apply.selection_changed`, `capture.own_window`).
- Key files: `locales/app/en.json`, `locales/locales.json`

**`doc/adr/`:**
- Purpose: Decision records.
- Contains: `0001-rune-menu-bar-app.md`, `0002-native-worker-and-unsafe-boundary.md`, `0003-capture-and-safe-replace.md`, `0004-providers-without-history.md`, `0005-distribution-and-updates.md`, `0006-local-ai-on-private-networks.md`, `0007-localization.md`, `0008-services-menu-entry.md`.
- Key files: `doc/adr/0002-native-worker-and-unsafe-boundary.md`, `doc/adr/0003-capture-and-safe-replace.md`, `doc/adr/0008-services-menu-entry.md`

**`scripts/`:**
- Purpose: Packaging and checks outside `cargo`.
- Contains: `scripts/bundle.sh` (dev or release `.app`, writes Info.plist including `LSUIElement` and `NSServices`), `scripts/release.sh` (DMG, ZIP, SHA256SUMS), `scripts/cli-history-check.sh` (no-history gate per CLI version), `scripts/sparkle-tools.sh`, `scripts/update-distribution.sh`, `scripts/make-icons.sh`, `scripts/icon-svgs.py`.
- Key files: `scripts/bundle.sh`, `scripts/release.sh`, `scripts/cli-history-check.sh`

**`.github/workflows/`:**
- Purpose: CI and releases.
- Contains: `.github/workflows/ci.yml` (fmt, clippy, test, `scripts/release.sh` on `macos-26`), `.github/workflows/release.yml`.
- Key files: `.github/workflows/ci.yml`

## Key File Locations

**Entry Points:**
- `src/main.rs`: `main` (app setup) and `start_native` (worker, updater, Services, status item).
- `src/eval.rs`: `run_from_args` for `--eval-styles`.
- `src/history_check.rs`: `run_from_args` for `--cli-check`.
- `src/mac/services.rs`: `ServiceProvider::rewrite_selection`, the Services entry.
- `src/mac/status.rs`: menu-bar item actions.

**Configuration:**
- `Cargo.toml`: dependencies, one pinned Rune revision for all Rune crates and patches, objc2 features, crate lints (`unsafe_code = "deny"`).
- `rust-toolchain.toml`: pinned toolchain.
- `rustfmt.toml`: width 120.
- `clippy.toml`: Rune's shared lints.
- `src/config.rs`: `Config` and `APP_ID` (`io.github.jellydn.retone` or `.dev`).
- `locales/locales.json`: locale list.
- `build.rs`: i18n codegen.

**Core Logic:**
- `src/flow.rs`: states, events, effects, transitions.
- `src/popup/mod.rs`: effect runner, key handling, provider request, poll.
- `src/native.rs`: worker commands and replies.
- `src/capture.rs`: capture order and `service_capture`.
- `src/apply.rs`: Replace checks and writes.
- `src/strategy.rs`: per-app strategy table and `RETONE_APPLY_OVERRIDE`.
- `src/mac/ax.rs`: `Target::still_matches`, `set_selected_text`.
- `src/mac/pasteboard.rs`: `snapshot`, `write_transient_text`, `restore_if_unchanged`.
- `src/styles.rs`: style instructions, rules, `build`, `build_single`, `clean_answer`.
- `src/provider/mod.rs`: provider dispatch.

**Testing:**
- `src/flow_tests.rs`: flow transitions (included from `src/flow.rs`).
- `src/placement_tests.rs`: placement (included from `src/placement.rs`).
- `src/mac/pasteboard_tests.rs`: pasteboard snapshot and restore.
- `src/provider/local_tests.rs`, `src/provider/cli_tests.rs`: endpoint checks and the CLI runner with stubs.
- Inline `#[cfg(test)] mod tests` in `src/capture.rs`, `src/strategy.rs`, `src/styles.rs`, `src/input.rs`, `src/config.rs`, `src/diff.rs`, `src/eval.rs`, `src/provider/mod.rs`, `src/provider/consent.rs`, `src/provider/claude.rs`, `src/popup/view.rs`, `src/settings/mod.rs`.
- Visual checks: headless scenes in `src/popup/scenes.rs` and `src/settings/mod.rs`, rendered with `--scene NAME --snap out.png` (`AGENTS.md`).

## Naming Conventions

**Files:**
- snake_case module per concern: `src/spike_log.rs`, `src/history_check.rs`.
- Tests split out as `<module>_tests.rs` beside the module, pulled in with `#[path = "..._tests.rs"] mod tests;`: `src/flow_tests.rs` from `src/flow.rs`.
- One file per CLI adapter, named by tool: `src/provider/opencode.rs`.
- ADRs as `NNNN-kebab-title.md`: `doc/adr/0008-services-menu-entry.md`.
- i18n keys as `area.name`, grouped by module: `apply.copy_only`, `capture.unreadable` in `locales/app/en.json`.

**Directories:**
- A directory per window or subsystem with `mod.rs` as the owner: `src/popup/mod.rs`, `src/settings/mod.rs`, `src/provider/mod.rs`, `src/mac/mod.rs`.
- Platform code under `src/mac/`, gated by `#[cfg(target_os = "macos")]` in `src/main.rs`.

## Where to Add New Code

**New Feature:**
- Primary code: new states, events or effects in `src/flow.rs`; their execution in `Popup::run` in `src/popup/mod.rs`; the view in `src/popup/view.rs`; UI text in `locales/app/en.json` read with `t!`.
- Tests: transitions in `src/flow_tests.rs`; a headless scene in `src/popup/scenes.rs` for each new visible state.

**New Component/Module:**
- New provider: an adapter in `src/provider/<name>.rs` that builds a `cli::Run` (see `src/provider/pi.rs`), a `Kind` and `Provider` variant in `src/provider/mod.rs`, a model field in `src/config.rs`, and Settings rows in `src/settings/`. Start with an empty `VERIFIED` until `scripts/cli-history-check.sh` passes (`doc/adr/0004-providers-without-history.md`).
- New style: a variant in `Style`, `PRESETS`, `id`, `name` and `instruction` in `src/styles.rs` (at most 9 presets, checked by a `const` assert).
- New Service: an `NSServices` item in `scripts/bundle.sh` whose `NSUserData` is a `Style::id`; no Rust change is needed (`src/mac/services.rs`, `src/popup/mod.rs` `on_service`).
- New app write strategy: an entry in `TABLE` in `src/strategy.rs`, only after the app matrix proves it.
- New native call: a safe wrapper in a `src/mac/` leaf module with `// SAFETY:` comments; callers that need the pasteboard or AX element go through `Command` in `src/native.rs`.
- New key decision: an ADR in `doc/adr/`.

**Utilities:**
- Shared helpers: pure helpers go in the matching top-level module (`src/input.rs`, `src/placement.rs`, `src/diff.rs`); state both windows need goes in `src/shared.rs`; provider helpers go in `src/provider/mod.rs` or `src/provider/cli.rs`.

## Special Directories

**`target/`:**
- Purpose: Cargo output, `target/bundle/Retone Dev.app` from `scripts/bundle.sh`, and release assets in `target/release/assets` (`AGENTS.md`).
- Generated: Yes
- Committed: No (`.gitignore`)

**`$OUT_DIR/i18n.rs`:**
- Purpose: The message catalog generated by `build.rs` and included by `src/i18n.rs`.
- Generated: Yes
- Committed: No

**`Casks/`:**
- Purpose: Homebrew cask `Casks/retone.rb`.
- Generated: Yes, by the release workflow after publishing (`doc/adr/0005-distribution-and-updates.md`)
- Committed: Yes

**`docs/screenshots/`:**
- Purpose: README images, rendered from headless scenes (for example `preview-example`, `provider-local`).
- Generated: Yes, with `--scene ... --snap`
- Committed: Yes

**`.amp/`:**
- Purpose: Local agent tooling scratch (`.amp/in/`).
- Generated: Yes
- Committed: No (`/.amp/in/` is in `.git/info/exclude`)

**`.planning/`:**
- Purpose: Planning and codebase-map documents such as this file.
- Generated: Yes, by agents
- Committed: Yes (`.planning/codebase/`)

---

*Structure analysis: 2026-11-24*
