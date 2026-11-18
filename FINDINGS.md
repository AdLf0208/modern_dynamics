# Modern Dynamics — upstream behaviour (analysis aid)

Read for behaviour only; the port is a clean-room Rust rewrite. No Java was
copied or translated, and no jar was decompiled.

## Repository

| | |
|---|---|
| Repo | <https://github.com/Technici4n/Modern-Dynamics> |
| Commit read | `e8e40f02966406acca712fbd36ee847f70bd528e` (2025-07-19, `main`, shallow clone in `ports/_shared/upstream/moderndynamics`) |
| Pack jar | `Modern-Dynamics-0.9.6.jar` (the repo HEAD is one commit past the 0.9.6 release; nothing read below differs from the jar's content files) |
| Licence | LGPL-3.0-only (`license_header.java`; the jar's `META-INF/neoforge.mods.toml`) |

Files read:

- `pipe/PipeBlockEntity.java` — the block entity, `connectionBlacklist`,
  `pipeConnections`/`inventoryConnections` masks, attachment placement and
  removal, saved/client tags.
- `pipe/PipeBlock.java` — the block: `waterlogged` state, no connection
  blockstates (connections are block-entity data and a client model), drops.
- `pipe/PipeBoundingBoxes.java` — core 6 px cube, 6 px arms 5 px deep,
  8 px/4 px inventory connectors, shape selection.
- `pipe/ItemPipeBlockEntity.java` — one `ItemHost` per item pipe.
- `network/NodeHost.java`, `network/NetworkNode.java` — pipe-to-pipe
  connections, the per-side attachment array, node/network updates.
- `network/item/ItemHost.java` — extraction, insertion, path order, travel.
- `attachment/AttachmentItem.java`, `IoAttachmentItem.java`,
  `attached/AttachedIo.java`, `attached/ItemAttachedIo.java`,
  `attached/UpgradeContainer.java`, `attachment/Setting.java`,
  `attachment/settings/RoutingMode.java` — the extractor/attractor/filter
  attachments, their defaults and upgrade arithmetic.
- `init/MdBlocks.java`, `init/MdItems.java`, `init/MdAttachments.java`,
  `pipe/PipeItem.java`, `Constants.java` — registrations and names.

## Behaviour extracted

### The block and its connections

- One block, `moderndynamics:item_pipe`, no facing: properties
  `mapColor METAL`, `noOcclusion`, `isRedstoneConductor == false`,
  `destroyTime 0.2`, **`waterlogged`** as its only block state. The shape is a
  6 px core, a 6 px arm 5 px deep to every connected side, and an 8 px/4 px
  connector on every side that reaches an inventory.
- Two independent connection masks live in the block entity:
  - `pipeConnections`: the neighbour is another pipe whose host accepts the
    connection (the `connectionBlacklist` bit of either side blocks it);
  - `inventoryConnections`: the neighbour is not a pipe and exposes an item
    handler on the touching side.
- Both masks are saved and sent to the client; the shape and the model follow
  them. Placing/removing a chest or a pipe schedules a host update, so the
  masks follow the world within a tick.
- A wrench in the off hand blacklists sides on placement, and a wrench
  right-click toggles a connection or removes an attachment. **The wrench is
  not in this slice.**

### Attachments are the mechanism (the pipe alone moves nothing)

An item pipe has no transfer of its own. Items move only when an **IO
attachment** is placed on one of its six sides:

- `moderndynamics:extractor` pulls from the inventory on its side into the
  pipe network (an extractor is what the pack calls a servo);
- `moderndynamics:attractor` pulls from the network into the inventory on its
  side;
- `moderndynamics:filter` filters what passes; `moderndynamics:inhibitor`
  blocks a side.
- At most one attachment per side. Placement is a right-click with the
  attachment item on the pipe (the hit side is derived from the click point;
  the core answers the clicked face), and it is consumed outside creative.

### The extractor, default numbers

`UpgradeContainer` with no upgrades is the default:

| Setting | Default | Where |
|---|---|---|
| items per operation | **4** | `getItemsPerOperation = 4 + upgrades` |
| operation delay | **40 ticks** | `getItemOperationTickDelay = 40 / (1 + upgrades)` |
| speed-up factor | 1 | `getItemSpeedupFactor` |
| routing mode | **CLOSEST** | `ItemAttachedIo` default; ROUND_ROBIN needs the advanced-behaviour upgrade |
| filter inversion | BLACKLIST (empty filter passes all) | `AttachedIo` |
| redstone mode | IGNORED | `AttachedIo` |
| filter size | 3 | `UpgradeContainer` |
| extract amount | max = 4 | `maxItemsExtracted` defaults to the maximum |

An extractor ticks when `currentTick - lastOperationTick[side] >= 40`; it
extracts up to `maxItemsExtracted` from the slots of the adjacent inventory,
inserts them into the network, and the items travel along the paths (visually,
as `TravelingItem`s) to be inserted into the inventories at the network's
other endpoints.

### Paths and insertion

- Paths are computed from the extracting pipe with the extractor's own side
  excluded (`pathCache.getPaths(node, side.getOpposite())`), so an extractor
  never sends back into the inventory it took from.
- `CLOSEST` keeps the path order (shortest first); `FURTHEST` reverses it;
  `RANDOM` shuffles; `ROUND_ROBIN` rotates by a per-attachment cursor that
  advances by the number of paths walked.
- At a path's end the item is inserted with `ItemHandlerHelper.insertItemStacked`
  (equal stacks topped up first, then empty slots). The endpoint's attachment
  filter and redstone mode are checked first.
- An item the target refuses is kept as **stuffed** in the attachment and
  retried (or dropped if the attachment is gone and the strategy is DROP).
  Removal of a pipe drops traveling and stuffed items.
- Weighing: the 4-item budget is the **maximum per operation**, not a
  per-destination quota.

### Registered content (0.9.6)

Blocks: `item_pipe`, `fluid_pipe`, `lv_cable`, `mv_cable`, `hv_cable`,
`ev_cable`, `superconductor_cable`, `machine_extender`. Items: the pipe block
items plus `attractor`, `extractor`, `filter`, `inhibitor`, `wrench`,
`debug_tool`, `machine_extender`. Recipes for `item_pipe` (glass + iron ingot,
8) and `extractor` (hopper, iron, paper, redstone). Creative tab
`itemGroup.moderndynamics.moderndynamics`.

## Slice 1 mapping (this port)

| Upstream | Port |
|---|---|
| `waterlogged` + no connection states; connections in the BE, drawn by a custom model loader | six boolean block states (`down`…`east`, as in `docs/mods.md`'s pipe example) driven by the port each level tick; a `multipart` blockstate with a core and one arm per connected side |
| `pipeConnections` + `inventoryConnections` | the same two masks, recomputed from `native_block` / `native_container_slots` each level tick; the states show their union (one arm style, so a chest arm and a pipe arm look alike) |
| attachment array in the node | a six-bit `attachments` mask in the block entity's saved compound; only the extractor is placed |
| extractor ticks on `currentTick - lastOperationTick >= 40` | an operation every 40 ticks, offset by the pipe's position (the pin reverts block-entity writes made from a tick, so a saved per-side cursor would not stick — same reason `enderio` derives its cycle from game time) |
| 4 items per operation | `ITEMS_PER_OPERATION = 4` |
| paths, CLOSEST ordering | endpoints ordered by graph distance from the extracting pipe, then face order |
| items travel as entities, then insert one by one | `native_move_items` moves them between the two inventories in one call (the item-travel renderer and the stuffed buffer are out of scope) |
| filter, redstone, upgrades, GUI, wrench | out of scope (gaps) |

## Deliberate deviations

- **One arm style for pipes and inventories**: the connector's 8 px/4 px
  profile is not modelled; the state only says "connected".
- **Outline is the 6 px core only**, as in every pipe port on this pin; the
  arms do not extend the targeting box (upstream's does).
- **No stuffed buffer**: an item the target refuses stays in the source
  inventory and is retried next operation. Nothing is lost.
- **No item travel animation**: items arrive at the destination in the same
  tick as the extraction.
- **Operation cadence from the pipe's position phase** rather than a
  per-attachment last-operation tick (pin limitation above); the rate and the
  batch size are upstream's.
- **Attachments are placed by right-click with the item** (upstream's path),
  but they cannot be removed or configured yet (no wrench, no screen).
- No waterlogging, no blacklist/wrench, no fluid/energy pipes, no attractor /
  filter / inhibitor, no upgrades, no machine extender, no config, no JEI/EMI/Jade.
