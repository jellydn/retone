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
