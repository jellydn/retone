# Technology Stack

**Analysis Date:** 2026-11-24

## Languages

**Primary:**
- Rust, edition 2024 (`Cargo.toml` `edition = "2024"`), toolchain 1.100.0 (`rust-toolchain.toml`) - all app code in `src/`, plus the build script `build.rs`

**Secondary:**
- POSIX shell (`#!/bin/sh`) - bundling, release, Sparkle download, cask/appcast update, CLI history gate, icon build (`scripts/bundle.sh`, `scripts/release.sh`, `scripts/sparkle-tools.sh`, `scripts/update-distribution.sh`, `scripts/cli-history-check.sh`, `scripts/make-icons.sh`)
- Python 3 (no version pin) - icon SVG generation (`scripts/icon-svgs.py`) and the SemVer compare inside `scripts/update-distribution.sh`
- Ruby (Homebrew Cask DSL) - `Casks/retone.rb`
- JSON - UI text catalog `locales/app/en.json` and `locales/locales.json`, compiled at build time (`build.rs`)
- XML (plist, RSS) - `Info.plist` written by `scripts/bundle.sh`; Sparkle `appcast.xml` written by `scripts/update-distribution.sh`

## Runtime

**Environment:**
- Native macOS binary, macOS 14.0+ (`LSMinimumSystemVersion` in `scripts/bundle.sh`; `depends_on macos: :sonoma` in `Casks/retone.rb`)
- Apple Silicon only: `lipo ... -verify_arch arm64` in `scripts/release.sh`; `depends_on arch: :arm64` in `Casks/retone.rb`
- Menu-bar agent app: `LSUIElement` true in the generated `Info.plist` (`scripts/bundle.sh`)

**Package Manager:**
- Cargo (bundled with Rust 1.100.0, `rust-toolchain.toml`)
- Lockfile: present (`Cargo.lock`, lockfile format `version = 4`, 491 packages); CI uses `--locked` (`.github/workflows/ci.yml`)

## Frameworks

**Core:**
- Rune (git `https://github.com/anomalyco/rune`, rev `3d4e2dfcecb71890f7f4d6167e82fe6db9dbb695`, crate version 0.0.1 in `Cargo.lock`) - UI framework: windows, popup, Settings, prefs store (`rune_core::prefs::Prefs` in `src/config.rs`, `src/shared.rs`), env and clock helpers (`clippy.toml`), headless `--snap`/`--scene` rendering (`src/main.rs`)
- rune-kit, rune-core (same rev, 0.0.1) - kit components and core helpers (`Cargo.toml`)
- rune-i18n (git rev `3d4e2dfcecb71890f7f4d6167e82fe6db9dbb695`, the shared Rune rev, 0.1.0) - `t!()`/`key!()` message catalog, checked at build time (`src/i18n.rs`, `build.rs`, `doc/adr/0007-localization.md`)
- objc2 0.6.5 family (objc2-foundation 0.3.2, objc2-app-kit 0.3.2, objc2-core-foundation 0.3.2, objc2-core-graphics 0.3.2, objc2-application-services 0.3.2, objc2-service-management 0.3.2) - AppKit, AX, CGEvent, pasteboard, status item, SMAppService bindings, macOS-only target deps (`Cargo.toml`, `src/mac/`)

**Testing:**
- Built-in Rust test harness (`cargo test --locked`); no dev-dependencies in `Cargo.toml`. Test files sit next to code: `src/flow_tests.rs`, `src/placement_tests.rs`, `src/provider/cli_tests.rs`, `src/provider/local_tests.rs`, `src/mac/pasteboard_tests.rs`, plus inline `#[cfg(test)]` modules (for example `src/provider/mod.rs`, `src/provider/claude.rs`)
- Headless scene renders for visual checks: `target/debug/retone --list-scenes`, `--scene NAME --snap out.png` (`AGENTS.md`, `src/main.rs`)
- Style prompt eval against a local model: `retone --eval-styles` (`src/eval.rs`, `AGENTS.md`)
- CLI no-history release gate: `scripts/cli-history-check.sh` runs `retone --cli-check` (`src/history_check.rs`)

**Build/Dev:**
- rustfmt (`rustfmt.toml`: `max_width = 120`, `use_small_heuristics = "Max"`)
- clippy (`clippy.toml`: Rune's shared lints; disallows `std::env::var`, `SystemTime::now`, `Instant::now`); crate lints in `Cargo.toml` `[lints.rust]` (`unsafe_code = "deny"`, `unsafe_op_in_unsafe_fn = "deny"`) and `[lints.clippy]` (`undocumented_unsafe_blocks = "deny"`, `unwrap_used`/`expect_used` warn)
- Build script `build.rs` - turns `locales/` into `$OUT_DIR/i18n.rs` with `rune_i18n::codegen::generate`
- macOS tools used by scripts: `codesign`, `security`, `ditto`, `hdiutil`, `SetFile`, `lipo`, `shasum` (`scripts/bundle.sh`, `scripts/release.sh`); `rsvg-convert`, `iconutil` (`scripts/make-icons.sh`); `curl`, `tar` (`scripts/sparkle-tools.sh`)
- Renovate (`renovate.json`: `config:recommended`, groups Rune and its patched crates as `rune`)

## Key Dependencies

**Critical:**
- `rune`/`rune-kit`/`rune-core` at one git rev - the whole UI and prefs layer (`Cargo.toml`)
- `[patch.crates-io]` - `winit` (0.30.13), `sctk-adwaita`, `parley_engine` (0.12.0), `wgpu-hal` (30.0.1), `accesskit_windows`, `accesskit_macos` (0.27.2) all replaced by forks in the Rune repo at rev `3d4e2df…` (`Cargo.toml`, versions from `Cargo.lock`); Renovate must bump them with Rune (`renovate.json`)
- `objc2-application-services` 0.3.2 - AX reads/writes on the focused element (`src/mac/ax.rs`)
- `objc2-core-graphics` 0.3.2 - synthetic ⌘C/⌘V events (`src/mac/events.rs`)
- `objc2-app-kit` 0.3.2 - pasteboard snapshot/restore, status item, screens (`src/mac/pasteboard.rs`, `src/mac/status.rs`, `src/mac/screens.rs`)
- `ureq` 3.5.2 (default features off; `json`, `native-tls-no-default`) - HTTP(S) client for the Local AI provider, with a custom resolver, no proxy, no redirects (`Cargo.toml`, `src/provider/local.rs`)

**Infrastructure:**
- `serde` 1.0.229 (derive), `serde_json` 1.0.154 - prefs values, provider JSON, SSE chunks (`Cargo.toml`, `src/config.rs`, `src/provider/local.rs`)
- `security-framework` 3.7.0 - Keychain generic password for the Local AI API key (`src/mac/keychain.rs`)
- `native-tls` 0.2.18 (via ureq) - TLS with the macOS trust store (`RootCerts::PlatformVerifier` in `src/provider/local.rs`)
- `objc2-service-management` 0.3.2 - launch at login via `SMAppService.mainApp` (`src/mac/login.rs`)
- Sparkle 2.10.1 - not a Cargo dependency; downloaded with a pinned SHA-256 (`scripts/sparkle-tools.sh`), embedded by `scripts/bundle.sh`, loaded at run time (`src/mac/updater.rs`)
- Note: `Cargo.lock` also holds objc2 0.5.3 / objc2-foundation 0.2.2 / objc2-app-kit 0.2.2 (transitive, older family)

## Configuration

**Environment:**
- User settings live in Rune prefs (`prefs.json`) under `APP_ID` (`io.github.jellydn.retone` release, `io.github.jellydn.retone.dev` debug) (`src/config.rs`); consent and disabled adapters also in prefs (`src/provider/consent.rs`)
- Local AI API key in the login Keychain, service `APP_ID`, account `local-api-key` (`src/mac/keychain.rs`)
- Runtime env vars (names only): `RETONE_SPIKE_LOG` (`src/spike_log.rs`), `RETONE_APPLY_OVERRIDE` (`src/capture.rs`, `src/strategy.rs`), `RETONE_API_KEY` (eval only, `src/eval.rs`), `HOME` (`src/provider/mod.rs`), `CLAUDE_CONFIG_DIR` (`src/provider/claude.rs`), `CODEX_HOME` (`src/provider/codex.rs`), `HOMEBREW_PREFIX` (`src/mac/updater.rs`)
- Build/release env vars (names only): `SPARKLE_FRAMEWORK`, `SPARKLE_PUBLIC_KEY`, `SIGNING_IDENTITY` (`scripts/bundle.sh`); `SPARKLE_PRIVATE_KEY` (`scripts/update-distribution.sh`); `GITHUB_REF_TYPE`, `GITHUB_REF_NAME` (`scripts/release.sh`)
- No default hotkey; the user records a Rune accelerator string (`Config::hotkey` in `src/config.rs`)

**Build:**
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `build.rs`, `clippy.toml`, `rustfmt.toml`, `renovate.json`
- `Info.plist` generated inline by `scripts/bundle.sh` (bundle ID, version from `Cargo.toml`, `NSServices`, Sparkle keys only with `SPARKLE_PUBLIC_KEY`)
- App icon assets prebuilt and committed in `assets/icon/` (`scripts/make-icons.sh`)

## Platform Requirements

**Development:**
- macOS on Apple Silicon with Rust 1.100.0 plus clippy and rustfmt (`rust-toolchain.toml`)
- Accessibility grant for the dev bundle `Retone Dev.app` to test capture/replace (`AGENTS.md`, `scripts/bundle.sh`)
- Optional: `rsvg-convert`, `iconutil`, `python3` for icons (`scripts/make-icons.sh`); the CLIs `pi`, `opencode`, `claude`, `codex` for the history gate (`scripts/cli-history-check.sh`)

**Production:**
- `Retone.app` (arm64, macOS 14+) shipped as DMG and ZIP with `SHA256SUMS` on GitHub Releases (`scripts/release.sh`, `.github/workflows/release.yml`) and through the Homebrew cask `Casks/retone.rb`
- Ad-hoc signed, or signed with a self-signed "Retone Release Signing" identity when present; no Developer ID (`scripts/bundle.sh`, `doc/adr/0005-distribution-and-updates.md`)

---

*Stack analysis: 2026-11-24*
