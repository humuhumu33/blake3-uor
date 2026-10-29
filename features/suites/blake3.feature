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

  @B3-12 @build
  Scenario: vendor/lexlean-stdlib is LexLean main 6bcc8d598's stdlib byte for byte: its files are exactly lexlean.toml, src/Sha256.lex.tex and src/Text.lex.tex, the listing of their sha256 digests hashes to AUTH-LEXLEAN-STDLIB's checksum, and the trace project's lock pins that manifest.
    Given vendor/lexlean-stdlib and the AUTH-LEXLEAN-STDLIB row of model/authorities.toml
    When the suite exercises B3-12
    Then the file set, the listing digest and the lock's manifest pin agree, and a one-byte change is detected

  @B3-13 @build
  Scenario: One compression with a known message and CV runs back from its full 16-word output to exactly its initial state (CV, IV words, counter, block length, flags), by a kernel theorem for the empty input and in the image's engine for every root output block of seeded traces; with only 8 output words kept it does not.
    Given root output blocks of seeded traces, their checkpoints, and the same outputs truncated to 8 words
    When the suite exercises B3-13
    Then unmix returns each checkpoint's initial state, and not for the truncations

  @B3-14 @build
  Scenario: Each operation's inverse undoes it: subtraction for add and addMsg, the same xor for xor and xorCv, rotate-left by n for rotate-right by n, over edge values and a seeded corpus, in the image's engine.
    Given edge values and seeded words for each operation kind
    When the suite exercises B3-14
    Then stepRoundTrip holds for every pair

  @B3-15 @build
  Scenario: Stepping back by stepBack agrees with replaying forward from the checkpoint at every step of a compression; the cursor moves by bit, op, G, round and compression and stays at the ends; seek returns the step record the word-level trace holds, with its reversible or lossy tag.
    Given checkpoints of a two-chunk trace, cursor moves at boundaries, and sampled addresses
    When the suite exercises B3-15
    Then prevAgrees holds, each move lands where expected, and each seek equals the word-level record

  @B3-16 @build
  Scenario: From a label, the trace's links lead back through parents and chunks to the checkpoints that produced it, and forward to the blobs that consume it.
    Given a four-chunk tree trace
    When the suite exercises B3-16
    Then the root leads to chunk 0's first checkpoint and that checkpoint to its chunk and the events

  @B3-17 @build
  Scenario: A slice of chunk N (Bao's layout: the input length, the parent nodes on the chunk's path from the root, the chunk) made by the exported sliceOf equals the bao crate's and one built from the blake3 crate's hazmat API, in every mode; the exported sliceCheck accepts it, hashing only the chunk and its path, and Bao's decoder accepts it; a wrong sibling or on-path CV at any level, a changed chunk byte, a short or long chunk, a wrong length field, index, root or key is refused with its own error, and kernel theorems hold the official 1025-byte input's slice accepted and two planted defects refused.
    Given seeded inputs up to 31,745 bytes and one of 1 MiB + 1, in hash, keyed and derive-key modes
    When the suite exercises B3-17
    Then every slice equals both oracles' and checks, Bao accepts the model's, and each planted defect meets its error

  @B3-18 @build
  Scenario: For two compression inputs differing by one bit, the image's diffTrace (forwards) and diffTraceBack (rebuilt from the outputs) give the same xor of left operand, right operand and result at every step, equal to the certified word-level trace files' records; diffReach gives the step at which the difference reaches each state word; a one-bit input change alters exactly the compressions on its chunk's slice path from the changed block on.
    Given a keyed 64-byte root compression and each of its message, key, counter, length and flags bits flipped
    When the suite exercises B3-18
    Then forwards equals backwards equals the trace files, reach matches the records, and a tree trace differs only on the slice path

  @B3-19 @build
  Scenario: The image's dependency map (for each result bit of each step, the input bits with a structural path to it) equals a propagation written from the BLAKE3 specification; every observed difference of every message and key bit lies inside it; the first step of full diffusion (every bit of every state word on every input bit) is fullTaintSteps' maximum, and at word level a kernel theorem fixes it at step 163.
    Given the map of every round from taintRound, and the 768 observed difference traces
    When the suite exercises B3-19
    Then the map equals the specification's, holds every observed difference, and the diffusion steps agree
