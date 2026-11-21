# Releases

## Make a release

1. Set `version` in `Cargo.toml` (for example `0.1.0-beta.2`), and run `cargo build` to update `Cargo.lock`.
2. Add a section at the top of [RELEASE_NOTES.md](RELEASE_NOTES.md) that starts with `## Retone <version> — <date>`.
3. Merge to `main`, then push the tag:

   ```bash
   git tag v0.1.0-beta.2 && git push origin v0.1.0-beta.2
   ```

The [Release workflow](.github/workflows/release.yml) then:

1. runs fmt, clippy and the tests;
2. builds `Retone.app` with `scripts/release.sh` and makes `Retone-<version>.dmg`, `Retone-<version>.zip` and `SHA256SUMS`;
3. publishes the GitHub release. A tag with `-` (such as `-beta.1`) is a **pre-release** and is not marked latest;
4. commits the new `Casks/retone.rb` (and `appcast.xml` when Sparkle is set up) to `main`.

The tag must match the `Cargo.toml` version, or the build stops.

## Build the assets locally

```bash
sh scripts/release.sh        # target/release/assets
```

## Signing

Releases are **ad-hoc signed** unless a stable certificate is available. With ad hoc signing, macOS asks for the Accessibility grant again after each update.

`scripts/bundle.sh --release` uses a codesigning identity named **Retone Release Signing** when the keychain has one, or `SIGNING_IDENTITY` when set. A stable self-signed certificate keeps the Accessibility grant across updates. CI does not import a certificate yet.

## Sparkle (automatic updates)

Sparkle is optional. Without a key the release has no automatic updates, because Sparkle must never install an update it cannot verify. Homebrew installs never use Sparkle.

To turn Sparkle on (once):

1. Download Sparkle 2.10.1 and make a key for Retone only:

   ```bash
   sh scripts/sparkle-tools.sh                         # prints the tools folder
   target/sparkle-2.10.1/bin/generate_keys --account retone
   target/sparkle-2.10.1/bin/generate_keys --account retone -x retone-sparkle-private.key
   ```

2. Add the **public** key as a repository variable, and the **private** key as a secret:

   ```bash
   gh variable set SPARKLE_PUBLIC_KEY --body "<public key from generate_keys>"
   gh secret set SPARKLE_PRIVATE_KEY < retone-sparkle-private.key
   rm retone-sparkle-private.key
   ```

The next release then embeds Sparkle, sets `SUFeedURL` to `appcast.xml` on `main`, and signs the DMG for the appcast. Keep a backup of the private key: if it is lost, installed copies cannot verify new updates.
