# Modern Dynamics for Horizon

A native Horizon port of **Modern Dynamics 0.9.6** (`Technici4n/Modern-Dynamics`,
**LGPL-3.0-only**): the **item pipe** with its **extractor** and **attractor**
attachments. An extractor pulls items out of the inventory on its side into the
pipe network; an attractor pulls them out of the network into the inventory on
its side. Either way 4 items move per operation, every 40 ticks.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8);
a published port travels under LGPL-3.0 with its complete corresponding source
(this repository).

- Format: one flat **SDK package** (`horizon-mod.toml` + `Cargo.toml` + `src/` +
  `assets/` + `data/`), mod API **0.2**, built by `cargo horizon-mod`.
- Pin: SDK nightly `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`, mixins
  `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, game 26.3.
- Class/tier: `content` / `machines`; approach per `horizon/docs/modpacks.md`
  (block entity + its own tick + item transfer, not a data-only carry).
- Behaviour, gaps and verification: [notes.md](notes.md); upstream analysis:
  [FINDINGS.md](FINDINGS.md); **art provenance: [PROVENANCE.md](PROVENANCE.md)**
  (the upstream art is CC-BY-NC-SA-4.0 and is *not* carried: the textures are
  drawn, the data files are rewritten, and the checker asserts 0 jar matches).

## Layout

```text
horizon-mod.toml                   The only manifest and the only identity
Cargo.toml                         One crate; horizon-mod-api from .horizon/sdk/
src/lib.rs                         The one impl Hooks: the pipe's BlockPart + BlockEntityPart
src/runtime.rs                     Pure pipe logic and its tests (no game access)
assets/moderndynamics/             The port's own art (drawn) and its blockstate, models, lang, items
data/moderndynamics/               The port's own data: block, items, creative tab, recipes, loot table
tools/gen_textures.py              Draws the four textures (deterministic, `--check`)
tools/assemble_content.py          Writes the models, the loot table and the three recipes (own bytes)
tools/check_provenance.py          Proves 0 packaged files match a jar entry; writes PROVENANCE.md
tools/recipe_check.py              Crafts every carried recipe in game, with a wrong-grid control
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
`--target-dir`; `BUILD_LOCAL=1` because the SDK tool is only installed here):

```sh
BUILD_LOCAL=1 ports/tools/build-slot.sh run --job sdk-md --port ports/moderndynamics --step test \
  -- bash -c 'cd ports/moderndynamics && cargo test'
```

The in-game scenario builds a game that carries the package with the launcher and
runs it (`ports/tools/launcher-game.sh run moderndynamics`); see `notes.md`.
