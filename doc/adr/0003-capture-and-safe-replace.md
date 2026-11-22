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
- The write strategy comes from a per-app table (`src/strategy.rs`), chosen once at capture: paste with restore by default (a longer settle time for Electron apps), Copy only for terminals, and AX set only for apps the app matrix proves. AX set checks that `AXSelectedText` is settable, writes it, and reads the same range back; a failed read-back is an error and never falls back to a paste, which could insert the text twice. `RETONE_APPLY_OVERRIDE=bundle.id=paste|ax-set|copy-only,…` tries a strategy without a rebuild (a rebuild needs a new Accessibility grant).

## Consequences

- Some apps refuse a replace that would have worked; the user can still copy the rewrite with ⌘C.
