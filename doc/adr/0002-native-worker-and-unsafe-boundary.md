# 0002. A native worker thread and one `unsafe` boundary

Date: 2026-11-21

## Status

Accepted

## Context

The pasteboard and AX element handles are not `Send`. The UI runs on Rune's thread, and provider calls must not block it.

## Decision

- A worker thread (`src/native.rs`) owns the pasteboard and the captured AX element for the whole capture → rewrite → replace cycle, and replies to the popup with messages.
- `unsafe` is denied crate-wide and allowed only in `src/mac/` leaf modules, each block with a `// SAFETY:` comment. No `unsafe impl Send`.
- Carbon Pasteboard is not used: it crashed when tests ran in parallel. `NSPasteboard` only.

## Consequences

- All native work is serialized on one thread, which also makes the clipboard snapshot and restore easy to reason about.
