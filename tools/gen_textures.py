#!/usr/bin/env python3
"""Draw this port's own pipe and attachment textures and prove the bytes are the port's.

Modern Dynamics' **code is LGPL-3.0-only** (the jar's `META-INF/neoforge.mods.toml`), but the
upstream repository's README applies **CC-BY-NC-SA-4.0 to the art**. A package published under the
code's LGPL cannot carry that art (ARCHITECTURE.md §8, the twilightforest ruling: "a mod whose code
is permissive/copyleft but whose art is ARR/NC is ported by regenerating the art, never by carrying
it"). The four textures this port used to carry verbatim — `pipe/item_pipe/base.png`,
`pipe/item_pipe/straight.png`, `item/extractor.png` and `item/attractor.png` — are therefore
**drawn here instead**.

Every number below is the recipe, so the art is auditable:

* a **32x32** pipe plate: a dark steel field, a lighter bevelled rim, a four-rivet pattern at the
  corners and a soft diagonal sheen. The core cube (6x6 px at 5..11) samples the whole texture, so
  the plate reads as a machined pipe wall at any face;
* a **32x32** arm band: two darker rails along the band's axis with a brass groove between them, so
  a straight pipe's arm reads differently from its core;
* a **16x16** extractor face: a brass funnel (a ring at the top, a narrowing throat, a nozzle at the
  bottom) on the same steel field, so the item reads as the machine that pulls items out;
* a **16x16** attractor face: a brass suction port — a ring around a hub — the opposite reading of
  the same field, so the item reads as the machine that pulls items in.

The generator reads no upstream file, holds no upstream pixel and uses no randomness: fixed
palette, integer arithmetic, one zlib level. `--check` re-renders and compares byte for byte.

    python3 tools/gen_textures.py            # write the four PNGs
    python3 tools/gen_textures.py --check    # exit 1 if any differs from what it would write
"""

from __future__ import annotations

import argparse
import struct
import zlib
from pathlib import Path

PORT = Path(__file__).resolve().parent.parent

BASE = "assets/moderndynamics/textures/pipe/item_pipe/base.png"
STRAIGHT = "assets/moderndynamics/textures/pipe/item_pipe/straight.png"
EXTRACTOR = "assets/moderndynamics/textures/item/extractor.png"
ATTRACTOR = "assets/moderndynamics/textures/item/attractor.png"

# The palette: one named colour per role. Steel for the pipe, brass for the moving part.
STEEL_DARK = (48, 54, 62)
STEEL = (74, 82, 92)
STEEL_LIGHT = (104, 114, 126)
STEEL_HIGHLIGHT = (140, 150, 162)
BRASS = (168, 128, 58)
BRASS_LIGHT = (206, 168, 88)
BRASS_DARK = (112, 82, 34)
ALPHA = 255


def mix(low: tuple[int, int, int], high: tuple[int, int, int], amount: float) -> tuple[int, int, int]:
    """Linear blend of two palette colours, ``amount`` 0 = low, 1 = high, rounded to a byte."""
    return (
        round(low[0] + (high[0] - low[0]) * amount),
        round(low[1] + (high[1] - low[1]) * amount),
        round(low[2] + (high[2] - low[2]) * amount),
    )


def pipe_plate() -> list[list[tuple[int, int, int, int]]]:
    """The 32x32 pipe wall: a bevelled steel plate with four rivets and a diagonal sheen."""
    size = 32
    rows: list[list[tuple[int, int, int, int]]] = []
    for y in range(size):
        row: list[tuple[int, int, int, int]] = []
        for x in range(size):
            # The rim: the outer two pixels are darker, the next one lighter (a bevel).
            edge = min(x, y, size - 1 - x, size - 1 - y)
            if edge == 0:
                colour = STEEL_DARK
            elif edge == 1:
                colour = STEEL_LIGHT
            else:
                # The field: a plate whose sheen falls off away from the top-left corner.
                sheen = (x + y) / (2.0 * (size - 1))
                colour = mix(STEEL, STEEL_LIGHT, 0.35 * (1.0 - sheen))
            # Four rivets, two pixels across, at (6,6), (24,6), (6,24), (24,24).
            for rx, ry in ((6, 6), (24, 6), (6, 24), (24, 24)):
                if abs(x - rx) <= 1 and abs(y - ry) <= 1:
                    colour = STEEL_HIGHLIGHT if (x, y) == (rx - 1, ry - 1) else STEEL_DARK
            row.append((*colour, ALPHA))
        rows.append(row)
    return rows


def arm_band() -> list[list[tuple[int, int, int, int]]]:
    """The 32x32 arm band: two steel rails with a brass groove between them."""
    size = 32
    rows: list[list[tuple[int, int, int, int]]] = []
    for y in range(size):
        row: list[tuple[int, int, int, int]] = []
        for x in range(size):
            # The band runs along x; its cross-section (y) is rail, groove, rail.
            across = min(y, size - 1 - y)
            if across == 0:
                colour = STEEL_DARK
            elif across <= 3:
                colour = STEEL_LIGHT
            elif across <= 9:
                # The groove: brass, lit along its centre line.
                centre = abs(y - (size - 1) / 2.0) / 9.0
                colour = mix(BRASS_LIGHT, BRASS_DARK, centre)
            elif across <= 12:
                colour = STEEL
            else:
                colour = mix(STEEL, STEEL_DARK, 0.5)
            # A seam every eight pixels along the band, so a long arm shows its segments.
            if x % 8 == 0:
                colour = mix(colour, STEEL_DARK, 0.45)
            row.append((*colour, ALPHA))
        rows.append(row)
    return rows


def extractor_face() -> list[list[tuple[int, int, int, int]]]:
    """The 16x16 extractor: a brass funnel on the steel field, nozzle pointing down."""
    size = 16
    centre = (size - 1) / 2.0
    rows: list[list[tuple[int, int, int, int]]] = []
    for y in range(size):
        row: list[tuple[int, int, int, int]] = []
        for x in range(size):
            dx = abs(x - centre) / centre
            if y < 2 or y > size - 2 or x == 0 or x == size - 1:
                colour = STEEL_DARK
            elif y == 2:
                colour = STEEL_LIGHT
            elif y <= 6:
                # The funnel's mouth: a brass ring, hollow in the middle.
                if dx > 0.55:
                    colour = mix(BRASS, BRASS_LIGHT, 1.0 - dx)
                else:
                    colour = STEEL_DARK
            elif y <= 10:
                # The throat: it narrows as it descends.
                throat = 0.55 - 0.4 * (y - 6) / 5.0
                colour = BRASS if dx > throat else STEEL_DARK
            elif y <= 13:
                # The nozzle: two brass walls around an opening.
                colour = BRASS_LIGHT if dx > 0.3 else STEEL_DARK
            else:
                colour = STEEL
            row.append((*colour, ALPHA))
        rows.append(row)
    return rows


def attractor_face() -> list[list[tuple[int, int, int, int]]]:
    """The 16x16 attractor: a brass suction port — a ring around a hub — on the steel field.

    The extractor's glyph is a funnel that narrows downward (it pushes items out of its
    side); the attractor's is the opposite reading: a ring that pulls everything toward its
    centre. Both sit on the same bevelled steel field, so the two items read as a pair.
    """
    size = 16
    centre = (size - 1) / 2.0
    rows: list[list[tuple[int, int, int, int]]] = []
    for y in range(size):
        row: list[tuple[int, int, int, int]] = []
        for x in range(size):
            distance = ((x - centre) ** 2 + (y - centre) ** 2) ** 0.5
            if y == 0 or y == size - 1 or x == 0 or x == size - 1:
                colour = STEEL_DARK
            elif y == 1 or x == 1 or y == size - 2 or x == size - 2:
                colour = STEEL_LIGHT
            elif 4.6 <= distance <= 6.0:
                # The ring: lit on its upper-left arc, shadowed on its lower-right one.
                shade = ((x - centre) + (y - centre)) / (2.0 * centre)
                colour = mix(BRASS_LIGHT, BRASS_DARK, 0.5 + 0.5 * shade)
            elif distance <= 1.8:
                # The hub the ring pulls toward.
                colour = BRASS_LIGHT if distance <= 1.0 else BRASS
            elif distance < 4.6:
                colour = STEEL_DARK
            else:
                colour = mix(STEEL, STEEL_DARK, distance / (2.0 * centre))
            row.append((*colour, ALPHA))
        rows.append(row)
    return rows


def png(rows: list[list[tuple[int, int, int, int]]]) -> bytes:
    """An RGBA PNG, encoded by hand: no image library, no timestamp, fixed zlib level."""
    height = len(rows)
    width = len(rows[0])
    raw = bytearray()
    for row in rows:
        raw.append(0)  # filter: none
        for pixel in row:
            raw.extend(pixel)

    def chunk(kind: bytes, body: bytes) -> bytes:
        return b"".join(
            (
                struct.pack(">I", len(body)),
                kind,
                body,
                struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF),
            )
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)  # 8-bit RGBA
    return b"".join(
        (
            b"\x89PNG\r\n\x1a\n",
            chunk(b"IHDR", header),
            chunk(b"IDAT", zlib.compress(bytes(raw), 9)),
            chunk(b"IEND", b""),
        )
    )


def build() -> dict[str, bytes]:
    """The four textures, keyed by their path inside the package."""
    return {
        BASE: png(pipe_plate()),
        STRAIGHT: png(arm_band()),
        EXTRACTOR: png(extractor_face()),
        ATTRACTOR: png(attractor_face()),
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
        print(f"{len(files)} textures drawn; {len(differences)} differ")
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
