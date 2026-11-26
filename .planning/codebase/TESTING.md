# Testing Patterns

**Analysis Date:** 2026-11-24

## Test Framework

**Runner:**
- Rust built-in test harness (`#[test]`, `cargo test`), toolchain `1.100.0` (`rust-toolchain.toml`). No extra test crates: `Cargo.toml` has no `[dev-dependencies]`.
- Config: `Cargo.toml` (lints) and `clippy.toml` (`allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`, `allow-dbg-in-tests`).

**Assertion Library:**
- Standard macros only: `assert!`, `assert_eq!`, `assert_ne!`, `matches!`, and `panic!` with `{other:?}` in `match` arms (`src/flow_tests.rs`, `src/strategy.rs`, `src/provider/local_tests.rs`).

**Run Commands:**
```bash
cargo test --locked              # Run all tests (95 passed, 0 failed, 0 ignored on 2026-11-24, ~0.3 s)
# Watch mode: not set up (no cargo-watch or bacon config in the repo)
# Coverage: not set up (no tarpaulin/llvm-cov config, no CI step)
target/debug/retone --list-scenes                                  # headless popup scenes (18)
target/debug/retone [--window settings] --scene NAME --snap out.png --theme dark   # render one scene
target/debug/retone --eval-styles --model qwen2.5:3b [--endpoint URL]              # style prompt eval
sh scripts/cli-history-check.sh pi|opencode|claude|codex MODEL [RUNS]              # CLI no-history gate
```
Commands from `AGENTS.md`; test count from a local `cargo test --locked` run; scene count from `target/debug/retone --list-scenes`.

## Test File Organization

**Location:**
- Co-located. Small suites are an inline `#[cfg(test)] mod tests { use super::*; ... }` at the end of the file: `src/strategy.rs`, `src/capture.rs:200`, `src/config.rs:217`, `src/diff.rs:88`, `src/input.rs`, `src/eval.rs`, `src/styles.rs:302`, `src/popup/view.rs:365`, `src/settings/mod.rs:510`, `src/provider/mod.rs:200`, `src/provider/consent.rs`, `src/provider/claude.rs`.
- Large suites move to a sibling file, loaded from the module with:
  ```rust
  #[cfg(test)]
  #[path = "pasteboard_tests.rs"]
  mod tests;
  ```
  (`src/mac/pasteboard.rs:144-146`; same pattern in `src/flow.rs`, `src/placement.rs`, `src/provider/cli.rs`, `src/provider/local.rs`).
- No `tests/` folder (integration test crate) and no `benches/`.

**Naming:**
- Sibling files: `<module>_tests.rs` (`src/flow_tests.rs`, `src/placement_tests.rs`, `src/provider/cli_tests.rs`, `src/provider/local_tests.rs`, `src/mac/pasteboard_tests.rs`).
- Test functions: a sentence in `snake_case` that states the expected behavior, often the rule it protects: `cancel_and_failures_never_apply_or_copy` (`src/flow_tests.rs`), `an_unreadable_selected_range_never_uses_the_clipboard` (`src/capture.rs`), `an_unverified_version_is_refused_before_any_request` (`src/provider/cli_tests.rs`).

**Structure:**
```
src/
├── flow.rs              -> flow_tests.rs         (13 tests, popup state machine)
├── placement.rs         -> placement_tests.rs    (11 tests)
├── provider/
│   ├── cli.rs           -> cli_tests.rs          (12 tests, stub CLIs for pi/opencode/claude/codex)
│   ├── local.rs         -> local_tests.rs        (12 tests, loopback HTTP stub)
│   ├── mod.rs, consent.rs, claude.rs              (inline tests)
├── mac/pasteboard.rs    -> pasteboard_tests.rs   (5 tests, real NSPasteboard)
├── capture.rs, config.rs, diff.rs, eval.rs, input.rs, strategy.rs, styles.rs (inline tests)
├── popup/view.rs, settings/mod.rs               (inline tests: VoiceOver text, shortcuts)
└── popup/scenes.rs, settings/mod.rs SCENES      (headless render scenes, not #[test])
```
Counts from `#[test]` in each file.

## Test Structure

**Suite Organization:**
```rust
// src/flow_tests.rs
use super::*;

fn effects(e: Effects<Effect>) -> Vec<Effect> {
    e.into_iter().collect()
}

/// Hotkey and capture with consent already given; returns the request's attempt.
fn start(flow: &mut Flow, text: &str) -> Attempt { ... }

#[test]
fn select_rewrite_replace_closes_after_the_apply_succeeds() {
    let mut flow = Flow::new(Style::Clear);
    let attempt = start(&mut flow, "  the meeting are moved  ");
    flow.step(Event::Answer { attempt, result: Ok("The meeting has moved.".into()) });
    let out = effects(flow.step(Event::Replace));
    let apply = match &out[..] {
        [Effect::Apply { attempt, text }] if text == "The meeting has moved." => *attempt,
        other => panic!("expected Apply, got {other:?}"),
    };
    assert_eq!(effects(flow.step(Event::Applied { attempt: apply, result: Ok(()) })), vec![Effect::ClosePopup]);
    assert_eq!(flow.state, State::Idle);
}
```

**Patterns:**
- Setup: plain helper functions at the top of the test file (`start`, `captured`, `applies` in `src/flow_tests.rs`; `Stub::new` in `src/provider/cli_tests.rs`; `mixed_snapshot` in `src/mac/pasteboard_tests.rs`). No fixtures framework.
- Teardown: RAII. Temp folders are dropped through `WorkDir` (`Stub { _dir: WorkDir, .. }` in `src/provider/cli_tests.rs`); pasteboards from `Board::unique()` release on drop (`src/mac/pasteboard.rs:137-139`).
- Serial tests use a static `Mutex` guard that survives poisoning (`serial()` in `src/mac/pasteboard_tests.rs`), because parallel unique pasteboards saw each other's items.
- Assertion: drive the pure state machine with `Event`s and assert the exact `Effect` list (`src/flow_tests.rs`); table loops with a message per case, `assert!(..., "{bad} must be refused")` (`src/provider/local_tests.rs`).

## Mocking

**Framework:** None. Real stand-ins instead of a mocking crate.

**Patterns:**
```rust
// src/provider/local_tests.rs: one-shot loopback HTTP server
fn stub(status: &str, body: &str) -> (String, mpsc::Receiver<String>) { stub_typed(status, "application/json", body) }

fn stub_typed(status: &str, content_type: &str, body: &str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    ...
    (format!("http://127.0.0.1:{port}/v1"), rx)   // endpoint, and the request it got
}

// src/provider/cli_tests.rs: a fake HOME plus a shell stub CLI that records argv, cwd, env, stdin
let stub = Stub::new("1.5.2", "echo 'The meeting moved to Tuesday.'");
let out = pi::rewrite(pi::VERIFIED, &stub.program, "ollama/qwen", &Style::Clear, SECRET, &Cancel::default(), &stub.home);
assert!(stub.record("stdin").contains(&format!("<text>\n{SECRET}\n</text>")));

// src/capture.rs: side effects injected as closures
fn no_read() -> Option<String> { panic!("the clipboard must not be read") }
assert_eq!(selection_or_clipboard("selected".into(), true, no_read), Ok(("selected".into(), false)));
```

**What to Mock:**
- Model providers: a loopback TCP server for the local provider (`src/provider/local_tests.rs`), a `#!/bin/sh` stub behind a symlink for each CLI (`shared_program` in `src/provider/cli_tests.rs`; one shared executable so macOS Gatekeeper scans it once, see comment at `src/provider/cli_tests.rs:39-43`).
- User home and CLI data folders: a temp `home` with `.pi/agent/sessions`, `.local/share/opencode/...` (`Stub::new` in `src/provider/cli_tests.rs`).
- Clipboard reads in capture logic: a closure argument (`src/capture.rs`).

**What NOT to Mock:**
- The flow state machine, placement, strategy table, config, diff, styles: tested as pure code (`AGENTS.md` "Keep pure logic ... free of AppKit, and test it there").
- The pasteboard: tests use a real private `NSPasteboard` from `pasteboardWithUniqueName` (`Board::unique` in `src/mac/pasteboard.rs:43-46`), never the general pasteboard.
- The CLI adapter code: tests call the real `pi::rewrite`, `opencode`, `claude`, `codex` functions (`src/provider/cli_tests.rs`).

## Fixtures and Factories

**Test Data:**
```rust
// src/provider/cli_tests.rs
const SECRET: &str = "the meeting are moved to tuesday";

// src/flow_tests.rs
fn captured(text: &str) -> Result<Captured, String> {
    Ok(Captured { text: text.into(), secure_field: false, can_replace: true, from_clipboard: false })
}

// src/popup/scenes.rs
const ORIGINAL: &str = "hi team, the meeting are moved to tuesday because of some reasons, pls update ur calendars";
const REWRITE: &str = "Hi team, the meeting has moved to Tuesday. Please update your calendars.";
```

**Location:**
- Inline constants and helper functions in each test file; no shared fixtures folder. Eval samples are `SAMPLES` in `src/eval.rs`; CLI-check sample is `SAMPLE` in `src/history_check.rs`. Reference screenshots are in `docs/screenshots/` (6 PNGs, used in docs, not compared by a test).

## Coverage

**Requirements:** None enforced. CI runs `cargo fmt --check`, `cargo clippy --locked --all-targets`, `cargo test --locked`, then `sh scripts/release.sh` to build and package (`.github/workflows/ci.yml`, job `check` on `macos-26`). No coverage tool or threshold.

**View Coverage:**
```bash
# Not configured. No coverage command in AGENTS.md, scripts/ or .github/workflows/.
```

## Test Types

**Unit Tests:**
- 95 `#[test]` functions in `src/` (local run, 2026-11-24). Focus: the popup state machine and the "nothing changes on failure or cancel" rule (`src/flow_tests.rs`), popup placement (`src/placement_tests.rs`), write strategy table and `RETONE_APPLY_OVERRIDE` parsing (`src/strategy.rs`), endpoint validation and resolver policy for loopback/private networks (`src/provider/local_tests.rs`), consent (`src/provider/consent.rs`), prefs load/save (`src/config.rs`), input limit (`src/input.rs`), word diff (`src/diff.rs`), style presets (`src/styles.rs`), eval scoring (`src/eval.rs`), VoiceOver announcements (`src/popup/view.rs`), shortcut validation (`src/settings/mod.rs`).

**Integration Tests:**
- Inside the unit harness, against real OS pieces: CLI adapters run real child processes (stdin only, `0700` temp folder, version gate, file-write guard, cancel kills the child) in `src/provider/cli_tests.rs`; the local provider talks HTTP to a real loopback socket in `src/provider/local_tests.rs`; clipboard snapshot and restore on a real `NSPasteboard` in `src/mac/pasteboard_tests.rs` (macOS only, behind `#[cfg(target_os = "macos")]` on `mod mac` in `src/main.rs`).
- Headless screenshot scenes: Rune renders a named state with no provider, AX or pasteboard work (`src/popup/scenes.rs` module doc; `src/main.rs` skips native start when `rune::is_headless()`). 18 popup scenes (`SCENES` in `src/popup/scenes.rs`: `preview-example` ... `consent-example`) and 16 Settings scenes (`SCENES` in `src/settings/mod.rs:441-508`: `general` ... `privacy-adapter-disabled`). Render with `--snap out.png` and look at the image by hand (`AGENTS.md` checklist). CI does not render or compare them.
- `--eval-styles`: runs `SAMPLES` through every preset style with the real prompt builder and provider, prints a side-by-side report, and exits 1 when a style misses its job (thresholds `CONCISE_MAX_RATIO`, `FIX_MAX_CHANGE`, `SUMMARY_MAX_RATIO`, `EXPAND_RATIOS`, `MIN_PAIR_DISTANCE`, `MIN_MEAN_DISTANCE` in `src/eval.rs`). Manual; needs a running model.
- `scripts/cli-history-check.sh`: release gate. Snapshots path, size, mtime and SHA-256 of every file in the CLI's data folders, runs `retone --cli-check` (`src/history_check.rs`), snapshots again, and fails on any change. A version goes into `VERIFIED` in `src/provider/<cli>.rs` only after it passes. Manual, on a real Mac.

**E2E Tests:**
- Not used. Capture and replace in real apps are checked by hand with the dev bundle and Accessibility permission (`AGENTS.md` checklist); `RETONE_SPIKE_LOG=1` and `RETONE_APPLY_OVERRIDE` help (`src/spike_log.rs`, `src/capture.rs:60`).

## Common Patterns

**Async Testing:**
```rust
// No async runtime. Concurrency is threads and channels: the stub server runs on a
// thread and sends the request it got back over mpsc (src/provider/local_tests.rs).
let (endpoint, request) =
    stub("200 OK", r#"{"choices":[{"message":{"content":"```\nThe meeting moved to Tuesday.\n```"}}]}"#);
let answer =
    rewrite(&endpoint, "qwen2.5:0.5b", None, &Style::Clear, "the meeting are moved", &Cancel::default(), &|_| {});
assert_eq!(answer, Ok("The meeting moved to Tuesday.".into()));
let request = request.recv().expect("request");
assert!(request.starts_with("POST /v1/chat/completions"), "{request}");

// Late answers: the flow fences by Attempt, so a stale answer changes nothing (src/flow_tests.rs).
flow.step(Event::Answer { attempt: first, result: Ok("stale".into()) });
assert!(matches!(&flow.state, State::Previewing { phase: Phase::Loading, .. }));
```

**Error Testing:**
```rust
// src/flow_tests.rs
flow.step(Event::Answer { attempt: attempt2, result: Err("Timed out after 60 s".into()) });
assert!(!applies(&effects(flow.step(Event::Replace))), "no Replace on a failed preview");

// src/capture.rs
assert!(selection_or_clipboard(String::new(), true, no_read).is_err());

// src/provider/local_tests.rs
assert!(validate_endpoint(bad, false).is_err(), "{bad} must be refused");
```

---

*Testing analysis: 2026-11-24*
