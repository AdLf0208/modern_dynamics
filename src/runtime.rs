// Pure item-pipe logic for Modern Dynamics (standalone crate lib, injected as
// `crate::game::moderndynamics`). No game access lives here.
//
// Upstream's item pipe moves nothing by itself: an `extractor` attachment on
// one of its six sides pulls up to `ITEMS_PER_OPERATION` items from the
// inventory on that side, every `OPERATION_TICK_DELAY` ticks, and inserts them
// into the inventories at the network's other endpoints, nearest first
// (`RoutingMode.CLOSEST`, the default). Connections are per side: another pipe
// continues the network, an inventory is an endpoint, anything else is not
// connected.

/// One of a pipe's six sides, in vanilla's `Direction.values()` order.
pub type Face = usize;

/// A block position, as the glue reads it from the level.
pub type Pos = [i32; 3];

pub const FACE_COUNT: usize = 6;

pub const DOWN: Face = 0;
pub const UP: Face = 1;
pub const NORTH: Face = 2;
pub const SOUTH: Face = 3;
pub const WEST: Face = 4;
pub const EAST: Face = 5;

/// `Direction.values()`'s names, which are also the block's state properties.
pub const FACE_NAMES: [&str; FACE_COUNT] = ["down", "up", "north", "south", "west", "east"];

/// Upstream `UpgradeContainer::getItemsPerOperation` with no upgrades: the
/// most items one extraction operation takes.
pub const ITEMS_PER_OPERATION: u32 = 4;

/// Upstream `UpgradeContainer::getItemOperationTickDelay` with no upgrades:
/// the ticks between two extraction operations.
pub const OPERATION_TICK_DELAY: u64 = 40;

/// Every face's connection bit, for [`connection_bits`]' `u8`.
const ALL_FACES: u8 = (1 << FACE_COUNT) - 1;

/// The face's unit vector, as `Direction.normal()` gives it.
pub const fn face_normal(face: Face) -> [i32; 3] {
    match face {
        DOWN => [0, -1, 0],
        UP => [0, 1, 0],
        NORTH => [0, 0, -1],
        SOUTH => [0, 0, 1],
        WEST => [-1, 0, 0],
        EAST => [1, 0, 0],
        _ => [0, 0, 0],
    }
}

/// The position one step from `pos` through `face`.
pub const fn step(pos: Pos, face: Face) -> Pos {
    let [dx, dy, dz] = face_normal(face);
    [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

/// The face a state property or command names (`down`..`east`), if it names one.
pub fn face_named(name: &str) -> Option<Face> {
    FACE_NAMES.iter().position(|known| *known == name)
}

/// What a side of a pipe reaches, read from the block beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neighbour {
    /// Air, or a block that is neither a pipe nor an item container.
    Nothing,
    /// Another item pipe, which the network continues through.
    Pipe,
    /// A block with an item container reachable from the touching side.
    Inventory,
}

/// The drawn connection state: a side connects to another pipe or to a
/// container, as upstream's `pipeConnections | inventoryConnections` does.
pub fn connection_bits(neighbours: [Neighbour; FACE_COUNT]) -> u8 {
    neighbours
        .iter()
        .enumerate()
        .filter(|(_, neighbour)| matches!(neighbour, Neighbour::Pipe | Neighbour::Inventory))
        .fold(0, |bits, (face, _)| bits | (1 << face))
}

/// Whether `face` is connected in a state from [`connection_bits`].
pub fn is_connected(bits: u8, face: Face) -> bool {
    face < FACE_COUNT && bits & (1 << face) != 0
}

/// The extractor attachments a pipe carries, one bit per side (upstream keeps
/// one attachment per side in its node host; only the extractor is placed by
/// this slice).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attachments {
    mask: u8,
}

impl Attachments {
    /// No attachment on any side, as a freshly placed pipe.
    pub const fn new() -> Self {
        Attachments { mask: 0 }
    }

    pub const fn has(&self, face: Face) -> bool {
        face < FACE_COUNT && self.mask & (1 << face) != 0
    }

    pub const fn with(self, face: Face, on: bool) -> Self {
        if face >= FACE_COUNT {
            return self;
        }
        let bit = 1 << face;
        Attachments {
            mask: if on {
                self.mask | bit
            } else {
                self.mask & !bit
            },
        }
    }

    /// The mask the block entity saves.
    pub const fn saved(&self) -> i32 {
        self.mask as i32
    }

    /// Restore the mask [`Attachments::saved`] wrote; bits past the six sides
    /// are dropped and a value that is not a mask at all falls back to none, so
    /// a damaged save behaves as a fresh pipe.
    pub fn from_saved(mask: i32) -> Self {
        Attachments {
            mask: u8::try_from(mask).map_or(0, |mask| mask & ALL_FACES),
        }
    }
}

/// A pipe's operation phase, from its position: it spreads a level's pipes
/// over the 40-tick cadence instead of moving them all on the same tick
/// (upstream ticks each attachment on its own counter).
pub fn phase_of(pos: Pos) -> u64 {
    (i64::from(pos[0]) + i64::from(pos[1]) + i64::from(pos[2]))
        .rem_euclid(OPERATION_TICK_DELAY as i64) as u64
}

/// Whether this tick is the pipe's extraction tick.
pub fn is_operation_tick(game_time: u64, phase: u64) -> bool {
    game_time
        .wrapping_add(phase)
        .is_multiple_of(OPERATION_TICK_DELAY)
}

/// Every pipe reachable from `start` through pipe-to-pipe links, with its
/// graph distance. `linked(pos, face)` answers whether the pipe at `pos`
/// continues into the pipe beside that face.
pub fn reachable(pipes: &[Pos], start: Pos, linked: impl Fn(Pos, Face) -> bool) -> Vec<(Pos, u32)> {
    let mut found: Vec<(Pos, u32)> = vec![(start, 0)];
    let mut next = 0;
    while next < found.len() {
        let (pos, distance) = found[next];
        next += 1;
        for face in 0..FACE_COUNT {
            let beside = step(pos, face);
            if !pipes.contains(&beside) || !linked(pos, face) {
                continue;
            }
            if found.iter().any(|(seen, _)| *seen == beside) {
                continue;
            }
            found.push((beside, distance + 1));
        }
    }
    found
}

/// One inventory a pipe reaches: the pipe and the face that reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    pub pipe: Pos,
    pub face: Face,
}

/// The order an extraction operation tries the network's destinations in: the
/// endpoints nearest the extracting pipe first, then face order (upstream's
/// `RoutingMode.CLOSEST`, its default). The extractor's own side is left out,
/// so it never sends items back into the inventory it took them from, and an
/// endpoint with no pipe in `reachable` is not in this network at all.
pub fn destination_order(
    reachable: &[(Pos, u32)],
    endpoints: &[Endpoint],
    source: Endpoint,
) -> Vec<usize> {
    let mut order: Vec<(u32, Face, usize)> = endpoints
        .iter()
        .enumerate()
        .filter(|(_, endpoint)| **endpoint != source)
        .filter_map(|(index, endpoint)| {
            let distance = reachable
                .iter()
                .find(|(pos, _)| *pos == endpoint.pipe)
                .map(|(_, distance)| *distance)?;
            Some((distance, endpoint.face, index))
        })
        .collect();
    order.sort_unstable();
    order.into_iter().map(|(_, _, index)| index).collect()
}

/// Run one operation's budget down the destination `order`: each destination
/// takes what it can, and the rest goes to the next. Returns how much moved,
/// never more than `budget`.
pub fn spend_budget(
    budget: u32,
    order: &[usize],
    mut insert: impl FnMut(usize, u32) -> u32,
) -> u32 {
    let mut spent = 0;
    for &index in order {
        let remaining = budget - spent;
        if remaining == 0 {
            break;
        }
        spent += insert(index, remaining).min(remaining);
    }
    spent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faces_follow_vanillas_direction_order() {
        assert_eq!(FACE_NAMES, ["down", "up", "north", "south", "west", "east"]);
        for (index, name) in FACE_NAMES.iter().enumerate() {
            assert_eq!(face_named(name), Some(index), "{name}");
        }
        assert_eq!(face_named("sideways"), None);
        assert_eq!(face_normal(NORTH), [0, 0, -1]);
        assert_eq!(face_normal(EAST), [1, 0, 0]);
        assert_eq!(face_normal(SOUTH), [0, 0, 1]);
        assert_eq!(step([4, 5, 6], UP), [4, 6, 6]);
        assert_eq!(step([4, 5, 6], WEST), [3, 5, 6]);
    }

    #[test]
    fn the_rate_is_four_items_every_forty_ticks() {
        assert_eq!(ITEMS_PER_OPERATION, 4, "upstream `getItemsPerOperation`");
        assert_eq!(OPERATION_TICK_DELAY, 40, "upstream 40 / (1 + upgrades)");
    }

    #[test]
    fn an_operation_runs_every_forty_ticks_offset_by_the_pipe_position() {
        // The pipe at (1, 100, 0) has phase 101 % 40 == 21.
        let phase = phase_of([1, 100, 0]);
        assert_eq!(phase, 21);
        assert!(is_operation_tick(19, phase));
        assert!(!is_operation_tick(20, phase));
        assert!(is_operation_tick(59, phase));
        assert!(!is_operation_tick(60, phase));
    }

    #[test]
    fn a_face_connects_to_a_pipe_or_an_inventory_only() {
        let neighbours = [
            Neighbour::Inventory,
            Neighbour::Pipe,
            Neighbour::Nothing,
            Neighbour::Inventory,
            Neighbour::Nothing,
            Neighbour::Nothing,
        ];
        let bits = connection_bits(neighbours);
        assert!(is_connected(bits, DOWN));
        assert!(is_connected(bits, UP));
        assert!(!is_connected(bits, NORTH));
        assert!(is_connected(bits, SOUTH));
        assert!(!is_connected(bits, WEST));
        assert!(!is_connected(bits, EAST));
        assert_eq!(connection_bits([Neighbour::Nothing; FACE_COUNT]), 0);
    }

    #[test]
    fn a_fresh_pipe_has_no_attachments_and_one_round_trips() {
        assert_eq!(Attachments::new().saved(), 0);
        let attached = Attachments::new().with(WEST, true);
        assert!(attached.has(WEST));
        assert!(!attached.has(EAST));
        assert_eq!(Attachments::from_saved(attached.saved()), attached);
        assert_eq!(Attachments::from_saved(-1), Attachments::new());
        assert_eq!(Attachments::new().with(WEST, false), Attachments::new());
    }

    #[test]
    fn reachable_follows_pipe_links_and_reports_their_distance() {
        let pipes = [[0, 0, 0], [1, 0, 0], [2, 0, 0], [9, 9, 9]];
        let linked = |pos: Pos, face: Face| {
            matches!(
                (pos, face),
                ([0, 0, 0], EAST) | ([1, 0, 0], WEST) | ([1, 0, 0], EAST) | ([2, 0, 0], WEST)
            )
        };
        let mut found = reachable(&pipes, [0, 0, 0], linked);
        found.sort_unstable();
        assert_eq!(found, vec![([0, 0, 0], 0), ([1, 0, 0], 1), ([2, 0, 0], 2)]);
    }

    #[test]
    fn destinations_are_nearest_first_then_face_order() {
        let reachable = vec![([0, 0, 0], 0), ([1, 0, 0], 1), ([2, 0, 0], 2)];
        let endpoints = [
            Endpoint {
                pipe: [2, 0, 0],
                face: UP,
            },
            Endpoint {
                pipe: [1, 0, 0],
                face: NORTH,
            },
            Endpoint {
                pipe: [1, 0, 0],
                face: DOWN,
            },
            Endpoint {
                pipe: [0, 0, 0],
                face: EAST,
            },
        ];
        let source = endpoints[3];
        assert_eq!(
            destination_order(&reachable, &endpoints, source),
            vec![2, 1, 0]
        );
    }

    #[test]
    fn the_extractors_own_side_is_never_a_destination() {
        let reachable = vec![([0, 0, 0], 0)];
        let endpoints = [
            Endpoint {
                pipe: [0, 0, 0],
                face: WEST,
            },
            Endpoint {
                pipe: [0, 0, 0],
                face: EAST,
            },
        ];
        assert_eq!(
            destination_order(&reachable, &endpoints, endpoints[0]),
            vec![1]
        );
        // A pipe whose network is not in `reachable` has no destination.
        assert!(destination_order(&[], &endpoints, endpoints[0]).is_empty());
    }

    #[test]
    fn the_budget_is_four_items_per_operation_and_spills_to_the_next_destination() {
        let mut calls: Vec<(usize, u32)> = Vec::new();
        let spent = spend_budget(ITEMS_PER_OPERATION, &[0, 1], |index, remaining| {
            calls.push((index, remaining));
            // The first destination takes two, the second takes one.
            remaining.min(if index == 0 { 2 } else { 1 })
        });
        assert_eq!(spent, 3);
        assert_eq!(calls, vec![(0, 4), (1, 2)]);
    }

    #[test]
    fn a_full_network_takes_nothing_and_keeps_the_budget_zero() {
        let mut calls = 0;
        let spent = spend_budget(ITEMS_PER_OPERATION, &[0], |_, _| {
            calls += 1;
            0
        });
        assert_eq!(spent, 0);
        assert_eq!(calls, 1);
        assert_eq!(
            spend_budget(ITEMS_PER_OPERATION, &[], |_, _| unreachable!()),
            0
        );
    }
}
