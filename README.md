<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/icon/logo-dark.svg">
    <img src="assets/icon/logo-light.svg" alt="Retone" width="360">
  </picture>
</p>

<h1 align="center">Welcome to Retone 👋</h1>

<p align="center">
  <strong>Retone</strong> rewrites the text you select in any macOS app. Press your own hotkey, pick a style, and replace the text in place — with a local model or your own AI CLI.
</p>

<div align="center">

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE) [![macOS](https://img.shields.io/badge/macOS-14+-blue.svg)](<>) [![Apple Silicon](https://img.shields.io/badge/Apple_Silicon-arm64-lightgrey.svg)](<>) [![Rust](https://img.shields.io/badge/Rust-1.99-orange.svg)](<>) [![Built with Rune](https://img.shields.io/badge/UI-Rune-purple.svg)](https://github.com/anomalyco/rune) [![CI](https://github.com/jellydn/retone/actions/workflows/ci.yml/badge.svg)](https://github.com/jellydn/retone/actions/workflows/ci.yml)

</div>

<div align="center">
  <img src="docs/screenshots/preview-example.png" alt="Retone popup with a rewrite preview" width="520"/>
</div>

> [!WARNING]
> **Beta.** Retone 0.1 is a first beta. It is tested in a small number of apps only. See [Known limits](#known-limits).

## Installation Requirements

- macOS 14 or later on Apple Silicon (tested on macOS 27 only)
- One AI provider: a local OpenAI-compatible server (Ollama, LM Studio, llama.cpp), [Pi](https://github.com/badlogic/pi-mono), or [OpenCode](https://opencode.ai)

> [!IMPORTANT]
> **First launch security warning**
>
> Retone is not signed with an Apple Developer certificate (it is ad-hoc signed). macOS can say that the app "cannot be opened".
>
> **Quick fix**: open **System Settings → Privacy & Security** and click **Open Anyway**, or run `xattr -dr com.apple.quarantine /Applications/Retone.app`.
>
> See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) for details.

## Motivation

Good writing tools often live in one app, or send your text to a cloud you did not choose. Retone works in every app where you can select text, uses the model you already run, and keeps no history: no text is saved, logged, or sent anywhere except to the provider you picked.

## Features

- ⌨️ **Your hotkey**: no default shortcut; record one in Settings, and Retone tests it with you
- 🎯 **Style shortcuts**: optional, one per style (Settings → Styles), to open the popup in that style
- 📍 **Popup at the caret**: the popup opens next to your text, or at the mouse pointer when the app does not report a caret
- ✍️ **Styles**: Clear, Concise, Professional, Technical, or your own instruction; press a number key to switch
- 👀 **Preview first**: see the rewrite before anything changes; Enter replaces, ⌘C copies, ⌘R retries, Esc cancels
- 🛡️ **Safe replace**: Retone replaces only when the same app, field and selection are still there; on failure or cancel nothing changes, and your clipboard comes back
- 🔀 **Switch provider in the popup**: click the provider label or press ⌘P to pick another provider or local model; Retone keeps the choice
- 🏠 **Local AI**: any OpenAI-compatible server on this Mac, or, after you turn it on, on your LAN or Tailscale; `http://` or `https://`
- 🧰 **CLI providers**: Pi and OpenCode, run with no sessions, no tools and no history
- 🔒 **No history**: the 2,000-character input limit, no text in logs, settings store no text
- 🎨 **Themes**: System, Dark, Light, or Kanagawa Lotus
- 🔄 **Updates**: Homebrew (`brew upgrade`), or Sparkle in builds signed for it

## Screenshots

<div align="center">
  <img src="docs/screenshots/consent-example.png" alt="First-use consent for a provider" width="380"/>
  <img src="docs/screenshots/no-permission-example.png" alt="Clipboard mode without Accessibility" width="380"/>
  <br/>
  <img src="docs/screenshots/settings-provider-local.png" alt="Settings: local provider" width="520"/>
</div>

## Installation

### Using Homebrew (Recommended)

Needs a current Homebrew (with `brew trust`); run `brew update` first if `brew trust` is unknown.

```bash
# Add the tap
brew tap jellydn/retone https://github.com/jellydn/retone

# Trust only the Retone cask (Homebrew refuses casks from untrusted taps)
brew trust --cask jellydn/retone/retone

# Install Retone
brew install --cask retone
```

Update with `brew upgrade --cask retone`. When Homebrew installed Retone, the app does not update itself.

### Direct download

1. Download `Retone-<version>.dmg` (or the `.zip`) from [Releases](https://github.com/jellydn/retone/releases). Betas are marked **Pre-release**.
2. Download `SHA256SUMS` to the same folder and check your file: `shasum -a 256 -c SHA256SUMS --ignore-missing`
3. DMG: open it and drag **Retone** to **Applications**. ZIP: double-click it to extract `Retone.app`, then move `Retone.app` to **Applications**.
4. Open Retone. If macOS blocks it, see the first launch warning above.

### From source

```bash
git clone https://github.com/jellydn/retone && cd retone
scripts/bundle.sh             # target/bundle/Retone Dev.app (dev bundle ID, ad hoc)
scripts/bundle.sh --release   # target/bundle/Retone.app
```

The pinned toolchain in `rust-toolchain.toml` installs through rustup.

## Accessibility permission

Retone reads the selection and pastes the rewrite with the macOS Accessibility API.

1. Open **Settings → General** from the Retone menu-bar item, and click **Open Settings** in the **Accessibility** row.
2. Turn on **Retone** in **System Settings → Privacy & Security → Accessibility**.

Without the permission, Retone works in **clipboard mode**: copy the text first, and Retone copies the rewrite back to the clipboard. It never pastes.

> [!NOTE]
> **Re-grant after an update.** Retone is ad-hoc signed, so macOS ties the grant to one exact build. After an update the switch can show **on** but not work. Turn Retone off and on in that list, or remove it with **−** and add it again.

## Usage

1. Select text in any app.
2. Press your hotkey (or choose **Rewrite Selection** from the Retone menu-bar item).
3. Pick a style with a number key, and wait for the preview.
4. **Enter** replaces the selection, **⌘C** copies the rewrite, **⌘R** tries again, **Esc** cancels.
5. To use another provider or model, press **⌘P** (or click the provider label at the top right), choose with **↑/↓** or a number, and press **Enter**. Retone saves the choice for the next rewrite and asks for consent first if that provider and model are new.

Retone does not run when Retone itself is in front, and refuses selections longer than 2,000 characters.

## Settings

Settings has a sidebar of pages. Each page is one card of rows, with rarely used settings under **Advanced**. The sidebar footer shows **What's new**, the version, and how this copy updates.

| Page     | What it does                                                                                                        |
| -------- | ------------------------------------------------------------------------------------------------------------------- |
| General  | Record the global hotkey and test it, default style, theme, Accessibility status, and **Launch at login**.          |
| Provider | Choose Local AI, Pi or OpenCode. For Local AI: address (with presets), LAN and Tailscale switch, API key and model. |
| Styles   | Default style and your custom instruction.                                                                          |
| Privacy  | The no-history rules, **Revoke all** consent, and turning an adapter on again after the guard.                      |

## Providers and the no-history rules

Retone asks for consent the first time it sends text to a provider and model, and saves that consent per provider and model. A Local AI server outside this Mac also gets its own consent per host.

| Provider | How Retone runs it                                                                                                                                                                                                                                                                  |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Local AI | HTTP or HTTPS to an OpenAI-compatible `/v1/chat/completions`. Presets for Ollama (`127.0.0.1:11434`), LM Studio (`127.0.0.1:1234`) and llama.cpp (`127.0.0.1:8080`). Optional API key in the Keychain. See [Local AI on your LAN or Tailscale](#local-ai-on-your-lan-or-tailscale). |
| Pi       | `pi -p --no-session` with tools, extensions and context files off, and `--thinking off` with the text on stdin, in a private temporary folder. Tested with Pi 1.5.2.                                                                                                                |
| OpenCode | `opencode run` with an in-memory database, auto-update off, and data folders in a private temporary folder that Retone deletes after the run. Tested with OpenCode 2.3.0.                                                                                                           |

### Choosing a local model

Use a 3B model or larger, such as `qwen2.5:3b` (about 2 GB). In Retone's style eval (`retone --eval-styles`), `qwen2.5:0.5b` often gave almost the same text for every style, did not shorten text for Concise, and sometimes dropped facts. `qwen2.5:3b` and `qwen2.5:7b` passed every check through Ollama and llama.cpp.

### Local AI on your LAN or Tailscale

By default, Retone sends text only to a server on this Mac (`127.0.0.1`, `localhost`, `::1`). To use a server on another machine, turn on **Settings → Provider → Allow LAN and Tailscale servers**, then enter its address, for example `http://192.168.1.20:11434/v1`, `http://100.101.1.2:11434/v1` or `https://box.tailnet.ts.net/v1`.

- Retone accepts private addresses only: `10/8`, `172.16/12`, `192.168/16`, `169.254/16`, Tailscale's `100.64/10`, and IPv6 `fc00::/7` and `fe80::/10`. Host names are allowed when every address they resolve to is private. Retone checks the addresses at connect time, uses no proxy and follows no redirects, so the text cannot go to a public address.
- Plain `http://` is not encrypted on your network. Tailscale encrypts the traffic for you; on a shared LAN, use `https://`. HTTPS uses the macOS trust store, so a certificate from your own CA works after you trust it in Keychain Access.
- The server must listen beyond loopback: for Ollama, set `OLLAMA_HOST=0.0.0.0`; in LM Studio, turn on **Serve on Local Network**.
- Retone asks for consent again for each new host.

#### llama.cpp (`llama-server`)

```sh
llama-server -m ~/models/qwen2.5-3b-instruct-q4_k_m.gguf --host 0.0.0.0 --port 8080 --api-key "$(openssl rand -hex 16)"
```

- Choose the **llama.cpp** preset (or your server's LAN or Tailscale address), then **List** in the **Model** row. llama-server serves one model and reports its file path as the ID; Retone shows only the file name. Add `--alias qwen2.5-3b` for a shorter name.
- With `--api-key`, paste the key in **API key** and click **Save**. Retone keeps it in the macOS Keychain (not in its settings file) and sends it as `Authorization: Bearer …`. Without the key, Retone says that the server needs one.
- Use `--host 0.0.0.0` only together with `--api-key` or on Tailscale: without a key, anyone on your network can use the server.

Rules for every CLI provider:

- Retone checks the CLI version and refuses versions it does not know.
- The text goes on stdin, never in arguments, so other processes cannot read it from the process list.
- Each run uses a `0700` temporary folder that Retone deletes afterwards.
- A guard compares the CLI's own data folders before and after each run (Pi: `~/.pi/agent/sessions`; OpenCode: its `~/.local/share`, `~/.local/state`, `~/.cache` and `~/.config` folders). When one changes, Retone turns that adapter off and names the folder. Other locations are not watched.
- A run that takes too long is stopped with its whole process group.
- Your provider still sees the text. For OpenCode with a hosted model, that model's provider sees it too.

## Known limits

This beta is tested on macOS 27.2 on Apple Silicon only.

- Tested apps: Slack (popup and reading work; replace has a fix that still needs a retest). TextEdit, Safari, Chrome, VS Code, Terminal and others are not tested yet.
- macOS 14 and 15, Intel Macs, displays with mixed scaling, and non-US keyboard layouts are not tested.
- The Accessibility grant does not survive updates (ad hoc signing).
- OpenCode with free hosted models is not tested.

## 📝 Documentation

- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Gatekeeper, Accessibility and Homebrew issues
- [RELEASES.md](RELEASES.md) - Release process, signing and Sparkle setup
- [RELEASE_NOTES.md](RELEASE_NOTES.md) - Changes in each release
- [Architecture Decisions](doc/adr/) - ADRs for key technical decisions
- [Agent Guidelines](AGENTS.md) - Guidelines for contributors and coding agents

## 🤝 Contributing

Contributions are welcome! Run `cargo fmt --check`, `cargo clippy --all-targets` and `cargo test` before you open a PR, and read [AGENTS.md](AGENTS.md).

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Author

👤 **Dung Huynh**

- Website: [https://productsway.com](https://productsway.com)
- Twitter: [@jellydn](https://twitter.com/jellydn)
- GitHub: [@jellydn](https://github.com/jellydn)

## Show your support

Give a ⭐️ if this project helped you!

[![kofi](https://img.shields.io/badge/Ko--fi-F16061?style=for-the-badge&logo=ko-fi&logoColor=white)](https://ko-fi.com/dunghd) [![paypal](https://img.shields.io/badge/PayPal-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://paypal.me/dunghd) [![buymeacoffee](https://img.shields.io/badge/Buy_Me_A_Coffee-FFDD00?style=for-the-badge&logo=buy-me-a-coffee&logoColor=black)](https://www.buymeacoffee.com/dunghd)
