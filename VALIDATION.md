# Modern Dynamics — validation (package 0.2.2)

The Ladder as run, with the evidence file each step wrote. Every compile step went through
`ports/tools/build-slot.sh` with `BUILD_LOCAL=1` (the SDK tool is installed on this box only) and
`CARGO_INCREMENTAL=0`; the in-game leg through `ports/tools/launcher-game.sh`. Logs live in
[`../_shared/evidence/moderndynamics/`](../_shared/evidence/moderndynamics/) and
[`../_shared/evidence/moderndynamics-e2e/`](../_shared/evidence/moderndynamics-e2e/), and each top-level
`log` that judges the package quotes the sha256 of the delivered file it ran against.

Pin: **Horizon `0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** (Nightly 2026-11-20) + mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, game 26.3. Delivered artifact:
**`moderndynamics.hmod` — 38,469 B, sha256 `2b848091f46272db89c2d7ed2a91e6a075c63486b3898704a746f5e8b940bf05`,
30 entries, no `Cargo.lock`.**

| Step | Command | Result | Evidence |
|---|---|---|---|
| repin | `[native] horizon = 0a875da…`, `.horizon/` deleted | SDK fetched at `0a875da`; no source edit (no changed hook is used) | `sdk-fetch-0a875da.log` |
| runtime RED | `cargo test --lib --offline` before the attractor API | `EXIT:101`, 41 errors — `cannot find type AttachmentKind`, `no method named accepts_endpoints`, `cannot find function attractor_sources` | `red-attractor-api.log` |
| unit tests | `cargo test --lib --offline` | **15 passed; 0 failed** | `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings | `sdk-clippy.log` |
| formatting | `cargo fmt --check` | clean | `sdk-fmt.log` |
| package A | `rm -f Cargo.lock && cargo horizon-mod build` | 38,469 B, sha `2b848091…` | `sdk-build-a.log` |
| package B | the same again | same bytes; `cmp` clean | `sdk-build-b.log`, `sdk-build-cmp.log` |
| content determinism | `tools/gen_textures.py --check`, `tools/assemble_content.py --check` | 0 differ | `sdk-content-check.log` |
| registry check | `cargo horizon-mod check moderndynamics.hmod` | `moderndynamics 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, EXIT 0 | `sdk-check.log`, `sdk-check-json.log` |
| fits (pin, tautology) | `cargo horizon-mod fits … --build 0a875da` | `0a875da  made for` | `sdk-fits-0a875da.log` |
| fits (other build) | `cargo horizon-mod fits … --build 0b14f8a` | `0b14f8a  fits` | `sdk-fits-0b14f8a.log` |
| surface used | `cargo horizon-mod check … --uses` | 306 entries | `sdk-uses.log` |
| data dialect | `python3 ../tools/check_261_dialect.py moderndynamics.hmod` | 4/4 in the 26.3 dialect | `sdk-dialect.log` |
| provenance | `python3 tools/check_provenance.py` | 30 packaged files, 0 identical to a jar entry; `PROVENANCE.md` written | `sdk-provenance.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py moderndynamics` | **1/1** | `sdk-package.txt` |
| in-game | `launcher-game.sh build moderndynamics` (`Compiling moderndynamics (1 of 1)`), then `tools/e2e.sh` | **PASS** + `EXIT:0` | `moderndynamics-e2e/e2e-launcher.log` |
| control | `GAME=$CONTROL_GAME tools/e2e.sh` | `FAIL: the game does not know moderndynamics:item_pipe…` + `EXIT:1` | `moderndynamics-e2e/e2e-launcher-control.log` |
| selftest | `SELFTEST=1 tools/e2e.sh` | `FAIL: SELFTEST: the target chest holds 20 stone…` + `EXIT:1` | `moderndynamics-e2e/e2e-launcher-selftest.log` |
| log gate | `check-e2e-logs.py moderndynamics-e2e --strict` | `logs=3 ok=3 missing-exit=0 bad-exit=0` | exit 0 |
| skills gate | `check-skills.py --notes ports/moderndynamics --strict` | PASS | `notes.md` |

The slice itself, the upstream rules, the red→green, the in-game transcript, the coverage table and the
harness lessons are in [`notes.md`](notes.md) under `## Slice 2 — the attractor (package 0.2.2)`.
