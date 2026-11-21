// Pure item-pipe logic for Modern Dynamics (standalone crate lib, injected as
// `crate::game::moderndynamics`). No game access lives here.
//
// Upstream's item pipe moves nothing by itself: an attachment on one of its six
// sides moves items. An `extractor` pulls up to `ITEMS_PER_OPERATION` items from
// the inventory on its side, every `OPERATION_TICK_DELAY` ticks, and inserts
// them into the inventories at the network's other endpoints, nearest first
// (`RoutingMode.CLOSEST`, the default); an `attractor` is the reverse — it pulls
// from the network's other inventories into the inventory on its side.
// Connections are per side: another pipe continues the network, an inventory is
// an endpoint, anything else is not connected.

use std::collections::BTreeSet;

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

/// The most pipes one network walk visits before it stops and says it is
/// incomplete. Upstream has no cap: a pipe line long enough to be a server-thread
/// stall is a griefed or machine-built network, not a design, so one operation
/// walks at most this many pipes and moves items to the endpoints it found. The
/// number is the campaign's budget for one synchronous graph walk (review
/// 1202-moderndynamics: "budget graph traversal").
pub const MAX_NETWORK_PIPES: usize = 256;

/// The operator level `/moderndynamics attach|detach` requires: level 2, "game
/// masters", the level `/setblock` and `/fill` need. The two sides edit another
/// player's pipe from any distance, so they are administrative (review
/// 1202-moderndynamics: "restrict the administrative paths to operators").
pub const ADMIN_LEVEL: u8 = 2;

/// Whether a command source's permission level may run the administrative sides.
pub const fn may_administer(level: u8) -> bool {
    level >= ADMIN_LEVEL
}

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

/// The attachments a pipe's side can carry: upstream's `IoAttachmentType`, of
/// which this slice places the two that move items (`ATTRACTOR` and
/// `EXTRACTOR`). The filter and the inhibitor are gaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    /// Pulls items out of the inventory on its side into the network.
    Extractor,
    /// Pulls items out of the network into the inventory on its side.
    Attractor,
}

impl AttachmentKind {
    /// The word a command or a state readout names this kind by.
    pub const fn name(self) -> &'static str {
        match self {
            AttachmentKind::Extractor => "extractor",
            AttachmentKind::Attractor => "attractor",
        }
    }

    /// The kind a command names, if it names one.
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "extractor" => Some(AttachmentKind::Extractor),
            "attractor" => Some(AttachmentKind::Attractor),
            _ => None,
        }
    }
}

/// The attachments a pipe carries, one per side at most (upstream's node host
/// keeps one attachment per side), as one bit mask per kind: the saved form is
/// two ints, so a pipe from the extractor-only slice keeps its extractors.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attachments {
    extractors: u8,
    attractors: u8,
}

impl Attachments {
    /// No attachment on any side, as a freshly placed pipe.
    pub const fn new() -> Self {
        Attachments {
            extractors: 0,
            attractors: 0,
        }
    }

    /// Whether the pipe carries no attachment at all.
    pub const fn is_empty(&self) -> bool {
        self.extractors == 0 && self.attractors == 0
    }

    /// Whether `face` carries an attachment of either kind.
    pub const fn has(&self, face: Face) -> bool {
        self.kind(face).is_some()
    }

    /// The attachment on `face`, if any.
    pub const fn kind(&self, face: Face) -> Option<AttachmentKind> {
        if face >= FACE_COUNT {
            return None;
        }
        let bit = 1 << face;
        if self.extractors & bit != 0 {
            Some(AttachmentKind::Extractor)
        } else if self.attractors & bit != 0 {
            Some(AttachmentKind::Attractor)
        } else {
            None
        }
    }

    /// The attachments with `face` carrying `kind`, replacing whatever was
    /// there: one attachment per side.
    pub const fn with(self, face: Face, kind: Option<AttachmentKind>) -> Self {
        if face >= FACE_COUNT {
            return self;
        }
        let bit = 1 << face;
        let extractors = self.extractors & !bit;
        let attractors = self.attractors & !bit;
        match kind {
            Some(AttachmentKind::Extractor) => Attachments {
                extractors: extractors | bit,
                attractors,
            },
            Some(AttachmentKind::Attractor) => Attachments {
                extractors,
                attractors: attractors | bit,
            },
            None => Attachments {
                extractors,
                attractors,
            },
        }
    }

    /// Whether an inventory beside this side can be a path target: upstream's
    /// `allowsItemConnection`, which is false for a servo only. An extractor
    /// pulls out of its side, so that side is never also a destination; an
    /// attractor's side is where the network's items are pushed.
    pub const fn accepts_endpoints(&self, face: Face) -> bool {
        !matches!(self.kind(face), Some(AttachmentKind::Extractor))
    }

    /// The extractor mask the block entity saves.
    pub const fn extractor_mask(&self) -> i32 {
        self.extractors as i32
    }

    /// The attractor mask the block entity saves.
    pub const fn attractor_mask(&self) -> i32 {
        self.attractors as i32
    }

    /// Restore the masks [`Attachments::extractor_mask`] and
    /// [`Attachments::attractor_mask`] wrote. Bits past the six sides are
    /// dropped, a value that is not a mask at all falls back to none, and a side
    /// that a damaged save puts in both masks keeps the extractor — the slice
    /// that wrote only that mask — so a side never carries two kinds.
    pub fn from_saved(extractors: i32, attractors: i32) -> Self {
        let extractors = u8::try_from(extractors).map_or(0, |mask| mask & ALL_FACES);
        let attractors = u8::try_from(attractors).map_or(0, |mask| mask & ALL_FACES);
        Attachments {
            extractors,
            attractors: attractors & !extractors,
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

/// Every pipe reachable from `start` through pipe-to-pipe links, with its graph
/// distance, as one bounded breadth-first walk. The walk keeps the nearest pipes
/// ([`MAX_NETWORK_PIPES`] of them) and reports whether it finished, so one
/// operation can never walk an unbounded network on the server thread. `linked(pos,
/// face)` answers whether the pipe at `pos` continues into the pipe beside that face.
pub struct Reach {
    found: Vec<(Pos, u32)>,
    complete: bool,
}

impl Reach {
    /// A walk's result from its parts (the tests build one directly).
    pub fn from_parts(found: Vec<(Pos, u32)>) -> Self {
        Self {
            found,
            complete: true,
        }
    }

    /// The pipes the walk visited, with their distance from the start.
    pub fn found(&self) -> &[(Pos, u32)] {
        &self.found
    }

    /// Whether the walk visited the whole network (false when it hit the budget).
    pub fn complete(&self) -> bool {
        self.complete
    }

    /// The distance from the start to `pos`, or `None` for a pipe this walk did not reach.
    pub fn distance(&self, pos: Pos) -> Option<u32> {
        self.found
            .iter()
            .find(|(seen, _)| *seen == pos)
            .map(|(_, distance)| *distance)
    }
}

/// Walk the network from `start`: breadth first over pipe-to-pipe links, at most
/// [`MAX_NETWORK_PIPES`] pipes, with a set for membership (the old walk asked a
/// `Vec` twice per link, which made it quadratic). `start` is always in the
/// result, as before.
pub fn reachable(pipes: &[Pos], start: Pos, linked: impl Fn(Pos, Face) -> bool) -> Reach {
    let known: BTreeSet<Pos> = pipes.iter().copied().collect();
    let mut found: Vec<(Pos, u32)> = vec![(start, 0)];
    let mut seen: BTreeSet<Pos> = BTreeSet::new();
    seen.insert(start);
    let mut next = 0;
    let mut complete = true;
    while next < found.len() {
        let (pos, distance) = found[next];
        next += 1;
        for face in 0..FACE_COUNT {
            let beside = step(pos, face);
            if !known.contains(&beside) || !linked(pos, face) || seen.contains(&beside) {
                continue;
            }
            if found.len() >= MAX_NETWORK_PIPES {
                complete = false;
                break;
            }
            seen.insert(beside);
            found.push((beside, distance + 1));
        }
        if !complete {
            break;
        }
    }
    Reach { found, complete }
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
    reachable: &Reach,
    endpoints: &[Endpoint],
    source: Endpoint,
) -> Vec<usize> {
    let mut order: Vec<(u32, Face, usize)> = endpoints
        .iter()
        .enumerate()
        .filter(|(_, endpoint)| **endpoint != source)
        .filter_map(|(index, endpoint)| {
            let distance = reachable.distance(endpoint.pipe)?;
            Some((distance, endpoint.face, index))
        })
        .collect();
    order.sort_unstable();
    order.into_iter().map(|(_, _, index)| index).collect()
}

/// The order an attractor's operation tries the network's sources in: the same
/// nearest-first walk an extractor uses, minus every endpoint that carries an
/// attractor of its own (upstream's `tickAttractor`: "don't allow attractors to
/// pull from other attractors"), so two attractors never pump the same chest in
/// opposite directions.
pub fn attractor_sources(
    reachable: &Reach,
    endpoints: &[Endpoint],
    source: Endpoint,
    is_attractor_site: impl Fn(Pos, Face) -> bool,
) -> Vec<usize> {
    destination_order(reachable, endpoints, source)
        .into_iter()
        .filter(|&index| {
            let endpoint = endpoints[index];
            !is_attractor_site(endpoint.pipe, endpoint.face)
        })
        .collect()
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
    fn a_side_carries_at_most_one_attachment_and_names_its_kind() {
        let none = Attachments::new();
        assert!(none.is_empty());
        assert_eq!(none.kind(EAST), None);
        assert_eq!(none.extractor_mask(), 0);
        assert_eq!(none.attractor_mask(), 0);
        // Placing a kind names it; placing the other kind on the same side replaces it,
        // because upstream's node host keeps one attachment per side.
        let extractor = none.with(EAST, Some(AttachmentKind::Extractor));
        assert_eq!(extractor.kind(EAST), Some(AttachmentKind::Extractor));
        assert!(extractor.has(EAST));
        let attractor = extractor.with(EAST, Some(AttachmentKind::Attractor));
        assert_eq!(attractor.kind(EAST), Some(AttachmentKind::Attractor));
        assert_eq!(attractor.extractor_mask(), 0);
        assert_eq!(attractor.attractor_mask(), 1 << EAST);
        // A side is cleared on its own; the other sides keep their kind.
        let both = attractor.with(WEST, Some(AttachmentKind::Extractor));
        let cleared = both.with(EAST, None);
        assert_eq!(cleared.kind(EAST), None);
        assert_eq!(cleared.kind(WEST), Some(AttachmentKind::Extractor));
        assert!(!cleared.is_empty());
        // Both masks survive the save/restore round trip, and a damaged save
        // behaves as a fresh pipe: one kind per side, six sides.
        assert_eq!(
            Attachments::from_saved(cleared.extractor_mask(), cleared.attractor_mask()),
            cleared
        );
        assert_eq!(Attachments::from_saved(-1, -1), Attachments::new());
        let contested = Attachments::from_saved(1 << EAST, 1 << EAST);
        assert_eq!(contested.kind(EAST), Some(AttachmentKind::Extractor));
        assert_eq!(contested.attractor_mask(), 0, "one kind per side");
        // The command and the placement read kinds by name.
        assert_eq!(
            AttachmentKind::named("attractor"),
            Some(AttachmentKind::Attractor)
        );
        assert_eq!(
            AttachmentKind::named("extractor"),
            Some(AttachmentKind::Extractor)
        );
        assert_eq!(AttachmentKind::named("filter"), None);
        assert_eq!(AttachmentKind::Attractor.name(), "attractor");
        assert_eq!(AttachmentKind::Extractor.name(), "extractor");
    }

    #[test]
    fn an_extractor_side_is_never_a_network_endpoint() {
        // Upstream's `allowsItemConnection` is false for one attachment only: a servo,
        // whose source inventory must not also be a destination. An attractor's side,
        // by contrast, is where other attachments push their items.
        let bare = Attachments::new();
        assert!(bare.accepts_endpoints(NORTH));
        let attractor = bare.with(NORTH, Some(AttachmentKind::Attractor));
        assert!(
            attractor.accepts_endpoints(NORTH),
            "an attractor side is a valid destination"
        );
        let extractor = bare.with(NORTH, Some(AttachmentKind::Extractor));
        assert!(!extractor.accepts_endpoints(NORTH));
    }

    #[test]
    fn an_attractor_pulls_nearest_first_and_never_from_another_attractor() {
        let reachable = Reach::from_parts(vec![([0, 0, 0], 0), ([1, 0, 0], 1), ([2, 0, 0], 2)]);
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
                pipe: [0, 0, 0],
                face: WEST,
            },
            // The attractor's own side, the inventory it fills.
            Endpoint {
                pipe: [0, 0, 0],
                face: EAST,
            },
        ];
        let source = endpoints[3];
        // Nothing else carries an attractor: nearest first, its own side left out.
        assert_eq!(
            attractor_sources(&reachable, &endpoints, source, |_, _| false),
            vec![2, 1, 0]
        );
        // The nearest endpoint carries an attractor too: it is not a source, the
        // next-nearest is (upstream's "don't allow attractors to pull from other attractors").
        let is_attractor = |pipe: Pos, face: Face| pipe == [1, 0, 0] && face == NORTH;
        assert_eq!(
            attractor_sources(&reachable, &endpoints, source, is_attractor),
            vec![2, 0]
        );
        // A network of attractors only has no source at all.
        assert!(
            attractor_sources(&reachable, &endpoints, source, |_, _| true).is_empty(),
            "an attractor never pulls from another attractor"
        );
    }

    #[test]
    fn a_walk_stops_at_the_network_budget() {
        // A line of 5,000 pipes is far past the cap. The walk visits at most
        // MAX_NETWORK_PIPES of them, says so, and never looks at more than six
        // links per pipe it visited (the old walk was quadratic on top of that).
        let pipes: Vec<Pos> = (0..5_000).map(|x| [x, 0, 0]).collect();
        let looked = std::cell::Cell::new(0usize);
        let linked = |pos: Pos, face: Face| {
            looked.set(looked.get() + 1);
            face == EAST && pos[0] < 4_999
        };
        let reach = reachable(&pipes, [0, 0, 0], linked);
        assert_eq!(MAX_NETWORK_PIPES, 256, "one walk's pipe budget");
        assert_eq!(reach.found().len(), MAX_NETWORK_PIPES);
        assert!(
            !reach.complete(),
            "a walk that hit the cap says it did not finish"
        );
        assert!(
            looked.get() <= MAX_NETWORK_PIPES * FACE_COUNT,
            "the walk looked at {} links for {} pipes",
            looked.get(),
            MAX_NETWORK_PIPES
        );
        assert_eq!(
            reach.distance([255, 0, 0]),
            Some(255),
            "the nearest pipes are kept"
        );
        assert_eq!(
            reach.distance([4_999, 0, 0]),
            None,
            "the far end is past the budget"
        );
    }

    #[test]
    fn a_walk_that_fits_is_complete_and_finds_every_pipe() {
        let pipes = [[0, 0, 0], [1, 0, 0], [2, 0, 0]];
        let reach = reachable(&pipes, [0, 0, 0], |pos, face| {
            matches!(
                (pos, face),
                ([0, 0, 0], EAST) | ([1, 0, 0], WEST) | ([1, 0, 0], EAST) | ([2, 0, 0], WEST)
            )
        });
        assert!(reach.complete());
        assert_eq!(reach.found().len(), 3);
        assert_eq!(reach.distance([2, 0, 0]), Some(2));
    }

    #[test]
    fn the_administrative_sides_need_an_operator() {
        assert_eq!(ADMIN_LEVEL, 2, "the level /setblock needs");
        assert!(!may_administer(0));
        assert!(!may_administer(1));
        assert!(may_administer(2));
        assert!(may_administer(3));
        assert!(may_administer(4));
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
        let reach = reachable(&pipes, [0, 0, 0], linked);
        let mut found = reach.found().to_vec();
        found.sort_unstable();
        assert_eq!(found, vec![([0, 0, 0], 0), ([1, 0, 0], 1), ([2, 0, 0], 2)]);
        assert!(reach.complete());
    }

    #[test]
    fn destinations_are_nearest_first_then_face_order() {
        let reachable = Reach::from_parts(vec![([0, 0, 0], 0), ([1, 0, 0], 1), ([2, 0, 0], 2)]);
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
        let reachable = Reach::from_parts(vec![([0, 0, 0], 0)]);
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
        assert!(
            destination_order(&Reach::from_parts(Vec::new()), &endpoints, endpoints[0]).is_empty()
        );
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
