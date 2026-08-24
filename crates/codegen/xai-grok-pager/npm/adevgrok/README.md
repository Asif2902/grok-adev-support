# ADEVGrok

ADEVGrok is an unofficial Android ARM64 distribution of the open-source
[Grok Build](https://github.com/xai-org/grok-build) terminal AI coding agent
(via the Failure Build fork). It is **not affiliated with or endorsed by
xAI**.

A full-screen TUI agent that understands your codebase, edits files, runs
shell commands, and drives long-running tasks. Bring your own provider:
x.ai's Grok models, OpenAI, Anthropic, Ollama, or any custom
OpenAI-compatible endpoint.

## Install

```sh
npm install -g adevgrok
```

Then run:

```sh
adevgrok
```

This npm package contains no binaries — `postinstall` downloads the right
prebuilt binary for your platform from
[GitHub Releases](https://github.com/Asif2902/grok-adev-support/releases),
verifies its sha256 checksum, makes it executable, and installs it to
`~/.adevgrok/bin`. If the download is skipped or fails, the first
`adevgrok` run fetches it automatically.

## Supported platforms

| Platform | Asset | Notes |
|---|---|---|
| **Android ARM64 (Termux)** | `adevgrok-<version>-android-aarch64` | The primary target — see below |
| Linux x86_64 / arm64 (glibc) | `adevgrok-<version>-linux-*` | |
| macOS Apple Silicon | `adevgrok-<version>-macos-aarch64` | Intel Macs: build from source |
| Windows x86_64 | `adevgrok-<version>-windows-x86_64.exe` | |

## Android (Termux)

There is no standalone `.apk` — ADEVGrok runs inside
[Termux](https://termux.dev/), which provides the terminal:

1. Install Termux from **[F-Droid](https://f-droid.org/en/packages/com.termux/)**
   (not the Play Store build).
2. In Termux:

   ```sh
   pkg update && pkg install nodejs ripgrep git
   npm install -g adevgrok
   adevgrok
   ```

Minimum supported Android version: **7.0 (Nougat)** — the native
`aarch64-linux-android` binary targets NDK API level 24, matching Termux's
own minimum. Note this is a Bionic-libc build; it is not interchangeable
with the glibc `linux-aarch64` asset.

## Uninstall

```sh
npm uninstall -g adevgrok
```

The downloaded binary lives outside the package in `~/.adevgrok/`; remove it
too if you want a fully clean machine:

```sh
rm -rf ~/.adevgrok        # Termux/Linux/macOS
# PowerShell: Remove-Item -Recurse -Force "$env:USERPROFILE\.adevgrok"
```

## Updating

```sh
npm update -g adevgrok
```

## Troubleshooting

- **Download failed during install** — just run `adevgrok`; it retries the
  download automatically.
- **Corporate proxy** — Node's built-in fetch ignores `HTTPS_PROXY`; set
  `ADEVGROK_SKIP_DOWNLOAD=1`, download the asset manually from the
  [releases page](https://github.com/Asif2902/grok-adev-support/releases)
  plus its `.sha256`, verify, then place it at `~/.adevgrok/bin/adevgrok`
  (`chmod +x`).
- **Different repo/fork** — set `ADEVGROK_GITHUB_REPO=<owner>/<repo>` to
  point the installer elsewhere.
- **Custom install location** — set `ADEVGROK_HOME=/some/dir`.

## Documentation

User guide (configuration, providers, MCP servers, headless mode):
[`crates/codegen/xai-grok-pager/docs/user-guide`](https://github.com/Asif2902/grok-adev-support/tree/main/crates/codegen/xai-grok-pager/docs/user-guide)

## License & attribution

Apache-2.0. Derived from xAI's Grok Build and the Failure Build fork; see
[NOTICES.md](./NOTICES.md) and
[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md). ADEVGrok's modifications
are limited to packaging/distribution.
