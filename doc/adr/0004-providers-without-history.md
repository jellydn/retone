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
- Claude Code: `-p --no-session-persistence`, `--tools ""`, `--strict-mcp-config`, `--disable-slash-commands`, `CLAUDE_CODE_SKIP_PROMPT_HISTORY=1`. User settings stay loaded, because many users set the API address and token there; `--settings '{"disableAllHooks":true}'` turns off their hooks (user scripts that can write files), and project and local settings are not loaded.
- Codex: `exec --ephemeral --ignore-user-config --ignore-rules --sandbox read-only`, with the shell, shell snapshots, memories, plugins, apps, browser and image tools disabled, and the prompt on stdin (`-`). The login still comes from `CODEX_HOME`.
- Claude Code and Codex start with no verified version, so Retone refuses them until `scripts/cli-history-check.sh` passes on a real Mac.
- Consent is asked once per provider and model, and saved without any text.
- 2,000-character input limit.

## Consequences

- A new CLI version needs a check (and a version bump in the adapter) before Retone accepts it. The check is `scripts/cli-history-check.sh`: 5 rewrites through the adapter, with every file under the CLI's data folders compared before and after. A folder time change with no file left behind (Pi makes and deletes a lock file on every start) is only a note.
