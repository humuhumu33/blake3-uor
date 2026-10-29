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
| Models generated, canonical | `tools/gen_blake3.py` output equals the 18 project modules and `prism/Blake3Image.lex.tex` | `LEXLEAN=… python3 -I tools/gen_blake3.py --check` | equal |
| Both projects check, format, lock | `lexlean check`, `fmt --check`, `lock --check` | in `lexlean/`, `lexlean-trace/` | pass |
| Core export with 16 kernel theorems | 11 official-vector theorems (≤ 2 compressions each) over trace-derived root words; 5 universal projection theorems | `LEXLEAN_VERIFY_WIDTH=1 lexlean production build --request production-request.json` in `lexlean/` | rc 0, 404 s, 3.3 GB peak, 5 packages, `export/attestation.json` |
| Trace export | `traceFile`, `traceAddress` | same, in `lexlean-trace/` | rc 0, 245 s, 1.9 GB peak, 2 packages, `export/attestation-trace.json` |
| B3-01 official vectors | 35 cases × 3 modes × 131 bytes, plus hex and incremental | `cargo test -p blake3-check --test official` | 175/175 |
| B3-02 blake3 crate | 25 boundary lengths × 3 modes × 9 output lengths | `--test differential` | 675/675 |
| B3-03 incremental = one-shot | 627 splits | same | pass |
| B3-04 typed key refusal | `keyLength n` | same | pass |
| B3-05 export bound to verified builds | 7 packages, file digests and semantic ids; 16 theorems with exact axioms | `--test export` | pass |
| B3-06 trace digest and address | root digest = crate, input DigestSet = crates, `traceAddress` = root DigestSet | `--test trace` | pass |
| B3-07 independent DigestSets | 1,920 DigestSets at all levels equal the `blake3` and `sha2` crates | same | pass |
| B3-08 checker accepts | 21 real traces (all modes, tree/word/bit) checked in the image's engine | same | pass |
| B3-09 checker rejects planted defects | 8 plants, each as its `TraceError` at its offset; another input's trace refused | same | pass |
| B3-10 determinism | byte-identical traces, whole or split | same | pass |
| B3-11 shared addresses | first chunk of two inputs: same 16 checkpoint labels and chunk label | same | pass |
| B3-12 vendored stdlib = upstream | exactly upstream's 3 files; listing digest = `AUTH-LEXLEAN-STDLIB` checksum `0990190f�`; trace lock pins the manifest | `--test vendor` | pass |
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
| B3-12 (`cargo test -p blake3-check --test vendor`) | `vendor/lexlean-stdlib differs from LexLean 6bcc8d598`, left `sha256:e1bec0c9�`, right `sha256:0990190f�` | rejected |
| `lexlean lock --check` in `lexlean-trace/` | `LLC0105: source package std is not the package the lock pins: tree_sha256: locked c0be8a51�, found 798bfe45�` | rejected |

Restored (`git checkout -- vendor/�`): both pass, and the committed tree passes
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

## The Prism image (`OPEN-B3-PRISM-IMAGE`)

`prism/Blake3Image.lex.tex` flattens the core, the stdlib's Sha256 (renamed
`sha…`) and the trace module into the one file the image builder reads; its
roots are the five hash roots, `acceptance`, `traceFile`, `traceAddress` and
`checkTrace`. Built and checked with the prism-browser-runtime tooling (outside
this repository), under WSL node 22 with its 2 GB runtime twin. The image's
runtime blobs (no compiler) are committed under `prism/registry/` so the tests
can run `checkTrace`; nothing was pushed to any registry.

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
| The stdlib's location | A byte-for-byte path copy under `vendor/`, the only form a LexLean source package takes; held to upstream by B3-12. |
| Kernel theorems for large inputs | Inputs past two compressions are held by tests. The bit-level adder is replayed and tested, not proved. |
| Preservation certificates | Export certification observes `Classical.choice, Quot.sound, propext` (`export/result*.json`); the model's theorems use only `Quot.sound, propext`. |
| Prism image tooling | `prism.mjs` and its compiler twin (LexLean `201519a1b` plus spike patches) live outside this repository. |
