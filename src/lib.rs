//! Modern Dynamics — the item pipe, its connections and the extractor, as one `impl Hooks`.
//!
//! Upstream a pipe is a block entity whose node keeps, per side, one attachment and the two
//! connection masks; the item host runs each extractor's operation every forty ticks. Here the
//! attachments are the pipe block entity's saved `attachments` mask, the connections are the
//! block's six boolean properties (recomputed in the pipe's own tick), and each extractor moves
//! its batch into the network's other endpoints through the `horizon:items` capability.
//!
//! The rules — the faces, the rate, the network walk, the destination order and the batch
//! budget — live in [`runtime`], byte-identical to the retired `.hnmod` bundle's `runtime.rs`
//! and tested by `cargo test` without the game. This file is only the game glue.
pub mod runtime;

use std::collections::BTreeSet;

use mod_api::{
    self, BlockEntityPart, BlockEntityRef, BlockEntityTickEvent, BlockEntityTicksEvent, BlockPart,
    BlockUpdateFlags, Capability, Command, CommandArgumentValue, Compound, Direction, Flow, Hand,
    Hooks, HostTransaction, ITEMS, Identifier, InteractionResult, ItemPlace, ItemResource,
    ItemStack, LevelRef, RegisterCommandsEvent, Registrar, RegistrationContext, ResourceHandler,
    RunModCommandEvent, ServerHost, Tag, Text, UseItemOnEvent, move_resources,
};

use runtime::{
    ADMIN_LEVEL, AttachmentKind, Attachments, Endpoint, FACE_COUNT, FACE_NAMES, Face,
    ITEMS_PER_OPERATION, MAX_NETWORK_PIPES, Neighbour, OPERATION_TICK_DELAY, Pos,
    attractor_sources, connection_bits, destination_order, face_named, face_normal, is_connected,
    is_operation_tick, may_administer, phase_of, reachable, spend_budget, step,
};

/// The item pipe block (and its item).
const PIPE: &str = "moderndynamics:item_pipe";
/// The extractor attachment item.
const EXTRACTOR: &str = "moderndynamics:extractor";
/// The attractor attachment item.
const ATTRACTOR: &str = "moderndynamics:attractor";

// The pipe's saved attachments are kept in the shapes a mod block entity's compound holds, as the
// retired bundle saved them: one `attachments` int, a bit per side.
//
// A compound's key type is UTF-16 and the API does not export it, so a key is taken from
// `Tag::String`'s own payload: the same conversion a mod's NBT string uses, with no name needed.

fn key_tag(name: &str) -> Tag {
    Tag::String(name.into())
}

fn tag<'a>(nbt: &'a Compound, name: &str) -> Option<&'a Tag> {
    let Tag::String(utf16) = key_tag(name) else {
        return None;
    };
    nbt.get(&utf16)
}

fn put(nbt: &mut Compound, name: &str, value: Tag) {
    if let Tag::String(utf16) = key_tag(name) {
        nbt.insert(utf16, value);
    }
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match tag(nbt, name) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// A pipe's saved attachments; a fresh pipe has none.
fn read_attachments(nbt: &Compound) -> Attachments {
    Attachments::from_saved(
        int(nbt, "attachments").unwrap_or(0),
        int(nbt, "attractors").unwrap_or(0),
    )
}

fn write_attachments(nbt: &mut Compound, attachments: &Attachments) {
    put(nbt, "attachments", Tag::Int(attachments.extractor_mask()));
    put(nbt, "attractors", Tag::Int(attachments.attractor_mask()));
}

/// The item a kind is placed with, and the item it drops as.
const fn item_of(kind: AttachmentKind) -> &'static str {
    match kind {
        AttachmentKind::Extractor => EXTRACTOR,
        AttachmentKind::Attractor => ATTRACTOR,
    }
}

/// The authoritative level the server ticks now: the one a block entity's tick belongs to (a
/// parked level's block entities are not ticked, and a client of a server ticks no level).
fn in_place_level(host: &mut dyn ServerHost) -> Option<LevelRef> {
    host.levels()
        .list_levels()
        .into_iter()
        .find(|level| level.in_place(host))
}

/// The pipe block entity at `pos`, if the block there is ours.
fn pipe_at(host: &mut dyn ServerHost, level: LevelRef, pos: Pos) -> Option<BlockEntityRef> {
    let block = level.block(host, pos)?;
    (block.id(host) == PIPE)
        .then(|| host.block_entity(level, pos))
        .flatten()
}

/// The attachments the pipe at `pos` carries, read from its saved compound. The pipe being
/// ticked is out of the world, so a tick passes its own mask instead (see `own` in
/// [`pipe_tick`]).
fn attachments_at(host: &mut dyn ServerHost, level: LevelRef, pos: Pos) -> Attachments {
    pipe_at(host, level, pos)
        .and_then(|entity| entity.data(host))
        .map(|data| read_attachments(&data))
        .unwrap_or_default()
}

/// What the block beside face `face` of the pipe at `pos` is, given that pipe's attachments. A
/// side that carries an attachment never continues into another pipe (upstream's
/// `ItemHost.canConnectTo`), so it can only be an inventory or nothing.
fn neighbour(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
    attachments: &Attachments,
) -> Neighbour {
    let beside = step(pos, face);
    if !attachments.has(face)
        && let Some(block) = level.block(host, beside)
        && block.id(host) == PIPE
    {
        return Neighbour::Pipe;
    }
    match inventory(host, level, beside, face) {
        Some(_) => Neighbour::Inventory,
        None => Neighbour::Nothing,
    }
}

fn neighbours(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    attachments: &Attachments,
) -> [Neighbour; FACE_COUNT] {
    std::array::from_fn(|face| neighbour(host, level, pos, face, attachments))
}

/// The item handler of the block beside face `face` of a pipe, if that block has one reachable
/// from the touching side (the pin's `ITEMS` capability, NeoForge's `Capabilities.Item.BLOCK`).
fn inventory(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
) -> Option<Capability<dyn ResourceHandler<ItemResource>>> {
    // The neighbour is reached through the side facing the pipe.
    let side = Direction::ALL[face].opposite();
    let handler = level.capability(host, &ITEMS, pos, Some(side))?;
    (handler.size(host) > 0).then_some(handler)
}

/// Give the pipe at `pos` the drawn connection state `bits`, writing only the faces that
/// changed. The state is the block's six boolean properties, so the drawn arms follow it.
fn show_connections(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, bits: u8) {
    for (face, name) in FACE_NAMES.iter().enumerate() {
        let want = if is_connected(bits, face) {
            "true"
        } else {
            "false"
        };
        if level.block_property(host, pos, name).as_deref() == Some(want) {
            continue;
        }
        // `UPDATE_CLIENTS`, as a crop's growth: the clients get the write and neighbours'
        // shapes update, without neighbour updates of their own.
        host.set_block_property_with(pos, name, want, BlockUpdateFlags::CLIENTS.bits());
    }
}

/// Every pipe reachable from `start` through pipe-to-pipe links, at most
/// [`MAX_NETWORK_PIPES`] of them: the same budget the runtime's walk keeps, so a
/// griefed or machine-built pipe line cannot make one operation walk it all. A set
/// answers membership, as the runtime's walk does.
fn network_of(
    host: &mut dyn ServerHost,
    level: LevelRef,
    start: Pos,
    own: &Attachments,
) -> Vec<Pos> {
    let mut found = vec![start];
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    seen.insert(start);
    let mut next = 0;
    while next < found.len() && found.len() < MAX_NETWORK_PIPES {
        let pos = found[next];
        next += 1;
        let attachments = if pos == start {
            *own
        } else {
            attachments_at(host, level, pos)
        };
        for face in 0..FACE_COUNT {
            if neighbour(host, level, pos, face, &attachments) != Neighbour::Pipe {
                continue;
            }
            let beside = step(pos, face);
            if seen.insert(beside) {
                found.push(beside);
                if found.len() == MAX_NETWORK_PIPES {
                    break;
                }
            }
        }
    }
    found.sort_unstable();
    found
}

/// The inventories the network's pipes reach, in pipe then face order. `own` is the ticked
/// pipe's mask, which its out-of-the-world entity cannot be read for. A side whose attachment
/// is an extractor is not an endpoint: its inventory is what that extractor takes from
/// (upstream's `allowsItemConnection`, false for a servo only).
fn endpoints(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pipes: &[Pos],
    own_pos: Pos,
    own: &Attachments,
) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    for &pipe in pipes {
        let attachments = if pipe == own_pos {
            *own
        } else {
            attachments_at(host, level, pipe)
        };
        for face in 0..FACE_COUNT {
            if attachments.accepts_endpoints(face)
                && neighbour(host, level, pipe, face, &attachments) == Neighbour::Inventory
            {
                endpoints.push(Endpoint { pipe, face });
            }
        }
    }
    endpoints
}

/// The pipe-to-pipe links of `pipes`, as (pipe, face) pairs the network walk follows.
fn pipe_links(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pipes: &[Pos],
    own_pos: Pos,
    own: &Attachments,
) -> BTreeSet<(Pos, Face)> {
    let mut links = BTreeSet::new();
    for &pipe in pipes {
        let attachments = if pipe == own_pos {
            *own
        } else {
            attachments_at(host, level, pipe)
        };
        for face in 0..FACE_COUNT {
            if neighbour(host, level, pipe, face, &attachments) == Neighbour::Pipe {
                links.insert((pipe, face));
            }
        }
    }
    links
}

/// Run the extraction operations of the pipe at `pos` for this tick: each attached side that
/// reaches an inventory moves up to [`ITEMS_PER_OPERATION`] items into the network's other
/// endpoints, nearest first, one destination after another until the batch is spent.
fn run_extractors(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, own: &Attachments) {
    if (0..FACE_COUNT).all(|face| own.kind(face) != Some(AttachmentKind::Extractor)) {
        return;
    }
    let Some(game_time) = level.game_time(host) else {
        return;
    };
    let game_time = game_time.max(0) as u64;
    if !is_operation_tick(game_time, phase_of(pos)) {
        return;
    }
    // The extracting sides first: a pipe whose attachments all face air or another pipe has
    // no source, so it never builds the network at all (six neighbour reads instead of a walk).
    let sources: Vec<Face> = (0..FACE_COUNT)
        .filter(|&face| own.kind(face) == Some(AttachmentKind::Extractor))
        .filter(|&face| neighbour(host, level, pos, face, own) == Neighbour::Inventory)
        .collect();
    if sources.is_empty() {
        return;
    }
    let pipes = network_of(host, level, pos, own);
    let endpoints = endpoints(host, level, &pipes, pos, own);
    // `reachable` takes a read-only predicate, so the links are worked out first: the
    // neighbour read needs the host mutably.
    let links = pipe_links(host, level, &pipes, pos, own);
    let reach = reachable(&pipes, pos, |pipe, face| links.contains(&(pipe, face)));
    for face in sources {
        let source = Endpoint { pipe: pos, face };
        let order = destination_order(&reach, &endpoints, source);
        if order.is_empty() {
            continue;
        }
        let Some(from) = inventory(host, level, step(pos, face), face) else {
            continue;
        };
        let targets: Vec<(usize, Capability<dyn ResourceHandler<ItemResource>>)> = order
            .iter()
            .filter_map(|&index| {
                let endpoint = endpoints[index];
                inventory(
                    host,
                    level,
                    step(endpoint.pipe, endpoint.face),
                    endpoint.face,
                )
                .map(|to| (index, to))
            })
            .collect();
        if targets.is_empty() {
            continue;
        }
        spend_budget(ITEMS_PER_OPERATION, &order, |index, remaining| {
            let Some((_, to)) = targets.iter().find(|(target, _)| *target == index) else {
                return 0;
            };
            host.transaction(|host, tx| move_resources(host, &from, to, |_| true, remaining, tx))
                .unwrap_or(0)
        });
    }
}

/// Run the attractor operations of the pipe at `pos` for this tick: each attractor side that
/// faces an inventory pulls up to [`ITEMS_PER_OPERATION`] items out of the network's other
/// inventories, nearest first, one source after another until the batch is spent (upstream's
/// `ItemHost.tickAttractor`, the reverse of the extractor).
fn run_attractors(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, own: &Attachments) {
    if (0..FACE_COUNT).all(|face| own.kind(face) != Some(AttachmentKind::Attractor)) {
        return;
    }
    let Some(game_time) = level.game_time(host) else {
        return;
    };
    let game_time = game_time.max(0) as u64;
    if !is_operation_tick(game_time, phase_of(pos)) {
        return;
    }
    // Only an attractor whose side reaches an inventory has anywhere to put its batch.
    let targets: Vec<(Face, Capability<dyn ResourceHandler<ItemResource>>)> = (0..FACE_COUNT)
        .filter(|&face| own.kind(face) == Some(AttachmentKind::Attractor))
        .filter_map(|face| inventory(host, level, step(pos, face), face).map(|to| (face, to)))
        .collect();
    if targets.is_empty() {
        return;
    }
    let pipes = network_of(host, level, pos, own);
    let endpoints = endpoints(host, level, &pipes, pos, own);
    let links = pipe_links(host, level, &pipes, pos, own);
    let reach = reachable(&pipes, pos, |pipe, face| links.contains(&(pipe, face)));
    // Which endpoint sides carry an attractor of their own: the predicate `attractor_sources`
    // takes is read-only, so the masks are worked out first, as `pipe_links` does.
    let mut attractor_sites: BTreeSet<(Pos, Face)> = BTreeSet::new();
    for &pipe in &pipes {
        let attachments = if pipe == pos {
            *own
        } else {
            attachments_at(host, level, pipe)
        };
        for face in 0..FACE_COUNT {
            if attachments.kind(face) == Some(AttachmentKind::Attractor) {
                attractor_sites.insert((pipe, face));
            }
        }
    }
    for (face, to) in &targets {
        let source = Endpoint {
            pipe: pos,
            face: *face,
        };
        let order = attractor_sources(&reach, &endpoints, source, |pipe, site| {
            attractor_sites.contains(&(pipe, site))
        });
        spend_budget(ITEMS_PER_OPERATION, &order, |index, remaining| {
            let endpoint = endpoints[index];
            let Some(from) = inventory(
                host,
                level,
                step(endpoint.pipe, endpoint.face),
                endpoint.face,
            ) else {
                return 0;
            };
            host.transaction(|host, tx| move_resources(host, &from, to, |_| true, remaining, tx))
                .unwrap_or(0)
        });
    }
}

/// One server tick of a pipe: recompute its six connections, then let each attachment that falls
/// on this tick move its batch — the extractors out of their sides, the attractors into theirs.
fn pipe_tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let Some(level) = in_place_level(host) else {
        return;
    };
    let pos = event.pos;
    let own = read_attachments(&event.entity.nbt);
    let bits = connection_bits(neighbours(host, level, pos, &own));
    show_connections(host, level, pos, bits);
    // The transfer reads the saved mask, so the tick's nbt snapshot is taken before any
    // property write: the attachment masks themselves are not changed by a tick.
    run_extractors(host, level, pos, &own);
    run_attractors(host, level, pos, &own);
}

/// The face a clicked normal names, if it names one.
fn face_of_normal(normal: [i32; 3]) -> Option<Face> {
    (0..FACE_COUNT).find(|&face| face_normal(face) == normal)
}

/// Set one side of the pipe at `pos` to `kind`, or clear it with `None`. Answers whether it
/// could be written. Used by the right-click placement and by the `/moderndynamics attach|detach`
/// harness path: upstream decides the side from the click *point* on the connector, which the
/// pin's hooks only give as the clicked face.
fn set_attachment(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
    kind: Option<AttachmentKind>,
) -> bool {
    let Some(entity) = pipe_at(host, level, pos) else {
        return false;
    };
    let Some(mut data) = entity.data(host) else {
        return false;
    };
    let attachments = read_attachments(&data).with(face, kind);
    write_attachments(&mut data, &attachments);
    entity.replace_data(host, data).is_ok()
}

/// The faces a predicate picks, as `north,south`.
fn face_list(pick: impl Fn(Face) -> bool) -> String {
    let names: Vec<&str> = (0..FACE_COUNT)
        .filter(|&face| pick(face))
        .map(|face| FACE_NAMES[face])
        .collect();
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(",")
    }
}

fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

fn tell(host: &mut dyn ServerHost, player: u128, line: String) {
    if let Some(player) = host.player(player) {
        let _ = player.tell(host, Text::literal(line));
    }
}

/// The item pipe: its block entity (the saved attachments and the tick) and its block (the
/// right-click placement with an attachment item).
struct Pipe;

#[mod_api::horizon_part]
impl BlockPart for Pipe {
    /// Right-clicking a pipe with an extractor or an attractor places it on the clicked side, as
    /// upstream's `PipeBlockEntity.useItemOn` does; the item is consumed outside creative. A side
    /// that already carries one is left for the (unported) screen.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<InteractionResult> {
        let click = event.click;
        let stack = click.stack?;
        let held = stack.item().id(host);
        let kind = match held.as_str() {
            EXTRACTOR => AttachmentKind::Extractor,
            ATTRACTOR => AttachmentKind::Attractor,
            _ => return None,
        };
        let level = in_place_level(host)?;
        let face = face_of_normal(click.hit.face)?;
        let entity = pipe_at(host, level, click.pos)?;
        let data = entity.data(host)?;
        if let Some(already) = read_attachments(&data).kind(face) {
            // Upstream opens the attachment's screen here; this slice has no screen.
            tell(
                host,
                click.player,
                format!(
                    "Modern Dynamics: this side already has an {} ({})",
                    already.name(),
                    FACE_NAMES[face]
                ),
            );
            return Some(InteractionResult::Success);
        }
        // One item leaves the hand and the side gets the attachment, or neither happens: both
        // writes share a transaction, so a failed placement never eats the item and a lost item
        // never leaves a free attachment behind. A creative player's hand gives without losing
        // any, as upstream's `stack.shrink` guard does.
        let place = ItemPlace::PlayerHand {
            player: click.player,
            hand: click.hand,
        };
        let resource = ItemResource::of(stack);
        let placed = host
            .transaction(|host, tx| {
                if place.extract(host, tx, &resource, 1) == 0 {
                    return Err(());
                }
                if set_attachment(host, level, click.pos, face, Some(kind)) {
                    Ok(())
                } else {
                    Err(())
                }
            })
            .is_ok();
        if !placed {
            return None;
        }
        tell(
            host,
            click.player,
            format!(
                "Modern Dynamics: {} placed on the {} side",
                kind.name(),
                FACE_NAMES[face]
            ),
        );
        Some(InteractionResult::Success)
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for Pipe {
    /// A pipe's block entity ticks on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _host: &dyn mod_api::Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        pipe_tick(host, event);
    }
}

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the pipe's block entity type and attach the pipe's behaviour to it.
    fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
        let kind = r.block_entity_type("item_pipe").blocks(&[PIPE]).build();
        let _ = r.block_entity_part(&kind, Pipe);
    }

    /// `/moderndynamics info|attach|detach`. `info` is a readout of the pipe a player names
    /// and needs no authority; `attach`/`detach` edit another player's pipe from any
    /// distance, so they are registered at the operator level `/setblock` needs.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event
            .commands
            .push(Command::new("moderndynamics info <pos:block_pos>"));
        event.commands.push(Command::op(
            "moderndynamics attach <pos:block_pos> <side:word> [kind:word]",
            ADMIN_LEVEL,
        ));
        event.commands.push(Command::op(
            "moderndynamics detach <pos:block_pos> <side:word>",
            ADMIN_LEVEL,
        ));
    }

    /// Run the `/moderndynamics` commands a player types: the connections, the attachments and
    /// the extraction cadence (the harness readout; upstream's screen is a gap).
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let invocation = event.command;
        let words: Vec<&str> = invocation.literals.iter().map(String::as_str).collect();
        if words.first() != Some(&"moderndynamics") {
            return Flow::Continue;
        }
        let Some(action) = words.get(1).copied() else {
            return Flow::Continue;
        };
        if !matches!(action, "info" | "attach" | "detach") {
            return Flow::Continue;
        }
        let Some(CommandArgumentValue::BlockPos(pos)) = invocation.argument("pos") else {
            feedback(
                host,
                "Usage: /moderndynamics info <x> <y> <z> or attach|detach <x> <y> <z> <side>",
            );
            return Flow::Cancel;
        };
        let pos = *pos;
        let level = host
            .levels()
            .list_levels()
            .into_iter()
            .find(|level| level.dimension() == invocation.dimension);
        let Some(level) = level else {
            feedback(host, "Modern Dynamics: no item pipe there.");
            return Flow::Cancel;
        };
        match action {
            "info" => {
                let Some(entity) = pipe_at(host, level, pos) else {
                    feedback(host, "Modern Dynamics: no item pipe there.");
                    return Flow::Cancel;
                };
                let Some(data) = entity.data(host) else {
                    feedback(host, "Modern Dynamics: no item pipe there.");
                    return Flow::Cancel;
                };
                let attachments = read_attachments(&data);
                let connected = face_list(|face| {
                    level.block_property(host, pos, FACE_NAMES[face]).as_deref() == Some("true")
                });
                feedback(
                    host,
                    &format!(
                        "Item Pipe at {},{},{}: links {connected}",
                        pos[0], pos[1], pos[2]
                    ),
                );
                feedback(
                    host,
                    &format!(
                        "Item Pipe at {},{},{}: extractors {}; attractors {}; {} items every {} ticks",
                        pos[0],
                        pos[1],
                        pos[2],
                        face_list(|face| attachments.kind(face) == Some(AttachmentKind::Extractor)),
                        face_list(|face| attachments.kind(face) == Some(AttachmentKind::Attractor)),
                        ITEMS_PER_OPERATION,
                        OPERATION_TICK_DELAY
                    ),
                );
            }
            _ => {
                // Defence in depth: the tree already refuses a usage below its level, but a
                // merged command tree is not a guarantee this body should rely on.
                if !may_administer(event.source.permission().level()) {
                    feedback(
                        host,
                        &format!(
                            "Modern Dynamics: /moderndynamics {action} needs an operator (permission level {ADMIN_LEVEL})."
                        ),
                    );
                    return Flow::Cancel;
                }
                let Some(CommandArgumentValue::String(side)) = invocation.argument("side") else {
                    feedback(
                        host,
                        "Usage: /moderndynamics attach <x> <y> <z> <side> [extractor|attractor], detach <x> <y> <z> <side>",
                    );
                    return Flow::Cancel;
                };
                let Some(face) = face_named(&side.to_lowercase()) else {
                    feedback(
                        host,
                        &format!("Unknown side '{side}': down, up, north, south, west or east."),
                    );
                    return Flow::Cancel;
                };
                // `attach` without a kind is the extractor, as the extractor-only slice's
                // form always meant; `detach` clears whatever the side carries.
                let kind = if action == "attach" {
                    match invocation.argument("kind") {
                        None => Some(AttachmentKind::Extractor),
                        Some(CommandArgumentValue::String(word)) => {
                            let named = AttachmentKind::named(&word.to_lowercase());
                            if named.is_none() {
                                feedback(
                                    host,
                                    &format!(
                                        "Unknown attachment '{word}': extractor or attractor."
                                    ),
                                );
                                return Flow::Cancel;
                            }
                            named
                        }
                        Some(_) => {
                            feedback(
                                host,
                                "Usage: /moderndynamics attach <x> <y> <z> <side> [extractor|attractor]",
                            );
                            return Flow::Cancel;
                        }
                    }
                } else {
                    None
                };
                if !set_attachment_command(host, level, pos, face, kind, attach_payer(event)) {
                    feedback(
                        host,
                        "Modern Dynamics: no item pipe there, or the attachment item is not in hand.",
                    );
                    return Flow::Cancel;
                }
                feedback(
                    host,
                    &format!(
                        "Item Pipe: {} on the {} side {}",
                        kind.unwrap_or(AttachmentKind::Extractor).name(),
                        FACE_NAMES[face],
                        if kind.is_some() { "on" } else { "off" }
                    ),
                );
            }
        }
        Flow::Cancel
    }
}

/// Who pays for an `/moderndynamics attach`: the acting player's main hand, or nobody when the
/// source has no hand (the console, a command block, a function). The administrative path is an
/// operator's, so its write is the write `/setblock` makes; the item is still the player's cost
/// when a player is the one attaching, exactly as the right-click path charges it.
fn attach_payer(event: &RunModCommandEvent<'_>) -> Option<u128> {
    match event.source {
        mod_api::CommandSource::Player { player, .. } => Some(player.uuid()),
        _ => None,
    }
}

/// Set one side to `kind`, or clear it, from a command: an `attach` from a player takes that
/// kind's item from the player's main hand inside the same transaction as the mask write (so a
/// refused placement never eats an item, and a lost item never leaves a free attachment behind).
/// `detach` costs nothing and never returns the item, as upstream's wrench takes it off.
fn set_attachment_command(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: Pos,
    face: Face,
    kind: Option<AttachmentKind>,
    payer: Option<u128>,
) -> bool {
    let Some(kind) = kind else {
        return set_attachment(host, level, pos, face, None);
    };
    let Some(player) = payer else {
        // No hand: the console's own path costs nothing, as `/setblock` does.
        return set_attachment(host, level, pos, face, Some(kind));
    };
    let Some(id) = Identifier::parse_namespaced(item_of(kind)) else {
        return false;
    };
    let Some(key) = host.item_key(&id) else {
        return false;
    };
    let resource = ItemResource::of(&ItemStack::of(key));
    let place = ItemPlace::PlayerHand {
        player,
        hand: Hand::Main,
    };
    host.transaction(|host, tx| {
        if place.extract(host, tx, &resource, 1) == 0 {
            return Err(());
        }
        if set_attachment(host, level, pos, face, Some(kind)) {
            Ok(())
        } else {
            Err(())
        }
    })
    .is_ok()
}
