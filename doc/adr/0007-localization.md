# 0007. Localization with rune-i18n

Date: 2026-11-22

## Status

Accepted.

## Context

All UI text was English literals in the view code. To add languages, the text must move to one place first, and a translator must not need to read Rust.

## Decision

- Use Rune's `rune-i18n`, at the same Rune revision as the other Rune crates. `build.rs` turns `locales/` into a catalog at build time, so nothing parses JSON at launch.
- `locales/app/en.json` holds every UI message. English is the source locale: a key that another locale does not have falls back to English.
- Code reads a message with `t!("key")`, or `t!("key", name = value)` for `{{name}}` placeholders. A key name that is not in the catalog stops the build.
- A real run uses the system's preferred languages. Headless renders (`--snap`, `--scene`) stay in English, so snapshots do not depend on the Mac's language.
- Not translated: the model prompts and the style rules (the model gets English instructions in every locale), product names (Retone, Ollama, Pi, OpenCode), key symbols and shell commands.
- First step: English only, with no change to the UI. The error and notice texts from capture, apply, the providers and the input checks, and the style and theme names, move in a later step. Languages come after that.

## Consequences

- A new language is one JSON file per layer and one entry in `locales/locales.json`.
- The ICU4X plural, number, list and locale crates (with their CLDR data) are now in the build.
- An unused key is not found by the build. Remove keys when their text goes.
