## Retone 0.1.0-beta.1 — 2026-11-21

First beta. Retone rewrites the selected text in any app with your own hotkey.

- Popup at the caret (mouse pointer fallback) with a preview before anything changes.
- Styles: Clear, Concise, Professional, Technical, and a custom instruction.
- Providers: local OpenAI-compatible servers on loopback, Pi and OpenCode, with no-history safeguards.
- Safe replace: same app, field and selection only; nothing changes on failure or cancel; the clipboard is restored.
- Clipboard mode when Accessibility is not granted.
- Settings: shortcut recorder, provider, style, theme, privacy, permissions, updates.

Known limits: ad-hoc signed (Gatekeeper warning; Accessibility must be granted again after each update), no automatic updates in this build, tested on macOS 27.2 on Apple Silicon and in few apps. See the README.

Install: `brew tap jellydn/retone https://github.com/jellydn/retone && brew trust --cask jellydn/retone/retone && brew install --cask retone`, or download the DMG below.
