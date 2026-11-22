# AGENTS.md - Agent Guidelines for Retone

## Project Overview

Retone is a macOS menu-bar app that rewrites the selected text in any app with a user-chosen hotkey.

**Stack**: Rust (toolchain pinned in `rust-toolchain.toml`), [Rune](https://github.com/anomalyco/rune) UI, objc2 for AppKit/AX | **Platform**: macOS 14+, Apple Silicon

## Commands

```bash
cargo fmt --check                 # format (rustfmt.toml: width 120)
cargo clippy --all-targets        # lint (clippy.toml has Rune's shared lints)
cargo test --locked               # unit tests
scripts/bundle.sh                 # dev bundle: target/bundle/Retone Dev.app
scripts/bundle.sh --release       # release bundle: Retone.app
sh scripts/release.sh             # DMG, ZIP, SHA256SUMS in target/release/assets
target/debug/retone --list-scenes # headless scenes for screenshots
target/debug/retone --eval-styles --model qwen2.5:3b [--endpoint URL] # style prompt eval (prints sample text)
sh scripts/cli-history-check.sh pi|opencode MODEL  # no-history gate for a CLI version (real Mac)
target/debug/retone [--window settings] --scene NAME --snap out.png --theme dark
```

`RETONE_SPIKE_LOG=1` prints capture and replace diagnostics to stderr. It never prints text.
`RETONE_APPLY_OVERRIDE=bundle.id=paste|ax-set|copy-only,...` tries a write strategy for one app without a rebuild.

## Layout

```
src/
├── main.rs        # app and window setup; native start in real runs only
├── flow.rs        # popup state machine (pure; tests in flow_tests.rs)
├── capture.rs     # selection capture: AX, then ⌘C, then clipboard mode
├── apply.rs       # replace checks, then paste or AX set
├── strategy.rs    # per-app write strategy table (pure)
├── placement.rs   # popup position (caret, then pointer)
├── native.rs      # worker thread that owns the pasteboard and the AX element
├── provider/      # local (HTTP/HTTPS, loopback or private network), cli runner, pi, opencode, consent
├── popup/         # popup view and scenes
├── settings/      # Settings window
├── updates.rs     # update channel: Sparkle, Homebrew, or manual
├── i18n.rs        # t!() and key!(): UI text from locales/app/en.json, checked at build time
└── mac/           # the only modules with `unsafe` (AX, events, pasteboard, status item, Sparkle)
```

## Rules (do not break)

- **No history**: never write selected or rewritten text to disk, logs, prefs or crash output.
- **Nothing changes on failure or cancel**: replace only after the target check passes; restore the clipboard unless another app changed it.
- **Input limit**: 2,000 characters.
- **No default hotkey**.
- **Local provider: loopback, or private addresses after the opt-in** (checked in the resolver at connect time; no proxy, no redirects; consent per host; the optional API key lives in the Keychain, never in prefs). CLI providers get text on stdin, a `0700` temp folder, a version gate, and the file-write guard.
- **`unsafe` only in `src/mac/`**, each block with a `// SAFETY:` comment, behind safe functions. No `unsafe impl Send`; use the native worker.
- **Bundle IDs**: release `io.github.jellydn.retone`, dev `io.github.jellydn.retone.dev`. Do not share prefs or grants between them.
- **One update owner**: Homebrew installs never start Sparkle; Sparkle starts only with `SUPublicEDKey` in the bundle.

## Code Style

- Clear names; comments explain why, not what.
- Keep pure logic (flow, placement, input, config) free of AppKit, and test it there.
- Use Rune's conventions and kit components before custom widgets.
- UI text goes in `locales/app/en.json` and is read with `t!("key")`, not written as a literal in view code. Model prompts stay in English.

## Commit Style

`type(scope): description` - Types: feat, fix, refactor, test, docs, chore, ci

## Agent Checklist

- [ ] `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test --locked`
- [ ] For UI changes, render the affected scenes with `--snap` and look at them
- [ ] For capture/replace changes, test in a real app with the dev bundle (needs Accessibility)
- [ ] Check the rules above, and add an ADR in `doc/adr/` for a new key decision
