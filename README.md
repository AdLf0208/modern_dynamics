# Modern Dynamics — a Horizon port

**Modern Dynamics** is a Minecraft mod. Modern item and fluid transport Minecraft mod. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Technici4n/Modern-Dynamics](https://github.com/Technici4n/Modern-Dynamics) — licence **LGPL-3.0-only**.

| | |
|---|---|
| Upstream mod | Modern Dynamics ([source](https://github.com/Technici4n/Modern-Dynamics)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `moderndynamics.hmod` (install it with the Horizon launcher) |
| Licence | **LGPL-3.0-only** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **The block.** `moderndynamics:item_pipe` (template `minecraft:stone`,
- **Connections.** A side connects to another item pipe, or to a block with an
- **The extractor.** `moderndynamics:extractor` (carried texture and model,
- **Transfer.** Every **40 ticks** (`getItemOperationTickDelay` with no
- **Readout.** `/moderndynamics info <x> <y> <z>` prints the connections, the
- **Reviewer:** [packaging defect found while repairing] the `.hnmod` did not contain its `data/` folder
- **Reviewer:** LGPL-3.0 source review is the reviewer's limitation
- **Reviewer:** licence notice

## Not yet ported

- **fluid pipe** and the five **energy cables** (LV/MV/HV/EV/superconductor)
- **attractor**, **filter** and **inhibitor** attachments, the **wrench**
- the **routing modes** other than CLOSEST (FURTHEST, RANDOM, ROUND_ROBIN —
- **item travel** rendering, the pipe's own textures on the arms per type,
- the **JEI/EMI/REI/Jade** plugins, the debug tool and the config.
- **Fluid pipe, the five energy cables (LV…EV, superconductor) and the machine
- **Attractor, filter and inhibitor** attachments, the **wrench** (connection
- **Routing modes** other than `CLOSEST` (FURTHEST, RANDOM, ROUND_ROBIN),

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `moderndynamics.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Modern Dynamics was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **LGPL-3.0-only**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Technici4n/Modern-Dynamics

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run moderndynamics` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/moderndynamics/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
