# External Integrations

**Analysis Date:** 2026-11-24

## APIs & External Services

**AI providers (closed list of five, `KINDS` in `src/provider/mod.rs`; no fallback between them; 60 s `DEADLINE`, answer cap 4× input with a floor of 400 chars, errors redacted by `redact()`):**
- Local AI - OpenAI-compatible server (Ollama `http://127.0.0.1:11434/v1`, LM Studio `http://127.0.0.1:1234/v1`, llama.cpp `llama-server` `http://127.0.0.1:8080/v1`) for `GET /models` and streamed `POST /chat/completions` (SSE) (`src/provider/local.rs`)
  - SDK/Client: `ureq` 3 with a custom `PrivateOnly` resolver (loopback only, or loopback + private ranges incl. Tailscale 100.64/10 and `fd7a:115c:a1e0::/48` after the `local_allow_network` opt-in), `proxy(None)`, `max_redirects(0)`, native TLS with the platform verifier, 1 MiB body limit (`src/provider/local.rs`, `src/config.rs`, `doc/adr/0006-local-ai-on-private-networks.md`)
  - Auth: optional Bearer token from the Keychain (account `local-api-key`, `src/mac/keychain.rs`); `RETONE_API_KEY` for the eval tool only (`src/eval.rs`)
- Pi CLI - `pi -p --no-session` with tools, extensions, skills, templates, context files and thinking off (`src/provider/pi.rs`)
  - SDK/Client: child process via the shared runner (prompt on stdin, `0700` temp working folder, version gate, process-group kill on cancel, data-folder write guard) (`src/provider/cli.rs`)
  - Auth: the CLI's own login; Retone sets `PI_OFFLINE` (`src/provider/pi.rs`). Verified version: `1.5.2`
- OpenCode CLI - `opencode run --standalone` with a private config whose only agent denies all tools (`src/provider/opencode.rs`)
  - SDK/Client: shared runner (`src/provider/cli.rs`); env set: `OPENCODE_DB`, `XDG_DATA_HOME`, `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, `OPENCODE_CONFIG_PROJECT_DISABLE`, `OPENCODE_DISABLE_PROJECT_CONFIG`, `OPENCODE_DISABLE_FILEWATCHER`, `OPENCODE_DISABLE_CLAUDE_CODE`, `OPENCODE_DISABLE_AUTOUPDATE`
  - Auth: a temp copy of the user's `~/.local/share/opencode/auth.json`, deleted with the temp folder (`src/provider/opencode.rs`). Verified version: `2.3.0`. `opencode/*-free` models get an extra consent note (`src/provider/consent.rs`)
- Claude Code CLI - `claude -p --no-session-persistence --tools "" --setting-sources user --strict-mcp-config --disable-slash-commands`, hooks off via `disableAllHooks` (`src/provider/claude.rs`)
  - SDK/Client: shared runner; env set: `CLAUDE_CODE_SKIP_PROMPT_HISTORY`, `DISABLE_AUTOUPDATER`
  - Auth: the user's Claude Code user settings (for example `ANTHROPIC_BASE_URL` and token), read from `CLAUDE_CONFIG_DIR` or `~/.claude` (absolute paths only) (`src/provider/claude.rs`). Verified version: `2.1.66`
- Codex CLI - `codex exec --ephemeral --ignore-user-config --ignore-rules --sandbox read-only` with 11 features disabled (`src/provider/codex.rs`)
  - SDK/Client: shared runner; env set: `CODEX_HOME`, `CODEX_SQLITE_HOME` (both pointed at Retone's temp folder)
  - Auth: a temp copy of `auth.json` from `CODEX_HOME` or `~/.codex` (`src/provider/codex.rs`). Verified versions: none (`VERIFIED = &[]`)
- Consent: asked once per provider and model; per `host:port` for a network Local AI endpoint, with a plain-HTTP warning (`src/provider/consent.rs`)

**macOS system services:**
- Accessibility (AX) - read selection/caret and optional AX set write on the focused element (`src/mac/ax.rs`); Settings opens the System Settings pane, never `tccutil` (`src/mac/permissions.rs`)
- Core Graphics events - synthetic ⌘C/⌘V with post-event access checks (`src/mac/events.rs`)
- NSPasteboard - in-memory snapshot and `changeCount`-guarded restore (`src/mac/pasteboard.rs`)
- NSStatusItem - menu-bar item, because Rune's tray has no macOS backend (`src/mac/status.rs`)
- macOS Services - four "Retone: …" entries (`NSMessage` `rewriteSelection`, no return types, no key equivalent) in `Info.plist` (`scripts/bundle.sh`, `src/mac/services.rs`, `doc/adr/0008-services-menu-entry.md`)
- SMAppService - launch at login (`src/mac/login.rs`)
- Keychain - generic password per bundle ID (`src/mac/keychain.rs`)

**Updates and distribution:**
- Sparkle 2 (2.10.1) - appcast at `https://raw.githubusercontent.com/jellydn/retone/main/appcast.xml`, daily checks (`SUScheduledCheckInterval` 86400) (`scripts/bundle.sh`); loaded at run time only when `SUPublicEDKey` is in the bundle (`src/mac/updater.rs`)
  - SDK/Client: `Sparkle.framework` from `https://github.com/sparkle-project/Sparkle/releases/download/…` with a pinned SHA-256 (`scripts/sparkle-tools.sh`)
  - Auth: EdDSA keys `SPARKLE_PUBLIC_KEY` (repo variable) and `SPARKLE_PRIVATE_KEY` (repo secret) (`.github/workflows/release.yml`)
- Homebrew cask - `Casks/retone.rb` in this repo, livecheck on GitHub releases incl. pre-releases; when `Caskroom/retone` exists under `HOMEBREW_PREFIX`, `/opt/homebrew` or `/usr/local`, Sparkle does not start (`src/mac/updater.rs`, `src/updates.rs`, `src/main.rs`)
- GitHub Releases - DMG/ZIP/`SHA256SUMS` at `https://github.com/jellydn/retone/releases/download/v<version>/` (`.github/workflows/release.yml`, `scripts/update-distribution.sh`)

## Data Storage

**Databases:**
- None. Settings, consent and disabled-adapter flags are in Rune prefs (`prefs.json`, key by `APP_ID`) (`src/config.rs`, `src/provider/consent.rs`, `src/shared.rs`); headless runs use `Prefs::memory()` (`src/shared.rs`)
- Connection: n/a
- Client: `rune_core::prefs::Prefs`

**File Storage:**
- Local filesystem only. Per-request `0700` temp folders `retone-<pid>-<nanos>-<n>` under the system temp dir, removed on drop (`src/provider/cli.rs`). No selected or rewritten text is written to disk (`AGENTS.md`, `doc/adr/0004-providers-without-history.md`)

**Caching:**
- None (`src/provider/`)

## Authentication & Identity

**Auth Provider:**
- None of its own. Retone uses each provider's existing login (CLI `auth.json` copies, Claude user settings) or an optional Local AI Bearer key (`src/provider/opencode.rs`, `src/provider/codex.rs`, `src/provider/claude.rs`, `src/provider/local.rs`)
- Implementation: secrets stay out of prefs; the API key is in the Keychain, and `ApiKey`'s `Debug` prints `***` (`src/mac/keychain.rs`, `src/provider/local.rs`). Code signature identity: ad hoc or "Retone Release Signing", with separate dev/release bundle IDs so TCC grants and prefs are not shared (`scripts/bundle.sh`, `doc/adr/0005-distribution-and-updates.md`)

## Monitoring & Observability

**Error Tracking:**
- None (no crash or telemetry SDK in `Cargo.toml`)

**Logs:**
- stderr only: `RETONE_SPIKE_LOG=1` diagnostics with bundle IDs, lengths and rects, never text (`src/spike_log.rs`); startup errors via `eprintln!` (`src/main.rs`). Provider errors redacted (selection and home path removed, 300-char limit) (`src/provider/mod.rs`)

## CI/CD & Deployment

**Hosting:**
- GitHub Releases for assets; Homebrew tap = this repo (`Casks/retone.rb`); appcast served from `main` via raw.githubusercontent.com (`scripts/bundle.sh`)

**CI Pipeline:**
- GitHub Actions on `macos-26`, actions pinned by SHA (`actions/checkout`, `Swatinem/rust-cache`, `actions/upload-artifact`, `softprops/action-gh-release`)
- `.github/workflows/ci.yml` - push to `main`, PRs, manual: fmt, clippy `--locked`, tests, `scripts/release.sh`, uploads `target/release/assets/*` for 7 days
- `.github/workflows/release.yml` - tags `v*`: same checks, Sparkle fetch only with both keys, release build, notes from `RELEASE_NOTES.md`, publish (tags with `-` are pre-releases), then commits `Casks/retone.rb` and `appcast.xml` to `main` with `git push origin main` (`contents: write`, `concurrency: release`)
- Renovate for dependency updates (`renovate.json`)

## Environment Configuration

**Required env vars:**
- None needed to run the app. Optional runtime: `RETONE_SPIKE_LOG`, `RETONE_APPLY_OVERRIDE`, `HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOMEBREW_PREFIX` (`src/spike_log.rs`, `src/capture.rs`, `src/provider/mod.rs`, `src/provider/claude.rs`, `src/provider/codex.rs`, `src/mac/updater.rs`); eval: `RETONE_API_KEY` (`src/eval.rs`)
- Release: `SPARKLE_PUBLIC_KEY`, `SPARKLE_PRIVATE_KEY`, `SPARKLE_FRAMEWORK`, `SPARKLE_TOOLS`, `SIGNING_IDENTITY`, `GITHUB_REF_TYPE`, `GITHUB_REF_NAME` (`.github/workflows/release.yml`, `scripts/bundle.sh`, `scripts/release.sh`, `scripts/update-distribution.sh`)

**Secrets location:**
- GitHub Actions: `secrets.SPARKLE_PRIVATE_KEY`, `vars.SPARKLE_PUBLIC_KEY` (`.github/workflows/release.yml`)
- User Mac: login Keychain (Local AI key, `src/mac/keychain.rs`); CLI logins in the CLIs' own folders, copied only into temp folders (`src/provider/opencode.rs`, `src/provider/codex.rs`)
- Signing identity in the local keychain, looked up with `security find-identity` (`scripts/bundle.sh`)

## Webhooks & Callbacks

**Incoming:**
- None over the network. Local inbound: macOS Services `rewriteSelection` messages to port `NSPortName` = app name (`scripts/bundle.sh`, `src/mac/services.rs`)

**Outgoing:**
- Local AI HTTP(S) requests to the configured endpoint (`src/provider/local.rs`)
- Sparkle appcast and DMG downloads from GitHub (`scripts/bundle.sh`, `scripts/update-distribution.sh`)
- CLI providers make their own network calls; Retone does not see them (`src/provider/cli.rs`)

---

*Integration audit: 2026-11-24*
