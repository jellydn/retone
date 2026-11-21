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
