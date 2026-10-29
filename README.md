# blake3-uor

**Try it in your browser: <https://humuhumu33.github.io/blake3-uor/>.** The page
runs the `hash`, `keyedHash` and `deriveKey` programs of Prism image `f20bd6c0`
in the image's own engine, checks every part by SHA-256 first, and compares
each result with an independent BLAKE3. Its source is the `gh-pages` branch.

BLAKE3 written in LexLean 1.2, trace first: every step BLAKE3 performs is a
recorded value of the model, and every digest is a projection of that trace.
All seven rounds, inputs of any length through the chunk tree, the 64-bit
chunk counter, `hash`, `keyed_hash` and `derive_key`, output of any length,
and an incremental hasher as pure functions. The trace serialises at the tree,
word and bit levels as ADR-015 content blobs (`blake3:` labels and
`{sha256, blake3}` DigestSets), and `checkTrace` replays one from its finest
level. A compression also runs backwards from its full output, given its
message and CV, and a cursor seeks and steps through any trace by bit, op, G,
round or compression. A Bao slice proves one chunk against the root hash
through the parent CVs on its path, and difference traces and a dependency
map follow every bit through one compression. No hashing operation is
handwritten Rust.

## Status

**BLAKE3 in LexLean already works as a hash.** In all three modes, at every
length, with any output size and incrementally, it matches the official
vectors and the `blake3` crate (B3-01 to B3-04).

| ID | What is checked | Evidence |
| --- | --- | --- |
| B3-01 | All 35 official BLAKE3 1.8.7 vectors in `hash`, `keyed_hash` and `derive_key`, 131 bytes of extended output each, and the hex digest | [official.rs](crates/blake3-check/tests/official.rs), [test_vectors.json](oracles/blake3-1.8.7/test_vectors.json) |
| B3-02 | Equal to the `blake3` crate 1.8.7 in all three modes at every block, chunk and tree boundary from 0 to 102,400 bytes, output lengths 0 to 1,031 bytes | [differential.rs](crates/blake3-check/tests/differential.rs) |
| B3-03 | Incremental hashing (start, one update per piece, finalize) equals one-shot for 627 splits, all three modes | same |
| B3-04 | A keyed-hash key that is not 32 bytes is refused as `keyLength n` | same |

"Every length" and "any output size" mean every boundary class the tests cover,
not a proof over all lengths: the kernel theorems prove official digests only
for inputs of at most two compressions. The definitions are in
[CONFORMANCE.md](CONFORMANCE.md) and the run is recorded in
[docs/blake3-verification.md](docs/blake3-verification.md#status). Run them with
`cargo test -p blake3-check --test official --test differential`.

Limits, kept visible: SIMD, threads and file I/O are not in LexLean, and the
PrismPM SDK binding is a development candidate, not a release. `checkTrace` and the phase 1 and phase 2 analysis
roots run only in the Prism image's engine, not as certified Rust (`sliceOf`
and `sliceCheck` are certified Rust). Throughput figures are measured, not
claimed. The full list is in
[What is not LexLean](docs/blake3-verification.md#what-is-not-lexlean-or-not-certified-rust).

The export reproduces every official BLAKE3 vector and agrees with the
`blake3` crate on a seeded boundary corpus. Sixteen kernel theorems hold: the
digest and every compression output are projections of the trace for every
input, and eleven small cases give the official digests. Every trace label
matches the `blake3` and `sha2` crates and PrismPM's `blake3.mjs`, and the
checker rejects each planted defect. The format is in [docs/trace.md](docs/trace.md);
evidence, sizes and throughput are in [docs/blake3-verification.md](docs/blake3-verification.md).

The repository is bound to the PrismPM SDK development candidate
`ghcr.io/uor-foundation/prismpm-sdk-candidate@sha256:9775f6b0…` (PrismPM main
`2e44eed0`), because no public SDK release exists yet. `just vv` and the
bootstrap workflow run in it and pass. The locks were rendered by this
repository's own `propose template update` workflow. Re-bind to a release
image when one exists.

## Layout

| Path | What it is |
| --- | --- |
| `tools/gen_blake3.py` | Generates every LexLean module and the image model; the modules are its `lexlean fmt` output |
| `lexlean/src/Blake3.lex.tex` | The core: the step machine and its word trace, the tree trace, modes, extended output, incremental hasher, hash roots |
| `lexlean/src/Blake3Kernel*.lex.tex` | One kernel theorem each: projections for every input, official vectors for tiny ones |
| `lexlean-trace/src/Blake3Trace.lex.tex` | Blobs, DigestSets, the tree/word/bit files, `traceFile`, `traceAddress`, `checkTrace` |
| `lexlean-reverse/src/Blake3Reverse*.lex.tex` | Inverse steps, `compressBack`, the cursor and seek, and the run-back kernel theorem |
| `lexlean-slice/src/Blake3Slice*.lex.tex` | Bao slices (`sliceOf`, `sliceCheck`) and three kernel theorems on the official 1025-byte input |
| `lexlean-analysis/src/Blake3Analysis*.lex.tex` | Difference traces, the dependency map, and the word-diffusion kernel theorem |
| `vendor/lexlean-stdlib/` | LexLean's stdlib (Sha256), byte for byte from LexLean main `6bcc8d598` |
| `export/` | The certified packages and attestations the production builds wrote, and the verified projects' attestations |
| `crates/blake3-check/` | Tests, the independent trace reader, throughput and trace-size measurements |
| `oracles/blake3-1.8.7/` | The official vectors, untrusted data, checksum in `model/authorities.toml` |
| `prism/` | The image model, the committed image (`IMAGE`, `registry/`), its engine loader and checks |

## Roots

| Root | Signature |
| --- | --- |
| `hash` | `Bytes -> Nat -> Bytes` (message, output length) |
| `hashHex` | `Bytes -> Bytes`, the 32-byte digest as lowercase hex |
| `keyedHash` | `Bytes -> Bytes -> Nat -> Result Bytes Blake3Error` (key, message, length) |
| `deriveKey` | `Bytes -> Bytes -> Nat -> Bytes` (context, key material, length) |
| `hashPieces` | `Mode -> List Bytes -> Nat -> Result Bytes Blake3Error`: start, one `update` per piece, `finalize` |
| `traceFile` | `Mode -> List Bytes -> Nat -> Nat -> Result Bytes Blake3Error`: the serialised trace at level 0, 1 or 2 |
| `traceAddress` | `Mode -> List Bytes -> Nat -> Result Bytes Blake3Error`: the root blob's DigestSet |
| `checkTrace` | `Bytes -> Result Bytes TraceError`, in the Prism image: the root's DigestSet, or the refusal |
| `sliceOf` | `Mode -> Bytes -> Nat -> Result Bytes Blake3Error`: the Bao slice of chunk N |
| `sliceCheck` | `Mode -> Bytes -> Nat -> Nat -> Bytes -> Result Bytes SliceError` (slice, chunk, input length, root hash): the chunk, or the refusal |
| `diffTrace`, `diffTraceBack`, `diffReach` | `Input -> Input -> …`, in the Prism image: per step, the xor of both runs' operands and results; the step each state word first differs |
| `taintRound`, `fullTaintSteps`, `wordDiffusion` | in the Prism image: round r's dependency map; the steps of full diffusion |

## Regenerate

```bash
LEXLEAN=~/lexlean-main/target/release/lexlean python3 -I tools/gen_blake3.py
```

Then, in `lexlean/`, `lexlean-trace/` and `lexlean-slice/`,
`LEXLEAN_VERIFY_WIDTH=1 lexlean production build --request
production-request.json`, and copy each bundle's packages and attestation to
`export/`; in `lexlean-reverse/` and `lexlean-analysis/`, `LEXLEAN_VERIFY_WIDTH=1
lexlean verify`. `cargo test -p blake3-check` runs the
tests; the trace checker's tests need `node` for the image's engine. Width 1
keeps one Lean process at a time; each build peaks near 3.3 GB.

## Repository boundary

[AGENTS.md](AGENTS.md), [TEMPLATE-CONTRACT.md](TEMPLATE-CONTRACT.md) and
[VERIFICATION.md](VERIFICATION.md) are the inherited template policy.

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
