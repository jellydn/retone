# 0004. Providers without history

Date: 2026-11-21

## Status

Accepted

## Context

Retone must not keep or leak writing history, but users want their own models: a local server, or CLIs such as Pi and OpenCode that keep sessions and logs by default.

## Decision

- Local provider: OpenAI-compatible HTTP to loopback hosts by default. Superseded in part by [ADR 0006](0006-local-ai-on-private-networks.md): private network addresses after an opt-in, and HTTPS.
- CLI providers share one runner: a version gate (only tested versions), text on stdin, a `0700` temp folder deleted afterwards, a process-group kill on timeout, and a guard that turns the adapter off when it sees file writes outside the temp folder.
- Pi: `--no-session` with tools, extensions and context files off. OpenCode: `OPENCODE_DB=:memory:`, `XDG_DATA_HOME` in the temp folder, auto-update off.
- Consent is asked once per provider and model, and saved without any text.
- 2,000-character input limit.

## Consequences

- A new CLI version needs a check (and a version bump in the adapter) before Retone accepts it.
