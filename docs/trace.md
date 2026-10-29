# The BLAKE3 trace, format 1

The trace is the primary definition. `hash`, `keyedHash`, `deriveKey`,
`hashPieces` and the extended output are projections of it; nothing computes a
digest any other way. This document fixes the trace's content, its addresses
and its one canonical byte form. It is purpose-neutral: the same bytes serve as
an explainer, an audit receipt, a cost ledger or a content-address proof.

## How the trace is the computation

A compression is a step machine run over a fixed schedule: 7 rounds of 8 G
calls of 14 steps, then the 16 feed-forward xors, 800 steps. The schedule is
one table of rows (G, op index, kind, destination, source); `machineStep`
runs row k on a 16-word state and logs a `Step` with its address (round, G,
op index), kind, destination, source, both operands and its result. The
compression's output is read back from the feed-forward steps of that log
(`compressOut`), and `compress` is defined as exactly that. The hasher logs
every compression as a `Node` (its inputs, place in the tree and output) and
every CV stack push and pop as an `Event`; `finalTrace` returns the ordered
events, and the digest is the root output blocks of those events
(`digestOf`). Step kinds are the 2026-10-23 HashSmash ledger's: `add` (state
+ state), `addMsg` (state + message word), `xor`, `ror` (rotate right),
`xorCv` (feed-forward with a CV word).

## Levels

| Level | Unit | Stored or derived |
| --- | --- | --- |
| tree | compressions (as checkpoints), CV stack pushes and pops, parent merges, chunks, output blocks, the root | stored: one checkpoint per compression |
| word | every add, xor and rotate with operands, result, destination and source; the 64 bytes each block was loaded from and its length; each state word's provenance; each round's permutation map | derived from a checkpoint by replay |
| bit | every 32-bit add as 32 full-adder records (bit, carry in, sum, carry out); every xor as 32 bit records; a rotation is wiring and has no record | derived from a word step by replay |

A coarser level is a projection of a finer one: the word and bit files are the
tree file with a section after each checkpoint, and a section's last 16 words
are the compression's output.

## Addresses

ADR-015 (PrismPM `docs/adr/015-dual-digests.md`) applies unchanged. Every trace
object down to a compression is a content blob. Its label is
`blake3:<64 lowercase hex>`, BLAKE3 in default mode with 32-byte output over the
blob's exact bytes. Its DigestSet is `{"blake3": …, "sha256": …}` over the same
bytes, and a verifier requires and checks both. In binary form a DigestSet is
its 32 BLAKE3 bytes followed by its 32 SHA-256 bytes. The `blake3` axis is
this model's own hash; the `sha256` axis is LexLean's stdlib `Sha256`.

The stdlib is vendored byte for byte from LexLean main `6bcc8d598` under
`vendor/lexlean-stdlib/`. AGENTS.md R6 bars path substitutes for shipped
dependencies, and this is a path source, but it is the only form LexLean
accepts: a source package is read from a directory of the workspace, `kind`
`path` and nothing else (SPEC 10.5), so there is nothing to fetch or lock
remotely. The copy is held to upstream by B3-12: its files must be exactly
upstream's three, the listing of their sha256 digests must hash to
`AUTH-LEXLEAN-STDLIB`'s checksum `0990190f…` (reproduce upstream with `git show
6bcc8d598:stdlib/<path> | sha256sum`), and the trace lock must pin its manifest
(LexLean's own `lock --check` pins the tree, `tree_sha256` `c0be8a51…`).

A step finer than a compression is not a blob. Its address is a path inside its
compression's blob: `(checkpoint label, round, G index, op index, bit index)`.
Round 7 is the feed-forward; its G index is 0 and its op index is the output
word. Every blob is a pure function of (mode, key or context, input, outLen),
so identical sub-computations have one address.

The hashing that computes the labels is not itself traced. A label is the
model's `hash` of a blob's bytes, and `hash` is a projection of that blob's
own trace, but that trace is discarded and never addressed: the trace of an
input contains its own compressions, never the compressions of its labels.
That is why there is no loop.

## Integers and words

Integers are unsigned little-endian: `u8`, `u32`, `u64`. A word is a `u32`. A
label inside a blob is its 32 raw BLAKE3 bytes.

## Blobs

| Kind | Bytes |
| --- | --- |
| `01` checkpoint | `01`, role `u8` (0 chunk block, 1 parent, 2 root output), first chunk `u64`, chunk count `u64`, index `u64` (block within the chunk, or output block), input offset `u64`, initial state 16 words, message 16 words (162 bytes) |
| `02` chunk | `02`, chunk `u64`, n `u8`, n checkpoint labels, has-cv `u8`, then the chunk CV as 8 words when has-cv is 1 |
| `03` parent | `03`, first chunk `u64`, chunk count `u64`, left label, right label, has-cv `u8`, then the checkpoint label and the CV (8 words) when has-cv is 1 |
| `04` output | `04`, output block `u64`, checkpoint label, the 64 output bytes |
| `05` events | `05`, count `u64`, events: compression `00` + checkpoint label; push `01` or pop `02` + depth `u8` + first chunk `u64` + chunk count `u64` + CV 8 words |
| `06` root | `06`, format `01`, mode `u8` (0 hash, 1 keyed, 2 derive key, 3 derive-key context), input length `u64`, output length `u64`, input DigestSet, tree-node label, events label, context root label (mode 2 only), output block count `u64`, output labels, the digest |

The root of a tree whose final compression only runs with ROOT has no
checkpoint of its own: its chunk blob lists its non-final blocks and has-cv 0,
or its parent blob has has-cv 0, and the output blobs carry the root
compressions (counter = output block, flags with ROOT). The trace's address is
the root blob's label; the root blob records the input's DigestSet and the
digest beside it, so (mode, input, outLen) maps to one address.

## The serialised trace

One file per level, the same blobs in the same order, with nothing optional:

```
"B3TRACE1"  level u8 (0 tree, 1 word, 2 bit)  blob count u64
per blob, in execution order (root last):
  length u32, blob bytes, DigestSet (64 bytes)
  for a checkpoint at level 1 or 2, its word section
```

Execution order is the order the hasher runs: each compression's checkpoint as
it runs, a chunk blob after its last block, a parent blob after its merge, the
output blobs, the root's tree node, the events blob, then the root. A
derive-key trace starts with the whole context trace (mode 3), ending in its
root, which the key material's root names.

A word section is:

```
load:        offset u64, block length u8, the 64 block bytes as read (zero past the length)
provenance:  16 bytes, state word i from: 0-7 cv word i, 8-11 IV word i-8, 12 counter low, 13 counter high, 14 block length, 15 flags
rounds 0-6:  permutation map (16 bytes: message slot j holds block word map[j]), then 8 G calls x 14 steps
feed-forward: 16 steps
output:      16 words
step:        round u8, G u8, op u8 (index in the G or output word), kind u8, destination u8, source u8, left word, right word, result word
```

The destination is a state index (`v0`-`v15`), or an output word in the
feed-forward. The source is a state index, a message slot (addMsg), the
rotation amount (ror), or a CV index (xorCv). At level 2 each add and addMsg
step is followed by 32 adder bytes, bit 0 first, each `a | b<<1 | carry-in<<2
| sum<<3 | carry-out<<4`; each xor and xorCv step by 32 bytes `a | b<<1 |
out<<2`. A ror step has no bit records. A word section is 14,665 bytes; a bit
section is 33,097.

## Checking

`checkTrace` reads a serialised trace at any level and, from the finest level
present, replays it: every blob against its DigestSet (both keys), every label
a blob names against the blobs before it, every parent's message against its
children's CVs, every chunk and parent CV and every output block against a
replay of its checkpoint, every checkpoint's IV words, load bytes, zero fill
and provenance, every permutation map against BLAKE3's, every step's address,
kind, wiring, operands and result against the step machine, every adder and
xor bit, the outputs, and the digest against the output blobs. It returns the
root's DigestSet. Given the mode, input and output length, the trace is that
input's trace when that DigestSet equals `traceAddress`. Each failure is a
typed `TraceError` naming the offset where it was found.

## Where each part runs

| Root | Certified rust-std package | Runs on |
| --- | --- | --- |
| `hash`, `keyedHash`, `deriveKey`, `hashPieces`, `hashHex` | yes, from the core project | Rust (`export/rust-std/blake3_*`), and the image's engine |
| `traceFile`, `traceAddress` | yes, from the trace project | Rust, and the image's engine |
| `checkTrace` | **no** | the LexLean calculus engine in the Prism image (`prism/IMAGE`) only |
| `unmix`, `stepRoundTrip`, `prevAgrees`, `cursorMove`, `seekStep` (phase 1) | **no** | the Prism image's engine only; the module and its kernel theorem are verified in `lexlean-reverse/` |

`checkTrace` and the phase-1 roots are verified LexLean definitions: Lean checks its module and the
engine runs its lowered program, the same program format and engine as every
other image root (whose program ids equal their Rust packages'). They are
not certified Rust, and nothing here should be read as saying they are. The
phase-1 roots were tried as rust-std exports too: `stepRoundTrip`, whose
program is a few word operations and whose certificate source is 200 KB, ran
out of 6 GB like `checkTrace` ([lexlean-issue-draft.md](lexlean-issue-draft.md)).

`checkTrace` has no rust-std export. Its preservation certificate does not
compile within 11 GB whenever the checker computes a `Nat` from the file's
bytes, by any of four conversions tried (a 32-step loop doubling a `Nat`, the
sum written out, a byte table in base 256, a search over `wordOfNat`); with the
conversion replaced by a constant it compiles at 3.5 GB, and `parse_decimal`
refuses a `Nat` target (LLT4001). A time-boxed attempt to read no `Nat` from
the file (positions from counters, file values compared as words) failed the
same way at 6 GB, and a tiny module reading a `Nat` from bytes certifies at
1.4 GB, so the trigger is that conversion inside a closure as large as the
traced BLAKE3 plus `Sha256`. The measurements and a reproduction are in
[lexlean-issue-draft.md](lexlean-issue-draft.md), unfiled. Two other certificate blowups were found and
fixed on the way, each recorded where it applies: a 256-leaf lookup called
inside a loop closure (`bitByte` replaces it), and the hash, trace and check
roots in one project (two projects, each with its own roots).

## What is proved and what is tested

| Claim | How |
| --- | --- |
| `compress` = `compressOut (compressTrace …)` for every input | kernel theorem by unfolding, all variables free |
| a node's output = `compressOut` of its word trace, for every pending node | kernel theorem by unfolding |
| `hash`, `deriveKey`, `hashPieces` = `digestOf` of their trace, for every input | kernel theorems by unfolding |
| traces of tiny inputs (at most two compressions) give the official digests | 11 kernel theorems by reflexivity |
| the trace's digests equal the official vectors and the `blake3` crate | tests B3-01 to B3-04, B3-06 |
| every DigestSet equals independent BLAKE3 and SHA-256 | test B3-07 (`blake3`, `sha2` crates); `prism/trace-checks.mjs --blake3` (PrismPM `blake3.mjs`, node SHA-256) |
| `checkTrace` accepts real traces and rejects each planted defect | tests B3-08, B3-09, in the engine |
| a trace is deterministic and the same however the input is split | test B3-10 |
| identical sub-computations share one address | test B3-11 |
| an add's 32 adder records assemble to the word result, for all operands | replayed by `checkTrace` for every add it reads, and tested; not proved, since LexLean's proof forms have no bit-vector decision |

## Running backwards, the cursor and navigation

**Inverses.** Every step has an inverse given its second operand: subtraction
modulo 2^32 for add and addMsg, the same xor for xor and xorCv, rotate-left by
n for rotate-right by n (`undo`). No schedule row writes its own source, so a
step's second operand is the same before and after it, and `stepBack` turns
the state after step k into the state before it. The feed-forward is undone
by `unfeed`: the upper half of the state after round 6 is out[i+8] ^ cv[i],
the lower half out[i] ^ that. `compressBack (out, cv, m)` runs one compression
back from its 16 output words, CV and message to its initial state: the CV,
the IV words, the counter, the block length and the flags. Going backwards is
inversion with the message and the CV known. Finding an unknown input from a
digest is infeasible by design (preimage resistance); nothing here attempts
it.

**Reversible and lossy steps** (`stepTag`): every round step is reversible
given the message (an addMsg step is lossy when the message is unknown); a
feed-forward step is reversible only when all 16 output words are kept, which
holds for root output blocks and not for a chaining value or the default
32-byte digest, which keep 8.

**Cursor.** An address is (compression in execution order, step 0-799, bit
0-31). `cursorMove` goes to the next or previous bit, op, G (14 steps), round
(112 steps; the feed-forward, steps 784-799, is one unit), or compression,
staying put at either end. `seekStep` returns the step at an address, replayed
from its compression's checkpoint, as its 18-byte record and its tag.
`prevAgrees` checks, for one compression, that stepping back with `stepBack`
gives at every step the state that replaying forward from the checkpoint
gives. Only checkpoints are stored: the tree-level file, 5.5 MB for 1 MiB
against 261 MB at word level and 582 MB at bit level
(docs/blake3-verification.md).

**Navigation.** Every blob names its inputs by label, so from any label the
trace leads back (a root to its tree node, a parent to its children, a chunk
to its block checkpoints, an output block to its root checkpoint) and forward
(to every blob naming it). This is done over the file by the independent
reader (`crates/blake3-check/src/trace.rs`: `producer`, `consumers`); finer
addresses inside a checkpoint are `seekStep` paths. Identical sub-computations
share one label (B3-11).

| Claim | How |
| --- | --- |
| one compression runs back exactly to its initial state (empty input, root flags) | kernel theorem by reflexivity, `lexlean-reverse/` |
| every root output block of seeded traces runs back to its checkpoint; 8 kept words do not | test B3-13, in the engine |
| `undo` inverts each operation, for all operands | tested over edge values and a seeded corpus (B3-14); not proved: LexLean's proof forms have no bit-vector decision, and `rfl` over free words does not reduce |
| stepping back agrees with replaying forward at every step | test B3-15, in the engine |
| the cursor's moves and `seekStep`'s records | test B3-15 against the word-level file |
| navigation by label | test B3-16 |
