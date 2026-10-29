# BLAKE3 verification evidence

Project-owned evidence for the claims in `model/ids.toml` and `model/ledger.toml`.
Recorded 2026-10-29 on WSL2 Ubuntu (x86_64, AMD Ryzen AI MAX 390, 12 threads,
16 GB), LexLean main `6bcc8d598` (`lexlean` 0.3.0), Lean `v4.32.1`, rustc
1.97.1. The trace itself is specified in [trace.md](trace.md).

## What is authored where

| Part | Where | Language |
| --- | --- | --- |
| Compression as a step machine (7 rounds, permutation between rounds, feed-forward), its word trace, the 64-bit counter, the seven flags | `lexlean/src/Blake3.lex.tex` | LexLean 1.2 |
| Chunk tree with the reference stack merge, every compression and stack push/pop logged as the trace | `finalTrace`, `update`, `pushChunk` | LexLean 1.2 |
| `hash`, `keyed_hash`, `derive_key`, output of any length, the incremental hasher | projections of the trace (`digestOf`) | LexLean 1.2 |
| Blobs, ADR-015 DigestSets, the tree/word/bit files, `traceAddress`, `checkTrace` | `lexlean-trace/src/Blake3Trace.lex.tex` | LexLean 1.2 |
| SHA-256 for the DigestSets | `vendor/lexlean-stdlib/` (LexLean stdlib, byte for byte) | LexLean 1.2 |
| The Rust that hashes and writes traces | `export/rust-std/*` | rendered by `lexlean production build` |
| The engine that runs `checkTrace` | the Prism image `prism/IMAGE` | LexLean calculus programs |
| Tests, oracles, measurements | `crates/blake3-check`, `prism/*.mjs` | Rust, JavaScript (evidence only) |

## Status

| Claim | Evidence | Command | Result |
| --- | --- | --- | --- |
| Models generated, canonical | `tools/gen_blake3.py` output equals the 26 project modules and `prism/Blake3Image.lex.tex` | `LEXLEAN=… python3 -I tools/gen_blake3.py --check` | equal |
| All five projects check, format, lock | `lexlean check`, `fmt --check`, `lock --check` | in `lexlean/`, `lexlean-trace/`, `lexlean-reverse/`, `lexlean-slice/`, `lexlean-analysis/` | pass |
| Core export with 16 kernel theorems | 11 official-vector theorems (≤ 2 compressions each) over trace-derived root words; 5 universal projection theorems | `LEXLEAN_VERIFY_WIDTH=1 lexlean production build --request production-request.json` in `lexlean/` | rc 0, 404 s, 3.3 GB peak, 5 packages, `export/attestation.json` |
| Trace export | `traceFile`, `traceAddress` | same, in `lexlean-trace/` | rc 0, 245 s, 1.9 GB peak, 2 packages, `export/attestation-trace.json` |
| B3-01 official vectors | 35 cases × 3 modes × 131 bytes, plus hex and incremental | `cargo test -p blake3-check --test official` | 175/175 |
| B3-02 blake3 crate | 25 boundary lengths × 3 modes × 9 output lengths | `--test differential` | 675/675 |
| B3-03 incremental = one-shot | 627 splits | same | pass |
| B3-04 typed key refusal | `keyLength n` | same | pass |
| B3-05 export bound to verified builds | 9 packages, file digests and semantic ids; 21 theorems in 4 projects with exact axioms | `--test export` | pass |
| B3-06 trace digest and address | root digest = crate, input DigestSet = crates, `traceAddress` = root DigestSet | `--test trace` | pass |
| B3-07 independent DigestSets | 1,920 DigestSets at all levels equal the `blake3` and `sha2` crates | same | pass |
| B3-08 checker accepts | 21 real traces (all modes, tree/word/bit) checked in the image's engine | same | pass |
| B3-09 checker rejects planted defects | 8 plants, each as its `TraceError` at its offset; another input's trace refused | same | pass |
| B3-10 determinism | byte-identical traces, whole or split | same | pass |
| B3-11 shared addresses | first chunk of two inputs: same 16 checkpoint labels and chunk label | same | pass |
| Reverse project | `Blake3Reverse` and the kernel theorem that one compression (empty input, root flags) runs back exactly to its initial state | `LEXLEAN_VERIFY_WIDTH=1 lexlean verify` in `lexlean-reverse/` | rc 0, 3 modules, `export/attestation-reverse.json` (`e526f5f7…`) |
| B3-13 run back | 20 root output blocks of seeded traces back to their checkpoints; their 8-word truncations not | `--test reverse` (engine) | pass |
| B3-14 inverses | 2,624 operations, edge values and seeded words, all five kinds | same | pass |
| B3-15 back = replay, cursor, seek | 4 compressions agree at all 800 steps; 15 cursor moves; 7 seeks equal the word-level records and tags | same | pass |
| B3-16 navigation | root → parent → chunk 0 → its first checkpoint; that checkpoint → its chunk and the events; outputs → root checkpoints | same | pass |
| Slice project | `Blake3Slice` and 3 kernel theorems: chunk 1 of the official 1025-byte input, its slice (cv0, cv1 from hazmat, `SLICE_CVS` in the generator) checks against the official hash; one sibling-CV bit flipped → `parent 0`; the chunk byte changed → `chunk` | `LEXLEAN_VERIFY_WIDTH=1 lexlean verify` in `lexlean-slice/` | rc 0, 5 modules, 255 s, 1.8 GB, `export/attestation-slice.json` (`90a7b462…`) |
| Slice export | `sliceOf`, `sliceCheck` | `LEXLEAN_VERIFY_WIDTH=1 lexlean production build --request production-request.json` in `lexlean-slice/` | rc 0, 283 s, 1.8 GB peak, 2 packages, `export/result-slice.json` (`9e305b0a…`) |
| Analysis project | `Blake3Analysis` and the kernel theorem `wordDiffusion = 163` | `LEXLEAN_VERIFY_WIDTH=1 lexlean verify` in `lexlean-analysis/` | rc 0, 4 modules, 17 s, 1.4 GB, `export/attestation-analysis.json` (`bad547a9…`) |
| Analysis export (one attempt) | `diffTrace`, `diffTraceBack`, `diffReach`, `taintRound` as rust-std | same, `lexlean-analysis/` | OOM at 6 GB after 3 min 41 s; the roots stay in the engine |
| B3-17 slices | 306 slices (15 lengths 0 to 31,745 B, every chunk, 3 modes; 7 chunks of a 1 MiB + 1 input in 3 modes) equal hazmat-built slices, and the `bao` 0.13.1 crate's in hash mode; each checks to its chunk; Bao's decoder accepts the model's; 31 planted defects (sibling and on-path CV at each of 11 levels, chunk byte, short, long, next index, wrong root, length field, longer input, 31-byte root, chunk past the end) and a 31-byte key refused, each with its error; the image's roots agree | `--test slice` (certified Rust) | 3/3 pass, 63 s |
| B3-18 difference traces | keyed 64-byte root compression; 768 message and key flips observed from certified `traceFile` word records; 17 flips (message, key, counter, length, flags bits) through the engine: `diffTrace` = `diffTraceBack` = the records, `diffReach` = the records' reach; a 4097-byte input with one bit of chunk 2, block 5 flipped differs in exactly 14 of 69 compressions (11 blocks, the 3 parents of its slice path) | `--test analysis` (engine 2 processes) | pass |
| B3-19 dependency map | `taintRound` 0-7: 3,276,800 bytes (BLAKE3 `357abf9e…`) equal a propagation written from the specification; 8,604,050 observed differing result bits of 768 flips all inside it; `fullTaintSteps` = the map's | same | pass, 3/3 in 223 s |
| B3-12 vendored stdlib = upstream | exactly upstream's 3 files; listing digest = `AUTH-LEXLEAN-STDLIB` checksum `0990190f…`; trace lock pins the manifest | `--test vendor` | pass |
| Engine checks with PrismPM BLAKE3 | 57 checks: acceptance at 0, 64, 1025 B in 3 modes at every level; every DigestSet = PrismPM `blake3.mjs` (commit `d9672958`, sha256 `09a3fb46…`) + node SHA-256; all plants | `node prism/trace-checks.mjs de0a8067… --registry … --blake3 <blake3.mjs>` (an image whose programs other than `acceptance` are byte-identical to `a77f4113…`) | 57/57 PASS, 41 min |
| Template gates without the SDK | fmt, clippy (also `--all-features`), workspace tests, `cargo deny`, model audits | each `just` slice | pass |
| `just vv` | needs `prismpm.lock` | `just vv` | red: `prismpm.lock is required` (expected, unbound) |

## Planted defect

G's second rotation was changed from 12 to 11 in a copy of the generator
(`ROTATIONS`: `(12, 20)` → `(11, 21)`), every module regenerated, and the gates
run on the copy. The committed tree was never edited.

| Gate | Diagnostic | Result |
| --- | --- | --- |
| Lean kernel (`lexlean verify`, core + `Blake3KernelOfficialHash0`) | `rfl failed: The left-hand side Blake3.rootWordsAt (Blake3.finalTrace (Blake3.update (Blake3.hasherWith Blake3.iv 0) { data := #[] }) 64) 0 is not definitionally equal to the right-hand side { v0 := 3108574127, … }` | rejected |
| Rust differential, against the defective export | all 4 tests of `--test differential` fail; `the_blake3_crate_agrees_on_every_boundary_b3_02`: left `[179]`, right `[175]` | rejected |

The trace checker's own planted defects are B3-09 and the engine checks above.

A second plant targets the vendored stdlib: the first round constant in
`vendor/lexlean-stdlib/src/Sha256.lex.tex` changed from `1116352408` to
`1116352409`, one byte.

| Gate | Diagnostic | Result |
| --- | --- | --- |
| B3-12 (`cargo test -p blake3-check --test vendor`) | `vendor/lexlean-stdlib differs from LexLean 6bcc8d598`, left `sha256:e1bec0c9…`, right `sha256:0990190f…` | rejected |
| `lexlean lock --check` in `lexlean-trace/` | `LLC0105: source package std is not the package the lock pins: tree_sha256: locked c0be8a51…, found 798bfe45…` | rejected |

Restored (`git checkout -- vendor/…`): both pass, and the committed tree passes
every gate in the status table.

## Trace sizes (`OPEN-B3-TRACE-SIZES`)

Measured with `cargo run --release -p blake3-check --bin trace_sizes`, hash
mode, 32-byte output. A word or bit section has a fixed size per checkpoint
(14,665 and 33,097 bytes), so the 1 MiB word and bit sizes are the tree size
plus that many sections; the same arithmetic equals the written files at 64 B
and 1 KiB.

| Input | Compressions | Tree level (checkpoints only) | Word level | Bit level |
| --- | --- | --- | --- | --- |
| 64 B | 1 | 896 B | 15,561 B (written) | 33,993 B (written) |
| 1 KiB | 16 | 5,321 B | 239,961 B (written) | 534,873 B (written) |
| 1 MiB | 17,407 | 5,537,705 B (written, 3.4 s) | 260,811,360 B (computed) | 581,657,184 B (computed) |

The tree level is what is stored; every finer step is replayed from its
checkpoint.

The trace's address (`traceAddress`, hash mode, 32-byte output), the root
blob's DigestSet, for the official empty input and for `abc`:

| Input | `blake3:` | `sha256:` |
| --- | --- | --- |
| empty | `e71d2a84a11166ce4bbd507bcfe44b21b58921ef312c134233927f3e32d09b00` | `d80c486b42c33754c74cc67db9b2de754008a1a3a4aae6489bc44f82f7701caa` |
| `abc` | `6c6408e979ed14b07879592ac4add528ba3b1ed07acd285c26fc234045d2d241` | `55738d4e01ffb3afba07d8f857ff5deb699ca0ea5713e96e38161ec7956a2ee6` |

Read from the image's `traceAddress`; B3-06 holds it equal to the certified
package and B3-07 every DigestSet to the `blake3` and `sha2` crates.

## Throughput (`OPEN-B3-THROUGHPUT`)

Measured, not asserted. `cargo run --release -p blake3-check --bin throughput`
(the crate's default build: runtime-selected SIMD, AVX-512 present) and with
`--features portable`. Single thread, 32-byte output, medians of three runs;
the export column is the median of all six. Load average 1.8 to 2.4 on 12
threads.

| Input | Trace-first export MiB/s | blake3 portable MiB/s | blake3 SIMD MiB/s | portable / export | SIMD / export |
| --- | --- | --- | --- | --- | --- |
| 64 B | 2.07 | 608 | 488 | 294x | 236x |
| 1 KiB | 2.18 | 1,138 | 795 | 522x | 365x |
| 16 KiB | 2.08 | 1,199 | 9,055 | 576x | 4,353x |
| 1 MiB | 2.09 | 1,180 | 10,593 | 565x | 5,068x |

The trace-first model builds the full 800-step word trace of every
compression, through a table-driven step machine, to read its output back. The
earlier plain model (2026-10-28, same machine, under heavier load) ran at about
172 MiB/s at 1 MiB: tracing by construction costs about 80x.

Ilya chose granularity and determinism over speed, so there is no second hash
path. The option, if wanted: a fast hash (the earlier plain model) beside the
traced one, held equal to it by differential tests only, with no proof that
the two agree, so its digests would no longer be projections of a trace by
construction.

## Slices, differences and diffusion

**Slice storage.** A slice of one chunk is 8 + 64 × depth + the chunk, with
depth at most ⌈log2 n⌉ for n chunks. For chunk 700 of a 1 MiB + 1 input
(1,025 chunks): 1,736 bytes and 11 parents; `sliceCheck` runs 27
compressions (11 parents, 16 blocks) where hashing the input runs 17,409.

**Difference reach** (B3-18, the step after which the runs first differ in
each state word; a round is 112 steps). Message bit 0 of the keyed root
compression:

| v0 | v1 | v2 | v3 | v4 | v5 | v6 | v7 | v8 | v9 | v10 | v11 | v12 | v13 | v14 | v15 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 77 | 91 | 98 | 5 | 61 | 75 | 89 | 4 | 102 | 60 | 74 | 2 | 93 | 100 | 58 |

All in round 0. The latest reach over all 768 message and key bits is in
round 1, v15 last at step 163. The share of state bits that differ after each
round, averaged over the 768: 0.256 after round 0, then 0.500, 0.499, 0.500,
0.500, 0.498, 0.500.

**Diffusion** (B3-19). Structurally, every bit of every state word depends on
every one of the 896 input bits after step 215, in round 1 (`fullTaintSteps`:
v0 168, v1 182, v2 196, v3 210, v4 215, v5 173, v6 187, v7 201, v8 200, v9
214, v10 172, v11 186, v12 184, v13 198, v14 212, v15 170). At word level,
every state word depends on every input word after step 163, a kernel
theorem. Observed against structural: for each of the 768 flips and each
word, the first step the word differs equals the first step the map allows
(12,288 of 12,288, never later), so every word differs under every flip by
step 163, the word-level bound exactly. No observed difference falls outside
the map. The map is an upper bound on influence and the differences a lower
bound, and at word granularity they meet.

**Storage per compression.** Difference trace 9,600 bytes (12 per step);
dependency map 3,276,800 bytes, one map for every compression, since it does
not depend on the values.

## The Prism image (`OPEN-B3-PRISM-IMAGE`)

`prism/Blake3Image.lex.tex` flattens the core, the stdlib's Sha256 (renamed
`sha…`), the trace, reverse, slice and analysis modules into the one file the image
builder reads; its roots are the five hash roots, `acceptance`, `traceFile`,
`traceAddress`, `checkTrace`, and the phase-1 roots `unmix`, `stepRoundTrip`,
`prevAgrees`, `cursorMove` and `seekStep`, and the phase-2 roots. Built and checked with the prism-browser-runtime tooling (outside
this repository), under WSL node 22 with its 2 GB runtime twin. The image's
runtime blobs (no compiler) are committed under `prism/registry/` so the tests
can run `checkTrace`; nothing was pushed to any registry.

The committed image is `f20bd6c0a76c6a1020e448140791f98dfa133257ae251ed8ddbbb36be0f7da9f`
(`prism/IMAGE`): `a8f22054…` below plus the phase-2 roots `sliceOf`,
`sliceCheck`, `diffTrace`, `diffTraceBack`, `diffReach`, `taintRound`,
`fullTaintSteps` and `wordDiffusion` (22 programs), every earlier layer
byte-identical. B3-17 to B3-19 run its roots. `prism.mjs verify f20bd6c0…`:
PASS on all five lines (every layer hashes to its name; 22 programs load, each
id = its digest; acceptance vectors 32/32 through `hashHex`; rebuilt from
source, identical programs; `source_id` and `semantic_id` match), 19.2 min.

Image `a8f220549bda467e995539b45320010505ddb9380aac714c3a0926ba3b3d45ac`:
`a77f4113…` below plus the phase-1 roots, every earlier layer byte-identical. `prism.mjs verify a8f22054…`: PASS on all five lines (every layer hashes to its name; 14 programs load, each id = its digest; acceptance vectors 32/32 through `hashHex`; rebuilt from source, identical programs; `source_id` and `semantic_id` match), 17.5 min.

Image `a77f411367e2513167fe7e006eadef36f621b03104541436ddd4f02e9a091833`:

| Check | Command | Result |
| --- | --- | --- |
| manifest, layers, engine | `prism.mjs verify a77f4113… --registry …` | PASS: every layer hashes to its name; 9 programs load, each id = its digest |
| acceptance vectors | same | PASS 32/32 through `hashHex` (official cases up to 8,193 B) |
| rebuild from source | same | PASS: identical programs, `source_id` and `semantic_id` match (21.8 min in all) |
| official vectors, three modes | `node prism/run-vectors.mjs a77f4113… --registry … --max-len 8193` | PASS 96/96 official outputs (32 cases x hash, keyed_hash, derive_key, 131 bytes), 55 min |
| `checkTrace` | B3-08, B3-09 and the 57 engine checks above | pass |

The image's `hash`, `traceFile` and `traceAddress` layers have the same program
ids as the `program` in the certified packages' `provenance.json` (`5d4888fe…`,
`bf693353…`, `0a979828…`): the engine and the Rust run the same lowered
programs. The image's acceptance list stops at 8,193 bytes because the engine
keeps every allocation of a call and the traces of the 16 KiB to 100 KiB
official inputs exhausted its memory (`RuntimeError: unreachable` after 57
minutes); the certified Rust runs all 35 cases (B3-01).

## What is not LexLean, or not certified Rust

| Part | Status |
| --- | --- |
| SIMD lanes, assembly, AVX-512 | Not expressible: LexLean 1.2 has no vector primitive. |
| Multithreading | Not expressible: LexLean is pure and has no parallel primitive. |
| Reading files (`b3sum`) | Host effect; the model takes `Bytes`. |
| PrismPM SDK binding | Absent: no public SDK image to bind. `just vv` stays red. |
| `checkTrace` as Rust | Not certified Rust: it runs only in the calculus engine of the Prism image, because its certificate does not compile within 11 GB ([trace.md](trace.md#where-each-part-runs), [lexlean-issue-draft.md](lexlean-issue-draft.md)). |
| Phase 1 and analysis roots as Rust | Not certified Rust: `unmix`, `stepRoundTrip`, `prevAgrees`, `cursorMove`, `seekStep`, `diffTrace`, `diffTraceBack`, `diffReach`, `taintRound`, `fullTaintSteps`, `wordDiffusion` run only in the image's engine; their certificates ran out of 6 GB ([lexlean-issue-draft.md](lexlean-issue-draft.md)). `sliceOf` and `sliceCheck` are certified Rust. |
| The stdlib's location | A byte-for-byte path copy under `vendor/`, the only form a LexLean source package takes; held to upstream by B3-12. |
| Kernel theorems for large inputs | Inputs past two compressions are held by tests. The bit-level adder is replayed and tested, not proved. |
| Preservation certificates | Export certification observes `Classical.choice, Quot.sound, propext` (`export/result*.json`); the model's theorems use only `Quot.sound, propext`. |
| Prism image tooling | `prism.mjs` and its compiler twin (LexLean `201519a1b` plus spike patches) live outside this repository. |
