# 0003. Capture order and safe replace

Date: 2026-11-21

## Status

Accepted

## Context

Users must be able to trust that Retone never edits the wrong text and never loses their clipboard.

## Decision

- Capture: AX `AXSelectedText` first; then ⌘C posted to the target app with a pasteboard snapshot and restore; then clipboard-only mode when Accessibility is missing.
- Replace only when the frontmost app is the same pid and the target still matches: the same selected range, and the same element or the same selected text read again (Electron apps return a new element object on each query).
- Paste with a transient pasteboard item, then restore the snapshot only if `changeCount` shows no other app wrote meanwhile.
- On failure or cancel, nothing changes. The hotkey with Retone in front is refused.

## Consequences

- Some apps refuse a replace that would have worked; the user can still copy the rewrite with ⌘C.
