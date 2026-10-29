# blake3-uor

BLAKE3 written in LexLean 1.2, trace first: every step BLAKE3 performs is a
recorded value of the model, and every digest is a projection of that trace.
All seven rounds, inputs of any length through the chunk tree, the 64-bit
chunk counter, `hash`, `keyed_hash` and `derive_key`, output of any length,
and an incremental hasher as pure functions. The trace serialises at the tree,
word and bit levels as ADR-015 content blobs (`blake3:` labels and
`{sha256, blake3}` DigestSets), and `checkTrace` replays one from its finest
level. No hashing operation is handwritten Rust.

## Status

The export reproduces every official BLAKE3 vector and agrees with the
`blake3` crate on a seeded boundary corpus. Sixteen kernel theorems hold: the
digest and every compression output are projections of the trace for every
input, and eleven small cases give the official digests. Every trace label
matches the `blake3` and `sha2` crates and PrismPM's `blake3.mjs`, and the
checker rejects each planted defect. The format is in [docs/trace.md](docs/trace.md);
evidence, sizes and throughput are in [docs/blake3-verification.md](docs/blake3-verification.md).

This repository is not yet bound to a PrismPM SDK image: `prismpm.lock` and
`template.lock` are absent, so `just vv` and the bootstrap workflow stay red
until a public SDK release exists to bind. Nothing here substitutes for it.

## Layout

| Path | What it is |
| --- | --- |
| `tools/gen_blake3.py` | Generates every LexLean module and the image model; the modules are its `lexlean fmt` output |
| `lexlean/src/Blake3.lex.tex` | The core: the step machine and its word trace, the tree trace, modes, extended output, incremental hasher, hash roots |
| `lexlean/src/Blake3Kernel*.lex.tex` | One kernel theorem each: projections for every input, official vectors for tiny ones |
| `lexlean-trace/src/Blake3Trace.lex.tex` | Blobs, DigestSets, the tree/word/bit files, `traceFile`, `traceAddress`, `checkTrace` |
| `vendor/lexlean-stdlib/` | LexLean's stdlib (Sha256), byte for byte from LexLean main `6bcc8d598` |
| `export/` | The certified packages and attestations both production builds wrote |
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

## Regenerate

```bash
LEXLEAN=~/lexlean-main/target/release/lexlean python3 -I tools/gen_blake3.py
```

Then, in `lexlean/` and in `lexlean-trace/`, `LEXLEAN_VERIFY_WIDTH=1 lexlean
production build --request production-request.json`, and copy each bundle's
packages and attestation to `export/`. `cargo test -p blake3-check` runs the
tests; the trace checker's tests need `node` for the image's engine. Width 1
keeps one Lean process at a time; each build peaks near 3.3 GB.

## Repository boundary

[AGENTS.md](AGENTS.md), [TEMPLATE-CONTRACT.md](TEMPLATE-CONTRACT.md) and
[VERIFICATION.md](VERIFICATION.md) are the inherited template policy.

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
