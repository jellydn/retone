# Coding Conventions

**Analysis Date:** 2026-11-24

## Naming Patterns

**Files:**
- One `snake_case.rs` file for each concern at the crate root: `src/flow.rs`, `src/capture.rs`, `src/apply.rs`, `src/strategy.rs`, `src/placement.rs`, `src/native.rs` (module list in `src/main.rs`).
- A folder with `mod.rs` for each larger area: `src/provider/mod.rs`, `src/popup/mod.rs`, `src/settings/mod.rs`, `src/mac/mod.rs`.
- Large test modules live in a sibling `<module>_tests.rs` file that the module loads with `#[path = "..."]`: `src/flow_tests.rs` (from `src/flow.rs`), `src/placement_tests.rs`, `src/provider/cli_tests.rs`, `src/provider/local_tests.rs`, `src/mac/pasteboard_tests.rs`.
- Each CLI provider has one file named for the tool: `src/provider/pi.rs`, `src/provider/opencode.rs`, `src/provider/claude.rs`, `src/provider/codex.rs`.

**Functions:**
- `snake_case` verbs or nouns that read as plain English: `replace`, `write_over`, `paste_over` in `src/apply.rs`; `selection_or_clipboard` in `src/capture.rs`; `for_app` in `src/strategy.rs`; `use_system_language` in `src/i18n.rs`.
- CLI entry points that the binary checks before the app starts are named `run_from_args` and return `Option<i32>` (exit code, or `None` for a normal start): `src/eval.rs`, `src/history_check.rs`, called from `main` in `src/main.rs`.
- Test names are full sentences in `snake_case` that state the behavior: `select_rewrite_replace_closes_after_the_apply_succeeds` (`src/flow_tests.rs`), `without_the_opt_in_only_this_mac_is_accepted` (`src/provider/local_tests.rs`), `cancel_kills_the_child_and_errors_are_redacted` (`src/provider/cli_tests.rs`).

**Variables:**
- `snake_case`, full words, no Hungarian prefixes: `links_shared`, `settings_popup`, `popup_spec` in `src/main.rs`.
- Constants are `SCREAMING_SNAKE_CASE` with a doc comment that says why the value is what it is: `CONCISE_MAX_RATIO`, `FIX_MAX_CHANGE`, `EXPAND_RATIOS` in `src/eval.rs`; `DEADLINE`, `ERROR_LIMIT` in `src/provider/mod.rs`.

**Types:**
- `PascalCase` nouns: `Flow`, `State`, `Event`, `Effect`, `Phase` (`src/flow.rs`); `Provider`, `Outcome`, `Cancel`, `Env`, `Kind` (`src/provider/mod.rs`); `Board`, `Snapshot` (`src/mac/pasteboard.rs`).
- Error enums name the reason, not the word "Error", when the crate owns them: `Refusal` (`src/input.rs:10`), `Failure` (`src/provider/cli.rs:138`).
- Scene tables use a local alias `type Scene = (&'static str, fn(&mut Popup));` (`src/popup/scenes.rs`).

## Code Style

**Formatting:**
- `rustfmt` with `cargo fmt --check` (`AGENTS.md`, run in CI by `.github/workflows/ci.yml`).
- `rustfmt.toml`: `max_width = 120`, `use_small_heuristics = "Max"` (short struct literals and calls stay on one line, for example `Captured { text: ..., secure_field: false, ... }` in `src/flow_tests.rs`).
- Rust edition 2024 (`Cargo.toml`); toolchain pinned to `1.100.0` with `clippy` and `rustfmt` (`rust-toolchain.toml`).

**Linting:**
- `cargo clippy --locked --all-targets` in CI (`.github/workflows/ci.yml`).
- `clippy.toml` copies Rune's shared lints: `unwrap`, `expect`, `panic` and `dbg` are allowed in tests only; `std::env::var`, `std::time::SystemTime::now` and `std::time::Instant::now` are disallowed in favor of `rune_core::env::var` and `rune_core::time::*` (for example `src/spike_log.rs:13`, `src/capture.rs:60`).
- `Cargo.toml` `[lints.rust]`: `unsafe_code = "deny"`, `unsafe_op_in_unsafe_fn = "deny"`, `unused_must_use = "deny"`.
- `Cargo.toml` `[lints.clippy]`: deny `undocumented_unsafe_blocks`, `await_holding_lock`, `dbg_macro`, `disallowed_methods`, `disallowed_types`, `let_underscore_must_use`; warn `unwrap_used`, `expect_used`, `todo`.
- `unsafe` rule: only `src/mac/` opts in, one `#[allow(unsafe_code)]` per FFI module (`src/mac/mod.rs:4-19`; `keychain`, `permissions` and `screens` stay safe). Each block has a `// SAFETY:` comment directly above it (`src/mac/events.rs:70`, `src/mac/pasteboard.rs:137`). Pure modules also add `#![forbid(unsafe_code)]` (`src/flow.rs`, `src/placement.rs:7`, `src/config.rs:3`, `src/provider/mod.rs:7`, and 11 more); the other modules rely on the crate-wide `deny` (`Cargo.toml`). No `unsafe impl Send`: AppKit and AX objects stay on the worker thread in `src/native.rs` (`AGENTS.md`, `doc/adr/0002-native-worker-and-unsafe-boundary.md`).
- Local lint exceptions are rare and carry a reason comment: `#[allow(clippy::single_range_in_vec_init)]` in `src/diff.rs:89-90`.

**UI text rule:**
- All UI words come from `locales/app/en.json` through `t!("key")` or `key!("key")` (`src/i18n.rs`). `key!` builds a `const` `Key`, so a missing name fails the build; `t!("key", name = value)` fills `{{name}}` placeholders.
- `#[macro_use] mod i18n;` is declared first in `src/main.rs` so `t!` is in scope everywhere.
- The rule also covers user-facing error strings: `src/provider/claude.rs:36`, `src/provider/claude.rs:74`, `src/mac/status.rs:110-114`.
- Model prompts and style rules stay in English and are not translated (`src/i18n.rs` module doc, `AGENTS.md`). Headless renders stay in English so snapshots do not depend on the Mac language (`src/main.rs`).

## Import Organization

**Order:**
1. `crate::` imports (sorted by path): `src/popup/mod.rs:10-16`
2. External crates (`rune`, `rune_core`, `objc2`, `serde`, `ureq`): `src/popup/mod.rs:17-18`
3. `std::` imports: `src/popup/mod.rs:19`, `src/main.rs`

rustfmt sorts within each `use` list, so lowercase module names come after types (`{Cancel, Env, Kind, Outcome, claude, cli, codex, opencode, pi}` in `src/history_check.rs`).

**Path Aliases:**
- None. Code uses `crate::`, `super::` and `self` in grouped imports (`use crate::placement::{self, Rect};` in `src/popup/mod.rs:12`). Test modules start with `use super::*;` (`src/flow_tests.rs`, `src/provider/local_tests.rs`).

## Error Handling

**Patterns:**
- Most fallible functions return `Result<T, String>` where the `String` is a localized, user-readable message that the popup shows: `Result<(), String>` (15 uses) and `Result<String, String>` (12 uses) across `src/` (for example `src/apply.rs:15`, `src/provider/mod.rs` `Outcome::answer`).
- Typed errors only where the caller branches on the reason: `Refusal` (`src/input.rs:10`), `Failure` (`src/provider/cli.rs:138`), Rune's `ShortcutError` (`src/settings/mod.rs`).
- `?` with `.ok_or(...)` and `.map_err(|_| ...)` to turn foreign errors into messages: `src/history_check.rs` `run`.
- Errors are bounded and redacted before display: `ERROR_LIMIT = 300` and redaction in `src/provider/mod.rs`; test `cancel_kills_the_child_and_errors_are_redacted` in `src/provider/cli_tests.rs`.
- Failure never changes the target app: the flow returns no `Apply` or `CopyText` effect after a failed preview or cancel (`src/flow.rs`, checked by `cancel_and_failures_never_apply_or_copy` in `src/flow_tests.rs`).
- No `unwrap()`/`expect()` in production code (clippy warns, `Cargo.toml`); poisoned locks are recovered with `unwrap_or_else(|poisoned| poisoned.into_inner())` (`src/mac/pasteboard_tests.rs`).
- Startup failures that do not stop the app print one `eprintln!("retone: ...")` line and continue: `start_native` in `src/main.rs`.

## Logging

**Framework:** `eprintln!` only, behind the `spike!` macro (`src/spike_log.rs`).

**Patterns:**
- Diagnostics are off unless `RETONE_SPIKE_LOG=1` is set at launch (`src/spike_log.rs:12-14`, `AGENTS.md`).
- Log lines never contain selected or rewritten text: only bundle IDs, the capture path, text lengths, rects and results (`src/spike_log.rs` module doc; "No history" rule in `AGENTS.md`).
- Developer CLIs (`--eval-styles`, `--cli-check`) print to stdout/stderr with a `retone --<command>:` prefix and use only built-in sample text (`src/eval.rs`, `src/history_check.rs`).

## Comments

**When to Comment:**
- Comments explain why, not what (`AGENTS.md`). Examples: "The panel loses key focus when the user clicks elsewhere: they moved on, so cancel." (`src/main.rs`); the Gatekeeper reason for one shared stub program (`src/provider/cli_tests.rs:39-43`).
- Every module starts with a `//!` doc that states its purpose and its limits (`src/main.rs`, `src/eval.rs`, `src/provider/mod.rs`, `src/mac/mod.rs`).
- Dependency choices in `Cargo.toml` carry a reason comment (`ureq`, `security-framework`, the Rune rev pin).
- Comments use short, plain English sentences, close to Simplified Technical English.

**JSDoc/TSDoc:**
- Not applicable (Rust). Rustdoc `///` comments on public functions, constants and macros, often with an example call: `t!` and `key!` in `src/i18n.rs`, `run_from_args` in `src/eval.rs`, constants in `src/eval.rs`.

## Function Design

**Size:** Small, single-purpose functions; the largest files are views (`src/settings/view.rs` 750 lines, `src/settings/mod.rs` 580, `src/popup/mod.rs` 472). Pure logic files stay under ~370 lines (`src/flow.rs` 284, `src/styles.rs` 363).

**Parameters:** Pass borrowed data (`&str`, `&Style`, `&Cancel`, `&Env`) and inject side effects as closures so logic stays testable: `selection_or_clipboard(text, has_range, read_clipboard)` in `src/capture.rs`; `on_partial: &dyn Fn(&str)` in `Provider::rewrite` (`src/provider/mod.rs:97`).

**Return Values:** `Result<_, String>` for fallible work; the flow state machine returns `Effects<Effect>` for the shell to run (`src/flow.rs`, Rune `rune::machine`); developer CLIs return `Option<i32>` exit codes (`src/eval.rs`).

## Module Design

**Exports:** `pub mod` for submodules used across the crate (`src/provider/mod.rs:9-15`, `src/mac/mod.rs`); private `mod` for helpers used by one parent (`mod icons; mod rows;` in `src/settings/mod.rs:8-9`). Test-only helpers are gated with `#[cfg(test)]` (`Board::unique` in `src/mac/pasteboard.rs:43`). macOS-only modules are gated with `#[cfg(target_os = "macos")]` in `src/main.rs`.

**Barrel Files:** Not used beyond `mod.rs` declarations and a few `pub use` lines (`src/i18n.rs` re-exports `KEYS` and `Key`; `src/spike_log.rs` re-exports `spike`).

## Commit Style

- Conventional Commits `type(scope): description`, types `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci` (`AGENTS.md`). History follows it, for example `fix(popup): ignore a Service or hotkey request while the popup is busy`, `test(provider): share one stub program across the CLI tests`, `chore(release): 0.1.0-beta.11` (`git log`).
- A new key decision gets an ADR in `doc/adr/` (`AGENTS.md`; 8 ADRs, `doc/adr/0001-rune-menu-bar-app.md` to `doc/adr/0008-services-menu-entry.md`).

---

*Convention analysis: 2026-11-24*
