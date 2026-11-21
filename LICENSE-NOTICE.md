# Licence provenance

- Mod: `moderndynamics` — original: <https://github.com/Technici4n/Modern-Dynamics> (Modern Dynamics).
- Original author(s): Technici4n.
- Original jar this port was written against: `Modern-Dynamics-0.9.6.jar`, which declares
  **LGPL-3.0-only (the jar's `META-INF/neoforge.mods.toml`)**.
- Copyright of the original: Copyright (c) Technici4n.
- The upstream repository's **README applies CC-BY-NC-SA-4.0 to the art**, which is a different
  licence from the code's: the code may be ported under LGPL-3.0 with its source, the art may not
  travel unless the port itself is non-commercial share-alike (rank 4, local-only).
- This port is **LGPL-3.0-only**, © 2026 the Horizon port authors. Its complete corresponding
  source is this repository (`ports/moderndynamics/`), as the licence requires.

## What travels in the package

`moderndynamics.hmod` is one flat SDK package: `horizon-mod.toml`, `Cargo.toml`,
`src/lib.rs`, `src/runtime.rs`, the content under `assets/moderndynamics/` and
`data/moderndynamics/`, and **`LICENSE` + `LICENSE-NOTICE.md` at the package
root** (the builder packs root files beside the code).

## The art is this port's own, not the upstream art

Upstream's textures are CC-BY-NC-SA-4.0, so **not one of its pixels travels here**. The three
textures the 0.2.0 package carried verbatim are replaced by drawings the port makes itself (and the
attractor's is drawn the same way, never carried):

| texture | how it is made |
|---|---|
| `textures/pipe/item_pipe/base.png` | drawn: a 32x32 steel plate, bevelled rim, four rivets, diagonal sheen |
| `textures/pipe/item_pipe/straight.png` | drawn: a 32x32 arm band, two rails with a brass groove and an eight-pixel seam |
| `textures/item/extractor.png` | drawn: a 16x16 brass funnel — ring, throat, nozzle — on the steel field |
| `textures/item/attractor.png` | drawn: a 16x16 brass suction port — a ring around a hub — on the steel field |

`tools/gen_textures.py` holds the palette and the closed-form pattern, uses no randomness and reads
no upstream file; `--check` re-renders the four PNGs and compares byte for byte.

The data files that were the jar's own bytes are **rewritten from what they express** (same
ingredients, pattern, drop and model layer; this port's own key order, indentation and trailing
newline) by `tools/assemble_content.py`: both attachment item models, the pipe's loot table (in
26.3's typed-condition shape) and the three shaped recipes.

`tools/check_provenance.py` proves the claim mechanically — it compares the sha256 of **every**
packaged file against **every** entry of the jar and asserts **0 matches** — and writes
`PROVENANCE.md`, the per-file audit trail. The 0.2.1 package is the first that passes: 0.2.0 had
seven jar-identical files, which is why the version exists.

## What is not carried

No Java code, no compiled classes, no Mojang material, and no upstream art. `src/runtime.rs` and
`src/lib.rs` are a clean-room Rust rewrite against Horizon's documented mod API (`horizon-mod-api`,
SDK `0a875da`); the behaviour they implement was read from the public repository and is recorded in
`FINDINGS.md`.

**Rank 1 (LGPL, copyleft) for the code.** A published port may be redistributed and modified under
the same licence with its complete corresponding source; publishing itself still needs the
maintainer's approval (ARCHITECTURE.md §8). The art in this package is the port's own and carries no
such restriction; it is offered under the same LGPL-3.0-only for simplicity.
