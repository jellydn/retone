# 0006. Local AI on private networks

Date: 2026-11-21

## Status

Accepted. Amends the loopback-only rule in [ADR 0004](0004-providers-without-history.md).

## Context

Users run their models on another machine: a desktop with a GPU on the LAN, or a server on their Tailscale network. The loopback-only rule kept the text on the Mac, but it blocked these setups.

## Decision

- An opt-in switch, off by default: "Allow LAN and Tailscale servers".
- With the opt-in, accept private addresses only: RFC 1918, `169.254/16`, `100.64/10` (Tailscale), IPv6 `fc00::/7` and `fe80::/10`. Host names are allowed. Public addresses are always refused.
- The check runs in the HTTP client's resolver, on the addresses it then connects to, so a name cannot resolve to a public address after the check. No proxy and no redirects.
- HTTPS through native-tls, with the macOS trust store, so a user's own CA works. No bundled TLS stack.
- An optional API key (llama-server `--api-key`) in the login Keychain, one item per bundle ID, sent as a Bearer token. Prefs store only a flag that a key exists, and the Keychain is read in the request worker, because a rebuilt ad-hoc signed app can get a Keychain prompt.
- Consent per host for servers outside the Mac, and the consent question says that plain http is not encrypted.

## Consequences

- The text can leave the Mac, but only to a private address the user chose and consented to.
- Plain http on a shared LAN can be read by others on that network; Retone warns, but does not block it, because Tailscale already encrypts.
- The resolver uses ureq's `unversioned` API, which can change in a ureq update; the version is pinned in `Cargo.lock`.
