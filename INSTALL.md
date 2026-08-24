# Installing ADEVGrok

ADEVGrok (`adevgrok`) is a terminal-based AI coding agent, based on xAI's
open-sourced Grok Build (via the Failure Build fork). It's bring-your-own-provider:
x.ai's Grok models, OpenAI, Anthropic, Ollama, or any custom OpenAI-compatible endpoint
(including a local server on your own network).

> **ADEVGrok is an unofficial distribution based on the open-source Grok
> Build project. It is not affiliated with or endorsed by xAI.**

All platform binaries are published on the
[GitHub Releases page](https://github.com/Asif2902/grok-adev-support/releases/latest).

---

## Recommended: install with npm (all platforms)

```sh
npm install -g adevgrok
```

Then launch:

```sh
adevgrok
```

The package downloads the correct prebuilt binary for your platform at
install time, verifies its sha256 checksum, and installs it to
`~/.adevgrok/bin`. Uninstall with:

```sh
npm uninstall -g adevgrok        # optionally: rm -rf ~/.adevgrok
```

---

## Linux (x86_64 / arm64)

```sh
curl -fsSL https://raw.githubusercontent.com/Asif2902/grok-adev-support/main/crates/codegen/xai-grok-pager/scripts/install.sh | bash
```

Or manually, picking the right asset for your CPU:

```sh
# x86_64
curl -fL --progress-bar -o adevgrok "https://github.com/Asif2902/grok-adev-support/releases/latest/download/adevgrok-<version>-linux-x86_64"
# arm64
curl -fL --progress-bar -o adevgrok "https://github.com/Asif2902/grok-adev-support/releases/latest/download/adevgrok-<version>-linux-aarch64"

chmod +x adevgrok
./adevgrok
```

---

## macOS (Apple Silicon / arm64)

Same installer script as Linux:

```sh
curl -fsSL https://raw.githubusercontent.com/Asif2902/grok-adev-support/main/crates/codegen/xai-grok-pager/scripts/install.sh | bash
```

Or manually:

```sh
curl -fL --progress-bar -o adevgrok "https://github.com/Asif2902/grok-adev-support/releases/latest/download/adevgrok-<version>-macos-aarch64"
chmod +x adevgrok
./adevgrok
```

**Intel Macs (x86_64):** not built by the release pipeline (GitHub no longer
reliably provisions hosted Intel Mac runners) — build from source instead
(see [Building from source](README.md#building-from-source) in the main
README).

---

## Windows (x86_64)

PowerShell:

```powershell
irm https://raw.githubusercontent.com/Asif2902/grok-adev-support/main/crates/codegen/xai-grok-pager/scripts/install.ps1 | iex
```

Or manually:

```powershell
Invoke-WebRequest -Uri "https://github.com/Asif2902/grok-adev-support/releases/latest/download/adevgrok-<version>-windows-x86_64.exe" -OutFile adevgrok.exe
.\adevgrok.exe
```

---

## Android (via Termux)

Android has no general-purpose terminal, so there's no standalone `.apk` —
ADEVGrok runs inside [Termux](https://termux.dev/) instead, which
provides a real terminal plus a Linux userland.

**Minimum supported Android version: 7.0 (Nougat)** — the binary is a
native `aarch64-linux-android` build targeting NDK API level 24, matching
Termux's own minimum.

### Option A: npm (recommended)

1. Install Termux from **[F-Droid](https://f-droid.org/en/packages/com.termux/)**
   — not the Play Store version, which is outdated and unmaintained.
2. In Termux:

   ```sh
   pkg update && pkg install nodejs ripgrep git
   npm install -g adevgrok
   adevgrok
   ```

The npm installer detects Termux and downloads the native
`adevgrok-<version>-android-aarch64` asset automatically. Updating later is
just `npm update -g adevgrok`.

### Option B: manual download

```sh
pkg update && pkg install ripgrep git
mkdir -p ~/adevgrok-app
cd ~/adevgrok-app
curl -fL --progress-bar --retry 10 --retry-delay 3 \
  -o adevgrok "https://github.com/Asif2902/grok-adev-support/releases/latest/download/adevgrok-<version>-android-aarch64"
chmod +x adevgrok
./adevgrok
```

(Replace `<version>` with the version from the
[latest release](https://github.com/Asif2902/grok-adev-support/releases/latest),
e.g. `adevgrok-0.1.220-alpha.4-android-aarch64`.)

After this first run, launch it again anytime with:

```sh
~/adevgrok-app/adevgrok
```

### Notes / known limitations on Android

- The npm-installed binary lives in `~/.adevgrok/bin`; the manual-download
  one lives wherever you put it. Either works; use one or the other.
- This is a native `aarch64-linux-android` build, distinct from the
  `linux-aarch64` build above (different C library — Bionic vs. glibc — not
  interchangeable).
- **Clipboard and microphone dictation report "unavailable"** — they compile
  and run fine, they just don't do anything yet. Real Termux support for
  these (via `termux-clipboard-get/set` and `termux-microphone-record`, from
  the separate Termux:API app) isn't wired up.
- If a download drops mid-transfer over a flaky connection, delete the
  partial file and retry:
  ```sh
  rm adevgrok
  # then re-run the curl command above
  ```

### Using a custom / self-hosted provider on Android

If you want to point ADEVGrok at a custom OpenAI-compatible endpoint
(your own API, a local llama.cpp-style server on your LAN, etc.) instead of
x.ai/OpenAI/Anthropic/Ollama:

```sh
mkdir -p ~/.failure
cat >> ~/.failure/config.toml <<'EOF'
[provider.custom]
base_url = "https://your-endpoint.example.com/v1"

[model.your-model-name]
provider = "custom"
EOF

adevgrok login --provider custom --api-key YOUR_API_KEY
adevgrok --model your-model-name
```

For a **local server on your own Wi-Fi network** (e.g. something serving an
OpenAI-compatible API at `http://192.168.1.50:8080`), same idea — use its LAN
address as the `base_url` (usually with a `/v1` suffix), and pass any
placeholder string as the API key if the local server doesn't check one:

```sh
mkdir -p ~/.failure
cat >> ~/.failure/config.toml <<'EOF'
[provider.local]
base_url = "http://192.168.1.50:8080/v1"

[model.local]
provider = "local"
EOF

adevgrok login --provider local --api-key none
adevgrok --model local
```

Your phone must be on the same network as whatever's hosting that server.

---

## First launch (all platforms)

On first launch with no provider configured, ADEVGrok walks you through
picking one interactively (x.ai, OpenAI, Anthropic, Ollama, or custom). For
x.ai specifically, you can skip the picker with an API key from
[console.x.ai](https://console.x.ai):

```sh
export XAI_API_KEY="xai-..."
```

To use a **named BYOP provider** directly (bypassing the picker) on any
platform, store a key once:

```sh
adevgrok login --provider openai --api-key sk-...
```

then launch normally — it remembers the choice.
