#!/usr/bin/env python3
"""Write this port's own copies of the content files, and prove they are the port's.

Modern Dynamics' code is LGPL-3.0-only, but its repository's README applies CC-BY-NC-SA-4.0 to the
**art**, and these content files were the jar's own bytes in the 0.2.0 package (found by
`tools/check_provenance.py`: it compares every packaged file's sha256 against every jar entry). The
textures are drawn by `tools/gen_textures.py`; the files below are **rewritten from what they
express** — the same recipes, the same loot, the same item models, written in this port's own key
order, indentation and trailing newline, which is the twilightforest precedent's
"rewritten-from-semantics" category (ARCHITECTURE.md §8).

What each file *means* (read from the jar's data, kept exactly):

* the item pipe is crafted shaped, one row `igi` — glass, iron ingot, glass — and gives eight;
* the extractor is shaped ` h ` / `mrm` / ` p ` — hopper over iron/redstone/iron over paper — and
  gives one;
* the attractor is shaped ` e ` / `mrm` / ` p ` — ender pearl over iron/redstone/iron over paper —
  and gives one;
* the pipe drops itself, gated on `minecraft:survives_explosion`, one roll, its own random sequence;
* each attachment's item model is `minecraft:item/generated` with `moderndynamics:item/<name>` as
  layer 0.

The loot table is in **26.3's** shape, not 1.21.1's: one typed `"condition"` per pool (vanilla
26.3's own `loot_table/blocks/hopper.json` writes exactly this), an integral `"rolls"` and no
`"bonus_rolls"`. The 1.21.1 array form (`"conditions": [ … ]`) is one of the three
game-confirmed-fatal data shapes (`ports/tools/check_261_dialect.py`) and made the pipe drop
nothing, so a broken pipe was lost.

    python3 tools/assemble_content.py            # write the files
    python3 tools/assemble_content.py --check    # exit 1 if any differs from what it would write
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

PORT = Path(__file__).resolve().parent.parent

MODEL = "assets/moderndynamics/models/item/extractor.json"
MODEL_ATTRACTOR = "assets/moderndynamics/models/item/attractor.json"
LOOT = "data/moderndynamics/loot_table/blocks/item_pipe.json"
RECIPE_EXTRACTOR = "data/moderndynamics/recipe/extractor.json"
RECIPE_ATTRACTOR = "data/moderndynamics/recipe/attractor.json"
RECIPE_PIPE = "data/moderndynamics/recipe/item_pipe.json"


def shaped(pattern: list[str], key: dict[str, str], item: str, count: int) -> dict:
    """One shaped crafting recipe in this port's shape: sorted keys, `id` before `count`.

    An ingredient is the 26.3 **string** form: `"minecraft:glass"`, or `"#c:ingots/iron"` for a tag.
    The 1.21.1 object form (`{"item": ...}` / `{"tag": ...}`) has no type key at 26.3, so the engine's
    ingredient walk refuses it ("recipes: invalid ingredient (unresolved tag or malformed input)")
    and the recipe never loads — see `notes.md`, `## Upstream defects fixed`.
    """
    return {
        "type": "minecraft:crafting_shaped",
        "category": "misc",
        "key": dict(sorted(key.items())),
        "pattern": pattern,
        "result": {"id": item, "count": count},
    }


def build() -> dict[str, bytes]:
    """The content files, keyed by their path inside the package, each with a trailing newline."""
    files: dict[str, object] = {
        MODEL: {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": "moderndynamics:item/extractor"},
        },
        MODEL_ATTRACTOR: {
            "parent": "minecraft:item/generated",
            "textures": {"layer0": "moderndynamics:item/attractor"},
        },
        LOOT: {
            "type": "minecraft:block",
            "random_sequence": "moderndynamics:blocks/item_pipe",
            "pools": [
                {
                    "rolls": 1,
                    "condition": {"type": "minecraft:survives_explosion"},
                    "entries": [{"type": "minecraft:item", "name": "moderndynamics:item_pipe"}],
                }
            ],
        },
        RECIPE_PIPE: shaped(
            ["igi"],
            {"g": "minecraft:glass", "i": "minecraft:iron_ingot"},
            "moderndynamics:item_pipe",
            8,
        ),
        RECIPE_EXTRACTOR: shaped(
            [" h ", "mrm", " p "],
            {
                "h": "minecraft:hopper",
                "m": "minecraft:iron_ingot",
                "p": "minecraft:paper",
                "r": "minecraft:redstone",
            },
            "moderndynamics:extractor",
            1,
        ),
        RECIPE_ATTRACTOR: shaped(
            [" e ", "mrm", " p "],
            {
                "e": "minecraft:ender_pearl",
                "m": "minecraft:iron_ingot",
                "p": "minecraft:paper",
                "r": "minecraft:redstone",
            },
            "moderndynamics:attractor",
            1,
        ),
    }
    return {
        name: (json.dumps(body, indent=2, sort_keys=True) + "\n").encode()
        for name, body in files.items()
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, default=PORT)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = build()
    if args.check:
        differences = [
            name
            for name, body in files.items()
            if not (args.out / name).is_file() or (args.out / name).read_bytes() != body
        ]
        print(f"{len(files)} content files written from their semantics; {len(differences)} differ")
        for name in differences:
            print(f"  differs: {name}")
        return 1 if differences else 0
    for name, body in files.items():
        target = args.out / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
        print(f"wrote {name} ({len(body)} B)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
