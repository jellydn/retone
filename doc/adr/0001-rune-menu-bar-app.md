# 0001. Rust and Rune for a menu-bar app

Date: 2026-11-21

## Status

Accepted

## Context

Retone needs a small popup that opens at the caret in any app, a Settings window, a global hotkey and a menu-bar item. FolderLens showed that Rune can make native-feeling macOS windows in Rust, with headless rendering for screenshots and tests.

## Decision

- Start from Rune's `examples/hello` and its conventions; pin one Rune `dev` revision for Rune and all its patches.
- An accessory app (`LSUIElement`): a launcher panel for the popup, a hidden Settings window, and a native `NSStatusItem` because Rune's tray has no macOS backend yet.
- Pure logic (the popup flow, placement, input limits, config) stays free of AppKit and has unit tests.

## Consequences

- Rune bumps must move all pinned crates together.
- The menu-bar item and Sparkle are small objc2 modules under `src/mac/`.
