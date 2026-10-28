# BLAKE3 verification evidence

Project-owned evidence for the claims in `model/ids.toml` and `model/ledger.toml`.
Recorded 2026-10-28 on WSL2 Ubuntu (x86_64, AMD Ryzen AI MAX 390, 12 threads,
16 GB), LexLean main `6bcc8d598` (`lexlean` 0.3.0, executable sha256
`beee5b6f…`), Lean `v4.32.1`, rustc 1.97.1.

## What is authored where

| Part | Where | Language |
| --- | --- | --- |
| Compression (7 rounds, message permutation between rounds), chunk and parent nodes, 64-bit chunk counter, the seven flags | `lexlean/src/Blake3.lex.tex` | LexLean 1.2 |
| Chunk tree: the reference stack merge, one merge per trailing zero bit of the chunk count | `pushChunk`, `mergeStep`, `finalOutput` | LexLean 1.2 |
| Modes: `hash`, `keyed_hash` (32-byte key, typed refusal otherwise), `derive_key` (context then key material) | `start`, `contextKey`, the roots | LexLean 1.2 |
| Output of any length: root compression with output-block counter 0, 1, 2, …, all 16 words | `rootWords`, `outputBytes` | LexLean 1.2 |
| Incremental hasher as pure functions | `hasherWith` (init), `update`, `finalize`; root `hashPieces` | LexLean 1.2 |
| Digest as bytes and as lowercase hex | `byteOf`, `hex`, root `hashHex` | LexLean 1.2 |
| The Rust that runs | `export/rust-std/*` | Rendered by `lexlean production build`, not handwritten |
| Tests, oracles, throughput harness | `crates/blake3-check` | Rust (evidence only, no hashing) |

Every loop is `iterate`, `iterate_until` or `list_fold`: rounds 7, blocks per
chunk `len/64`, chunks per update `(size-1)/1024`, merges at most 64, output
blocks `ceil(outLen/64)`, hex digits the input length. Nothing recurses.

## Status

| Claim | Evidence | Command | Result |
| --- | --- | --- | --- |
| Model generated, canonical | `tools/gen_blake3.py` output equals the 12 committed modules after `lexlean fmt` | `LEXLEAN=… python3 -I tools/gen_blake3.py --check` | 12 modules equal their generator |
| Model checks, formats, locks | `lexlean check`, `fmt --check`, `lock --check` | in `lexlean/` | pass |
| 11 kernel theorems (≤ 2 compressions each): hash at 0, 1, 63, 64, 65 bytes; extended block 1 at 0 bytes; keyed and derive at 0 and 1 byte; split update `00 01` + `02` = `00 01 02` | `export/attestation.json`, status `verified`, each theorem observed axioms exactly `Quot.sound, propext` | `LEXLEAN_VERIFY_WIDTH=1 lexlean production build --request production-request.json` | rc 0, 447.6 s, 3.06 GB peak, result `9fc5705a…`, attestation `b482941d…`, semantic `ef19daab…` |
| Certified Rust packages | `export/rust-std/` (5 packages), provenance binds each file and the attested semantic id | same build | 5 packages exported |
| B3-01 official vectors | 35 cases × hash, keyed_hash, derive_key at 131 bytes, plus `hashHex` and `hashPieces` | `cargo test -p blake3-check --test official` | 175/175 outputs equal |
| B3-02 blake3 crate | 25 boundary lengths (0 … 102400) × 3 modes × 9 output lengths (0 … 1031) | `cargo test -p blake3-check --test differential` | 675/675 outputs equal |
| B3-03 incremental = one-shot | empty, per-byte (≤ 2048), per-64, per-1024 and seeded random splits, 3 modes | same | 627/627 splits equal |
| B3-04 typed key refusal | keys of 0, 1, 16, 31, 33, 64 bytes give `keyLength n` in `keyedHash` and `hashPieces` | same | pass |
| B3-05 export bound to verified build | file sha256s, semantic id, 11 theorems with exact axioms; a changed byte is caught | `cargo test -p blake3-check --test export` | pass |
| Local Prism image (`OPEN-B3-PRISM-IMAGE`) | image `df8e6277d372c13d4b5fa6f41e5845ec46e6e1d1a93228d968f8eb8bbbacc497`, 6 roots | `prism.mjs verify` and `node prism/run-vectors.mjs` | 5/5 PASS, 35/35 acceptance vectors; 105/105 official outputs in 25.0 s |
| Template gates without the SDK | `cargo fmt --check`, `clippy -D warnings` (with and without `--all-features`), `cargo test --workspace`, `cargo deny check`, `xtask` model/limits/deferral audits | each `just` slice | pass |
| `just vv` | needs `prismpm.lock` and the `prismpm` CLI | `just vv` | red: `prismpm.lock is required` (expected, unbound) |

The official vectors are `oracles/blake3-1.8.7/test_vectors.json` from BLAKE3
release 1.8.7 (commit `f3149ec5…`), sha256 `dcb91ea8…f624`, read as untrusted
data against a closed shape. The oracle crate is `blake3` 1.8.7.

## Planted defect

The second rotation of G was changed from 12 to 11 (`ROTATIONS` in a copy of
`tools/gen_blake3.py`: `(12, 20)` → `(11, 21)`), the modules regenerated, and
every gate run on the copy. The committed tree was never edited.

| Gate | Command (on the defective copy) | Diagnostic | Result |
| --- | --- | --- | --- |
| Lean kernel | `LEXLEAN_VERIFY_WIDTH=1 lexlean verify` with `Blake3` and `Blake3KernelOfficialHash0` | `LLV7002: Lean rejected Blake3Uor.Blake3KernelOfficialHash0: Tactic rfl failed: The left-hand side Blake3.rootWords (…) 0 is not definitionally equal to the right-hand side { v0 := 3108574127, … }` | rc 1, rejected |
| Rust differential, official vectors | `cargo test -p blake3-check --test official` against the defective export (result `9ac06ae3…`) | `assertion left == right failed: hash, input_len 0`, left `[179, 128, 216, …]`, right `[175, 19, 73, …]` | rc 101, rejected |
| Rust differential, blake3 crate | `cargo test -p blake3-check --test differential` | all 4 tests fail; `the_blake3_crate_agrees_on_every_boundary_b3_02` at `len 0, out 1`: left `[179]`, right `[175]` | rc 101, rejected |
| Prism image | `prism.mjs verify d87f21e6…` | `FAIL acceptance vectors 0/35 through hashHex` (the rebuild still matches: the image is a faithful build of a wrong model) | rc 1, rejected |
| Prism official vectors | `node prism/run-vectors.mjs d87f21e6…` | `FAIL … 0/105 official outputs` | rejected |

Restored: the committed tree passes every gate in the status table above.

## Throughput (`OPEN-B3-THROUGHPUT`)

Measured, not asserted. `cargo run --release -p blake3-check --bin throughput`
(the crate's default build: runtime-selected SIMD, AVX-512 present) and the
same with `--features portable` (the crate's `no_sse2 no_sse41 no_avx2
no_avx512 no_neon`). Single thread, 32-byte output, each cell the median of
three runs; the export column is the median of all six runs. Other sessions'
Lean jobs were running (load average 4.5 rising to 7.8 on 12 threads), and
single runs varied by up to about ±40%.

| Input | LexLean export MiB/s | blake3 portable MiB/s | blake3 SIMD MiB/s | portable / export | SIMD / export |
| --- | --- | --- | --- | --- | --- |
| 64 B | 24 | 481 | 377 | 20x | 16x |
| 1 KiB | 166 | 979 | 672 | 5.9x | 4.0x |
| 16 KiB | 226 | 1014 | 7563 | 4.5x | 33x |
| 1 MiB | 172 | 828 | 8615 | 4.8x | 50x |

Above one chunk, the export runs at roughly a fifth of the crate's portable
code. The SIMD build's lead past 1 KiB is its parallel lanes over chunks, which
LexLean has no primitive for. Short inputs lose most to fixed costs: the
counter words are built bit by bit (`wordOfNat`, since `checked_convert` takes
no `Nat`), and every output byte is a balanced 256-leaf lookup (`byteOf`).

## The Prism image

Built with the prism-browser-runtime tooling (outside this repository, at
`spikes/prism-browser-runtime`), run under WSL node 22 with its 2 GB runtime
twin; nothing was published or pushed to any registry.

```bash
node prism.mjs build lexlean/src/Blake3.lex.tex --registry ~/b3-prism/registry
node prism.mjs verify df8e6277… --registry ~/b3-prism/registry
node prism/run-vectors.mjs df8e6277d372c13d4b5fa6f41e5845ec46e6e1d1a93228d968f8eb8bbbacc497 --registry ~/b3-prism/registry
```

The image's five hashing layers have the same program ids as the `program`
recorded in each exported package's `provenance.json` (`hash` `e285c4b4…`,
`hashHex` `f6a98766…`, `keyedHash` `26f2dfb5…`, `deriveKey` `aefa95ab…`,
`hashPieces` `59f367c2…`): the browser engine and the Rust packages run the
same lowered programs. The sixth root, `acceptance`, carries the 35 official
inputs (built in the model by doubling the 251-byte cycle) and their digests.
`hashHex` of `abc` in the image is `6437b3ac…9d85`.

## What is not LexLean, and why

| Part | Status |
| --- | --- |
| SIMD lanes, assembly, AVX-512 | Not expressible: LexLean 1.2 has no vector primitive. |
| Multithreading (rayon) | Not expressible: LexLean is pure and has no parallel primitive. |
| Reading files (`b3sum`) | Host effect; the model takes `Bytes`. A file reader would be a host binding, not LexLean. |
| PrismPM SDK binding | Absent: no public SDK image to bind (PrismPM's only release, 0.2.0, is a CLI tarball). `just vv` stays red. |
| Kernel theorems for large inputs | A 16-compression `rfl` exhausted 8 GB in an earlier study, so inputs past two compressions are held by the differential, not by theorems. Split-update equality is a theorem only for one tiny case; in general it is tested (B3-03). |
| Preservation certificates | Each root's export certification observes `Classical.choice, Quot.sound, propext` (`export/result.json`); the model's own theorems use only `Quot.sound, propext`. |
| Prism image tooling | `prism.mjs` and its compiler twin (LexLean `201519a1b` plus spike patches) live outside this repository; their runs are reported, not gated here. |
