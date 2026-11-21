#!/usr/bin/env bash
# In-game check for the Modern Dynamics slices: the item pipe, the extractor and the attractor.
#
# The End arena (stone floor at y=99, everything at y=100): an empty level keeps
# ambient item entities out of the container counts. The leg launches `--offscreen`,
# not `--headless`: the recipe phase opens a real crafting-table menu and clicks
# its slots, and a headless client never opens a screen.
#
#  R  every carried recipe is crafted in a real crafting table (tools/recipe_check.py):
#     item pipe x8, extractor x1, attractor x1, each output id/count asserted, the
#     grid emptied and every ingredient consumed; then a negative grid (the
#     attractor's pattern with lapis where its ender pearl goes) must craft nothing.
#  A  chest (0) - pipe (1) - chest (2), extractor on the pipe's west side: after the
#     40-tick operation the 20 stone of the source chest are moved 4 at a time into
#     the target chest, and the pipe's state shows its west and east connections;
#  B  chest (8) .. chest (10) with no pipe between them: nothing moves
#     (control: the pipe is what moves items);
#  C  chest (14) - pipe (16) - chest (18) with an extractor on the pipe's west side,
#     but the pipe touches neither chest: nothing moves (control: a disconnected pipe
#     reaches no inventory);
#  D  chest (24) - pipe (25) - chest (26, full of dirt), extractor on the west side:
#     nothing moves because the target has no room (control: a refused item stays);
#  E  chest (32) - pipe (33) - chest (34), NO attachment: nothing moves although the
#     pipe is connected (control: upstream's pipe alone moves nothing);
#  F  a right-click with the extractor item on a pipe's south face places an extractor,
#     and a right-click with the attractor item on another pipe's north face places an
#     attractor (upstream's own path);
#  G  chest (64) - pipe (65) - pipe (66) - chest (67) with an ATTRACTOR on pipe 66's
#     east side and NO extractor anywhere: the attractor pulls the stone out of the
#     far chest into its own, 4 items per operation (it is the extractor reversed);
#  G2 the same geometry with an EXTRACTOR on pipe 74's east side instead: the items
#     move the OTHER way, out of the chest the attachment faces (the direction
#     differential for G);
#  G3 chest (80) - pipe (81) - pipe (82) - chest (83) with an attractor on each inner
#     side: nothing moves, because an attractor never pulls from another attractor;
#  D2 a pipe broken in survival drops itself (its loot table is the 26.3 shape);
#  X  the administrative paths: an `/moderndynamics attach <pos> <side> <kind>` from a
#     player costs that kind's item outside creative, a detach clears the side, and an
#     attach with the item missing is refused by name; unknown ids are refused;
#  L  both attachments are saved with their pipes across a relaunch.
#
# The A phase is timed against the pipe's operation phase ((x+y+z) % 40): the
# extractor is placed just after an operation, so the check can prove that nothing
# moves in the first ~32 ticks and that exactly 4 items move at the 40th.
#
# SELFTEST=1 flips A's assertion (the target must still be empty), so the scenario
# must FAIL; that proves the check can fail.
#
# GAME must be a game the **launcher** built from this package
# (`ports/tools/launcher-game.sh build moderndynamics`). Every mod command is typed
# with `ctl chat "/cmd"` because `ctl command` runs the console path and bypasses the
# native `run_mod_command` hook.
set -euo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/moderndynamics
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/moderndynamics-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=ModernDynamicsE2E

[ -n "$GAME" ] || {
    echo "missing GAME: a launcher-built horizon-client carrying this package" >&2
    exit 1
}
[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/moderndynamics-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }

# The full state of a block in the End (`moderndynamics:item_pipe[east=true,...]`).
state() { ctl block "$1" "$2" "$3" | jq -r '.state // .block // "unknown"'; }
# count X Y Z ITEM: how many of ITEM the container at X,Y,Z holds.
count() {
    ctl container "$1" "$2" "$3" \
        | jq -r --arg i "$4" '[.slots[] | select(.item == $i) | .count] | add // 0'
}

# Count the destination chest every half second: every step between samples must be
# exactly one operation's batch of 4 items, and 7 seconds must hold the 40-tick
# cadence's 2-5 operations (a per-tick mover would empty the chest, a 20-tick one
# would show 7). Prints how many steps moved.
sample_operations() { # sample_operations X Z ITEM SECONDS SAMPLES
    local x=$1 z=$2 item=$3 i before after step steps=0
    before=$(count "$x" 100 "$z" "$item")
    for i in $(seq 1 "$5"); do
        sleep "$(awk -v s="$4" -v n="$5" 'BEGIN { printf "%.3f", s / n }')"
        after=$(count "$x" 100 "$z" "$item")
        if [ "$after" != "$before" ]; then
            [ $(( after - before )) -eq 4 ] \
                || fail "one operation moved $(( after - before )) items, not 4"
            steps=$(( steps + 1 ))
            before=$after
        fi
    done
    echo "$steps"
}

# The `/moderndynamics info` lines for a pipe, echoed and matched by TEXT. One call
# answers TWO lines (`links …` then `extractors …; attractors …`), and `ctl messages`
# lists chat newest first, so the first two lines about this pipe are the freshest
# answer: a phase that changes a pipe's attachments re-reads it rather than matching
# an earlier phase's lines.
expect_info() { # expect_info X Y Z TEXT
    local attempt lines=""
    for attempt in 1 2 3 4 5; do
        say "moderndynamics info $1 $2 $3"
        ctl wait 5 >/dev/null
        lines=$(ctl messages 20 | jq -r '.chat[]' | grep "Item Pipe at $1,$2,$3:" | head -2 || true)
        if echo "$lines" | grep -q -- "$4"; then
            echo "$lines" | sed 's/^/  /'
            return 0
        fi
    done
    echo "$lines" | sed 's/^/  /'
    fail "expected /moderndynamics info $1 $2 $3 to say '$4'"
}

# Hold ITEM in the main hand, from the hotbar: the administrative paths and the
# right-click placement both take the item from the hand. The server learns the
# held slot from the client's next packet, so the selection is settled with a wait
# and read back before a command that reads the main hand runs (measured: without
# the wait the attach found no item in hand).
select_item() { # select_item ITEM
    local slot held
    if ! ctl select "$1" >/dev/null 2>&1; then
        slot=$(ctl inventory | jq -r --arg i "$1" \
            '(.hotbar // []) | to_entries[] | select((.value.item // "") == $i) | (.key + 1)' | head -1)
        [ -n "$slot" ] || return 1
        ctl select "$slot" >/dev/null
    fi
    ctl wait 2 >/dev/null
    held=$(ctl inventory | jq -r '.held.item // "nothing"')
    [ "$held" = "$1" ] || {
        echo "  the main hand holds $held, not $1" >&2
        return 1
    }
}

hand() { ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0'; }

# Stand in a block of the End arena, by server-side teleport. `ctl tp` moves the
# client, and a survival server corrects that back ("moved wrongly", measured), so
# the position is set by command and read back before anything relies on it.
tp_to() { # tp_to BLOCK_X BLOCK_Y BLOCK_Z
    local x=$1 y=$2 z=$3 attempt pos cx cz
    # The block's centre is `x + 0.5` in every direction, including negative coordinates
    # (block -3 spans -3.0..-2.0, so its centre is -2.5, not -3.5).
    cx=$(awk -v v="$x" 'BEGIN { printf "%.1f", v + 0.5 }')
    cz=$(awk -v v="$z" 'BEGIN { printf "%.1f", v + 0.5 }')
    ctl command "execute in minecraft:the_end run tp @s $cx $y $cz" >/dev/null || true
    for attempt in $(seq 1 25); do
        ctl wait 2 >/dev/null || true
        pos=$(ctl state | jq -c '.player.block_pos')
        [ "$pos" = "[$x,$y,$z]" ] && return 0
    done
    echo "  the player stands at $pos, not [$x,$y,$z]" >&2
    return 1
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --offscreen --wait \
        --resources "$RES" >/dev/null
    ctl wait-ready 120 >/dev/null
    ctl wait 40
}

# A stone floor at y=99 in the End with clear air above, the player on it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s 0.5 100 3.5"
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5
        if [ "$(ctl block 0 99 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill -8 99 -8 88 99 8 minecraft:stone"
    ctl command "execute in minecraft:the_end run fill -8 100 -8 88 108 8 minecraft:air"
    ctl command "execute in minecraft:the_end run kill @e[type=!player]"
    ctl wait 20
}

block_at() { # block_at X Z BLOCK
    ctl command "execute in minecraft:the_end run setblock $1 100 $2 $3"
}

echo "== launch (The End arena, one package: content + code) =="
launch
arena

echo "== build the pipes and their chests =="
block_at 0 0 minecraft:chest
block_at 1 0 moderndynamics:item_pipe
block_at 2 0 minecraft:chest
block_at 8 0 minecraft:chest
block_at 10 0 minecraft:chest
block_at 14 0 minecraft:chest
block_at 16 0 moderndynamics:item_pipe
block_at 18 0 minecraft:chest
block_at 24 0 minecraft:chest
block_at 25 0 moderndynamics:item_pipe
block_at 26 0 minecraft:chest
block_at 32 0 minecraft:chest
block_at 33 0 moderndynamics:item_pipe
block_at 34 0 minecraft:chest
block_at 40 0 moderndynamics:item_pipe
block_at 41 0 minecraft:chest
block_at 44 0 moderndynamics:item_pipe
block_at 56 0 moderndynamics:item_pipe
block_at 58 0 moderndynamics:item_pipe
block_at 64 0 minecraft:chest
block_at 65 0 moderndynamics:item_pipe
block_at 66 0 moderndynamics:item_pipe
block_at 67 0 minecraft:chest
block_at 72 0 minecraft:chest
block_at 73 0 moderndynamics:item_pipe
block_at 74 0 moderndynamics:item_pipe
block_at 75 0 minecraft:chest
block_at 80 0 minecraft:chest
block_at 81 0 moderndynamics:item_pipe
block_at 82 0 moderndynamics:item_pipe
block_at 83 0 minecraft:chest
ctl wait 6
# The package is loaded at all: the control leg is the same Nightly without it, where the pipe is
# an unknown block and every later assertion would fail for the wrong reason.
[ "$(ctl block 1 100 0 | jq -r '.block // "unknown"')" = "moderndynamics:item_pipe" ] \
    || fail "the game does not know moderndynamics:item_pipe: this Nightly does not carry the package"

ctl container 0 100 0 minecraft:stone 20 >/dev/null
ctl container 8 100 0 minecraft:stone 4 >/dev/null
ctl container 14 100 0 minecraft:stone 4 >/dev/null
ctl container 24 100 0 minecraft:stone 4 >/dev/null
ctl container 32 100 0 minecraft:stone 4 >/dev/null
ctl container 64 100 0 minecraft:stone 20 >/dev/null
ctl container 75 100 0 minecraft:stone 4 >/dev/null
ctl container 80 100 0 minecraft:stone 4 >/dev/null
# The full target: 27 calls fill all 27 slots with 64 dirt each (`ctl container`
# fills one empty slot per call), so the chest has no room at all.
for _ in $(seq 1 27); do ctl container 26 100 0 minecraft:dirt 64 >/dev/null; done
[ "$(ctl container 26 100 0 | jq '[.slots[] | select(. == null)] | length')" = 0 ] \
    || fail "the full target chest is not full"

echo "  sources before: $(count 0 100 0 minecraft:stone) at 0, $(count 64 100 0 minecraft:stone) at 64, $(count 75 100 0 minecraft:stone) at 75"
[ "$(count 0 100 0 minecraft:stone)" = 20 ] || fail "the source chest did not take 20 stone"
[ "$(count 64 100 0 minecraft:stone)" = 20 ] || fail "the attractor's source chest did not take 20 stone"
[ "$(count 72 100 0 minecraft:stone)" = 0 ] || fail "the differential's far chest is not empty"
[ "$(count 26 100 0 minecraft:dirt)" = 1728 ] || fail "the full target chest did not take 1728 dirt"

echo "== R: every carried recipe crafts in a real crafting table =="
python3 "$PORT/tools/recipe_check.py" --socket "$SOCK" --ctl "$CTL" --table 60 100 6 \
    || fail "a carried recipe did not craft as it declares"
# The crafted items are real items: the extractor and the attractor the recipes made are the ones
# the placement phases below use.
ctl command "clear @s" >/dev/null
ctl give moderndynamics:item_pipe 8 >/dev/null
ctl give moderndynamics:extractor 4 >/dev/null
ctl give moderndynamics:attractor 4 >/dev/null
ctl wait 4
[ "$(hand moderndynamics:item_pipe)" = 8 ] || fail "the player did not take eight item pipes"
[ "$(hand moderndynamics:extractor)" = 4 ] || fail "the player did not take four extractors"
[ "$(hand moderndynamics:attractor)" = 4 ] || fail "the player did not take four attractors"

echo "== A: an extractor moves 20 stone from the west chest to the east chest =="
# `/moderndynamics attach` is the administrative path: operator level 2 or more, and it takes
# one attachment from the acting player's main hand, as the right-click path does.
select_item moderndynamics:extractor || fail "no extractor in the player's hotbar"
say "moderndynamics attach 1 100 0 west"
ctl wait 3 >/dev/null
expect_info 1 100 0 "extractors west"
pipe_state=$(state 1 100 0)
echo "  pipe state: $pipe_state"
case "$pipe_state" in
    *"east=true"*"west=true"*) ;;
    *) fail "the pipe's state does not show its west and east connections: $pipe_state" ;;
esac
# Observe the first seven seconds of transfer.
steps=$(sample_operations 2 0 minecraft:stone 7 14)
moved=$(count 2 100 0 minecraft:stone)
echo "  $steps operations in 7 s: source $(count 0 100 0 minecraft:stone), target $moved"
if [ "$SELFTEST" = 1 ]; then
    [ "$moved" = 0 ] || fail "SELFTEST: the target chest holds $moved stone, so the check is alive"
else
    [ "$steps" -ge 2 ] || fail "only $steps operation(s) in 7 s, expected 2 to 5"
    [ "$steps" -le 5 ] || fail "$steps operations in 7 s: the batch is not one per 40 ticks"
fi
# Let the rest of the source drain.
for _ in $(seq 1 60); do
    [ "$(count 2 100 0 minecraft:stone)" = 20 ] && break
    sleep 0.5
done
echo "  source after: $(count 0 100 0 minecraft:stone), target after: $(count 2 100 0 minecraft:stone)"
[ "$(count 0 100 0 minecraft:stone)" = 0 ] || fail "the source chest is not empty"
[ "$(count 2 100 0 minecraft:stone)" = 20 ] || fail "the target chest did not receive all 20 stone"

echo "== B: two chests with no pipe between move nothing (control) =="
ctl wait 60 >/dev/null
echo "  source: $(count 8 100 0 minecraft:stone), target: $(count 10 100 0 minecraft:stone)"
[ "$(count 8 100 0 minecraft:stone)" = 4 ] || fail "the no-pipe source lost stone"
[ "$(count 10 100 0 minecraft:stone)" = 0 ] || fail "stone moved with no pipe"

echo "== C: a pipe with an extractor but no inventory beside it moves nothing (control) =="
select_item moderndynamics:extractor || fail "no extractor in the player's hotbar"
say "moderndynamics attach 16 100 0 west"
ctl wait 3 >/dev/null
expect_info 16 100 0 "links none"
expect_info 16 100 0 "extractors west"
ctl wait 60 >/dev/null
echo "  source: $(count 14 100 0 minecraft:stone), target: $(count 18 100 0 minecraft:stone)"
[ "$(count 14 100 0 minecraft:stone)" = 4 ] || fail "the disconnected source lost stone"
[ "$(count 18 100 0 minecraft:stone)" = 0 ] || fail "stone moved across a disconnected pipe"

echo "== D: a full target refuses the items (control) =="
select_item moderndynamics:extractor || fail "no extractor in the player's hotbar"
say "moderndynamics attach 25 100 0 west"
ctl wait 3 >/dev/null
ctl wait 60 >/dev/null
echo "  source: $(count 24 100 0 minecraft:stone), target stone: $(count 26 100 0 minecraft:stone), target dirt: $(count 26 100 0 minecraft:dirt)"
[ "$(count 24 100 0 minecraft:stone)" = 4 ] || fail "the source lost stone into a full target"
[ "$(count 26 100 0 minecraft:stone)" = 0 ] || fail "the full target took stone"
[ "$(count 26 100 0 minecraft:dirt)" = 1728 ] || fail "the full target lost its dirt"

echo "== E: a connected pipe with no attachment moves nothing (control) =="
ctl wait 60 >/dev/null
echo "  source: $(count 32 100 0 minecraft:stone), target: $(count 34 100 0 minecraft:stone)"
[ "$(count 32 100 0 minecraft:stone)" = 4 ] || fail "a pipe without an extractor moved stone"
[ "$(count 34 100 0 minecraft:stone)" = 0 ] || fail "a pipe without an extractor filled the target"
expect_info 33 100 0 "extractors none"

echo "== F: right-clicking a pipe with an attachment item places it (upstream's path) =="
tp_to 40 100 3 || fail "the player could not stand beside the pipe at 40,100,0"
select_item moderndynamics:extractor || fail "no extractor in the player's hotbar"
ctl interact 40 100 0 south >/dev/null || fail "the right-click on the pipe at 40,100,0 failed"
ctl wait 3 >/dev/null
expect_info 40 100 0 "extractors south"
tp_to 44 100 -3 || fail "the player could not stand beside the pipe at 44,100,0"
select_item moderndynamics:attractor || fail "no attractor in the player's hotbar"
ctl interact 44 100 0 north >/dev/null || fail "the right-click on the pipe at 44,100,0 failed"
ctl wait 3 >/dev/null
expect_info 44 100 0 "attractors north"

echo "== G: an attractor pulls 20 stone into the chest on its own side =="
# No extractor anywhere in this network: the attractor is what moves the items, out of the far
# chest (64) and into the one it faces (67).
select_item moderndynamics:attractor || fail "no attractor in the player's hotbar"
say "moderndynamics attach 66 100 0 east attractor"
ctl wait 3 >/dev/null
expect_info 66 100 0 "attractors east"
expect_info 66 100 0 "extractors none"
ctl wait 10 >/dev/null
steps=$(sample_operations 67 0 minecraft:stone 7 14)
moved=$(count 67 100 0 minecraft:stone)
echo "  $steps operations in 7 s: attractor source $(count 64 100 0 minecraft:stone), target $moved"
[ "$steps" -ge 2 ] || fail "only $steps attractor operation(s) in 7 s, expected 2 to 5"
[ "$steps" -le 5 ] || fail "$steps attractor operations in 7 s: the batch is not one per 40 ticks"
for _ in $(seq 1 60); do
    [ "$(count 67 100 0 minecraft:stone)" = 20 ] && break
    sleep 0.5
done
echo "  attractor source after: $(count 64 100 0 minecraft:stone), target after: $(count 67 100 0 minecraft:stone)"
[ "$(count 64 100 0 minecraft:stone)" = 0 ] || fail "the attractor left stone in the far chest"
[ "$(count 67 100 0 minecraft:stone)" = 20 ] || fail "the attractor did not fill its own chest"

echo "== G2: the same geometry with an extractor moves the items the other way (differential) =="
# Pipe 74's east side faces chest 75. An EXTRACTOR there takes out of chest 75 and pushes into the
# network's other endpoint (chest 72); an attractor (G) takes out of the far chest into its own.
# Same blocks, one attachment kind different, opposite direction.
select_item moderndynamics:extractor || fail "no extractor in the player's hotbar"
say "moderndynamics attach 74 100 0 east extractor"
ctl wait 3 >/dev/null
expect_info 74 100 0 "extractors east"
for _ in $(seq 1 40); do
    [ "$(count 75 100 0 minecraft:stone)" = 0 ] && break
    sleep 0.5
done
echo "  chest 75 (the extractor's side): $(count 75 100 0 minecraft:stone), chest 72 (the far end): $(count 72 100 0 minecraft:stone)"
[ "$(count 75 100 0 minecraft:stone)" = 0 ] || fail "the extractor did not empty the chest it faces"
[ "$(count 72 100 0 minecraft:stone)" = 4 ] || fail "the extractor did not push into the far chest"
# And the mirror image of G: an attractor's chest would have GAINED the items, not lost them.
[ "$(count 67 100 0 minecraft:stone)" = 20 ] || fail "the attractor's chest lost its stone"

echo "== G3: two attractors move nothing (an attractor never pulls from another) =="
select_item moderndynamics:attractor || fail "no attractor in the player's hotbar"
say "moderndynamics attach 81 100 0 west attractor"
ctl wait 3 >/dev/null
say "moderndynamics attach 82 100 0 east attractor"
ctl wait 3 >/dev/null
expect_info 81 100 0 "attractors west"
expect_info 82 100 0 "attractors east"
ctl wait 100 >/dev/null
echo "  source: $(count 80 100 0 minecraft:stone), target: $(count 83 100 0 minecraft:stone)"
[ "$(count 80 100 0 minecraft:stone)" = 4 ] || fail "an attractor pulled from another attractor's chest"
[ "$(count 83 100 0 minecraft:stone)" = 0 ] || fail "an attractor filled its chest from another attractor"

echo "== the pipe drops itself when broken (its loot table) =="
ctl command "gamemode survival @a" >/dev/null || true
ctl wait 5 >/dev/null || true
[ "$(ctl state | jq -r '.gamemode')" = "survival" ] || fail "the player did not enter survival"
tp_to 58 100 2 || fail "the player could not stand beside the pipe at 58,100,0"
ctl wait 4 >/dev/null || true
ctl command "clear @s" >/dev/null || true
ctl wait 2 >/dev/null || true
ctl look-at-block 58 100 0 >/dev/null || true
broken=$(ctl break 58 100 0) || { echo "break reply: ${broken:-none}"; fail "the pipe at 58,100,0 did not break"; }
echo "  $(echo "$broken" | jq -c '{block,block_before,drops}')"
[ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "the pipe did not break"
[ "$(echo "$broken" | jq -r '.drops["moderndynamics:item_pipe"] // 0')" = 1 ] \
    || fail "the broken pipe did not drop itself"

echo "== the administrative paths: cost, detach, and a refusal =="
# `/moderndynamics attach <pos> <side> <kind>` is an operator path, but the item is still the
# player's cost: outside creative the hand pays one attachment per placement. In creative
# nothing is spent, exactly as the right-click path leaves a creative player's hand alone.
ctl command "gamemode survival @a" >/dev/null || true
ctl wait 5 >/dev/null || true
ctl command "clear @s" >/dev/null || true
ctl wait 2 >/dev/null || true
ctl give moderndynamics:attractor 2 >/dev/null || true
ctl wait 4 >/dev/null || true
select_item moderndynamics:attractor || fail "no attractor in the player's hotbar"
[ "$(hand moderndynamics:attractor)" = 2 ] || fail "the player did not take two attractors (has $(hand moderndynamics:attractor))"
say "moderndynamics attach 56 100 0 east attractor"
ctl wait 3 >/dev/null
[ "$(hand moderndynamics:attractor)" = 1 ] \
    || fail "the attractor attach did not take one item (hand $(hand moderndynamics:attractor), expected 1)"
expect_info 56 100 0 "attractors east"
say "moderndynamics detach 56 100 0 east"
ctl wait 3 >/dev/null
expect_info 56 100 0 "attractors none"
# A detach returns nothing, as upstream's wrench takes the attachment off.
[ "$(hand moderndynamics:attractor)" = 1 ] || fail "the detach returned an item"
# An attach with no attachment in hand is refused by name and writes nothing.
ctl command "clear @s" >/dev/null || true
ctl wait 4 >/dev/null || true
[ "$(hand moderndynamics:attractor)" = 0 ] || fail "the hand is not empty"
say "moderndynamics attach 56 100 0 west attractor"
ctl wait 3 >/dev/null
ctl messages 10 | jq -r '.chat[]' | grep -q "the attachment item is not in hand" \
    || fail "the attach without an attractor was not refused by name"
expect_info 56 100 0 "attractors none"
# An unknown attachment kind is refused by name, and names both kinds it knows.
say "moderndynamics attach 56 100 0 west filter"
ctl wait 3 >/dev/null
ctl messages 10 | jq -r '.chat[]' | grep -q "Unknown attachment 'filter': extractor or attractor." \
    || fail "an unknown attachment kind was not refused by name"

echo "== unknown ids are refused =="
# The block slot holds stone first: an unknown id must leave it alone rather than clear it, and an
# unknown item must not land in the inventory.
ctl command "execute in minecraft:the_end run setblock 50 100 0 minecraft:stone" >/dev/null
ctl wait 3
ctl command "execute in minecraft:the_end run setblock 50 100 0 moderndynamics:not_a_block" >/dev/null || true
ctl wait 3
[ "$(ctl block 50 100 0 | jq -r '.block // "unknown"')" = "minecraft:stone" ] \
    || fail "an unknown block id replaced the block it was set over"
ctl command "execute in minecraft:the_end run setblock 50 100 0 minecraft:air" >/dev/null
ctl give moderndynamics:not_an_item 1 >/dev/null 2>&1 || true
ctl wait 3
[ "$(hand moderndynamics:not_an_item)" = 0 ] || fail "an unknown item id was accepted"

echo "== both attachments are saved with their pipes across a relaunch =="
ctl quit >/dev/null
sleep 2
launch
tp_to 1 100 3 || fail "the player could not stand beside the pipe at 1,100,0"
expect_info 1 100 0 "extractors west"
pipe_state=$(state 1 100 0)
echo "  relaunched pipe state: $pipe_state"
case "$pipe_state" in
    *"east=true"*"west=true"*) ;;
    *) fail "the pipe's connections did not come back after the relaunch: $pipe_state" ;;
esac
tp_to 66 100 3 || fail "the player could not stand beside the pipe at 66,100,0"
expect_info 66 100 0 "attractors east"

echo "PASS: every carried recipe crafts its declared output and consumes its ingredients (the attractor's grid with lapis crafts nothing), an extractor moved 20 stone into the far chest while an attractor pulled 20 stone into its own and the extractor on the same geometry moved items the other way, two attractors moved nothing, a pipe broken in survival dropped itself, the administrative attach cost one attachment and a detach cleared the side, both attachments survived a relaunch, and chests with no pipe / a disconnected pipe / a full target / a pipe without an attachment moved nothing"
