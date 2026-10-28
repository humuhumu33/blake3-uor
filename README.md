# blake3-uor

BLAKE3 written in LexLean 1.2 and exported by LexLean as certified Rust: all
seven rounds, inputs of any length through the chunk tree, the 64-bit chunk
counter, `hash`, `keyed_hash` and `derive_key`, output of any length, and an
incremental hasher as pure functions. No hashing operation is handwritten Rust.

## Status

The export reproduces every official BLAKE3 vector and agrees with the
`blake3` crate on a seeded boundary corpus; eleven small cases are also Lean
kernel theorems. Evidence, the planted defect and throughput measurements are
in [docs/blake3-verification.md](docs/blake3-verification.md).

This repository is not yet bound to a PrismPM SDK image: `prismpm.lock` and
`template.lock` are absent, so `just vv` and the bootstrap workflow stay red
until a public SDK release exists to bind. Nothing here substitutes for it.

## Layout

| Path | What it is |
| --- | --- |
| `tools/gen_blake3.py` | Generates every LexLean module; the modules are its `lexlean fmt` output |
| `lexlean/src/Blake3.lex.tex` | The model: compression, chunk tree, modes, extended output, incremental hasher, roots |
| `lexlean/src/Blake3Kernel*.lex.tex` | One reflexivity theorem each, against the official vectors |
| `lexlean/production-request.json` | The five roots LexLean exports to `rust-std` |
| `export/` | The certified packages and evidence `lexlean production build` wrote |
| `crates/blake3-check/` | The differential: official vectors, the `blake3` crate, splits, key length, throughput |
| `oracles/blake3-1.8.7/` | The official vectors, untrusted data, checksum in `model/authorities.toml` |
| `prism/run-vectors.mjs` | Runs a local Prism image of the model on every official vector |

## Roots

| Root | Signature |
| --- | --- |
| `hash` | `Bytes -> Nat -> Bytes` (message, output length) |
| `hashHex` | `Bytes -> Bytes`, the 32-byte digest as lowercase hex |
| `keyedHash` | `Bytes -> Bytes -> Nat -> Result Bytes Blake3Error` (key, message, length) |
| `deriveKey` | `Bytes -> Bytes -> Nat -> Bytes` (context, key material, length) |
| `hashPieces` | `Mode -> List Bytes -> Nat -> Result Bytes Blake3Error`: start, one `update` per piece, `finalize` |

## Regenerate

```bash
LEXLEAN=~/lexlean-main/target/release/lexlean python3 -I tools/gen_blake3.py
```

Then, in `lexlean/`, `LEXLEAN_VERIFY_WIDTH=1 lexlean production build --request
production-request.json` and copy its bundle to `export/`; `cargo test -p
blake3-check` runs the differential. Width 1 keeps one Lean process at a time;
each theorem module peaks near 3 GB.

## Repository boundary

[AGENTS.md](AGENTS.md), [TEMPLATE-CONTRACT.md](TEMPLATE-CONTRACT.md) and
[VERIFICATION.md](VERIFICATION.md) are the inherited template policy.

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE).
