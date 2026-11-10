#!/usr/bin/env bash
# Proves the built content package (content/moderndynamics.hmod) loads on a real
# game: the shared, unmodded 1d63e0a game with `--dev-mod <the .hmod>` must
# register the item pipe block and the extractor item. A second launch WITHOUT
# `--dev-mod` is the negative control: the same ids must be unknown there, so
# the check cannot pass vacuously.
set -euo pipefail
export LC_ALL=C

CLONE=${CAMPAIGN_ROOT:?set CAMPAIGN_ROOT to a checkout of the ATM10-Horizon campaign tree}
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-$PIN_BIN/horizon-client}
HMOD=${HMOD:-$CLONE/ports/moderndynamics/content/moderndynamics.hmod}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/moderndynamics-hmod.sock}

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -f "$HMOD" ] || { echo "missing package: $HMOD" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/moderndynamics-hmod-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

cleanup() {
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "${KEEP:-0}" = 1 ]; then echo "kept scratch: $SCRATCH" >&2; else rm -rf "$SCRATCH" 2>/dev/null || true; fi
}
trap cleanup EXIT

launch() { # launch WORLD [--dev-mod PATH]
    local world=$1; shift
    rm -f "$SOCK"
    cd "$SCRATCH"
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261105 --creative --headless --wait \
        --resources "$RES" "$@" >/dev/null
    ctl wait-ready 120 >/dev/null
    ctl wait 20
}

echo "== launch WITH the package (--dev-mod $HMOD) =="
launch ModernDynamicsHmod --dev-mod "$HMOD"
ctl command "setblock 0 100 0 moderndynamics:item_pipe" >/dev/null
ctl wait 4
block=$(ctl block 0 100 0 | jq -r '.state // .name // empty')
echo "block 0,100,0 = $block"
case "$block" in
    moderndynamics:item_pipe*) ;;
    *) fail "the package did not register the item pipe block" ;;
esac
ctl give moderndynamics:extractor 1 >/dev/null
ctl wait 4
held=$(ctl inventory | jq -r '.counts["moderndynamics:extractor"] // 0')
if [ "$held" = 0 ]; then
    held=$(ctl inventory | jq -r '[.hotbar[]? | select(type == "object") | select(.item == "moderndynamics:extractor") | .count] | add // 0')
fi
echo "extractor in the inventory: ${held:-0}"
[ "${held:-0}" != 0 ] || fail "the package did not register the extractor item"
echo "PASS: the packaged .hmod loaded (block moderndynamics:item_pipe, item moderndynamics:extractor)"

echo "== launch WITHOUT the package (negative control) =="
ctl quit >/dev/null
sleep 2
launch ModernDynamicsHmodNegative
setblock=$(ctl command "setblock 0 100 0 moderndynamics:item_pipe")
echo "setblock reply: $setblock"
case "$setblock" in
    *"Unknown block"*|*"unknown block"*|*"error"*) ;;
    *) fail "without --dev-mod the item pipe was still known: $setblock" ;;
esac
give=$(ctl command "give @s moderndynamics:extractor 1")
echo "give reply: $give"
case "$give" in
    *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
    *) fail "without --dev-mod the extractor was still known: $give" ;;
esac
block=$(ctl block 0 100 0 | jq -r '.state // .name // empty')
echo "block 0,100,0 = $block (control)"
echo "PASS: without --dev-mod the item pipe and the extractor are absent"
