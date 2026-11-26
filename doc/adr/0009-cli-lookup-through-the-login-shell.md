# 0009. Find the CLI providers through the login shell

Date: 2026-11-24

## Status

Accepted

## Context

Retone ran the CLI providers (`pi`, `opencode`, `claude`, `codex`) by bare name. An app opened from Finder or at login gets launchd's `PATH`, `/usr/bin:/bin:/usr/sbin:/sbin`, so a CLI in Homebrew, `~/.local/bin` or a Node install was "not found". Tests that started Retone from a terminal did not show this. Some CLIs are `#!/usr/bin/env node` scripts, so the CLI process also needs a `PATH` that contains `node`.

## Decision

- On the first CLI request (a worker thread), Retone runs `$SHELL -l -c` once and reads the shell's `PATH` from its output, after a marker. If the shell fails, or takes more than 5 seconds, Retone keeps its own `PATH`.
- Retone then adds the usual install folders that exist (`~/.bun/bin`, `~/.local/bin`, `~/.npm-global/bin`, `~/Library/pnpm`, `~/.volta/bin`, `/opt/homebrew/bin`, `/usr/local/bin`), because a shell can add them only in interactive sessions.
- A bare CLI name is looked up in that `PATH`, as `command -v` does. The CLI then runs with that `PATH`, with its own folder first.
- **Settings → Provider → Program** takes an optional absolute path for the chosen CLI. It replaces the lookup. The path must be an absolute path to a file, or Save refuses it. Prefs keep it in `cli_programs` by provider key.

## Consequences

- The user's login startup files run once per Retone run, with no text involved. This is what Terminal does at each new window.
- A folder that the shell adds only in interactive sessions (fish's `status is-interactive` block, `~/.zshrc`) is seen only when it is one of the install folders above. The Program field covers other folders.
- A `--version` that fails after the program was found shows the CLI's own error line (for example a missing `node`), not "Cannot run".
- The first CLI rewrite of a run waits for the shell's startup time, once.
- The unit tests that run CLI stubs also read the login shell's `PATH` once per test run.
