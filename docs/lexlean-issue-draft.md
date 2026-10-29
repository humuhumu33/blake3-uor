# Draft LexLean issue: rust-std preservation certificate exceeds 11 GB

Not filed. LexLean has an owner moratorium on new issues until the v0.4 tag,
and filing needs Ilya's yes. Everything below was measured on 2026-10-29 with
LexLean main `6bcc8d598` (`lexlean` 0.3.0), Lean `v4.32.1`, WSL2 x86_64,
`LEXLEAN_VERIFY_WIDTH=1`, each run in a systemd scope with the stated
`MemoryMax`.

## Summary

`lexlean production build` cannot certify a `rust-std` root whose closure both
contains a large computation (here a traced BLAKE3 and the stdlib `Sha256`)
and computes a `Nat` from the bytes of its input. The Lean process compiling
the root's certificate grows past 11 GB. The same root with that `Nat`
replaced by a constant certifies at 3.5 GB; roots with the same large closure
but no `Nat` read from data certify at 1.9 GB; a small module that reads a
`Nat` from bytes certifies at 1.4 GB.

## Measurements

| Root (in `lexlean-trace` of humuhumu33/blake3-uor, `checkTrace`) | Conversion from file bytes to `Nat` | Cap | Outcome |
| --- | --- | --- | --- |
| full checker | 32-step `iterate` doubling a `Nat` | 6 GB, 8 GB, 11 GB (certificate alone, by hand) | killed (OOM) each time; the `lean` process had 7.2 GB resident at the 8 GB kill |
| full checker | the 32 bit weights summed, written out | 6 GB | killed |
| full checker | a 256-leaf table byte → `Nat` literal, combined in base 256 | 6 GB | killed |
| near-empty checker (header, per-entry DigestSet check only) | 32-step doubling loop | 6 GB | killed |
| near-empty checker | byte table | 6 GB | killed |
| near-empty checker | search: the `k` whose `wordOfNat k 8` equals the byte | 6 GB | killed |
| near-empty checker | none: the conversion replaced by the constant `0` | 6 GB | exported, 3.5 GB peak |
| `traceFile`, `traceAddress` (same traced BLAKE3 and `Sha256` closure, no `Nat` from data) | none | 6 GB | exported, 1.9 GB peak |
| tiny module: `iterate_until` over `Bytes` whose step adds `natOfWord` of a word read at the position | 32-step doubling loop | 6 GB | exported, 1.4 GB peak |
| `stepRoundTrip (kind : Nat) (left right : UInt32) : Bool` in `lexlean-reverse`: `undo op (perform op left right) right == left`, a few word operations (certificate source 200 KB) | none | 6 GB | killed while compiling its certificate |
| `diffTrace`, `diffTraceBack`, `diffReach`, `taintRound` in `lexlean-analysis` (imports the core and `lexlean-reverse`; no `Nat` from data) | none | 6 GB | killed (OOM) after 3 min 41 s |
| `sliceOf`, `sliceCheck` in `lexlean-slice` (imports the core only; the input length is a parameter, compared word by word with the slice's field) | none | 6 GB | exported, 1.8 GB peak, 283 s |

`parse_decimal` was tried as a fifth route (`format_decimal` of the word, then
`parse_decimal` to `Option Nat`); linking refuses it: `LLT4001: primitive
ParseDecimal target must be an integer`. `checked_convert` likewise takes no
`Nat` (`LLT4001: primitive CheckedConvert requires an integer input`).

The kill happens while `lean` compiles the root's `a` certificate
(`.lexlean/verified/.staging-*/preserve-src/LexLeanPreserve/C…/R*.lean`, 5.4
to 7.6 MB of source); the shared `LexLeanPreservation` libraries compile
first and are not the problem.

## Reproduce

```bash
git clone https://github.com/humuhumu33/blake3-uor && cd blake3-uor   # once the branch is public
git checkout feat/blake3-lexlean
# the near-empty checker: every blob case stubbed, only the DigestSet check left
for c in checkpointCase chunkCase parentCase outputCase eventsCase rootCase; do
  sed -i "s#($c data ck pos len next blob label kind ds)#(ctor Result.ok [Check, TraceError] ck)#" lexlean-trace/src/Blake3Trace.lex.tex
done
# export checkTrace from the trace project
python3 - <<'PY'
import json, re, pathlib
p = pathlib.Path("lexlean-trace/src/Blake3Trace.lex.tex"); t = p.read_text()
t = re.sub(r"(\n  name: checkTrace,(?:.|\n)*?  executable: true)\n\}", r"\1,\n  production: {effects: [allocation, overflow], targets: [rust-std]}\n}", t, count=1)
p.write_text(t)
r = json.load(open("lexlean-trace/production-request.json"))
r["roots"].append({"root": "Blake3UorTrace.Blake3Trace.checkTrace", "package": {"name": "blake3_check_trace", "version": "0.1.0"}, "targets": r["roots"][0]["targets"]})
r["roots"].sort(key=lambda x: x["root"])
open("lexlean-trace/production-request.json", "w").write(json.dumps(r, separators=(",", ":")) + "\n")
PY
(cd lexlean && lexlean lock) && cd lexlean-trace && lexlean lock
LEXLEAN_VERIFY_WIDTH=1 systemd-run --user --scope -p MemoryMax=6G -p MemorySwapMax=0 \
  lexlean production build --request production-request.json      # killed
# then replace natOfWord's body with 0 (and its axioms with none): exported at 3.5 GB
```

A second case without any `Nat` from data: in a project whose other roots
include the traced hash (`unmix`, `prevAgrees`, `seekStep`), the certificate of
the tiny `stepRoundTrip` root ran out of 6 GB. Every root that failed without a
`Nat` from data has `lexlean-reverse`'s definitions in its closure (`undo`,
`operand`, `rowFor`, `roundOf`); roots over the same traced hash without them
(`traceFile`, `traceAddress`, `sliceOf`, `sliceCheck`) certify at 1.8 to 1.9 GB.

## Question for the owner

Is the certificate's size or proof search expected to grow with data-dependent
`Nat` values in a large closure, and is there a supported way to read a
length or count from `Bytes` as a `Nat` in a certified `rust-std` root?
