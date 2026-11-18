# Modern Dynamics for Horizon

A native Horizon port of **Modern Dynamics 0.9.6** (`Technici4n/Modern-Dynamics`,
**LGPL-3.0-only**): the **item pipe** and its **extractor** attachment. Items
move between the inventories a pipe network joins, 4 at a time, every 40 ticks.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8);
a published port travels under LGPL-3.0 with its complete corresponding source
(this repository).

- Format: one flat **SDK package** (`horizon-mod.toml` + `Cargo.toml` + `src/` +
  `assets/` + `data/`), mod API **0.2**, built by `cargo horizon-mod`.
- Pin: SDK nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4`, mixins
  `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, game 26.3.
- Class/tier: `content` / `machines`; approach per `horizon/docs/modpacks.md`
  (block entity + its own tick + item transfer, not a data-only carry).
- Behaviour, gaps and verification: [notes.md](notes.md); upstream analysis:
  [FINDINGS.md](FINDINGS.md); content provenance:
  `../_shared/carry-work/moderndynamics/SOURCE.md`.

## Layout

```text
horizon-mod.toml                   The only manifest and the only identity
Cargo.toml                         One crate; horizon-mod-api from .horizon/sdk/
src/lib.rs                         The one impl Hooks: the pipe's BlockPart + BlockEntityPart
src/runtime.rs                     Pure pipe logic and its 10 tests (byte-identical to 0.1.0)
assets/moderndynamics/             Carried assets: blockstate, models, textures, lang, items
data/moderndynamics/               Carried data: block, item, creative tab, recipe, loot table
tools/e2e.sh                       Fail-capable in-game pipe scenario (needs a launcher-built game)
moderndynamics.hmod                The delivered package (port root, never target/)
LICENSE, LICENSE-NOTICE.md         Inside the package
```

## Build and verify

```sh
cd ports/moderndynamics
cargo horizon-mod build            # fetches .horizon/sdk/ on first use; packs target/horizon-mod/
cp target/horizon-mod/moderndynamics.hmod .
cargo horizon-mod check moderndynamics.hmod
cargo horizon-mod fits moderndynamics.hmod --build nightly
```

Run the compile steps through a build slot (`CARGO_INCREMENTAL=0`, no
`--target-dir`):

```sh
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-md --port ports/moderndynamics --step test \
  -- bash -c 'cd ports/moderndynamics && cargo test'
```

The in-game scenario needs a game built by the launcher against this SDK
(`horizon-launcher native-build`), which this machine does not have; see
`notes.md`'s gap list.
