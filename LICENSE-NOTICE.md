# Licence notice — Modern Dynamics

This port carries the original mod's assets and data and rewrites its behaviour
in Rust. The original licence and copyright notices are reproduced verbatim in
[LICENSE](LICENSE).

## Where the licence was read

| | |
|---|---|
| Pack jar | `Modern-Dynamics-0.9.6.jar` (707,745 B, SHA-256 `4357833f16a8f890e4399de3c606a27644add1384d5d1e7246648a818e9cbdab`) |
| Declared licence | `LGPLv3` in `META-INF/neoforge.mods.toml` |
| Licence text | the jar's own `LICENSE_Modern-Dynamics` (7,652 B, the full LGPL-3.0 text) (7,652 B) |
| SPDX identifier | `LGPL-3.0-only` |
| Copyright line | `Copyright (c) Technici4n and Modern Dynamics contributors` |

The jar also ships `LICENSE_Modern-Dynamics`. The jar names the author(s) `Technici4n` but
carries no separate project copyright line, so the copyright line above uses
that author list; replace it with the upstream `LICENSE` copyright statement if
one is added later.

## The port

- Original: Modern Dynamics 0.9.6 by
  Technici4n — LGPL-3.0-only
  (https://github.com/Technici4n/Modern-Dynamics).
- Port to Horizon by AdLf0208: the Rust runtime, the Horizon declarations and the
  build/verification tooling in this repository.
- This repository is the complete corresponding source: the licence requires
  that it travels with every copy and with every modified version.
- No Mojang file is included. The carried files are the mod's own assets and
  data; `data/minecraft/tags/…` files only add this mod's ids to vanilla tags.
