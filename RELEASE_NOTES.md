## Retone 0.1.0-beta.10 — 2026-11-23

- VoiceOver: every help line in Settings is now read aloud with its row, including warnings and errors. Before, VoiceOver read only the row titles.
- VoiceOver hint: when your rewrite shortcut or a style shortcut uses ⌃⌥ (for example a Hyper key), Settings now says "With VoiceOver on, press VO-Tab first." VoiceOver uses ⌃⌥ for its own commands, so press VO-Tab to pass the shortcut to Retone.
- VoiceOver now reads the popup's rewrite area as text (it was an unknown element).
- For developers: the CLI provider tests no longer fail at random on macOS, and they run in under a second.

Tested with VoiceOver speech on: the popup reads the original text, the rewrite, the Retry, Copy and Replace buttons, and says "Rewrite ready" when a rewrite is done.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.9 — 2026-11-23

- The preview highlights the changed words: removed words in the original, new words in the rewrite.
- Local AI answers stream into the preview as they arrive. Cancel (Esc, ⌘R or a style key) stops the request at once.
- Optional shortcut per style in **Settings → Styles**: it opens the popup with that style already chosen.
- Replace now picks how to write for each app. Terminals (Terminal, iTerm2, Ghostty, WezTerm, kitty, Alacritty, Warp) offer Copy only, because a pasted newline can run a command. Slack, Discord, Notion and VS Code wait longer before the clipboard is restored.
- Popup accessibility: VoiceOver labels for the provider, styles, original text, rewrite and buttons, and announcements when a rewrite is ready or fails.
- Claude Code as a provider, verified for no-history use with Claude Code 2.1.66. Other versions are refused until they pass the check.
- Codex as a provider (not yet verified, so Retone refuses it for now). It runs with a private, temporary `CODEX_HOME`, so it does not use or change your `~/.codex`.
- For developers: `scripts/cli-history-check.sh` checks that a CLI version (Pi, OpenCode, Claude Code, Codex) writes no history before it is marked verified.
- All UI text now comes from an English catalog (`locales/app/en.json`), ready for translations.

Not tested yet in this release: Replace with the AX set strategy in Safari and Chrome (only reachable with the developer override `RETONE_APPLY_OVERRIDE`), and the popup with VoiceOver speech on. The accessibility tree was checked.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.8 — 2026-11-22

- Fix: the selected text always wins. Retone reads the selection first (Accessibility, then ⌘C with the clipboard restored) and uses the clipboard only when nothing is selected. Clipboard text offers Copy only, never Replace.
- If an app reports a selection that Retone cannot read, Retone now shows an error instead of rewriting unrelated clipboard text.
- The popup says when it used the clipboard, and why: nothing was selected, or Accessibility is off.
- Hint: if Retone keeps using the clipboard although text is selected, the Accessibility grant is stale after an update. The switch can show on but not work. Remove Retone in System Settings → Privacy & Security → Accessibility, add it again, then open Retone again.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.7 — 2026-11-22

- Fix: in Settings, the window's close, minimize and zoom buttons no longer cover the first sidebar item. The sidebar now starts below them.
- The Retone logo and name now head the Settings sidebar.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.6 — 2026-11-21

- New Settings design: a sidebar with General, Provider, Styles and Privacy. Each page is one card of rows with an icon, a short help line and the control on the right, with rarely used settings under **Advanced**. The sidebar footer shows **What's new**, the version, and how this copy updates.
- **Launch at login** switch in **Settings → General**. macOS keeps this setting (System Settings → General → Login Items).
- Accessibility status and **Open Settings** are now in **Settings → General** (the Permissions page is gone).
- Address and Custom style open their own page; provider, default style and theme are dropdowns.
- Rune UI update.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.5 — 2026-11-21

- Fix: Replace works in Slack and other Chromium and Electron apps. They report no focused field while the Retone popup is open, so Retone now reads the captured field again. It replaces only when that field still has focus and still holds the same selection.
- Local AI on your LAN or Tailscale: turn on **Settings → Provider → Allow LAN and Tailscale servers** (off by default). Retone accepts private addresses only, checks them when it connects, and asks for consent again for each new host.
- HTTPS for Local AI, with the macOS trust store.
- llama.cpp: a `llama-server` preset (`127.0.0.1:8080`), and an optional API key (`--api-key`), kept in the macOS Keychain. The Llama menu-bar app works through its address, for example `http://100.102.105.7:9931/v1`.
- New style prompts: each style has its own rules and an example, so Clear, Concise, Professional and Technical now give clearly different rewrites, with no invented greetings, sign-offs or details.
- For developers: `retone --eval-styles` runs sample texts through every style and checks the results.
- Use a local model of 3B or larger (for example `qwen2.5:3b`). Smaller models often ignore the style.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.4 — 2026-11-21

- Fix: Replace was refused in every app with "another app is in front". The check before the paste asked macOS for the focused app while the Retone popup had focus. It now checks the field in the target app, and the live frontmost-app check is the last check before ⌘V.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating (System Settings → Privacy & Security → Accessibility: remove Retone, then add it again).

Update: `brew update && brew upgrade --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.3 — 2026-11-21

The v0.1.0-beta.2 tag failed to build and was never published, so this release carries its changes.

- App icon, menu-bar icon and logo (the rewrite loop with a text cursor), also used as the DMG volume icon.
- Switch provider and model from the popup: press ⌘P or click the provider label. Retone keeps the choice and asks for consent first for a new provider and model.
- Fix: after a failed Replace, Retry or a style change no longer offers Replace again (it could insert the text twice).
- Fix: the release build mounts its DMG at an explicit mount point (beta.2 failed there).
- Homebrew: the cask uses `depends_on macos: :sonoma`; install needs `brew trust --cask jellydn/retone/retone`.

Still ad-hoc signed without automatic updates: grant Accessibility again after updating. See the README for known limits.

Install (run `brew update` first): `brew tap jellydn/retone https://github.com/jellydn/retone && brew trust --cask jellydn/retone/retone && brew install --cask retone`, or download the DMG below.

## Retone 0.1.0-beta.1 — 2026-11-21

First beta. Retone rewrites the selected text in any app with your own hotkey.

- Popup at the caret (mouse pointer fallback) with a preview before anything changes.
- Styles: Clear, Concise, Professional, Technical, and a custom instruction.
- Providers: local OpenAI-compatible servers on loopback, Pi and OpenCode, with no-history safeguards.
- Safe replace: same app, field and selection only; nothing changes on failure or cancel; the clipboard is restored.
- Clipboard mode when Accessibility is not granted.
- Settings: shortcut recorder, provider, style, theme, privacy, permissions, updates.

Known limits: ad-hoc signed (Gatekeeper warning; Accessibility must be granted again after each update), no automatic updates in this build, tested on macOS 27.2 on Apple Silicon and in few apps. See the README.

Install (run `brew update` first): `brew tap jellydn/retone https://github.com/jellydn/retone && brew trust --cask jellydn/retone/retone && brew install --cask retone`, or download the DMG below.
