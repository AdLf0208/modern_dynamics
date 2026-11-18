# Licence provenance

- Mod: `moderndynamics` — original: <https://github.com/Technici4n/Modern-Dynamics> (Modern Dynamics).
- Original author(s): Technici4n.
- Original jar this port was written against: `Modern-Dynamics-0.9.6.jar`, which declares
  **LGPL-3.0-only (the jar's `META-INF/neoforge.mods.toml`)**.
- Copyright of the original: Copyright (c) Technici4n.
- This port is **LGPL-3.0-only**, © 2026 the Horizon port authors. Its complete corresponding
  source is this repository (`ports/moderndynamics/`), as the licence requires.

## What travels in the package

`moderndynamics.hmod` is one flat SDK package: `horizon-mod.toml`, `Cargo.toml`,
`src/lib.rs`, `src/runtime.rs`, the carried `assets/moderndynamics/` and
`data/moderndynamics/`, and **`LICENSE` + `LICENSE-NOTICE.md` at the package
root** (the builder packs root files beside the code). The retired `.hnmod`
shipped the notice as ignored data (`data/moderndynamics/license/license.txt`,
written by `tools/stamp_license.py`) because its content was a second archive;
one package needs no such detour, so the two files ride at the root instead.

## What is carried, and what is not

- **Carried** from the jar, under LGPL-3.0: the item pipe's blockstate, block and item models,
  its two pipe textures and the extractor texture, the `en_us` lang file, and the block
  definition, item definition, creative tab, recipes and loot table.
- **Not carried:** no Java code, no compiled classes, no Mojang material. `src/runtime.rs` and
  `src/lib.rs` are a clean-room Rust rewrite against Horizon's documented mod API
  (`horizon-mod-api`, SDK `0b14f8a`); the behaviour they implement was read from the public
  repository and is recorded in `FINDINGS.md`.

**Rank 1 (LGPL, copyleft).** A published port may be redistributed and modified under the same
licence with its complete corresponding source; publishing itself still needs the maintainer's
approval (ARCHITECTURE.md §8).
