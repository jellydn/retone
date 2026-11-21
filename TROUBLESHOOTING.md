# Troubleshooting Guide

## "Retone" Not Opened — Gatekeeper warning

> **"Retone" Not Opened.** Apple could not verify "Retone" is free of malware…

Retone is ad-hoc signed, not signed with an Apple Developer ID, and not notarized. Gatekeeper blocks such apps when they come from the internet.

**Method 1: System Settings (recommended)**

1. Open Retone and close the warning.
2. Open **System Settings → Privacy & Security**.
3. In **Security**, click **Open Anyway** next to the Retone message, then **Open**.

**Method 2: Terminal**

```bash
xattr -dr com.apple.quarantine /Applications/Retone.app
```

Only do this for a copy you downloaded from https://github.com/jellydn/retone/releases and checked with `SHA256SUMS`.

## The Accessibility switch is on, but Retone cannot read or replace text

macOS ties the Accessibility grant to the exact signature of the app. An ad-hoc signed app gets a new signature with each build, so after an update the old grant no longer applies, even though the switch shows **on**.

1. Open **System Settings → Privacy & Security → Accessibility**.
2. Select **Retone** and click **−** to remove it.
3. Click **+** and add `/Applications/Retone.app` again (or open Retone and use **Settings → Permissions**).
4. Quit Retone from the **Rt** menu and open it again.

## The popup opens at the mouse pointer, not at the text

Some apps (for example Electron apps such as Slack) do not report where the caret is. Retone then opens the popup at the mouse pointer. This is expected.

## "Nothing changed" after Enter

Retone replaces the text only when the same app, the same field and the same selection are still in front. If you switched apps, typed, or changed the selection while the rewrite ran, Retone does not paste. Use **⌘C** to copy the rewrite and paste it yourself.

## My clipboard changed

Retone saves the clipboard before it copies or pastes, and puts it back afterwards. If another app writes to the clipboard during a rewrite, Retone keeps that newer content and does not restore. File a bug if the clipboard changes in other cases.

## The hotkey does nothing

- Open **Settings → Shortcut** and record the hotkey again. Retone shows which app holds a chord when macOS tells it.
- The hotkey does nothing while Retone's own window is in front.

## Homebrew

- `Error: Refusing to load cask jellydn/retone/retone from untrusted tap`: run `brew trust --cask jellydn/retone/retone` (note the full name, `jellydn/retone/retone`), then install again. If you trusted a misspelled name, list your entries with `brew trust --json v1` and remove the wrong one, for example `brew untrust --cask jellydn/retone/reton` or `brew untrust --tap jellydn/reton`. If `brew trust` is unknown, run `brew update` first.
- `brew install --cask retone` fails with a checksum error right after a release: wait a few minutes and run `brew update`; the cask updates after the release assets upload.
- Homebrew installs do not update themselves. Use `brew upgrade --cask retone`.

## Provider errors

- **Local**: the endpoint must be on loopback (`127.0.0.1`, `localhost` or `::1`), and the model ID must match exactly. Use the model list in **Settings → Provider**.
- **Pi / OpenCode**: Retone accepts only the CLI versions it was tested with. If Retone turned an adapter off because the CLI saved files, the message names the data folder that changed. Look in that folder for new session or history files and remove them only if they hold text you rewrote. Then report the CLI version in an issue.
