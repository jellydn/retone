# 0005. Distribution, signing and updates

Date: 2026-11-21

## Status

Accepted

## Context

There is no Apple Developer ID. The app needs Accessibility, which macOS ties to the code signature. Users install from GitHub Releases or a Homebrew cask in this repo.

## Decision

- Releases are ad-hoc signed, or signed with a stable self-signed "Retone Release Signing" certificate when available, which keeps the Accessibility grant across updates.
- Assets: DMG, ZIP and `SHA256SUMS` from `scripts/release.sh`; tags with `-` publish as GitHub pre-releases.
- Sparkle 2 is loaded at run time from the bundle, and only when the bundle has `SUPublicEDKey`. Without a key the release ships without Sparkle, never with unverified updates.
- Exactly one update owner: when Homebrew installed Retone, Sparkle does not start and Settings points to `brew upgrade`.
- The release workflow writes `Casks/retone.rb` (and `appcast.xml` with a key) to `main` after publishing.

## Consequences

- Without the certificate, users must grant Accessibility again after each update.
- Sparkle needs the `SPARKLE_PUBLIC_KEY` variable and `SPARKLE_PRIVATE_KEY` secret (see RELEASES.md).
