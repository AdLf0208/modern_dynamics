// Game-side glue for Modern Dynamics' item pipe (injected into `crate::game`
// as `moderndynamics_state`; it needs the game, so it is not part of the
// standalone library).
//
// Upstream a pipe is a block entity whose node keeps, per side, one attachment
// and the two connection masks; the item host runs each extractor's operation
// every 40 ticks. This slice keeps the attachments in the block entity's saved
// compound (an `attachments` mask, as upstream's node data is), draws the
// connections as the block's six boolean properties, and after the level's tick
// recomputes them and lets every extractor move its batch into the network's
// other endpoints.

use std::collections::BTreeSet;

use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::text::Text;
use horizon_core::geometry::Direction;
use horizon_world::nbt::{Compound, JavaString, Tag};

use crate::game::moderndynamics::{
    Attachments, Endpoint, FACE_COUNT, FACE_NAMES, Face, ITEMS_PER_OPERATION, Neighbour, Pos,
    connection_bits, destination_order, face_normal, is_connected, is_operation_tick, phase_of,
    reachable, spend_budget, step,
};
use crate::game::native_hooks::flags;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

/// The item pipe block (and its item).
pub(super) const PIPE: &str = "moderndynamics:item_pipe";
/// The extractor attachment item.
pub(super) const EXTRACTOR: &str = "moderndynamics:extractor";

fn key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match nbt.get(&key(name)) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// A pipe's saved attachments; a fresh pipe has none.
fn read_attachments(nbt: &Compound) -> Attachments {
    match int(nbt, "attachments") {
        Some(mask) => Attachments::from_saved(mask),
        None => Attachments::new(),
    }
}

fn write_attachments(nbt: &mut Compound, attachments: &Attachments) {
    nbt.insert(key("attachments"), Tag::Int(attachments.saved()));
}

/// A placed pipe gets a block entity; a fresh one holds no attachments, which
/// [`read_attachments`] answers for an empty compound.
pub(super) fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == PIPE && entity.is_none() {
        *entity = Some((PIPE.to_string(), Compound::new()));
    }
}

/// The pipe block entity at `pos`, if it is ours.
fn pipe_at<'a>(game: &'a crate::game::Game<'_>, pos: Pos) -> Option<&'a ModBlockEntity> {
    match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Mod(entity)) if entity.id == PIPE => Some(entity),
        _ => None,
    }
}

/// Every pipe in the level, in a stable order.
fn pipe_positions(game: &crate::game::Game<'_>) -> Vec<Pos> {
    let mut positions: Vec<Pos> = game
        .world
        .block_entities
        .iter()
        .filter_map(|(pos, entity)| match entity {
            BlockEntity::Mod(entity) if entity.id == PIPE => Some([pos.0, pos.1, pos.2]),
            _ => None,
        })
        .collect();
    positions.sort_unstable();
    positions
}

/// What the block beside face `face` of the pipe at `pos` is. A side that
/// carries an attachment never continues into another pipe (upstream's
/// `ItemHost.canConnectTo`), so it can only be an inventory or nothing.
fn neighbour(game: &crate::game::Game<'_>, dimension: &str, pos: Pos, face: Face) -> Neighbour {
    let beside = step(pos, face);
    let attached = pipe_at(game, pos).is_some_and(|entity| read_attachments(&entity.nbt).has(face));
    if !attached && game.native_block(beside) == PIPE {
        return Neighbour::Pipe;
    }
    // The neighbour is reached through the side facing the pipe.
    let across = Direction::ALL[face].opposite();
    match game.native_container_slots(dimension, beside, across) {
        Some(slots) if !slots.is_empty() => Neighbour::Inventory,
        _ => Neighbour::Nothing,
    }
}

fn neighbours(game: &crate::game::Game<'_>, dimension: &str, pos: Pos) -> [Neighbour; FACE_COUNT] {
    std::array::from_fn(|face| neighbour(game, dimension, pos, face))
}

/// Give the pipe at `pos` the drawn connection state `bits`, writing only the
/// faces that changed. The state is the block's six boolean properties, so the
/// drawn arms follow it.
fn show_connections(game: &mut crate::game::Game<'_>, pos: Pos, bits: u8) {
    for face in 0..FACE_COUNT {
        let want = is_connected(bits, face);
        let value = if want { "true" } else { "false" };
        if game.native_block_property(pos, FACE_NAMES[face]).as_deref() == Some(value) {
            continue;
        }
        // `flags::UPDATE_CLIENTS`, as a crop's growth: the clients get the
        // write and neighbours' shapes update, without neighbour updates of
        // their own.
        game.native_set_block_property_with(pos, FACE_NAMES[face], value, flags::UPDATE_CLIENTS);
    }
}

/// The network a pipe belongs to: every pipe reachable through other pipes.
fn network_of(game: &crate::game::Game<'_>, dimension: &str, start: Pos) -> Vec<Pos> {
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    let mut queue = vec![start];
    seen.insert(start);
    while let Some(pos) = queue.pop() {
        for face in 0..FACE_COUNT {
            if neighbour(game, dimension, pos, face) != Neighbour::Pipe {
                continue;
            }
            let beside = step(pos, face);
            if seen.insert(beside) {
                queue.push(beside);
            }
        }
    }
    seen.into_iter().collect()
}

/// One inventory a network reaches: the pipe and the face that reach it.
fn inventory_of(endpoint: Endpoint) -> (Pos, Direction) {
    let beside = step(endpoint.pipe, endpoint.face);
    (beside, Direction::ALL[endpoint.face].opposite())
}

/// The inventories the network's pipes reach, in pipe then face order.
fn endpoints(game: &crate::game::Game<'_>, dimension: &str, pipes: &[Pos]) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    for &pipe in pipes {
        for face in 0..FACE_COUNT {
            if neighbour(game, dimension, pipe, face) == Neighbour::Inventory {
                endpoints.push(Endpoint { pipe, face });
            }
        }
    }
    endpoints
}

/// Run the extraction operations of `pipes` for this tick: each attached side
/// that reaches an inventory moves up to [`ITEMS_PER_OPERATION`] items into the
/// network's other endpoints, nearest first.
fn tick_network(game: &mut crate::game::Game<'_>, dimension: &str, pipes: &[Pos], game_time: u64) {
    let attached: Vec<Endpoint> = pipes
        .iter()
        .flat_map(|&pipe| {
            let attachments = pipe_at(game, pipe)
                .map(|entity| read_attachments(&entity.nbt))
                .unwrap_or_default();
            (0..FACE_COUNT)
                .filter(move |&face| attachments.has(face))
                .map(move |face| Endpoint { pipe, face })
        })
        .collect();
    if attached.is_empty() {
        return;
    }
    let endpoints = endpoints(game, dimension, pipes);
    for source in attached {
        if !is_operation_tick(game_time, phase_of(source.pipe)) {
            continue;
        }
        if neighbour(game, dimension, source.pipe, source.face) != Neighbour::Inventory {
            continue;
        }
        // Upstream extracts from the side's inventory into the network, whose
        // paths never go back through that side (`side.getOpposite()`).
        let reach = reachable(pipes, source.pipe, |pos, face| {
            neighbour(game, dimension, pos, face) == Neighbour::Pipe
        });
        let order = destination_order(&reach, &endpoints, source);
        if order.is_empty() {
            continue;
        }
        let (from_pos, from_face) = inventory_of(source);
        spend_budget(ITEMS_PER_OPERATION, &order, |index, remaining| {
            let (to_pos, to_face) = inventory_of(endpoints[index]);
            game.native_move_items(
                dimension, from_pos, from_face, to_pos, to_face, None, remaining,
            )
        });
    }
}

/// After the level's tick: refresh every pipe's drawn connections and run the
/// extractors whose 40-tick operation falls on this tick.
pub(super) fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let all = pipe_positions(game);
    if all.is_empty() {
        return;
    }
    let game_time = game.world.game_time.max(0) as u64;
    let mut done: BTreeSet<Pos> = BTreeSet::new();
    for &pos in &all {
        if done.contains(&pos) {
            continue;
        }
        let network = network_of(game, dimension, pos);
        for &member in &network {
            let bits = connection_bits(neighbours(game, dimension, member));
            show_connections(game, member, bits);
        }
        tick_network(game, dimension, &network, game_time);
        done.extend(network);
    }
}

/// The face a clicked normal names, if it names one.
fn face_of_normal(normal: [i32; 3]) -> Option<Face> {
    (0..FACE_COUNT).find(|&face| face_normal(face) == normal)
}

/// Set or clear the extractor on one side of the pipe at `pos`. Answers
/// whether an item pipe was there. Used by the right-click placement (which
/// consumes the item) and by the `/moderndynamics attach|detach` harness path:
/// upstream decides the side from the click *point* on the connector, which the
/// pin's hooks only give as the clicked face.
fn set_attachment(game: &mut crate::game::Game<'_>, pos: Pos, face: Face, on: bool) -> bool {
    let Some(current) = pipe_at(game, pos).map(|entity| read_attachments(&entity.nbt)) else {
        return false;
    };
    let attachments = current.with(face, on);
    if let Some(BlockEntity::Mod(entity)) =
        game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
    {
        if entity.id == PIPE {
            write_attachments(&mut entity.nbt, &attachments);
        }
    }
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    true
}

/// Right-clicking a pipe with an extractor places it on the clicked side, as
/// upstream's `PipeBlockEntity.useItemOn` does; the item is consumed outside
/// creative. A side that already carries one is left for the (unported) screen.
pub(super) fn on_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: Pos,
    face_normal: [i32; 3],
    canceled: &mut bool,
) {
    if game.native_block(pos) != PIPE {
        return;
    }
    let Some((held, _, _)) = game.native_held() else {
        return;
    };
    if held != EXTRACTOR {
        return;
    }
    let Some(face) = face_of_normal(face_normal) else {
        return;
    };
    let Some(current) = pipe_at(game, pos).map(|entity| read_attachments(&entity.nbt)) else {
        return;
    };
    if current.has(face) {
        // Upstream opens the attachment's screen here; this slice has no screen.
        *canceled = true;
        game.native_feedback(Text::literal(format!(
            "Modern Dynamics: this side already has an extractor ({})",
            FACE_NAMES[face]
        )));
        return;
    }
    set_attachment(game, pos, face, true);
    // Creative players keep their item, as upstream's `stack.shrink` guard does.
    if game.native_acting_player().is_none_or(|player| player.game_mode != 1) {
        game.native_take_items(EXTRACTOR, 1);
    }
    *canceled = true;
    game.native_feedback(Text::literal(format!(
        "Modern Dynamics: extractor placed on the {} side",
        FACE_NAMES[face]
    )));
}

/// Declare `/moderndynamics info|attach|detach`.
pub(super) fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("moderndynamics info <pos:block_pos>"));
    commands.push(NativeCommand::new(
        "moderndynamics attach <pos:block_pos> <side:word>",
    ));
    commands.push(NativeCommand::new(
        "moderndynamics detach <pos:block_pos> <side:word>",
    ));
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

fn parse_pos(words: &[&str; 3]) -> Option<Pos> {
    Some([
        words[0].parse().ok()?,
        words[1].parse().ok()?,
        words[2].parse().ok()?,
    ])
}

/// Run `/moderndynamics info <x> <y> <z>`, `attach <x> <y> <z> <side>` or
/// `detach <x> <y> <z> <side>`: the connections, the attachments and the
/// extraction cadence (the harness readout; upstream's screen is a gap).
pub(super) fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["moderndynamics", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /moderndynamics info <x> <y> <z>",
                ));
                return;
            };
            let Some(entity) = pipe_at(game, pos) else {
                game.native_feedback(Text::literal("Modern Dynamics: no item pipe there."));
                return;
            };
            let attachments = read_attachments(&entity.nbt);
            let connected = face_list(|face| {
                game.native_block_property(pos, FACE_NAMES[face]).as_deref() == Some("true")
            });
            game.native_feedback(Text::literal(format!(
                "Item Pipe at {x},{y},{z}: links {connected}"
            )));
            game.native_feedback(Text::literal(format!(
                "Item Pipe at {x},{y},{z}: extractors {}; {} items every {} ticks",
                face_list(|face| attachments.has(face)),
                ITEMS_PER_OPERATION,
                crate::game::moderndynamics::OPERATION_TICK_DELAY
            )));
        }
        ["moderndynamics", action @ ("attach" | "detach"), x, y, z, side] => {
            *ran = true;
            let on = *action == "attach";
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /moderndynamics attach|detach <x> <y> <z> <side>",
                ));
                return;
            };
            let Some(face) = crate::game::moderndynamics::face_named(&side.to_lowercase()) else {
                game.native_feedback(Text::literal(format!(
                    "Unknown side '{side}': down, up, north, south, west or east."
                )));
                return;
            };
            if !set_attachment(game, pos, face, on) {
                game.native_feedback(Text::literal("Modern Dynamics: no item pipe there."));
                return;
            }
            game.native_feedback(Text::literal(format!(
                "Item Pipe: extractor on the {} side {}",
                FACE_NAMES[face],
                if on { "on" } else { "off" }
            )));
        }
        _ => {}
    }
}
