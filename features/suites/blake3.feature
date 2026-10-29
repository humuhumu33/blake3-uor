Feature: blake3

  BLAKE3 authored in LexLean and exported by LexLean as certified Rust packages.

  @B3-01 @build
  Scenario: The exported packages reproduce all 35 cases of the official BLAKE3 1.8.7 test vectors in hash, keyed_hash and derive_key modes, 131 bytes of extended output each, and the 32-byte digest as lowercase hex.
    Given oracles/blake3-1.8.7/test_vectors.json, parsed against its closed shape
    When the suite exercises B3-01
    Then every mode's 131-byte output and the hex digest equal the file's for every input length

  @B3-02 @build
  Scenario: The exported packages equal the blake3 crate 1.8.7 in all three modes on a seeded corpus whose lengths cover every block, chunk and tree boundary from 0 to 102400 bytes, at output lengths from 0 to 1031 bytes.
    Given a seeded corpus at every boundary length, with a seeded key and context
    When the suite exercises B3-02
    Then each mode's output at each output length equals the blake3 crate's

  @B3-03 @build
  Scenario: Incremental hashing (start, one update per piece, finalize) equals one-shot hashing for every tested split of every corpus input, in all three modes.
    Given empty, single-byte, chunk-aligned, block-aligned and seeded random splits of each corpus input
    When the suite exercises B3-03
    Then hashPieces over the pieces equals the one-shot digest in every mode

  @B3-04 @build
  Scenario: A keyed-hash key that is not 32 bytes is refused as Blake3Error.keyLength carrying its length, by keyedHash and by hashPieces.
    Given keys of 0, 1, 16, 31, 33 and 64 bytes
    When the suite exercises B3-04
    Then both roots return keyLength with that length, and a 32-byte key is accepted

  @B3-05 @build
  Scenario: The committed packages are the ones verified LexLean builds exported: each package file hashes to its provenance record, each package names the semantic identity its project's attestation verified, and the core attestation records all sixteen kernel theorems with exactly the axioms Quot.sound and propext.
    Given export/rust-std/ and export/attestation.json as lexlean production build wrote them
    When the suite exercises B3-05
    Then every file digest, the semantic identity and every theorem's axioms agree, and one changed byte is detected

  @B3-06 @build
  Scenario: The trace the certified traceFile writes records, in its root blob, the digest BLAKE3 computes and the input's ADR-015 DigestSet, and the trace's address is that root blob's DigestSet, in all three modes.
    Given seeded inputs from 0 to 4097 bytes in hash, keyed_hash and derive_key
    When the suite exercises B3-06
    Then the root digest equals the blake3 crate's, the input DigestSet equals the crates', and traceAddress equals the root's DigestSet

  @B3-07 @build
  Scenario: Every DigestSet in a trace, at every level, equals BLAKE3 and SHA-256 of its blob as the blake3 and sha2 crates compute them, and every label a blob names belongs to a blob before it.
    Given tree, word and bit traces of seeded inputs up to 16384 bytes
    When the suite exercises B3-07
    Then each recorded DigestSet equals the blake3 and sha2 crates' over its blob, and each named label precedes its use

  @B3-08 @build
  Scenario: checkTrace, run by the LexLean calculus engine in the committed Prism image, accepts every real trace at the tree, word and bit levels and returns the address traceAddress computes.
    Given real traces from traceFile at each level, in each mode
    When the suite exercises B3-08
    Then the engine's checkTrace returns each trace's address

  @B3-09 @build
  Scenario: checkTrace rejects a flipped operation result, a flipped carry bit, a wrong permutation index, a reordered step, a wrong flag with its label kept or relabelled, a changed checkpoint with its label kept, and a forged parent link, each as the TraceError that names it; another input's trace is not this input's address.
    Given real traces with one planted defect each
    When the suite exercises B3-09
    Then the engine's checkTrace refuses each with the TraceError for that defect at its offset

  @B3-10 @build
  Scenario: The same mode, input and output length give byte-identical traces, however the input is split into updates.
    Given one input traced whole and split into four updates
    When the suite exercises B3-10
    Then the traces are byte for byte the same

  @B3-11 @build
  Scenario: Identical sub-computations have one address: the first chunk of two different inputs resolves to the same checkpoint and chunk labels.
    Given two inputs sharing their first 1024 bytes
    When the suite exercises B3-11
    Then their first chunk's checkpoint and chunk labels are the same
