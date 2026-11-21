#!/usr/bin/env python3
"""Craft every carried Modern Dynamics recipe in a real crafting table.

The parity rule (AGENTS.md, 2026-11-20): a carried recipe that is never crafted
is dead data. This drives all three shaped recipes the package ships — the item
pipe (8), the extractor (1) and the attractor (1) — through `horizon-ctl`:

- the ingredients are given, placed into the grid one by one (slots 100..108,
  row-major), the result slot (200) is read back, and taking it must leave the
  grid empty and add exactly `count` of the declared id to the inventory, with
  every ingredient consumed;
- a **negative grid** follows: the attractor's pattern with a lapis lazuli where
  its ender pearl goes. No recipe in this package or in vanilla matches that
  grid, so the result slot must stay empty — the differential that proves the
  three PASS lines above are about these recipes and not about "any grid
  crafts".

Usage:
    python3 tools/recipe_check.py --socket SOCK --ctl CTL --table 60 100 6
"""

from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
NS = "moderndynamics"
GRID = {
    (0, 0): 100, (1, 0): 101, (2, 0): 102,
    (0, 1): 103, (1, 1): 104, (2, 1): 105,
    (0, 2): 106, (1, 2): 107, (2, 2): 108,
}
RESULT_SLOT = 200

# The differential: the attractor's own pattern with lapis where its ender pearl goes.
# No carried recipe and no vanilla recipe matches it, so it must craft nothing.
NEGATIVE = {
    "name": "attractor grid with lapis instead of an ender pearl",
    "type": "minecraft:crafting_shaped",
    "pattern": [" l ", "mrm", " p "],
    "key": {
        "l": "minecraft:lapis_lazuli",
        "m": "minecraft:iron_ingot",
        "p": "minecraft:paper",
        "r": "minecraft:redstone",
    },
    "result": {"id": "moderndynamics:attractor", "count": 1},
}


class Client:
    def __init__(self, ctl: str, socket: str):
        self.ctl = ctl
        self.socket = socket

    def run(self, *args: str):
        out = subprocess.run(
            [self.ctl, "--socket", self.socket, *args], text=True, capture_output=True
        )
        try:
            return json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")

    def wait(self, ticks: int = 1) -> None:
        self.run("wait", str(ticks))

    def close(self) -> None:
        try:
            self.run("close")
        except SystemExit:
            pass

    def slots(self) -> list[dict]:
        screen = self.run("screen")
        if "slots" not in screen:
            raise SystemExit(f"no menu is open: {screen}")
        return screen["slots"]

    def stack(self, slot: int) -> dict | None:
        for entry in self.slots():
            if entry["slot"] == slot and entry.get("item"):
                return entry
        return None

    def counts(self) -> dict[str, int]:
        return self.run("inventory").get("counts", {})

    def grid_is_empty(self) -> bool:
        return all(self.stack(slot) is None for slot in GRID.values())

    def empty_inventory_slot(self) -> int:
        for entry in self.slots():
            slot = entry["slot"]
            if slot <= 35 and not entry.get("item"):
                return slot
        raise SystemExit("no empty inventory slot to put a craft into")


def recipes() -> list[tuple[str, dict]]:
    """Every shaped recipe the package carries, by file stem."""
    found = []
    for path in sorted((ROOT / "data" / NS / "recipe").glob("*.json")):
        record = json.loads(path.read_text())
        if "pattern" not in record:
            raise SystemExit(f"{path.name} is not a shaped recipe; extend this checker")
        found.append((path.stem, record))
    if not found:
        raise SystemExit("the package carries no recipe")
    return found


def placements(recipe: dict) -> dict[str, list[int]]:
    """Item id -> the grid slots it fills, for one craft of a shaped recipe."""
    targets: dict[str, list[int]] = {}
    for y, row in enumerate(recipe["pattern"]):
        for x, symbol in enumerate(row):
            if symbol != " ":
                item = recipe["key"][symbol]
                targets.setdefault(item, []).append(GRID[(x, y)])
    return targets


def give(client: Client, item: str, count: int) -> None:
    client.run("give", item, str(count))
    client.wait(1)


def place(client: Client, item: str, targets: list[int]) -> None:
    source = None
    for entry in client.slots():
        if entry.get("item") == item and entry["slot"] <= 35:
            source = entry["slot"]
            break
    if source is None:
        raise SystemExit(f"missing ingredient {item} in the inventory")
    client.run("click-slot", str(source))
    client.wait(1)
    for target in targets:
        client.run("click-slot", str(target), "right")
        client.wait(1)
    client.run("click-slot", str(source))
    client.wait(1)


def open_table(client: Client, table: list[int]) -> None:
    """Put a fresh crafting table down and open it with a real right-click."""
    client.close()
    client.wait(2)
    x, y, z = table
    client.run("setblock", str(x), str(y), str(z), "minecraft:air")
    client.run("setblock", str(x), str(y), str(z), "minecraft:crafting_table")
    client.wait(2)
    client.run("command", f"tp @s {x - 0.5} {y} {z + 0.5}")
    client.wait(2)
    client.run("command", "clear @s")
    client.wait(1)
    client.run("interact", str(x), str(y), str(z))
    client.wait(2)


def grid_up(client: Client, recipe: dict) -> dict[str, int]:
    """Give and place one craft's ingredients; returns what was needed per item."""
    need: dict[str, int] = {}
    for item, slots in placements(recipe).items():
        need[item] = len(slots)
    for item, count in need.items():
        give(client, item, count)
    client.wait(2)
    for item, slots in placements(recipe).items():
        place(client, item, slots)
    return need


def crafted(client: Client, stem: str, recipe: dict, need: dict[str, int]) -> str:
    """Assert one craft's result, then take it and assert the grid and inventory."""
    want_id = recipe["result"]["id"]
    want_count = recipe["result"].get("count", 1)
    got = client.stack(RESULT_SLOT)
    if not got:
        raise SystemExit(f"FAIL {stem}: the grid crafted nothing, expected {want_count} {want_id}")
    if got["item"] != want_id or got["count"] != want_count:
        raise SystemExit(
            f"FAIL {stem}: crafted {got['count']} {got['item']}, "
            f"expected {want_count} {want_id}"
        )
    before = client.counts()
    client.run("click-slot", str(RESULT_SLOT))
    client.wait(1)
    client.run("click-slot", str(client.empty_inventory_slot()))
    client.wait(1)
    after = client.counts()
    if not client.grid_is_empty():
        raise SystemExit(f"FAIL {stem}: the grid still holds items, so they were not consumed")
    if after.get(want_id, 0) != before.get(want_id, 0) + want_count:
        raise SystemExit(
            f"FAIL {stem}: the inventory holds {after.get(want_id, 0)} {want_id} "
            f"after the craft, expected {before.get(want_id, 0) + want_count}"
        )
    for item in need:
        if after.get(item, 0) != 0:
            raise SystemExit(
                f"FAIL {stem}: {after[item]} {item} are left after the craft, so the "
                "ingredients were not consumed"
            )
    foreign = [
        item
        for item, count in after.items()
        if item.startswith(f"{NS}:") and item != want_id and count
    ]
    if foreign:
        raise SystemExit(f"FAIL {stem}: the craft also produced {foreign}")
    return (
        f"PASS {stem}: {want_count} {want_id}, the grid emptied and the "
        f"{sum(need.values())} ingredients were consumed"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--socket", required=True)
    parser.add_argument("--ctl", required=True)
    parser.add_argument("--table", nargs=3, type=int, required=True, metavar=("X", "Y", "Z"))
    args = parser.parse_args()
    client = Client(args.ctl, args.socket)
    try:
        for stem, recipe in recipes():
            open_table(client, args.table)
            need = grid_up(client, recipe)
            print(crafted(client, stem, recipe, need))

        open_table(client, args.table)
        need = grid_up(client, NEGATIVE)
        got = client.stack(RESULT_SLOT)
        if got:
            raise SystemExit(
                f"FAIL negative: {got['count']} {got['item']} crafted from the lapis grid, "
                "so the attractor recipe is not what makes the attractor"
            )
        print(f"PASS negative: {NEGATIVE['name']} crafts nothing")
    finally:
        client.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
