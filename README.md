<div align="center">

<img src="docs/media/docs/media/social-preview.png" alt="ADEVGrok — terminal AI coding agent (unofficial Grok Build distribution)." width="100%">

<h1>
  ADEVGrok (<code>adevgrok</code>)
</h1>

[![Release](https://github.com/Asif2902/grok-adev-support/actions/workflows/release.yml/badge.svg)](https://github.com/Asif2902/grok-adev-support/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-Apache--2.0-red.svg)](LICENSE)
[![Built with Rust](https://img.shields.io/badge/built%20with-Rust-red.svg)](https://www.rust-lang.org)

**ADEVGrok** is an unofficial distribution of a terminal-based AI coding
agent, based on xAI's open-sourced Grok Build (via the Failure Build fork).
It runs as a full-screen TUI that understands your
codebase, edits files, executes shell commands, searches the web, drives a
real browser, runs structured git operations (status, diff, log, commit),
and manages long-running tasks — interactively, headlessly for
scripting/CI, or embedded in editors via the Agent Client Protocol (ACP). It is bring-your-own-provider:
x.ai's Grok models, OpenAI, Anthropic, Ollama, or any custom
OpenAI-compatible endpoint — configurable via CLI flags, the `/provider`
slash command, or `config.toml`, with the full model catalog from every
configured provider merged in automatically. It can also serve itself to
remote MCP clients (like Claude) via the built-in `/mcp start` bridge.

> **ADEVGrok is an unofficial Android ARM64 distribution based on the
> open-source Grok Build project. It is not affiliated with or endorsed by
> xAI.** The packaged program's agent functionality is unchanged from the
> upstream open-source code; packaging, branding, the default storage
> directory, and distribution differ. See
> [`LICENSE`](LICENSE) and [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES)
> for attribution.

[Installing the released binary](#installing-the-released-binary) ·
[Building from source](#building-from-source) ·
[Changelog](#changelog) ·
[Documentation](#documentation) ·
[Repository layout](#repository-layout) ·
[Development](#development) ·
[Contributing](#contributing) ·
[License](#license)

![ADEVGrok welcome screen](docs/media/welcome-demo.gif)

This repository contains the Rust source for the CLI/TUI agent and its
agent runtime, forked from the `SOURCE_REV` monorepo commit of xAI's Grok
Build recorded at the repository root. The user-facing command is
`adevgrok`; internal crate and binary names retain their upstream
identifiers.

</div>

---

## Installing

```sh
npm install -g adevgrok
```

Then launch:

```sh
adevgrok
```

The npm package ships no binaries — on install it downloads the right
prebuilt binary from this repo's [GitHub Releases](https://github.com/Asif2902/grok-adev-support/releases)
(including the native **Android ARM64** build for Termux), verifies its
sha256 checksum, and installs it to `~/.adevgrok/bin`. See the
[package README](crates/codegen/xai-grok-pager/npm/adevgrok/README.md)
for details, Android/Termux walkthrough, minimum supported Android version
(**7.0**, NDK API level 24), and uninstall instructions:

```sh
npm uninstall -g adevgrok
```

Alternatively, install a release binary directly:

```sh
curl -fsSL https://raw.githubusercontent.com/Asif2902/grok-adev-support/main/crates/codegen/xai-grok-pager/scripts/install.sh | bash
```

On Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/Asif2902/grok-adev-support/main/crates/codegen/xai-grok-pager/scripts/install.ps1 | iex
```

Both installers pull binaries from this repo's releases,
built by `.github/workflows/release.yml` as
`adevgrok-<version>-<platform>` assets for linux-x86_64/arm64,
macos-aarch64 (Apple Silicon), windows-x86_64, and android-aarch64. macOS
Intel (x86_64) isn't built — GitHub no longer reliably provisions hosted
Intel Mac runners — so Intel Mac users should fall back to
[Building from source](#building-from-source) below.

**Android** has no general-purpose terminal, so there's no standalone
`.apk` — instead, the android-aarch64 build runs inside
[Termux](https://termux.dev/), which provides one. On Termux just run
`npm install -g adevgrok`; for the manual download steps see
[`INSTALL.md`](INSTALL.md).

For copy-pasteable install commands for every platform (including the full
Android/Termux walkthrough and custom-provider setup), see
[`INSTALL.md`](INSTALL.md).

## Building from source

Requirements:

- **Rust** — the toolchain is pinned by [`rust-toolchain.toml`](rust-toolchain.toml);
  `rustup` installs it automatically on first build.
- **[DotSlash](https://dotslash-cli.com)** — required so hermetic tools under
  [`bin/`](bin/) (notably [`bin/protoc`](bin/protoc)) can download and run.
  Install it and ensure `dotslash` is on your `PATH` **before** building:

  ```sh
  cargo install dotslash
  # or: prebuilt packages — https://dotslash-cli.com/docs/installation/
  /usr/bin/env dotslash --help   # sanity check
  ```

- **protoc** — proto codegen resolves [`bin/protoc`](bin/protoc) via DotSlash,
  or falls back to a `protoc` on `PATH` / `$PROTOC`.
- macOS and Linux are supported build hosts; Windows builds are best-effort
  and not currently tested from this tree.

```sh
cargo run -p xai-grok-pager-bin              # build + launch the TUI
cargo build -p xai-grok-pager-bin --release  # release binary: target/release/xai-grok-pager
cargo check -p xai-grok-pager-bin            # fast validation
```

The binary artifact is named `xai-grok-pager`; official installs ship it as
`adevgrok`. On first launch, pick a provider (x.ai, OpenAI, Anthropic,
Ollama, or a custom endpoint) — see the
[authentication guide](crates/codegen/xai-grok-pager/docs/user-guide/02-authentication.md).

## Changelog

New features and bug fixes are tracked in [`docs/feed.xml`](docs/feed.xml), an
RSS feed hand-curated per commit (not autogenerated). Subscribe via the raw
URL:

```text
https://raw.githubusercontent.com/Asif2902/grok-adev-support/main/docs/feed.xml
```

## Documentation

This fork's user guide is the authoritative documentation (below).
[docs.x.ai/build/overview](https://docs.x.ai/build/overview) is xAI's
upstream Grok Build documentation — useful background on shared concepts,
but it describes the upstream product and may not match this fork.

The user guide ships with the pager crate:
[`crates/codegen/xai-grok-pager/docs/user-guide/`](crates/codegen/xai-grok-pager/docs/user-guide/)
— getting started, keyboard shortcuts, slash commands, configuration, theming,
MCP servers, skills, plugins, hooks, headless mode, sandboxing, and more.

## Repository layout

| Path | Contents |
|------|----------|
| `crates/codegen/xai-grok-pager-bin` | Composition-root package; builds the `xai-grok-pager` binary |
| `crates/codegen/xai-grok-pager` | The TUI: scrollback, prompt, modals, rendering |
| `crates/codegen/xai-grok-shell` | Agent runtime + leader/stdio/headless entry points |
| `crates/codegen/xai-grok-tools` | Tool implementations (terminal, file edit, search, ...) |
| `crates/codegen/xai-grok-workspace` | Host filesystem, VCS, execution, checkpoints |
| `crates/codegen/...` | The rest of the CLI crate closure (config, MCP, markdown, sandbox, ...) |
| `crates/common/`, `crates/build/`, `prod/mc/` | Small shared leaf crates pulled in by the closure |
| `third_party/` | Vendored upstream source (Mermaid diagram stack) — see below |

> [!IMPORTANT]
> The root `Cargo.toml` (workspace members, dependency versions, lints,
> profiles) is **generated** — treat it as read-only. Prefer editing per-crate
> `Cargo.toml` files.

## Development

```sh
cargo check -p <crate>        # always target specific crates; full-workspace builds are slow
cargo test -p xai-grok-config # per-crate tests
cargo clippy -p <crate>       # lint config: clippy.toml at the repo root
cargo fmt --all               # rustfmt.toml at the repo root
```

## Contributing

> [!NOTE]
> External contributions are not accepted. See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## License

First-party code in this repository is licensed under the **Apache License,
Version 2.0** — see [`LICENSE`](LICENSE).

Third-party and vendored code remains under its original licenses. See:

- [`THIRD-PARTY-NOTICES`](THIRD-PARTY-NOTICES) — crates.io / git dependencies,
  bundled UI themes, and **in-tree source ports** (including openai/codex and
  sst/opencode tool implementations)
- [`crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md`](crates/codegen/xai-grok-tools/THIRD_PARTY_NOTICES.md)
  — crate-local notice for the codex and opencode ports (license texts +
  Apache §4(b) change notice)
- [`third_party/NOTICE`](third_party/NOTICE) — vendored Mermaid-stack index
