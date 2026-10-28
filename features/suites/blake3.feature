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
  Scenario: The committed packages are the ones a verified LexLean build exported: each package file hashes to its provenance record, each package names the attested semantic identity, and the attestation records all eleven kernel theorems with exactly the axioms Quot.sound and propext.
    Given export/rust-std/ and export/attestation.json as lexlean production build wrote them
    When the suite exercises B3-05
    Then every file digest, the semantic identity and every theorem's axioms agree, and one changed byte is detected
