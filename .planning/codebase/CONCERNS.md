# Codebase Concerns

**Analysis Date:** 2026-11-24 (main at `59f2b3f`, release 0.1.0-beta.11)

Yardstick: the "Rules (do not break)" list in `AGENTS.md`. Each item was checked against the
current code. Severity: **high** = can break main or a rule now; **medium** = real failure in a
plausible scenario; **low** = small risk or cost.

## Status (2026-11-24 follow-up)

| # | Status |
|---|--------|
| 1, 2, 11 | Fixed in #45 (CI step "One Rune revision in Cargo.toml", `rune-i18n` aligned and in the Renovate group, comments fixed). Renovate #42/#33 closed; the bump is done by hand. |
| 3, 12 | Fixed in #46 (key and push token only in the last release step; timeouts 60/45 min). |
| 4 | In progress: late ⌘C restore. |
| 5 | Documented in #49 (README, ADR 0003). No setting added. |
| 6 | Fixed in #47 (with a regression test). |
| 7 | In progress: resolve CLIs through the login shell, Settings path as fallback. |
| 8 | Partly fixed in #48: only the main hotkey uses up the test. The flag still stays set if Settings closes without a press. |
| 9, 10, 14 | Open, low. |
| 13 | Not changed: the dev build is unlocked on purpose; release and CI use `--locked`. |

## Summary

| # | Sev | Location | Problem | Smallest fix |
|---|-----|----------|---------|--------------|
| 1 | high | `renovate.json:10-20`, `Cargo.toml:37-42` | Renovate does not bump `[patch.crates-io]` pins; open PR #42 repeats the #22 break | CI check for one Rune rev; bump patch lines by hand on the Renovate branch |
| 2 | medium | `renovate.json:10-20`, `Cargo.toml:14,24` | `rune-i18n` is outside the group, on a second rev; PRs #33 and #42 move it separately | Add `rune-i18n` to the group; align it to the Rune rev |
| 3 | medium | `.github/workflows/release.yml:7-8,21-22,24` | Sparkle private key and write token are visible to every step, including dependency build scripts | Step-level env for the key; `persist-credentials: false`; split build and publish |
| 4 | medium | `src/capture.rs:187-197` | ⌘C that lands after 500 ms overwrites the clipboard with no restore | Watch longer after the timeout and restore if the count moves |
| 5 | medium | `src/capture.rs:172-197` | ⌘C fallback puts the selection on the pasteboard without the transient marker: clipboard managers keep it | Document it; optional "no ⌘C fallback" setting |
| 6 | medium | `src/provider/mod.rs:99-107`, `src/provider/local.rs:267-274` | Local provider errors are not redacted or length-capped, but the comment says they are | Pass the local result through `redact()` |
| 7 | medium | `src/provider/mod.rs:136-146` | CLIs are found by bare name on `PATH`; a Finder launch has no Homebrew or `~/.local/bin` on `PATH` | Resolve absolute paths once (known dirs or login shell) |
| 8 | low | `src/popup/mod.rs:143-146`, `src/settings/mod.rs:307-310` | An armed hotkey test that is never pressed swallows the next hotkey or Service request (and the Service text) | Clear the flag when Settings hides; consume only on the main hotkey |
| 9 | low | `src/mac/ax.rs:163-183`, `src/apply.rs:43` | AX-set read-back failure says "failed" but the text may have changed | Keep as is; word the error "may have changed, check the field" |
| 10 | low | `src/popup/view.rs:234-237` | Word diff (O(n·m), up to ~640k cells) runs on every view build while Ready | Compute once when the answer lands; keep it in `Popup` |
| 11 | low | `Cargo.toml:10,17` | Stale comments: "one Rune revision" and "Plain HTTP only / TLS never needed" | Fix the comments |
| 12 | low | `.github/workflows/ci.yml:17`, `release.yml:16` | No `timeout-minutes` on macOS jobs (default 360 min) | `timeout-minutes: 45` |
| 13 | low | `scripts/bundle.sh:40` | Dev build without `--locked` can change `Cargo.lock` silently | Add `--locked` |
| 14 | low | `src/spike_log.rs:4-5`, `src/strategy.rs:27-40` | Spike logger marked "remove after Phase 2"; no AX-set entries yet | Track as a Phase 2 task |

## Tech Debt

**Two Rune revisions in one manifest (item 2, item 11):**
- Issue: `rune`, `rune-kit`, `rune-core` and all six `[patch.crates-io]` pins use
  `3d4e2dfc…` (9 lines). `rune-i18n` uses `3f0d9973…` (`Cargo.toml:14`, `:24`). The comment at
  `Cargo.toml:10` says "One Rune `dev` revision for Rune and every patch below".
- Files: `Cargo.toml:10-14,24,37-42`, `renovate.json:10-20`
- Impact: Harmless today. In `Cargo.lock`, `rune-i18n` depends on no other Rune crate, and
  there is only one `rune-core`. But the rule in the comment is false, and Renovate has two
  independent PRs for the same repo: #33 (`renovate/rune-i18n-digest`, i18n → `03a3705`) and
  #42 (`renovate/rune`, rune/kit/core → `03a3705`). If you merge only one, the two revs move apart again.
  `crates/rune-i18n` exists at `3d4e2df` (in the local cargo checkout), so you can align it.
- Fix approach: add `"rune-i18n"` to `matchDepNames` in `renovate.json`. Move both
  `rune-i18n` lines to the Rune rev in the same PR, and confirm with `cargo build --locked`.

**Stale `ureq` comment (item 11):**
- Issue: `Cargo.toml:17` says "Plain HTTP only … TLS is never needed". But `native-tls-no-default`
  is turned on and `src/provider/local.rs:155` builds a TLS config for LAN HTTPS servers.
- Impact: Someone may remove the TLS feature "as unused" and break HTTPS endpoints.
- Fix approach: change the comment to "HTTP or HTTPS to loopback/private hosts; native-tls for the macOS trust store".

**Spike diagnostics still in the product (item 14):**
- Issue: `src/spike_log.rs:4-5` says "Remove once the app matrix (Phase 2) replaces the spikes".
  `src/strategy.rs` has no `AxSet` entries ("AX set entries come only from the app matrix").
- Impact: None for privacy today: every `spike!` call prints only lengths, bundle IDs, ranges and
  `t!()` error strings (checked: `capture.rs:39-135`, `apply.rs:17-67`, `popup/mod.rs:388`).
  But each new `spike!` is a place where text could leak, and no test guards it.
- Fix approach: keep it, but track Phase 2. Optionally add a unit test or review note that
  `spike!` arguments never include `captured.text`/`rewrite`.

## Known Bugs

**Late ⌘C overwrites the user's clipboard (item 4):**
- Symptoms: The user's clipboard is replaced by the selected text and is not restored. Also, when
  the app reports no AX range, Retone uses the *old* clipboard text as input (Copy only, with
  the "from clipboard" note).
- Files: `src/capture.rs:187-197` (`COPY_WAIT` = 500 ms at `:18`)
- Trigger: A slow app (Electron, a busy web page) that has no AX selection range handles the
  posted ⌘C after more than 500 ms. `copy_selection` returns `Ok("")` at `:190`. After that,
  nothing watches the pasteboard, so `restore_if_unchanged` never runs for the late write.
  This breaks "restore the clipboard unless another app changed it": the change is Retone's own ⌘C.
- Workaround: none for the user.
- Fix: after the timeout, keep the snapshot and poll up to about 1 s more. If `change_count` moves,
  restore (the write is Retone's ⌘C) and return an error such as `capture.copy_slow`, not an
  empty selection. Needs a real-app test with the dev bundle.

**Armed hotkey test swallows real requests (item 8):**
- Symptoms: The user clicks Test in Settings, then closes Settings without pressing the
  shortcut. The next hotkey press does nothing visible. A Services menu request is also dropped,
  with its text.
- Files: `src/settings/mod.rs:307-310` (`set_testing(true)`; nothing sets it back to false),
  `src/popup/mod.rs:143-146` (`open()` consumes the flag for hotkey, style hotkey *and* Service).
- Trigger: Test → close Settings → use a Service or a hotkey.
- Fix: call `set_testing(false)` when the Settings window hides or the page changes. Check
  `take_testing()` only in `on_hotkey`, not in `open()`.

**CLI providers not found when launched from Finder (item 7):**
- Symptoms: "Pi/OpenCode/Claude Code not found" although the CLI works in Terminal.
- Files: `src/provider/mod.rs:136-146` (`pi_program: "pi".into()` and the others), used by
  `Command::new(run.program)` in `src/provider/cli.rs:154`.
- Trigger: Retone starts from Finder, a login item or Homebrew. launchd gives
  `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, which does not include `/opt/homebrew/bin`,
  `~/.local/bin`, `~/.bun/bin` or npm global folders. No code changes `PATH` (`rg PATH src` finds only a comment).
- Workaround: start Retone from a shell.
- Fix: resolve each program once to an absolute path. Search known folders, or run
  `$SHELL -lc 'command -v NAME'` with a short deadline. Show the resolved path in Settings.
  This also closes a small security gap: today the first `pi` on `PATH` gets the text on stdin.
  The version gate checks only its `--version` output.

## Security Considerations

**Release job exposes secrets to all build code (item 3):**
- Risk: `SPARKLE_PRIVATE_KEY` is set in job-level `env` (`release.yml:21-22`), so every step can
  read it: `cargo clippy`, `cargo test`, `release.sh` → `cargo build`, and every `build.rs` and
  proc macro from crates.io and from the git-pinned Rune revs. `permissions: contents: write`
  (`:7-8`) plus the default `persist-credentials: true` on checkout (`:24`) also leaves a
  push-capable token in `.git/config` for the same steps. A compromised dependency can then sign
  a malicious appcast entry or push to `main`.
- Current mitigation: actions are pinned by SHA. Tag-only trigger. Sparkle downloads are
  SHA-256 checked (`scripts/sparkle-tools.sh:16`).
- Recommendations (smallest first): (1) move `SPARKLE_PRIVATE_KEY` to the `env` of the
  "Update cask and appcast" step only. `sparkle-tools.sh` and the build need only the public
  key. (2) Add `with: persist-credentials: false` to checkout, and give the token to the push
  step only (`git push https://x-access-token:${{ github.token }}@github.com/...`). (3) Later,
  split the work into a `build` job (`contents: read`, uploads assets) and a `publish` job
  (`contents: write`, no cargo).

**⌘C fallback leaks the selection into clipboard-manager history (item 5):**
- Risk: When AX gives no text, `copy_selection` posts ⌘C (`src/capture.rs:184`). The *target
  app* writes the selection to the general pasteboard without the
  `org.nspasteboard.TransientType` marker. Clipboard managers (Maccy, Raycast, Alfred) record it
  before Retone restores the clipboard. The paste path marks its own write as transient
  (`src/mac/pasteboard.rs:110-117`), but Retone cannot mark another app's write. So the text
  goes into a third-party history on disk, which conflicts with the README's "no history" claim.
- Current mitigation: AX is tried first. The clipboard is restored afterwards.
- Recommendations: document this in README and ADR 0003. `rg` finds no mention. Optionally add a
  Privacy setting "Do not use ⌘C" that turns the fallback off (capture then fails with a clear
  message).

**Local provider errors are not redacted (item 6):**
- Risk: `status_message` (`src/provider/local.rs:269-274`) puts the server's
  `error.message` in the popup. The comment says "the caller redacts it anyway", but `redact()`
  runs only for CLI results (`src/provider/cli.rs:80`; `rg 'redact\('` finds no other caller).
  `Provider::rewrite` gives the local result through unchanged (`src/provider/mod.rs:99-107`).
  A server that echoes the prompt in a validation error (context length, template errors) shows
  user text with no `ERROR_LIMIT` (300 chars) cap, and VoiceOver reads it aloud
  (`popup/mod.rs:327-334`). Nothing is written to disk, so no rule is broken. But the promise in
  `provider/mod.rs:5` ("its errors are redacted") is not kept.
- Recommendations: in `provider/mod.rs:99`, map the error:
  `.map_err(|e| redact(&e, text, env.home.to_str()))`.

**Checked and OK (no concern):**
- No history: no `eprintln!`/`println!` prints selected or rewritten text in the app paths.
  `eval.rs` and `history_check.rs` print sample text only in dev CLI modes (documented). Prefs
  hold consent keys, model IDs, endpoint and adapter-disabled reasons only
  (`config.rs:57-77`, `consent.rs`). The API key is in the Keychain (`mac/keychain.rs`), and
  `ApiKey` Debug is masked (`local.rs:34-38`).
- Panics: no `unwrap()`/`expect()` in non-test code. Index use is guarded (`styles.rs:293`).
  `diff.rs:18` caps the table size.
- `unsafe`: only in `src/mac/` modules with `#[allow(unsafe_code)]` (`mac/mod.rs`). Every
  `unsafe {}` block has a `// SAFETY:` comment. The crate denies `unsafe_code` and has no
  `unsafe impl`.
- 2,000-character limit: every entry path (hotkey, style hotkey, menu item, Service with and
  without Accessibility) ends in `Event::Captured` → `Flow::captured` → `input::check`
  (`flow.rs:248`) before any `Request` effect.
- Nothing changes on failure or cancel: `paste_over` restores on every early return after its
  write (`apply.rs:62-76`). The flow emits `Apply` only from `Previewing{Ready, can_replace:true}`
  and turns Replace off after a failed apply (`flow.rs:214-225`).
- Local network rules: loopback/private check in the resolver at connect time
  (`local.rs:141-150`). `.proxy(None)`, `.max_redirects(0)` (`:159-160`). `user@host` refused
  (`:58`). Consent key includes the host (`consent.rs:13-20`).
- UI text: view code uses `t!()`. The only literals are the brand name `"Retone"`
  (`popup/view.rs:97`, `settings/view.rs:69`) and CLI product names (`provider/mod.rs:46-49`).

## Performance Bottlenecks

**Word diff on every view build (item 10):**
- Problem: `preview_body` calls `crate::diff::changed_words(original, rewrite)` each time the
  view is built while the phase is `Ready` (`src/popup/view.rs:234-237`).
- Cause: LCS table of `words(original) × words(rewrite)` u32 cells. Worst case is input 2,000
  chars (~400 words) and answer cap 8,000 chars (~1,600 words) = ~640k cells (~2.5 MB) allocated
  per build. Hover, scroll and key events rebuild the view.
- Improvement path: compute the ranges once when `Event::Answer` lands, and store them next to
  `Phase::Ready` or in `Popup`.

## Fragile Areas

**Capture and replace timing (`capture.rs`, `apply.rs`, `strategy.rs`):**
- Files: `src/capture.rs`, `src/apply.rs`, `src/strategy.rs`, `src/mac/ax.rs`
- Why fragile: fixed waits (`COPY_WAIT` 500 ms, `SERVICE_FOCUS_WAIT` 500 ms, paste settle
  400/700 ms, AX timeout 0.25 s) tuned per app. The frontmost check and ⌘V are not atomic
  (`apply.rs:63-64` says so). The AX-set read-back can fail after a write that landed (item 9,
  `ax.rs:160-183`). The flow then shows "Replace failed" while the field may have changed.
- Safe modification: change one wait at a time. Test in the dev bundle with the apps in
  `strategy.rs`. Use `RETONE_APPLY_OVERRIDE` before you add table entries.
- Test coverage: only `selection_or_clipboard` and the strategy table have unit tests.
  `copy_selection`, `paste_over`, `service_capture` and `still_matches` have none.

**Settings and popup size:**
- Files: `src/settings/view.rs` (750 lines), `src/settings/mod.rs` (580), `src/popup/mod.rs` (472),
  `src/popup/view.rs` (435). Total `src/` is 8,148 lines.
- Why fragile: Settings view and state hold all pages. Changes to one page re-render the
  scenes of all pages.
- Safe modification: render all `settings::SCENES` with `--snap` after each change. Split by
  page when a page grows.

## Scaling Limits

**CLI runner:**
- Current capacity: one request at a time per popup. Stdout capped at `answer_cap*4+1024` bytes,
  stderr at 16 KiB (`provider/cli.rs:23,100`). Deadline 60 s.
- Limit: no practical limit for one user.
- Scaling path: none needed.

## Dependencies at Risk

**Rune git revs with `[patch.crates-io]` (item 1, high):**
- Risk: Renovate's cargo manager does not update `[patch.crates-io]` entries. The group in
  `renovate.json:10-20` lists `winit`, `sctk-adwaita`, `parley_engine`, `wgpu-hal`,
  `accesskit_*`, but these names never match. Evidence: open PR #42 (`fix(deps): update rune to
  03a3705`) changes only `Cargo.toml:11-13`. The six patch pins at `Cargo.toml:37-42` stay on
  `3d4e2df`. This is the same shape as #22, which broke main and was fixed by `d778594` ("left
  the [patch.crates-io] pins … at 935d040, so … main did not build"). Its CI `check` was still
  pending when this audit ran.
- Impact: if #42 is merged as is, the new `rune` may call winit or parley APIs the old patch pins
  do not have, and main fails to build.
- Migration plan:
  1. Before merging #42, push a commit to `renovate/rune` that moves the six patch lines (and
     `rune-i18n`, see item 2) to `03a3705…`, then run `cargo update -p winit` (or a full
     `cargo build`) so that `Cargo.lock` follows.
  2. Add a CI check. **Yes, it helps, and it can be one step.** Put it in `ci.yml` before clippy,
     so a Renovate PR fails in seconds:
     ```yaml
     - name: One Rune revision in Cargo.toml
       run: |
         revs=$(grep -o 'anomalyco/rune", rev = "[0-9a-f]*"' Cargo.toml | sort -u)
         [ "$(printf '%s\n' "$revs" | wc -l)" -eq 1 ] || { echo "Rune revs differ:"; echo "$revs"; exit 1; }
     ```
     Today this check fails (2 revs: `3d4e2df` ×9, `3f0d997` ×2). Align `rune-i18n` first, or
     leave `rune-i18n` out with `grep -v rune-i18n` if you decide to keep it separate. A
     `Cargo.lock` variant (`grep -o 'anomalyco/rune?rev=[0-9a-f]*' Cargo.lock | sort -u`)
     catches the same drift after resolution.
  3. In `renovate.json`, remove the six patch crate names (they have no effect). Add
     `rune-i18n`. Change the description to say the patch pins need a manual bump that the CI
     check enforces. A regex `customManagers` entry could bump the patch lines, but Renovate
     would not update `Cargo.lock` for it, and `--locked` would fail anyway.

**Ad-hoc signed releases:**
- Risk: CI has no "Retone Release Signing" identity, so `bundle.sh:108-112` signs ad hoc. Each
  update loses the Accessibility grant (README:116, :189). Gatekeeper blocks first launch
  (README:35).
- Impact: after each update, Replace and the AX capture silently stop working until the user
  grants access again. Users then fall back to clipboard-only mode.
- Migration plan: import a stable self-signed certificate into the CI keychain from a secret
  (ADR 0005 allows it). Later, use Developer ID and notarization.

## Missing Critical Features

**Stable code signature in CI builds:**
- Problem: see above. Release builds made on GitHub Actions are always ad hoc.
- Blocks: keeping the Accessibility grant across Sparkle or Homebrew updates.

## Test Coverage Gaps

**Native capture and replace paths:**
- What's not tested: `copy_selection` timeout and restore, `paste_over` early-return restores,
  `service_capture` matching, `Target::still_matches`.
- Files: `src/capture.rs`, `src/apply.rs`, `src/mac/ax.rs`
- Risk: a change breaks "nothing changes on failure" without a failing test. Item 4 is an
  example. A `Board::unique()` pasteboard (already used in `pasteboard_tests.rs`) can cover the
  restore logic if the ⌘C/⌘V posting goes behind a trait.
- Priority: High

**Rune revision consistency:**
- What's not tested: that all Rune git revs in `Cargo.toml` match (item 1).
- Files: `Cargo.toml`, `.github/workflows/ci.yml`
- Risk: a Renovate PR breaks main (happened with #22; #42 is the same shape).
- Priority: High

**Distribution scripts:**
- What's not tested: the SemVer guard in `scripts/update-distribution.sh:17-29`, and the
  release-notes `awk` in `release.yml`.
- Files: `scripts/update-distribution.sh`, `.github/workflows/release.yml`
- Risk: a wrong compare stops cask updates (it exits 0 with "not changed"), and nobody notices.
- Priority: Low

**Hotkey test flag:**
- What's not tested: that a Service or a later hotkey is not consumed by a stale test (item 8).
- Files: `src/popup/mod.rs`, `src/settings/mod.rs`
- Risk: lost Service requests.
- Priority: Low

---

*Concerns audit: 2026-11-24*
