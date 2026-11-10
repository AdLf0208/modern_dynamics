#!/usr/bin/env bash
# In-game check for Modern Dynamics slice 1: the item pipe with an extractor.
#
# The End arena (stone floor at y=99, everything at y=100): an empty level keeps
# ambient item entities out of the container counts.
#
#  A  chest (0) - pipe (1) - chest (2), an extractor placed on the pipe's west
#     side: after the 40-tick operation the 8 stone of the source chest are
#     moved 4 at a time into the target chest, and the pipe's state shows its
#     west and east connections;
#  B  chest (8) .. chest (10) with no pipe between them: nothing moves
#     (control: the pipe is what moves items);
#  C  chest (14) - pipe (16) - chest (18) with an extractor on the pipe's west
#     side, but the pipe touches neither chest: nothing moves (control: a
#     disconnected pipe reaches no inventory);
#  D  chest (24) - pipe (25) - chest (26, full of dirt), extractor on the west
#     side: nothing moves because the target has no room (control: a refused
#     item stays in its slot);
#  E  chest (32) - pipe (33) - chest (34), NO extractor: nothing moves although
#     the pipe is connected (control: upstream's pipe alone moves nothing).
#
# The A phase is timed against the pipe's operation phase ((x+y+z) % 40): the
# extractor is placed just after an operation, so the check can prove that
# nothing moves in the first ~32 ticks and that exactly 4 items move at the 40th.
#
# SELFTEST=1 flips A's assertion (the target must still be empty), so the
# scenario must FAIL; that proves the check can fail.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
set -euo pipefail
export LC_ALL=C

CLONE=${CAMPAIGN_ROOT:?set CAMPAIGN_ROOT to a checkout of the ATM10-Horizon campaign tree}
PORT=$CLONE/ports/moderndynamics
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/moderndynamics-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=ModernDynamicsE2E

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/moderndynamics-home.XXXXXX)
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
ticks() { ctl state | jq -r '.ticks'; }

# Count the target chest's stone every half second: every step between samples
# must be exactly one operation's batch of 4 items, and 7 seconds must hold the
# 40-tick cadence's 2-5 operations (a per-tick mover would empty the chest, a
# 20-tick one would show 7).
sample_operations() { # sample_operations SECONDS SAMPLES
    local i before after step
    before=$(count 2 100 0 minecraft:stone)
    step=$(( $2 ))
    local steps=0
    for i in $(seq 1 "$step"); do
        sleep "$(awk -v s="$1" -v n="$2" 'BEGIN { printf "%.3f", s / n }')"
        after=$(count 2 100 0 minecraft:stone)
        if [ "$after" != "$before" ]; then
            [ $(( after - before )) -eq 4 ] \
                || fail "one operation moved $(( after - before )) items, not 4"
            steps=$(( steps + 1 ))
            before=$after
        fi
    done
    echo "$steps"
}

# The `/moderndynamics info` lines for a pipe, echoed and matched by TEXT.
expect_info() { # expect_info X Y Z TEXT
    say "moderndynamics info $1 $2 $3"
    ctl wait 3 >/dev/null
    lines=$(ctl messages 10 | jq -r '.chat[]' | grep "Item Pipe at $1,$2,$3:" | tail -2)
    echo "$lines" | sed 's/^/  /'
    echo "$lines" | grep -q -- "$4" || fail "expected /moderndynamics info to say '$4'"
}

cleanup() {
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
}
trap cleanup EXIT

launch() {
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$WORLD" --seed 20261105 --creative --headless --wait \
        --resources "$RES" --dev-mod "$PORT/content" >/dev/null
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
    ctl command "execute in minecraft:the_end run fill -8 99 -8 44 99 8 minecraft:stone"
    ctl command "execute in minecraft:the_end run fill -8 100 -8 44 108 8 minecraft:air"
    ctl command "execute in minecraft:the_end run kill @e[type=!player]"
    ctl wait 20
}

block_at() { # block_at X Z BLOCK
    ctl command "execute in minecraft:the_end run setblock $1 100 $2 $3"
}

# Put an extractor on the pipe's west side, standing near it.
select_extractor() {
    if ctl select moderndynamics:extractor >/dev/null 2>&1; then
        return 0
    fi
    local slot
    slot=$(ctl inventory | jq -r '(.hotbar // []) | to_entries[] | select((.value.item // "") == "moderndynamics:extractor") | (.key + 1)' | head -1)
    [ -n "$slot" ] || return 1
    ctl select "$slot" >/dev/null
}

echo "== launch (The End arena, content + native bundle) =="
launch
arena

echo "== build the four pipes and their chests =="
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
ctl wait 6

ctl container 0 100 0 minecraft:stone 20 >/dev/null
ctl container 8 100 0 minecraft:stone 4 >/dev/null
ctl container 14 100 0 minecraft:stone 4 >/dev/null
ctl container 24 100 0 minecraft:stone 4 >/dev/null
ctl container 32 100 0 minecraft:stone 4 >/dev/null
# The full target: 27 calls fill all 27 slots with 64 dirt each (`ctl container`
# fills one empty slot per call), so the chest has no room at all.
for _ in $(seq 1 27); do ctl container 26 100 0 minecraft:dirt 64 >/dev/null; done
[ "$(ctl container 26 100 0 | jq '[.slots[] | select(. == null)] | length')" = 0 ] \
    || fail "the full target chest is not full"
ctl give moderndynamics:extractor 1 >/dev/null
ctl wait 4

echo "  A source before: $(count 0 100 0 minecraft:stone), target before: $(count 2 100 0 minecraft:stone)"
[ "$(count 0 100 0 minecraft:stone)" = 20 ] || fail "the source chest did not take 20 stone"
[ "$(count 26 100 0 minecraft:dirt)" = 1728 ] || fail "the full target chest did not take 1728 dirt"

echo "== A: an extractor moves 20 stone from the west chest to the east chest =="
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
steps=$(sample_operations 7 14)
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
say "moderndynamics attach 16 100 0 west"
ctl wait 3 >/dev/null
expect_info 16 100 0 "links none"
expect_info 16 100 0 "extractors west"
ctl wait 60 >/dev/null
echo "  source: $(count 14 100 0 minecraft:stone), target: $(count 18 100 0 minecraft:stone)"
[ "$(count 14 100 0 minecraft:stone)" = 4 ] || fail "the disconnected source lost stone"
[ "$(count 18 100 0 minecraft:stone)" = 0 ] || fail "stone moved across a disconnected pipe"

echo "== D: a full target refuses the items (control) =="
say "moderndynamics attach 25 100 0 west"
ctl wait 3 >/dev/null
ctl wait 60 >/dev/null
echo "  source: $(count 24 100 0 minecraft:stone), target stone: $(count 26 100 0 minecraft:stone), target dirt: $(count 26 100 0 minecraft:dirt)"
[ "$(count 24 100 0 minecraft:stone)" = 4 ] || fail "the source lost stone into a full target"
[ "$(count 26 100 0 minecraft:stone)" = 0 ] || fail "the full target took stone"
[ "$(count 26 100 0 minecraft:dirt)" = 1728 ] || fail "the full target lost its dirt"

echo "== E: a connected pipe with no extractor moves nothing (control) =="
ctl wait 60 >/dev/null
echo "  source: $(count 32 100 0 minecraft:stone), target: $(count 34 100 0 minecraft:stone)"
[ "$(count 32 100 0 minecraft:stone)" = 4 ] || fail "a pipe without an extractor moved stone"
[ "$(count 34 100 0 minecraft:stone)" = 0 ] || fail "a pipe without an extractor filled the target"
expect_info 33 100 0 "extractors none"

echo "== F: right-clicking a pipe with the extractor item places it (upstream's path) =="
ctl tp 40.5 100 3.5 >/dev/null
select_extractor || fail "no extractor in the player's hotbar"
ctl interact 40 100 0 south >/dev/null || fail "the right-click on the pipe at 40,100,0 failed"
ctl wait 3 >/dev/null
expect_info 40 100 0 "extractors south"

echo "== the extractor is saved with the pipe across a relaunch =="
ctl quit >/dev/null
sleep 2
launch
expect_info 1 100 0 "extractors west"
pipe_state=$(state 1 100 0)
echo "  relaunched pipe state: $pipe_state"
case "$pipe_state" in
    *"east=true"*"west=true"*) ;;
    *) fail "the pipe's connections did not come back after the relaunch: $pipe_state" ;;
esac

echo "PASS: an extractor on a pipe moved 20 stone from one chest to another, exactly 4 items at each 40-tick operation, the pipe's state showed both connections, a right-click placed an extractor, and chests with no pipe / a disconnected pipe / a full target / a pipe without an extractor moved nothing"
echo "EXIT:0"
