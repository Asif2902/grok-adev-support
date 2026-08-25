# ADEVGrok — License & Attribution Notices

ADEVGrok is an **unofficial** redistribution of the Grok Build terminal AI
coding agent. It is **not affiliated with, endorsed by, or sponsored by xAI,
OpenAI, or Anthropic.**

> ADEVGrok is an unofficial Android ARM64 distribution based on the
> open-source Grok Build project. It is not affiliated with or endorsed by
> xAI.

## Provenance and modifications (Apache-2.0 §4(b))

The software in the ADEVGrok binary derives from the following lineage:

1. **Grok Build** — originally open-sourced by **xAI** at
   <https://github.com/xai-org/grok-build>, licensed under the Apache
   License, Version 2.0.
2. **Failure Build** — a community fork of Grok Build (bring-your-own-provider
   support; Android ARM64 target). Licensed under Apache-2.0.
3. **ADEVGrok** — this distribution (`Asif2902/grok-adev-support`), which
   packages the existing Android ARM64 build for installation via npm.

ADEVGrok's modifications to that upstream code are confined to packaging,
distribution, and presentation: renaming the user-facing CLI command from
`failure` to `adevgrok`, renaming release artifacts, adding an npm installer
that downloads release binaries, updating user-facing documentation and
branding strings (including the welcome-screen logo and product name), and
changing the default per-user storage directory from `~/.failure` to
`~/.adevgrok`. Agent functionality (login, task execution, tools, MCP) is
otherwise unchanged from the fork it was built from. This file constitutes
the prominent notice of those changes required by Apache License 2.0 §4(b).

## First-party license

First-party code is licensed under the **Apache License, Version 2.0**
(<http://www.apache.org/licenses/LICENSE-2.0>). The full license text ships
with the source repository as [`LICENSE`](https://github.com/Asif2902/grok-adev-support/blob/main/LICENSE)
and is also available from <http://www.apache.org/licenses/LICENSE-2.0>.

Copyright remains with the respective upstream authors:
xAI (Grok Build) and Failure Build fork contributors.

## Third-party components

The binary embeds third-party code and prebuilt tool binaries under their
original licenses (Apache-2.0, MIT, and others), described in full — with
license texts and change notices — in
[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md), included verbatim in
this package.

Additional notices covering vendored dependencies and UI themes are kept in
the source repository ([`THIRD-PARTY-NOTICES`](https://github.com/Asif2902/grok-adev-support/blob/main/THIRD-PARTY-NOTICES),
[`third_party/NOTICE`](https://github.com/Asif2902/grok-adev-support/blob/main/third_party/NOTICE)).
