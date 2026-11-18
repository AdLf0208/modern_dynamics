# Modern Dynamics → Horizon

Port of **Modern Dynamics 0.9.6** (`Technici4n/Modern-Dynamics`, **LGPLv3** per
the jar's `neoforge.mods.toml`), Minecraft 1.21.1 NeoForge, from the exact
ATM10 jar: `Modern-Dynamics-0.9.6.jar`, **707,745 B**, SHA-1
`3f206d55dc002f217a2a4e5b1cce9327fd67bfa6` (matches the pack's
`modrinthHash`), SHA-256
`4357833f16a8f890e4399de3c606a27644add1384d5d1e7246648a818e9cbdab`,
CurseForge hash `1914015235`. Provenance and the carried files:
[`../_shared/carry-work/moderndynamics/SOURCE.md`](../_shared/carry-work/moderndynamics/SOURCE.md).

**Slice 1 scope.** The **item pipe**: the `moderndynamics:item_pipe` block,
its six-side connection state, and item transfer between the inventories it
joins through the **extractor** attachment (`moderndynamics:extractor`) at
upstream's defaults. Fluid/energy pipes, the attractor/filter/inhibitor, the
wrench, upgrades, the screen and the rest of the mod are gaps.

**Class + tier:** `content` / **`machines`** (`ports/ledger.csv`; provenance
`ports/_shared/source-map/atm10-sources.csv` row `moderndynamics`). Approach
per Horizon's `docs/modpacks.md`: a machines mod is a block entity plus a
capability-style transfer on its own tick, so this is a native slice with a
content half (block, item, models, textures, lang, recipes, loot), not a
data-only carry. `precheck-mod.py moderndynamics` reports `content · machines ·
core · P0`, licence `LGPL-3.0-only` rank 1.

**Licence:** LGPL-3.0-only → **rank 1** (copyleft). A published port may be
redistributed under the same licence with its complete corresponding source;
local-only until the maintainer approves publishing (ARCHITECTURE.md §8).
**Why now:** the licence-first, core-before-add-ons queue; Modern Dynamics is a
core mechanics mod with no required parent (its optional MI/AE2/IE/RS/TR
integrations are data-file compat, not dependencies).

**Format (2026-11-18): migrated to the SDK layout.** This port is now **one flat
`moderndynamics.hmod`** — `horizon-mod.toml` + `Cargo.toml` + `src/` + `assets/` +
`data/` — built by `cargo horizon-mod` against the SDK nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, launcher ≥ 0.2.70), version
**0.2.0**. The retired `.hnmod` container, `mixins/<id>/`, `horizon-mixin.toml`,
`compatibility.json`, `rust-toolchain.toml`, the `tools/horizon.py` stage/build
wrapper and the AE2 content embedding are gone; `src/runtime.rs` is
**byte-identical** to the retired file and so are its 10 tests. The whole recipe,
the helper→hook table, the traps and the current gaps are in
[`## Migration to the SDK format`](#migration-to-the-sdk-format); the sections
below describe the retired bundle and are history.

**Upstream analysis:** repository <https://github.com/Technici4n/Modern-Dynamics>,
commit `e8e40f02966406acca712fbd36ee847f70bd528e` (2026-07-19, shallow clone in
`ports/_shared/upstream/moderndynamics`, deleted after these notes were
written). Files read and the behaviour extracted (connections, the extractor's
defaults, path order, insertion) are in [`FINDINGS.md`](FINDINGS.md). No Java
was copied or translated and the jar was not decompiled.

## What slice 1 does

- **The block.** `moderndynamics:item_pipe` (template `minecraft:stone`,
  hardness 0.2 from upstream's `destroyTime`, no occlusion, no tool, no
  redstone conduction, a 6 px core outline) with six boolean state properties
  `down`…`east` and a `multipart` blockstate (6 px core + one 6 px arm per
  connected side, the carried `pipe/item_pipe/base` and `straight` textures).
  A self-drop loot table and the jar's recipe (glass + iron ingot → 8).
- **Connections.** A side connects to another item pipe, or to a block with an
  item container reachable from the touching side (the pin's
  `native_container_slots`, API 48). The state is recomputed after every level
  tick and written only when it changed.
- **The extractor.** `moderndynamics:extractor` (carried texture and model,
  the jar's recipe) is placed on a pipe side by right-clicking the pipe with it
  (`right_click_block`, API 21; consumed outside creative), exactly upstream's
  `useItemOn` on a visible side. The side is saved in the block entity as an
  `attachments` mask, so it survives a relaunch.
- **Transfer.** Every **40 ticks** (`getItemOperationTickDelay` with no
  upgrades) each extractor moves up to **4 items** (`getItemsPerOperation`)
  from the inventory on its side into the network's other inventories, nearest
  first (`RoutingMode.CLOSEST`, the default), one destination after another
  until the batch is spent, through `native_move_items` (API 48) so an item the
  target refuses stays in its slot. An attached side never continues the pipe
  network (upstream's `ItemHost.canConnectTo`).
- **Readout.** `/moderndynamics info <x> <y> <z>` prints the connections, the
  extractor sides and the cadence; `/moderndynamics attach|detach <x> <y> <z>
  <side>` is the harness path for the side selection upstream takes from the
  click point.

### Deliberate deviations

| Upstream | Here | Why |
|---|---|---|
| Connections are block-entity data drawn by a custom model loader | six boolean states + `multipart` | the pin has no custom model loader; this is `docs/mods.md`'s own pipe example |
| `waterlogged` state | none | content blocks cannot waterlog here; the pipe is not a water source |
| A pipe/tank arm style for pipes vs an 8 px connector for inventories | one arm style for both | the drawn state only says "connected"; cosmetic |
| Outline is core + arms + connectors | the 6 px core only | one shape table cannot vary with six booleans without 64 entries (same as `enderio`) |
| Each extractor ticks on its own `lastOperationTick` | an operation every 40 ticks offset by the pipe's position | the pin reverts block-entity writes made from a tick, so a saved per-side cursor would not stick; the rate and batch are upstream's |
| Items travel as `TravelingItem`s along the paths, with a stuffed buffer for refused items | `native_move_items` moves them between the two inventories in one call; a refused item stays in the source | no item-travel renderer or stuffed buffer in this slice; nothing is lost |
| The attached side is chosen from the click point on the connector | the clicked face, or `/moderndynamics attach` | the pin's hook gives the clicked face only; a side facing a chest is occluded by that chest |
| Extractor settings in a screen (filters, redstone, routing) | none | screen work; upstream's defaults are baked |

## Slice boundary and gaps

Implemented: the pipe block/item and carried content, the six-side connection
state, extractor placement (item and command), the 40-tick / 4-item operation
with nearest-first insertion, refusal (no loss), `/moderndynamics info`
`attach|detach`, and persistence across a relaunch.

Not implemented (planned slices):

1. **fluid pipe** and the five **energy cables** (LV/MV/HV/EV/superconductor)
   and the **machine extender**;
2. **attractor**, **filter** and **inhibitor** attachments, the **wrench**
   (connection blacklist, attachment removal, off-hand placement rules), the
   attachment **screen** (filters, oversending, max items, redstone mode) and
   the **attachment upgrade** data (item/fluid speed, count, frequency);
3. the **routing modes** other than CLOSEST (FURTHEST, RANDOM, ROUND_ROBIN —
   round robin is upgrade-gated upstream, so the default slice does not
   rotate), priorities, and the stuffed-item retry path;
4. **item travel** rendering, the pipe's own textures on the arms per type,
   waterlogging, the pickup/clone behaviours;
5. the **JEI/EMI/REI/Jade** plugins, the debug tool and the config.

Known limits: a refused batch is retried from the source next operation (no
stuffed buffer); the connection refresh runs for every loaded pipe each level
tick (fine for a handful, a per-chunk cache is later work); `/moderndynamics
attach` exists only because the pin's right-click gives the face, not the point.

## Migration to the SDK format

*Written 2026-11-18 from this port, after `imfast` (code-only) and `rangedpumps`
(block entity + content) proved the recipe. This port is the first **machines**
one with a carried content half, so its file map is the one to copy.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired `.hnmod` and
the registry closed this port's submission (PR **#826**) with *"republish this as
a native `.hmod` with the SDK from the next Nightly"*. One flat `.hmod` is now the
unit: content, native code, or both. Pins: SDK nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (package_format 6, content API 0.2,
rust 1.99.1), mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, game 26.3,
`cargo-horizon-mod` from `horizon-rust/horizon@main`.

### Files created, moved and deleted

| Action | Path |
|---|---|
| created | `horizon-mod.toml` — the only manifest and the only identity (`id`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pins) |
| created | `Cargo.toml` — one crate, `moderndynamics` 0.2.0, edition 2024, `publish = false`, only `horizon-mod-api` from `.horizon/sdk/` |
| created | `src/lib.rs` — the one `impl Hooks` plus the pipe's `BlockPart` and `BlockEntityPart` |
| created | `.gitignore` — `/target`, `/.horizon`, `/Cargo.lock` (trap 8) |
| created | `src/runtime.rs` — `cp` from the retired tree, byte-identical: sha256 `c2def55decb2f76048f7852139cd3a64f6cfe01f7c96e4535d144d1c0a54082e` (the same bytes are inside the package) |
| moved | `content/assets/moderndynamics/` → `assets/moderndynamics/`; `content/data/moderndynamics/` → `data/moderndynamics/` (byte-identical, same 20 files) |
| rewritten | `README.md`, `LICENSE-NOTICE.md`, `tools/e2e.sh`, this section |
| deleted → evidence | `mixins/moderndynamics/` (11 files: `horizon-mixin.toml`, `runtime.rs`, `support.rs`, `block_entity.rs`, `level_tick.rs`, `right_click.rs`, `register.rs`, `perform.rs`, `embedded_content.rs`, `data/moderndynamics-content.hmod`), `moderndynamics.hnmod` (27,024 B), `compatibility.json`, `rust-toolchain.toml`, the old `Cargo.toml`/`Cargo.lock`, `content/` (`horizon-mod.toml`, the built `.hmod`), `tools/horizon.py`, `tools/package.py`, `tools/package.sh`, `tools/stamp_license.py`, `tools/hmod-load.sh`, `tools/test_stage.py`, `tools/test_template.py`, `tools/audit/`, the retired `tools/e2e.sh` |
| kept | `notes.md`, `FINDINGS.md`, `LICENSE`, `moderndynamics.hmod` (rebuilt at the port root) |

The retired tree's last state is kept, for reading only, at
[`ports/_shared/evidence/moderndynamics/retired-hnmod-0.1.0/`](../_shared/evidence/moderndynamics/retired-hnmod-0.1.0/)
(`mixins/`, the `.hnmod`, `content/` with its 20-entry `.hmod`, the old tools and
`e2e.sh`).

### One package, content included

The retired bundle had to embed its content (AE2's `content_root` pattern) because
the registry keeps one file per project. **The SDK package carries the content
itself** — `assets/` and `data/` sit at the package root beside `src/` — so the
`embedded-content` injection, `native_data!`, the `.horizon-cache` write and its
byte-identity check are all gone, and the portrait is the whole deliverable.

### Helper → hook → SDK method

| Old `.hnmod` injection | Old helper | New SDK surface |
|---|---|---|
| `block_entity` HEAD (`(id, NBT)` factory) | `block_entity.rs` → `support.rs` | `Registrar::block_entity_type("item_pipe").blocks(&[PIPE]).build()` + `Registrar::block_entity_part(&kind, Pipe)` — the game creates the entity, starting empty |
| — (no tick hook was needed: a level sweep did the work) | — | `BlockEntityPart::block_entity_ticks` → `event.ticks = true`, then `BlockEntityPart::block_entity_tick` per pipe |
| `level_tick_post` HEAD (walk **every** pipe in `game.world.block_entities`) | `level_tick.rs` → `support.rs` | **no world scan exists**: each pipe refreshes its own six connections in its own tick, and the network walk starts from that pipe (`LevelRef::block`, `LevelRef::capability(&ITEMS, ..)`, `BlockEntityRef::data`) |
| `native_container_slots` / `native_move_items` | `support.rs` | `LevelRef::capability(host, &ITEMS, pos, Some(side))` → `ResourceHandler<ItemResource>`, then `move_resources(..)` inside `host.transaction(..)` |
| `native_block_property` / `native_set_block_property_with` | `support.rs` | `LevelRef::block_property(host, pos, key)` / `host.set_block_property_with(pos, key, value, BlockUpdateFlags::CLIENTS.bits())` |
| `right_click_block` HEAD (held item, take it, creative guard) | `right_click.rs` → `support.rs` | `BlockPart::use_item_on` on the pipe block (`BlockClick { pos, hit.face, stack, hand, player }`), consumption through `ItemPlace::PlayerHand { player, hand }.extract(..)` **in the same transaction as the attachment write** — which already leaves a creative player's hand alone |
| `register_commands` HEAD | `register.rs` | `Hooks::register_commands` + `Command::new(..)` (same three usage strings) |
| `perform_command` HEAD (`&str` line) | `perform.rs` | `Hooks::run_mod_command` with a parsed `CommandInvocation` (`literals`, `argument("pos") -> BlockPos`, `argument("side") -> String`) |
| `content_root` HEAD (AE2 embedding) | `embedded_content.rs` | **not needed** — one package carries content and code |
| `game.native_feedback(Text)` | `support.rs` | `host.commands().feedback(Text)` for commands, `PlayerRef::tell(host, Text)` for the right-click readout |
| `game.world.block_entities` (direct map) | `support.rs` | `event.entity.nbt` in the pipe's own tick (writable and written back), `BlockEntityRef::data`/`replace_data` elsewhere |

`src/runtime.rs`'s decisions (the 40-tick / 4-item rate, the phase, the network
walk, nearest-first order, the batch budget, the attachment mask and its saved
form) are unchanged; only the game calls around them are typed now.

**One placement deviation, for the better:** the retired helper wrote the
attachment and then took the item, so a refused write still consumed it. The SDK
version puts both writes in one `host.transaction(..)`: one item leaves the hand
*iff* the side gets its extractor, and an abort rolls both back. A creative
player's hand is left alone by `ItemPlace::extract`, as upstream's
`stack.shrink` guard intends.

### Build and check (the SDK ladder)

`build-slot.sh` still speaks the retired tool, so — as the brief says — the SDK
commands run **directly in the port** (`CARGO_INCREMENTAL=0`, no staging, no
`--target-dir`); the runner is used only as the lease around them:

```sh
cd ports/moderndynamics
cargo horizon-mod build                   # fetches .horizon/sdk/ on first use, packs
cp target/horizon-mod/moderndynamics.hmod ./moderndynamics.hmod
rm -f Cargo.lock                          # trap 8, before the final build
cargo horizon-mod check moderndynamics.hmod
cargo horizon-mod check moderndynamics.hmod --json
cargo horizon-mod fits moderndynamics.hmod --build nightly
```

### Traps (measured on this migration)

1. **`cargo horizon-mod` must run inside the package.** From the campaign root it
   answers `error: horizon-mod.toml: No such file or directory (os error 2)`.
2. **`build-slot.sh run --step check -- bash -c 'cd ports/moderndynamics && …'`
   is the right shape**, but the SDK tool does the compile itself: there is no
   staged tree and no `python3 tools/horizon.py` any more.
3. **`runtime.rs` is `impl Fn`-only where it walks.** `reachable(pipes, start,
   linked: impl Fn(..))` cannot call the host, so the pipe-to-pipe links are
   worked out first (`pipe_links`, a `BTreeSet<(Pos, Face)>`) and the predicate
   only reads them. A closure that tried `attachments_at(host, ..)` failed with
   `E0596: cannot borrow *host as mutable … Fn closure`.
4. **There is no `host.game_extract`.** `game_stack`/`game_insert`/`game_extract`
   live on `ItemsBackendMut`, the game-side backend, not on any host trait. The
   mod-facing write is `ItemPlace::extract` (or `insert`/`exchange`) inside
   `host.transaction(..)`; `ItemPlace::PlayerHand` is a player's hand.
5. **`host.set_block_property_with` takes `i32` flags, not `BlockUpdateFlags`.**
   Pass `BlockUpdateFlags::CLIENTS.bits()` (`bits()` is `doc(hidden)` but public).
6. **A pipe's entity is out of the world during its own tick.** `host.block_entity`
   answers `None` for it, so the tick passes its own attachment mask down instead
   of reading it back; a `neighbour(..)` that asked `attachments_at` for the pipe
   being ticked would silently lose its extractor sides.
7. **Rust 1.97 clippy denies `question_mark`**: `let Some(x) = f() else { return
   None };` must be `let x = f()?;` in an `Option` function.
8. **`Cargo.lock` must not be packed.** `cargo test`/`clippy` write one;
   `cargo horizon-mod build` then adds it and the artifact changes. `rm -f
   Cargo.lock` immediately before the final build; the delivered package has 25
   entries and no lock.
9. **The builder packs root files.** `README.md`, `LICENSE`, `LICENSE-NOTICE.md`
   are inside the `.hmod`, so editing the README changes the sha256: rebuild
   before quoting one. `notes.md`, `FINDINGS.md` and `tools/` are not packed.
10. **`check --json` reports `"capabilities":[]` and `"side":null`** for a native
    package even though the manifest carries both (`rangedpumps`' check reports the
    same). Read `side`/`capabilities` from the manifest, not from the report.
11. **No `--dev-mod`, no embedded copy, no negative control leg.** With one package
    the old `EMBEDDED`/`CONTROL` legs and the `.horizon-cache` comparison are
    meaningless; `tools/e2e.sh` keeps the A–F scenario and takes
    `GAME=<launcher-built horizon-client>`.

### Gaps (current, superseding the retired-format list below)

1. **Fluid pipe, the five energy cables (LV…EV, superconductor) and the machine
   extender** are not ported.
2. **Attractor, filter and inhibitor** attachments, the **wrench** (connection
   blacklist, attachment removal, off-hand rules), the attachment **screen**
   (filters, oversending, max items, redstone mode) and the attachment **upgrade**
   data (speed, count, frequency).
3. **Routing modes** other than `CLOSEST` (FURTHEST, RANDOM, ROUND_ROBIN),
   priorities, and the stuffed-item retry path.
4. **Item travel** rendering, per-type pipe textures on the arms, waterlogging,
   the pickup/clone behaviours.
5. **JEI/EMI/REI/Jade** plugins, the debug tool and the config.
6. **No in-game re-proof at this build.** The scenario needs a game the launcher
   built from this package at SDK `0b14f8a`; this machine has neither a
   `0b14f8a` game nor `horizon-launcher`. `tools/e2e.sh` was rewritten for the
   new layout and its assertions are unchanged; the 0.1.0 run's four logs
   (`e2e.log`, `e2e-selftest.log`, `e2e-embedded.log`,
   `e2e-embedded-selftest.log`) remain the behaviour's evidence and
   `check-e2e-logs.py moderndynamics --strict` still audits them.
7. **The connections refresh per pipe, in that pipe's tick** (the pin has no
   loaded-block-entity scan), so a pipe shows a neighbour change at most one
   server tick late; the 0.1.0 level sweep had the same one-tick latency but
   walked every pipe in one place. Two extractors that fall on the same tick
   now run in the game's block-entity order rather than in pipe-position order —
   only visible when two extractions contend for one destination in the same
   operation; upstream ticks its attachments in their own insertion order too.
8. **`/moderndynamics attach|detach` remains** because the pin's click gives the
   clicked face, not the click point on the connector.
9. **`BlockClick.dimension` is a `LevelDimension` while `LevelRef::dimension()` is
   a `DimensionKey`**, so `use_item_on` takes the in-place level rather than
   matching the click's dimension. It cannot matter (the click runs in the level
   being ticked), but the two types should agree — worth an upstream note.
10. **No `icon.png`**: optional (square, ≤512×512, ≤256 KiB); upstream ships none,
    so none is packed.
11. **The ARCHITECTURE §3 gap "block-entity writes inside a tick are reverted" is
    lifted** by the SDK: `BlockEntityTickEvent` carries a writable `entity.nbt`
    that is written back. This port still derives the operation phase from the
    pipe's position (byte-identical `runtime.rs`), so it does not use the new
    freedom yet; a future slice may keep a real per-side cursor instead.

### Changelog for the resubmission (0.2.0)

> Supersedes **#826**, closed because Horizon retired `.hnmod` (#17992). Changes:
> one flat SDK package (`horizon-mod.toml`, `api = "0.2"`, `[native]` pinned to
> `0b14f8a` / mixins `e59da90`, game 26.3) instead of an `.hnmod` plus a separate
> content mod; the carried content now travels **inside** the package
> (`assets/moderndynamics/`, `data/moderndynamics/`), so the AE2 embedded-content
> shim is gone; the six injection helpers became one `impl mod_api::Hooks` plus
> the pipe's `BlockPart`/`BlockEntityPart` (a declared block entity type instead of
> a factory hook, a tick on the pipe itself instead of a level sweep, the
> right-click placement as `use_item_on` with `ItemPlace` consumption, the
> transfer through the `horizon:items` capability and `move_resources`, the
> commands as `register_commands` + `run_mod_command`); `src/runtime.rs` and its
> 10 tests are unchanged, byte for byte. Behaviour is the same item pipe, its six
> connections, the extractor at 4 items per 40 ticks nearest-first, and
> `/moderndynamics info|attach|detach`.

### Migration verification (SDK `0b14f8a`, 2026-11-18)

Every compile/test step ran through `ports/tools/build-slot.sh` (`BUILD_LOCAL=1`,
`CARGO_INCREMENTAL=0`); logs in `ports/_shared/evidence/moderndynamics/`.

| Check | Command | Result |
|---|---|---|
| Runtime unit tests | `cargo test` | **10 passed**, 0 failed (`sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package | `cargo horizon-mod build` | `target/horizon-mod/moderndynamics.hmod`, EXIT 0 (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check moderndynamics.hmod` | `moderndynamics 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)`, **33,593 B**, sha256 `20fca6d83481645936840daa4259fdc84914bcccf7bcae76ef3db5498bf6f7d8`, **EXIT 0** (`sdk-check.log`) |
| Check, machine form | `cargo horizon-mod check … --json` | `{"ok":true,…,"id":"moderndynamics","kind":"native-mod","version":"0.2.0","size":33593}` (`sdk-check-json.log`) |
| What it uses | `--uses` / the package's `uses.json` | **294** surface entries; the hooks are `block_entity(_tick/_ticks)`, `use_item_on`, `register_commands`, `run_mod_command` (`sdk-uses.log`) |
| Fits the pinned build | `cargo horizon-mod fits moderndynamics.hmod --build nightly` | `0b14f8a  made for`, **EXIT 0** (`sdk-fits.log`) |
| `runtime.rs` untouched | `unzip -p … src/runtime.rs \| sha256sum` | `c2def55d…082e`, byte for byte the retired file, and `cmp` clean |
| Package contents | `unzip -l` | **25 entries**: manifests, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `src/lib.rs`, `src/runtime.rs`, `uses.json`, all 10 asset files and all 5 data files; **no `Cargo.lock`** |
| Fresh-clone repeat | `.horizon/`, `target/` and `Cargo.lock` removed, then `cargo horizon-mod build` | SDK fetched by itself; same **33,593 B**, same sha256 (`sdk-fresh-clone.log`) |

### Not re-run in this migration

- **The in-game e2e** (A–F, the relaunch, the selftest) — no launcher-built game
  at `0b14f8a` and no `horizon-launcher` here; gap 6. The 0.1.0 logs stand, and
  the scenario is the same code with the calls renamed in the table above.
- **The staged ladder** (`tools/horizon.py stage/check/test/build`) — those tools
  are deleted with the retired format.
- **The `.hmod` load check and its unmodded negative control**
  (`tools/hmod-load.sh`) — it launched the shared `1d63e0a` game with
  `--dev-mod content`; a native package needs the launcher's native build, so the
  script is archived rather than rewritten.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Verification

*This table is the **0.1.0 (retired `.hnmod`) ladder**, kept as the behaviour's
evidence. The SDK package's own verification is the table in
[`## Migration to the SDK format`](#migration-to-the-sdk-format) above.*

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6). Every build, test
and e2e ran through `ports/tools/build-slot.sh` (`CARGO_INCREMENTAL=0`, no bare
cargo, no `--target-dir`); the in-game run used the pin-matched shared
`horizon-ctl` (`ports/_shared/bin/horizon-1d63e0a5/horizon-ctl`) and the
port-built fast game, built and run in **one** e2e lease.

### Ladder

| Step | Command (through the slot runner) | Result |
|---|---|---|
| runtime RED | `cargo test --locked --offline` (port) | `exit=101`, **80** `error[E…]` (the API did not exist yet) |
| runtime GREEN | `cargo test --locked --offline` | **10 passed**, 0 failed — `evidence/moderndynamics/test.log` |
| fmt + clippy | `cargo fmt --all --check`; `cargo clippy --all-targets --locked --offline -- -D warnings` | pass, **0 warnings** |
| stage | `python3 tools/horizon.py stage --horizon …/pins/horizon-1d63e0a` | `status: ok`, **6 injections, 2 modules** |
| check | `python3 tools/horizon.py check …` | `cargo_exit: 0`, **0 warnings** — `evidence/moderndynamics/check.log` |
| embedded tests | `python3 tools/horizon.py test …` (`--lib game::moderndynamics::tests`) | **10 passed**, 0 failed, 1240 filtered — `evidence/moderndynamics/test.log` |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0` — `evidence/moderndynamics/build.json` |
| e2e | `build-slot.sh e2e -- … -- bash tools/e2e.sh` | **PASS**, `EXIT:0` — `evidence/moderndynamics/e2e.log` |
| selftest | `SELFTEST=1` the same way | **exit 1**, `FAIL: SELFTEST: the target chest holds 16 stone, so the check is alive` — `evidence/moderndynamics/e2e-selftest.log` |
| embed ladder | `cargo fmt --check`; `rustfmt --check mixins/moderndynamics/embedded_content.rs`; `cargo clippy … -D warnings`; `cargo test`; `tools/horizon.py check/test/build` | 0 warnings, **10 passed**, `cargo_exit: 0` on all three |
| e2e, one file only | `EMBEDDED=1 CONTROL=1 … tools/e2e.sh` | **PASS**, `EXIT:0`; the control shows the ids unknown — `evidence/moderndynamics/e2e-embedded.log` |
| embed fail-capability | `EMBEDDED=1 SELFTEST=1 … tools/e2e.sh` | **exit 1** — `evidence/moderndynamics/e2e-embedded-selftest.log` |
| content `.hmod` | `tools/package.sh` (`cargo-horizon-mod build` + `check`) | **9,188 B**, SHA-256 `990b6690872c10dfcef3ee4701a719968ef1d33cbd730507f4c8630f67ce934b`, **18 entries**, `moderndynamics 0.9.6 (mod, API 0.2, both)`, "no code: content only" |
| native `.hnmod` (content embedded) | `tools/package.sh` (`tools/package.py` + `check --kind native-mod`) | **20,220 B**, SHA-256 `488a6780a3b9537904dc0c750639f95078281de53d281a76170f5ec82468d211`, **11 entries** incl. `mixins/moderndynamics/data/moderndynamics-content.hmod`; `native-mod, format native-1/1d63e0a5…/5a64af93…, for 26.3` |
| `.hmod` load check | `tools/hmod-load.sh` (shared game) | `PASS`: the packaged `.hmod` registers `moderndynamics:item_pipe` and `moderndynamics:extractor` |
| `.hmod` negative control | same script, second launch **without** `--dev-mod` | `{"chat":["Unknown block type 'moderndynamics:item_pipe'…]}` and `{"chat":["Unknown item 'moderndynamics:extractor'…]}`, control block `minecraft:air` — the check is not vacuous |

## Embedded content (AE2 pattern) — one listable file

*Retired 2026-11-18: the SDK package carries its content itself, so nothing is
embedded. Kept as history for the `.hnmod` era.*

The registry keeps **one file per project** (playbook §1.0), so a port with both
halves could not be listed faithfully. This bundle now carries its content,
exactly as the merged AE2 entry does:

- **`mixins/moderndynamics/data/moderndynamics-content.hmod`** is the built
  `content/moderndynamics.hmod`, byte for byte (9,188 B,
  `990b6690872c10dfcef3ee4701a719968ef1d33cbd730507f4c8630f67ce934b`).
  `tools/package.sh` rebuilds the content mod, copies it in, and
  `tools/package.py` packs `data/**` as `mixins/<id>/data/…` with fixed zip
  timestamps, refusing a copy that is not byte-identical to
  `content/<id>.hmod` so a rebuilt content mod can never ship stale inside the
  bundle.
- **`mixins/moderndynamics/embedded_content.rs`** with the `embedded-content`
  `[[injection]]` (`target = "horizon_client::game::mods::content_root"`,
  `kind = "HEAD"`, `cancellable = true`, bindings `root`/`enabled`/`dev`)
  compiles the file in with `native_data!` (API 41), writes it to
  `.horizon-cache/moderndynamics-embedded-content.hmod` when the copy on disk
  differs, appends it to the dev-mod list and returns its `content_root`; the
  one-level `dev.contains(&file)` guard makes the recursive call run the vanilla
  body, as AE2 does. An unwritable cache logs a warning and `Continue`s.

Verified in game with **only the bundle** (`EMBEDDED=1`, no `--dev-mod`): the
whole scenario A–F passes (a 20-stone transfer at 4 items per 40-tick operation,
the right-click extractor, the relaunch) and the file the helper wrote is
byte-identical to `content/moderndynamics.hmod`. The control (`CONTROL=1`)
launches the same binary with `.horizon-cache` replaced by a plain file, so the
helper refuses its content: the extractor item never arrives and
`setblock … moderndynamics:item_pipe` leaves `minecraft:air`.

This makes the `.hnmod` the **single deliverable** of this port: one file
installs the item pipe, the extractor, their models, textures, recipes and lang
**and** the block entity, the connection state and the extractor operations. It
does not by itself publish it (licence approval and the copyleft reviewer note
are the publisher's step, ARCHITECTURE.md §8).

```
== control: the same game with no embedded content file ==
  without the embedded file: no extractor item, the block stayed minecraft:air
  launch: no --dev-mod; the bundle carries its own content
  embedded content: 9188 B at /tmp/opencode/moderndynamics-home.5OAfyq/.horizon-cache/moderndynamics-embedded-content.hmod, identical to content/moderndynamics.hmod
PASS: an extractor on a pipe moved 20 stone from one chest to another, exactly 4 items at each 40-tick operation, … a pipe without an extractor moved nothing
EXIT:0
```

The same scenario with `SELFTEST=1` (still one file) ends `EXIT:1`, so the
one-file run is fail-capable. `check-e2e-logs.py moderndynamics --strict` →
4/4; `upstream_ready.py --copyleft moderndynamics` → **READY**.

Evidence for this slice:
[`ports/_shared/evidence/moderndynamics/`](../_shared/evidence/moderndynamics/)
(`embed-ladder.log` — fmt/clippy/test/package/check/test/build,
`package.log` — both artifacts and the native check, `e2e-lease.log` — the three
legs, `e2e-embedded.log`, `e2e-embedded-selftest.log`).

`tools/test_template.py` and `tools/test_stage.py` are the template's own
fixtures: the first asserts a one-injection fixture workspace (it fails here at
the missing `native-hooks.json` and on the injection count), the second points
at the old in-tree `ports/horizon` checkout. Neither is the ladder; the staged
check/test/build above is the evidence, as `enderio` records for the same
scaffold.

The first slice's scratch capture `e2e-early-debug.log` (no verdict line, an
intermediate debugging run) was renamed `early-debug.log`: it is not an e2e
verdict log, and `check-e2e-logs.py` audits every `e2e*.log`.

### In-game scenario (quoted from `evidence/moderndynamics/e2e.log`)

```
== A: an extractor moves 20 stone from the west chest to the east chest ==
  Item Pipe at 1,100,0: extractors west; 4 items every 40 ticks
  Item Pipe at 1,100,0: links west,east
  pipe state: moderndynamics:item_pipe[down=false,east=true,north=false,south=false,up=false,west=true]
  4 operations in 7 s: source 4, target 16
  source after: 0, target after: 20
== B: two chests with no pipe between move nothing (control) ==
  source: 4, target: 0
== C: a pipe with an extractor but no inventory beside it moves nothing (control) ==
  Item Pipe at 16,100,0: extractors west; 4 items every 40 ticks
  Item Pipe at 16,100,0: links none
  source: 4, target: 0
== D: a full target refuses the items (control) ==
  source: 4, target stone: 0, target dirt: 1728
== E: a connected pipe with no extractor moves nothing (control) ==
  source: 4, target: 0
  Item Pipe at 33,100,0: extractors none; 4 items every 40 ticks
  Item Pipe at 33,100,0: links west,east
== F: right-clicking a pipe with the extractor item places it (upstream's path) ==
  Item Pipe at 40,100,0: extractors south; 4 items every 40 ticks
  Item Pipe at 40,100,0: links east
== the extractor is saved with the pipe across a relaunch ==
  Item Pipe at 1,100,0: extractors west; 4 items every 40 ticks
  relaunched pipe state: moderndynamics:item_pipe[down=false,east=true,north=false,south=false,up=false,west=true]
PASS: an extractor on a pipe moved 20 stone from one chest to another, exactly 4 items at each 40-tick operation, the pipe's state showed both connections, a right-click placed an extractor, and chests with no pipe / a disconnected pipe / a full target / a pipe without an extractor moved nothing
EXIT:0
```

What the scenario proves: the 20 stone is the extractor's own transfer (the
same pipe moves nothing in E without one), the batch is **exactly 4 per
operation** (every sampled step is +4, and 7 s holds 4 operations, not 70), the
state carries both connections, a **disconnected** pipe moves nothing (C), a
**full** target refuses and the source keeps its stone (D), and the attachment
survives a **relaunch**. `SELFTEST=1` flips the target assertion and the run
fails (`target chest holds 16 stone`), so the check can fail.

Harness notes for the next slice: the arena is The End at y=100 with a stone
floor at y=99; `ctl container X Y Z ITEM 64` fills **one** slot, so 27 calls
fill a chest; a native command must be typed with `ctl chat "/cmd"` (the
`perform_command` hook); `ctl interact X Y Z FACE` only aims at a face visible
from the player's eye (a face against a chest is not clickable), hence the
`attach` command; `ctl tp` next to each pipe before an `interact`.

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-moderndynamics.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** [packaging defect found while repairing] the `.hnmod` did not contain its `data/` folder
  **Repair:** Fixed: `tools/package.py` packed only the bundle root, so `native_data!` had no `botanypots`-style content file to compile. It now packs `data/`; the artifact grew from 16,724 to 27,024 bytes.
- **Reviewer:** LGPL-3.0 source review is the reviewer's limitation
  **Repair:** Not a code item: the registry accepts LGPL and the complete corresponding source travels with the repository. The notice states it.
- **Reviewer:** licence notice
  **Repair:** `LICENSE` + `LICENSE-NOTICE.md` in the `.hnmod`, `data/moderndynamics/license/` in the carried `.hmod`.
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices
