# 0008. Right-click entry through macOS Services

Date: 2026-11-23

## Status

Accepted

## Context

Users want a right-click entry for selected text, like the Scramble browser extension. Scramble uses Chrome's `contextMenus` API, so it works only on web pages. A native app cannot add its own submenu to other apps' context menus. Services is the public way, and since macOS 10.6 Services have no submenus.

## Decision

- Five `NSServices` items in Info.plist: Fix Spelling and Grammar (`fix`), Improve Writing (`clear`), Summarize (`summarize`), Bullet Points (`bullets`) and TL;DR (`tldr`, added later for long pages and articles). `NSUserData` is the style id. The other styles stay in the popup, so the context menu stays short.
- Send-only: `NSSendTypes` is plain text and there is no `NSReturnTypes`. A returning service would replace the selection with no preview, make the asking app wait for the model, and could not be cancelled.
- The provider (`src/mac/services.rs`) only hands the text to the popup, which opens in that style. Capture then reads no selection and no clipboard. Replace is offered only when Accessibility is on and the focused field still selects the same text, so `apply.rs` runs its usual checks. Otherwise the popup offers Copy only.
- No `NSKeyEquivalent`: Retone has no default shortcut.

## Consequences

- Nothing changes in the asking app on failure or cancel, because the service never writes back.
- Apps without Services support (many Electron apps) show no items; the hotkey stays the main path.
- Services need no TCC grant and work with ad-hoc signing.
- When macOS launches Retone for a service, the request can arrive before the provider is registered. This must be checked on a real Mac.
