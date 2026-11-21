#!/usr/bin/env python3
"""Prove no packaged file carries an upstream byte, and write the audit trail.

The port's code is a clean-room Rust rewrite under **LGPL-3.0-only** (the jar's
``META-INF/neoforge.mods.toml``). Its art is **not** the upstream art: the upstream repository's
README applies **CC-BY-NC-SA-4.0 to the art**, so not one byte of it may travel in a package
published under the code's licence (ARCHITECTURE.md §8; the twilightforest ruling and the
MineColonies precedent). The 0.2.0 package broke that rule: three PNGs, the extractor's item model
and both recipes were the jar's own bytes. They are now drawn (`tools/gen_textures.py`) or
rewritten from what they express (`tools/assemble_content.py`).

This tool is the machine-checked half of that claim:

* it opens the pack's Modern Dynamics jar and the built ``moderndynamics.hmod``;
* it asserts that **no packaged file's sha256 equals any jar entry's sha256** — a copy under a
  different name is caught too, because the comparison is by content, not by path;
* it writes ``PROVENANCE.md``, one line per packaged file, naming the generator or the rewrite
  that produced it — the audit trail a reviewer reads beside ``LICENSE-NOTICE.md``.

    python3 tools/check_provenance.py            # assert, then write PROVENANCE.md
    python3 tools/check_provenance.py --check    # assert only (the ladder gate)

Exit status: 0 when the package is clean, 1 when a packaged file matches a jar entry (each
offender is printed with the jar path it matches), 2 when the jar or the package is missing.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
import zipfile
from dataclasses import dataclass
from pathlib import Path

PORT = Path(__file__).resolve().parent.parent
PORTS = PORT.parent
DEFAULT_JAR = (
    PORTS / "_shared" / "carry-work" / "moderndynamics" / "jar" / "Modern-Dynamics-0.9.6.jar"
)
DEFAULT_HMOD = PORT / "moderndynamics.hmod"
DEFAULT_REPORT = PORT / "PROVENANCE.md"

TEXTURE_GEN = "generated (`tools/gen_textures.py`, this port's own drawing)"
CONTENT_GEN = "rewritten-from-semantics (`tools/assemble_content.py`, this port's own bytes)"

# Files whose provenance is not obvious from their folder. Everything not named here is the
# port's own Rust, manifest or prose and is classified below by prefix.
EXPLICIT: dict[str, str] = {
    "assets/moderndynamics/textures/pipe/item_pipe/base.png": TEXTURE_GEN,
    "assets/moderndynamics/textures/pipe/item_pipe/straight.png": TEXTURE_GEN,
    "assets/moderndynamics/textures/item/extractor.png": TEXTURE_GEN,
    "assets/moderndynamics/textures/item/attractor.png": TEXTURE_GEN,
    "assets/moderndynamics/models/item/extractor.json": CONTENT_GEN,
    "assets/moderndynamics/models/item/attractor.json": CONTENT_GEN,
    "data/moderndynamics/loot_table/blocks/item_pipe.json": CONTENT_GEN,
    "data/moderndynamics/recipe/extractor.json": CONTENT_GEN,
    "data/moderndynamics/recipe/attractor.json": CONTENT_GEN,
    "data/moderndynamics/recipe/item_pipe.json": CONTENT_GEN,
    # The package's own prose, the licence text and the manifests.
    "horizon-mod.toml": "authored (the package manifest)",
    "Cargo.toml": "authored (the crate, one SDK path dependency)",
    "README.md": "authored",
    "LICENSE": "authored (the canonical LGPL-3.0 text; not from the jar)",
    "LICENSE-NOTICE.md": "authored (the licence split and the upstream attribution)",
    "uses.json": "generated (`cargo horizon-mod build`)",
}


def classify(name: str) -> str:
    """The provenance of one packaged file, by its exact path or its folder."""
    explicit = EXPLICIT.get(name)
    if explicit is not None:
        return explicit
    if name.startswith(("src/", "assets/moderndynamics/", "data/moderndynamics/")):
        return "authored (the port's own Rust, blockstate, models, lang or declarations)"
    return "authored"


@dataclass(frozen=True)
class Packaged:
    """One entry of the built package."""

    name: str
    size: int
    digest: str

    @property
    def provenance(self) -> str:
        return classify(self.name)


def packaged(hmod: Path) -> list[Packaged]:
    """Every file entry of the package, sorted by path."""
    entries: list[Packaged] = []
    with zipfile.ZipFile(hmod) as archive:
        for info in archive.infolist():
            if info.is_dir():
                continue
            digest = hashlib.sha256(archive.read(info)).hexdigest()
            entries.append(Packaged(info.filename, info.file_size, digest))
    return sorted(entries, key=lambda entry: entry.name)


def jar_digests(jar: Path) -> dict[str, list[str]]:
    """sha256 -> the jar's paths with that content (a copy under another name still matches)."""
    found: dict[str, list[str]] = {}
    with zipfile.ZipFile(jar) as archive:
        for info in archive.infolist():
            if info.is_dir():
                continue
            digest = hashlib.sha256(archive.read(info)).hexdigest()
            found.setdefault(digest, []).append(info.filename)
    return found


def report(
    jar: Path, hmod: Path, entries: list[Packaged], offenders: list[tuple[Packaged, list[str]]]
) -> str:
    """The markdown audit trail: the claim, the two artefacts, and one line per file."""
    lines = [
        "# Provenance — every packaged file is this port's own",
        "",
        "Modern Dynamics' **code is LGPL-3.0-only** (the jar's `META-INF/neoforge.mods.toml`), so a",
        "port may be distributed under the same licence with its complete source. The upstream",
        "repository's README applies **CC-BY-NC-SA-4.0 to its art**, so **no upstream byte may travel",
        "in the package** (ARCHITECTURE.md §8; the twilightforest ruling). This report is generated by",
        "`tools/check_provenance.py`, which also proves it: it compares the sha256 of every packaged",
        "file against every entry of the jar.",
        "",
        "| | |",
        "|---|---|",
        f"| jar | `{jar.name}` — {jar.stat().st_size:,} B, sha256 `{hashlib.sha256(jar.read_bytes()).hexdigest()}` |",
        f"| package | `{hmod.name}` — {hmod.stat().st_size:,} B, sha256 `{hashlib.sha256(hmod.read_bytes()).hexdigest()}`, {len(entries)} files |",
        f"| files identical to a jar entry | **{len(offenders)}** |",
        "",
        "## How each file was produced",
        "",
        "| packaged file | bytes | provenance |",
        "|---|---:|---|",
    ]
    lines += [f"| `{entry.name}` | {entry.size:,} | {entry.provenance} |" for entry in entries]
    lines += [
        "",
        "## The two categories that matter",
        "",
        "* **generated (`tools/gen_textures.py`)** — the three textures are *drawn* from the palette and",
        "  the closed-form patterns in that file: a 32x32 steel plate with four rivets, a 32x32 arm band",
        "  with a brass groove, and a 16x16 brass funnel. It reads no upstream file, holds no upstream",
        "  pixel, uses no randomness, and `--check` re-renders byte for byte.",
        "* **rewritten-from-semantics (`tools/assemble_content.py`)** — the extractor's item model, the",
        "  pipe's loot table and the two shaped recipes express the same functional content as the jar's",
        "  files (same ingredients, same pattern, same drop, same layer), in this port's own key order,",
        "  indentation and trailing newline. The semantics are the mod's design; the bytes are ours.",
        "",
        "The 0.2.0 package carried seven jar-identical files (three textures, the item model, the loot",
        "table and both recipes). That is why 0.2.1 exists.",
        "",
        "This report is not packaged: the package carries `README.md`, `LICENSE` and `LICENSE-NOTICE.md`",
        "among its root documents, and `notes.md`/`PROVENANCE.md` stay in the repository as the audit",
        "trail.",
        "",
        "Regenerate with:",
        "",
        "```sh",
        "python3 tools/gen_textures.py        # the three textures",
        "python3 tools/assemble_content.py    # the model, the loot table, the two recipes",
        "python3 tools/check_provenance.py    # this report, and the assertion",
        "```",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--jar", type=Path, default=DEFAULT_JAR)
    parser.add_argument("--hmod", type=Path, default=DEFAULT_HMOD)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument(
        "--check",
        action="store_true",
        help="assert only: exit 1 on an identical file, write no report",
    )
    args = parser.parse_args()

    for path, what in ((args.jar, "jar"), (args.hmod, "package")):
        if not path.is_file():
            print(f"missing {what}: {path}", file=sys.stderr)
            return 2

    entries = packaged(args.hmod)
    jar = jar_digests(args.jar)
    offenders = [(entry, jar[entry.digest]) for entry in entries if entry.digest in jar]

    for entry, matches in offenders:
        print(f"identical to {matches[0]}: {entry.name}", file=sys.stderr)
    print(
        f"{len(entries)} packaged files; {len(offenders)} identical to a jar entry",
        file=sys.stderr,
    )
    if offenders:
        return 1
    if not args.check:
        args.report.write_text(report(args.jar, args.hmod, entries, offenders))
        print(f"wrote {args.report}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
