#!/usr/bin/env python3
"""Generate lexlean/src/Blake3.lex.tex, the BLAKE3 model.

BLAKE3 is mostly regular structure: eight G applications, sixteen-word
records, a 256-leaf byte table. Writing that by hand invites a typo that
type-checks, so it is generated here and the generator is reviewed instead.

The official vectors (oracles/blake3-1.8.7/test_vectors.json) supply the
expected words of the reflexivity theorems. The file is untrusted data: it is
parsed against a closed shape and nothing in it is executed.

    LEXLEAN=<lexlean> python3 -I tools/gen_blake3.py            # rewrite the module
    LEXLEAN=<lexlean> python3 -I tools/gen_blake3.py --check    # fail if it is stale

The layout is whatever `lexlean fmt` makes of the rendered text, so the
committed module is canonical and the generator never imitates the formatter.
"""

import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCES = ROOT / "lexlean" / "src"
MODULE = SOURCES / "Blake3.lex.tex"
VECTORS = ROOT / "oracles" / "blake3-1.8.7" / "test_vectors.json"

IV = [0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
      0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19]
PERMUTATION = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8]
# (a, b, c, d) state indices of the eight G applications of one round; G i
# reads message words 2i and 2i+1.
G_LANES = [(0, 4, 8, 12), (1, 5, 9, 13), (2, 6, 10, 14), (3, 7, 11, 15),
           (0, 5, 10, 15), (1, 6, 11, 12), (2, 7, 8, 13), (3, 4, 9, 14)]
FLAGS = [("flagChunkStart", 1), ("flagChunkEnd", 2), ("flagParent", 4),
         ("flagRoot", 8), ("flagKeyedHash", 16), ("flagDeriveKeyContext", 32),
         ("flagDeriveKeyMaterial", 64)]
ROUNDS = 7
# Every rotation of G, as (amount, 32 - amount). A planted defect edits this.
ROTATIONS = [(16, 16), (12, 20), (8, 24), (7, 25)]

AX = "  axioms: [Quot.sound, propext],\n"
PRODUCTION = "  production: {effects: [allocation, overflow], targets: [rust-std]}\n"


def u32(value):
    return f"({value} : UInt32)"


def structure(name, fields):
    rows = ",\n".join(f"    {{name: {f}, type: {t}}}" for f, t in fields)
    return (f"structure{{\n  name: {name},\n  type_parameters: [],\n  parameters: [],\n"
            f"  fields: [\n{rows}\n  ]\n}}\n")


def inductive(name, constructors):
    rows = ",\n".join(f"    {{name: {c}, fields: [{', '.join(f)}]}}" for c, f in constructors)
    return (f"inductive{{\n  name: {name},\n  type_parameters: [],\n  parameters: [],\n"
            f"  constructors: [\n{rows}\n  ]\n}}\n")


def definition(name, parameters, result, body, axioms=True, production=False):
    text = (f"definition{{\n  name: {name},\n  parameters: [{', '.join(parameters)}],\n"
            f"  result: {result},\n  body: {body},\n")
    if axioms:
        text += AX
    text += "  executable: true"
    if production:
        text += ",\n" + PRODUCTION.rstrip("\n")
    return text + "\n}\n"


def theorem(name, statement, axioms=True):
    text = (f"theorem{{\n  name: {name},\n  parameters: [],\n  statement: {statement},\n"
            "  proof: reflexivity{}")
    if axioms:
        text += ",\n" + AX.rstrip("\n")
    return text + "\n}\n"


def record(name, values):
    return f"(record {name} {{{', '.join(f'{k} := {v}' for k, v in values)}}})"


def words16(values):
    return record("Words16", [(f"v{i}", u32(v)) for i, v in enumerate(values)])


def cv_literal(values):
    return record("Cv", [(f"h{i}", u32(v)) for i, v in enumerate(values)])


def bits_tree(variable, width, leaf):
    """A balanced test of `variable`'s low `width` bits, most significant first.

    Rendered programs are read under a nesting bound (SPEC 17.16), so a
    lookup is a binary tree of depth `width`, never a linear chain.
    """
    def node(bit, prefix):
        if bit < 0:
            return leaf(prefix)
        mask = 1 << bit
        test = f"(prim equal (prim bit_and {variable} {u32(mask)} : UInt32) {u32(0)} : Bool)"
        return f"(if {test} then {node(bit - 1, prefix)} else {node(bit - 1, prefix | mask)})"
    return node(width - 1, 0)


def g_function(index):
    a, b, c, d = G_LANES[index]
    q = (f"(gcore (project s v{a}) (project s v{b}) (project s v{c}) (project s v{d}) "
         f"(project m v{2 * index}) (project m v{2 * index + 1}))")
    slots = {a: "qa", b: "qb", c: "qc", d: "qd"}
    fields = [(f"v{i}", f"(project q {slots[i]})" if i in slots else f"(project s v{i})")
              for i in range(16)]
    return definition(f"g{index}", ["s : Words16", "m : Words16"], "Words16",
                      f"(let q : Quad := {q}; {record('Words16', fields)})")


def words_le(data):
    return [int.from_bytes(data[i:i + 4], "little") for i in range(0, len(data), 4)]


def load_vectors():
    raw = json.loads(VECTORS.read_text(encoding="utf-8"))
    if set(raw) != {"_comment", "key", "context_string", "cases"}:
        raise SystemExit(f"{VECTORS}: unexpected top-level keys {sorted(raw)}")
    key, context = raw["key"], raw["context_string"]
    if key != "whats the Elvish word for friend":
        raise SystemExit(f"{VECTORS}: unexpected key {key!r}")
    if context != "BLAKE3 2019-12-27 16:29:52 test vectors context":
        raise SystemExit(f"{VECTORS}: unexpected context {context!r}")
    cases = {}
    for case in raw["cases"]:
        if set(case) != {"input_len", "hash", "keyed_hash", "derive_key"}:
            raise SystemExit(f"{VECTORS}: unexpected case keys {sorted(case)}")
        length = case["input_len"]
        if not isinstance(length, int) or length < 0 or length in cases:
            raise SystemExit(f"{VECTORS}: bad input_len {length!r}")
        outputs = {}
        for mode in ("hash", "keyed_hash", "derive_key"):
            text = case[mode]
            if not re.fullmatch(r"[0-9a-f]{262}", text):
                raise SystemExit(f"{VECTORS}: input_len {length} {mode} is not 131 hex bytes")
            outputs[mode] = bytes.fromhex(text)
        cases[length] = outputs
    return key.encode(), context.encode(), cases


def message(length):
    return bytes(i % 251 for i in range(length))


def literal(data):
    return "0x" + data.hex()


def render():
    """Every module of the project, by file name: the model and its theorems."""
    key, context, cases = load_vectors()
    out = []
    emit = out.append
    emit("\\begin{lexlean}{Blake3}\n\\useglossary{lexlean.std.nat@1.2.0}\n"
         # The title must be a phrase of the declared glossary; lexlean.std.nat
         # has no hashing phrase, so this is the one stdlib Sha256 uses.
         "\\title{Natural number addition}\n\\begin{semanticmodule}\n")
    decls = []
    add = decls.append

    add(structure("Cv", [(f"h{i}", "UInt32") for i in range(8)]))
    add(structure("Words16", [(f"v{i}", "UInt32") for i in range(16)]))
    add(structure("Quad", [(f, "UInt32") for f in ("qa", "qb", "qc", "qd")]))
    # The reference implementation's `Output`: a compression waiting for its
    # flags, so the same node can yield a chaining value or root output blocks.
    add(structure("Output", [("chain", "Cv"), ("block", "Words16"), ("counter", "Nat"),
                             ("blockLen", "UInt32"), ("flags", "UInt32")]))
    # The incremental hasher. `pending` holds the bytes of the chunk not yet
    # committed (a full chunk stays pending until more input proves it is not
    # the last); `stack` holds subtree chaining values, newest first.
    add(structure("Hasher", [("keyWords", "Cv"), ("modeFlags", "UInt32"), ("chunkCount", "Nat"),
                             ("pending", "Bytes"), ("stack", "(List Cv)")]))
    add(inductive("Mode", [("hashMode", []), ("keyedMode", ["Bytes"]), ("deriveMode", ["Bytes"])]))
    add(inductive("Blake3Error", [("keyLength", ["Nat"])]))

    add(definition("widen", ["value : UInt32"], "UInt64",
                   "(match (prim checked_convert value : (Option UInt64))\n"
                   "    | Option.some wide => wide\n    | Option.none => (0 : UInt64))"))
    add(definition("narrow", ["value : UInt64"], "UInt32",
                   "(match (prim checked_convert (prim bit_and value (4294967295 : UInt64) : UInt64) : (Option UInt32))\n"
                   "    | Option.some low => low\n    | Option.none => (0 : UInt32))"))
    add(definition("add", ["left : UInt32", "right : UInt32"], "UInt32",
                   "(match (prim checked_add (widen left) (widen right) : (Option UInt64))\n"
                   "    | Option.some sum => (narrow sum)\n    | Option.none => (0 : UInt32))"))
    add(definition("shiftRight", ["value : UInt32", "amount : UInt32"], "UInt32",
                   "(match (prim shift_right value amount : (Option UInt32))\n"
                   "    | Option.some shifted => shifted\n    | Option.none => (0 : UInt32))"))
    add(definition("shiftLeft", ["value : UInt32", "amount : UInt32"], "UInt32",
                   "(match (prim shift_left value amount : (Option UInt32))\n"
                   "    | Option.some shifted => shifted\n    | Option.none => (0 : UInt32))"))
    add(definition("rotateRight", ["value : UInt32", "amount : UInt32", "complement : UInt32"], "UInt32",
                   "(prim bit_or (shiftRight value amount) (shiftLeft value complement) : UInt32)"))
    add(definition("xor", ["left : UInt32", "right : UInt32"], "UInt32",
                   "(prim bit_xor left right : UInt32)"))

    (r1, c1), (r2, c2), (r3, c3), (r4, c4) = ROTATIONS
    add(definition("gcore", [f"{v} : UInt32" for v in "abcdxy"], "Quad",
                   "(let a1 : UInt32 := (add (add a b) x); "
                   f"(let d1 : UInt32 := (rotateRight (xor d a1) {u32(r1)} {u32(c1)}); "
                   "(let c1 : UInt32 := (add c d1); "
                   f"(let b1 : UInt32 := (rotateRight (xor b c1) {u32(r2)} {u32(c2)}); "
                   "(let a2 : UInt32 := (add (add a1 b1) y); "
                   f"(let d2 : UInt32 := (rotateRight (xor d1 a2) {u32(r3)} {u32(c3)}); "
                   "(let c2 : UInt32 := (add c1 d2); "
                   f"(let b2 : UInt32 := (rotateRight (xor b1 c2) {u32(r4)} {u32(c4)}); "
                   "(record Quad {qa := a2, qb := b2, qc := c2, qd := d2})))))))))"))
    for i in range(8):
        add(g_function(i))
    body = "s"
    for i in range(8):
        body = f"(g{i} {body} m)"
    add(definition("round", ["s : Words16", "m : Words16"], "Words16", body))
    add(definition("permute", ["m : Words16"], "Words16",
                   record("Words16", [(f"v{i}", f"(project m v{p})") for i, p in enumerate(PERMUTATION)]),
                   axioms=False))
    add(definition("iv", [], "Cv", cv_literal(IV), axioms=False))
    for name, value in FLAGS:
        add(definition(name, [], "UInt32", u32(value), axioms=False))

    start = record("Words16", [(f"v{i}", f"(project cv h{i})") for i in range(8)]
                   + [(f"v{8 + i}", u32(IV[i])) for i in range(4)]
                   + [("v12", "counterLo"), ("v13", "counterHi"), ("v14", "blockLen"), ("v15", "flags")])
    finish = record("Words16", [(f"v{i}", f"(xor (project s v{i}) (project s v{i + 8}))") for i in range(8)]
                    + [(f"v{8 + i}", f"(xor (project s v{8 + i}) (project cv h{i}))") for i in range(8)])
    # Seven rounds with the message permuted between them; the permutation
    # after the seventh round is computed and discarded.
    add(definition("compress", ["cv : Cv", "m : Words16", "counterLo : UInt32", "counterHi : UInt32",
                                "blockLen : UInt32", "flags : UInt32"], "Words16",
                   f"(let s : Words16 := (first (prim iterate (fun (p : (Words16 * Words16)) => "
                   f"((round (first p) (second p)), (permute (second p)))) {ROUNDS} ({start}, m) "
                   f": (Words16 * Words16))); {finish})"))
    add(definition("cvOf", ["w : Words16"], "Cv",
                   record("Cv", [(f"h{i}", f"(project w v{i})") for i in range(8)]), axioms=False))
    # checked_convert takes no Nat, so a Nat becomes a word bit by bit, as in
    # stdlib Sha256: the low `width` bits, least significant first.
    add(definition("wordOfNat", ["value : Nat", "width : Nat"], "UInt32",
                   "(second (second (prim iterate (fun (bits : (Nat * (UInt32 * UInt32))) => "
                   "(let remaining : Nat := (first bits); (let mask : UInt32 := (first (second bits)); "
                   "(let acc : UInt32 := (second (second bits)); ((prim quotient remaining 2 0 : Nat), "
                   "((shiftLeft mask (1 : UInt32)), (if ((prim remainder remaining 2 0 : Nat) =? 1) then "
                   "(prim bit_or acc mask : UInt32) else acc))))))) width (value, ((1 : UInt32), (0 : UInt32))) "
                   ": (Nat * (UInt32 * UInt32)))))"))
    # The 64-bit counter as its low and high words. Below 2^32 the high word
    # is zero without walking its bits.
    add(definition("counterLow", ["count : Nat"], "UInt32",
                   "(wordOfNat (prim remainder count 4294967296 0 : Nat) 32)"))
    add(definition("counterHigh", ["count : Nat"], "UInt32",
                   "(if (count <? 4294967296) then (0 : UInt32) else "
                   "(wordOfNat (prim quotient count 4294967296 0 : Nat) 32))"))

    add(definition("byteAt", ["message : Bytes", "size : Nat", "position : Nat"], "UInt32",
                   "(if (position <? size) then (match (prim index message position : (Option UInt8))\n"
                   "    | Option.some byte => (match (prim checked_convert byte : (Option UInt32))\n"
                   "      | Option.some widened => widened\n      | Option.none => (0 : UInt32))\n"
                   "    | Option.none => (0 : UInt32)) else (0 : UInt32))"))
    add(definition("wordLE", ["b0 : UInt32", "b1 : UInt32", "b2 : UInt32", "b3 : UInt32"], "UInt32",
                   "(prim bit_or b0 (prim bit_or (shiftLeft b1 (8 : UInt32)) (prim bit_or (shiftLeft b2 (16 : UInt32)) "
                   "(shiftLeft b3 (24 : UInt32)) : UInt32) : UInt32) : UInt32)"))
    add(definition("blockWord", ["message : Bytes", "size : Nat", "offset : Nat"], "UInt32",
                   "(wordLE (byteAt message size offset) (byteAt message size (offset + 1)) "
                   "(byteAt message size (offset + 2)) (byteAt message size (offset + 3)))"))
    add(definition("loadBlock", ["message : Bytes", "size : Nat", "offset : Nat"], "Words16",
                   record("Words16", [(f"v{i}", f"(blockWord message size (offset + {4 * i}))")
                                      for i in range(16)])))
    add(definition("keyWordsOf", ["material : Bytes"], "Cv",
                   "(cvOf (loadBlock material (prim length material : Nat) 0))"))
    add(definition("lastOffset", ["len : Nat"], "Nat",
                   "(if (len =? 0) then 0 else (prim multiply (prim quotient (prim subtract len 1 : Nat) 64 0 : Nat) 64 : Nat))",
                   axioms=False))
    add(definition("startFlag", ["offset : Nat"], "UInt32",
                   "(if (offset =? 0) then (flagChunkStart) else (0 : UInt32))", axioms=False))
    # One chunk of `len` bytes (at most 1024) at `begin`: every block but the
    # last compressed into the chaining value, the last left as an Output.
    add(definition("chunkOutput", ["key : Cv", "base : UInt32", "message : Bytes", "begin : Nat",
                                   "len : Nat", "chunk : Nat"], "Output",
                   "(let limit : Nat := (begin + len); (let last : Nat := (lastOffset len); "
                   "(let lo : UInt32 := (counterLow chunk); (let hi : UInt32 := (counterHigh chunk); "
                   "(let walked : (Nat * Cv) := (prim iterate (fun [base, begin, hi, limit, lo, message] "
                   "(walk : (Nat * Cv)) => (let pos : Nat := (first walk); ((pos + 64), "
                   "(cvOf (compress (second walk) (loadBlock message limit (begin + pos)) lo hi (64 : UInt32) "
                   "(prim bit_or base (startFlag pos) : UInt32)))))) (prim quotient last 64 0 : Nat) (0, key) "
                   ": (Nat * Cv)); "
                   "(record Output {chain := (second walked), block := (loadBlock message limit (begin + last)), "
                   "counter := chunk, blockLen := (wordOfNat (prim subtract len last : Nat) 7), "
                   "flags := (prim bit_or base (prim bit_or (flagChunkEnd) (startFlag last) : UInt32) : UInt32)}))))))"))
    add(definition("chainOf", ["node : Output"], "Cv",
                   "(cvOf (compress (project node chain) (project node block) (counterLow (project node counter)) "
                   "(counterHigh (project node counter)) (project node blockLen) (project node flags)))"))
    add(definition("rootWords", ["node : Output", "index : Nat"], "Words16",
                   "(compress (project node chain) (project node block) (counterLow index) (counterHigh index) "
                   "(project node blockLen) (prim bit_or (project node flags) (flagRoot) : UInt32))"))
    parent_block = record("Words16", [(f"v{i}", f"(project left h{i})") for i in range(8)]
                          + [(f"v{8 + i}", f"(project right h{i})") for i in range(8)])
    add(definition("parentOutput", ["left : Cv", "right : Cv", "key : Cv", "base : UInt32"], "Output",
                   f"(record Output {{chain := key, block := {parent_block}, counter := 0, "
                   "blockLen := (64 : UInt32), flags := (prim bit_or base (flagParent) : UInt32)})"))
    # The reference stack merge: after chunk `total` completes, merge once per
    # trailing zero bit of `total`. A Nat below 2^64 has at most 64.
    add(definition("mergeStep", ["key : Cv", "base : UInt32", "merging : (Cv * (Nat * (List Cv)))"],
                   "(Option (Cv * (Nat * (List Cv))))",
                   "(let total : Nat := (first (second merging)); (if ((prim remainder total 2 0 : Nat) =? 0) then "
                   "(match (second (second merging))\n"
                   "    | List.cons top rest => (ctor Option.some [(Cv * (Nat * (List Cv)))] "
                   "((chainOf (parentOutput top (first merging) key base)), ((prim quotient total 2 0 : Nat), rest)))\n"
                   "    | List.nil => (ctor Option.none [(Cv * (Nat * (List Cv)))])) "
                   "else (ctor Option.none [(Cv * (Nat * (List Cv)))])))"))
    add(definition("pushChunk", ["key : Cv", "base : UInt32", "stack : (List Cv)", "fresh : Cv", "total : Nat"],
                   "(List Cv)",
                   "(let merged : (Cv * (Nat * (List Cv))) := (first (prim iterate_until (fun [base, key] "
                   "(merging : (Cv * (Nat * (List Cv)))) => (mergeStep key base merging)) 64 (fresh, (total, stack)) "
                   ": ((Cv * (Nat * (List Cv))) * Bool))); (cons (first merged) (second (second merged))))"))
    add(definition("hasherWith", ["key : Cv", "base : UInt32"], "Hasher",
                   "(record Hasher {keyWords := key, modeFlags := base, chunkCount := 0, pending := 0x, "
                   "stack := (nil Cv)})", axioms=False))
    # Commit every chunk except the last 1 to 1024 bytes, which stay pending.
    add(definition("update", ["state : Hasher", "input : Bytes"], "Hasher",
                   "(let joined : Bytes := (prim append (project state pending) input : Bytes); "
                   "(let size : Nat := (prim length joined : Nat); "
                   "(let full : Nat := (if (size =? 0) then 0 else (prim quotient (prim subtract size 1 : Nat) 1024 0 : Nat)); "
                   "(let base : UInt32 := (project state modeFlags); (let key : Cv := (project state keyWords); "
                   "(let committed : (Nat * (List Cv)) := (prim iterate (fun [base, joined, key, state] "
                   "(progress : (Nat * (List Cv))) => (let chunk : Nat := ((project state chunkCount) + (first progress)); "
                   "(((first progress) + 1), (pushChunk key base (second progress) "
                   "(chainOf (chunkOutput key base joined (prim multiply (first progress) 1024 : Nat) 1024 chunk)) "
                   "(chunk + 1))))) full (0, (project state stack)) : (Nat * (List Cv))); "
                   "(let kept : Nat := (prim multiply full 1024 : Nat); "
                   "(record Hasher {keyWords := key, modeFlags := base, chunkCount := ((project state chunkCount) + full), "
                   "pending := (match (prim slice joined kept (prim subtract size kept : Nat) : (Option Bytes))\n"
                   "    | Option.some rest => rest\n    | Option.none => 0x), stack := (second committed)}))))))))"))
    add(definition("finalOutput", ["state : Hasher"], "Output",
                   "(let key : Cv := (project state keyWords); (let base : UInt32 := (project state modeFlags); "
                   "(let tail : Bytes := (project state pending); "
                   "(prim list_fold (fun [base, key] (node : Output) (left : Cv) => "
                   "(parentOutput left (chainOf node) key base)) "
                   "(chunkOutput key base tail 0 (prim length tail : Nat) (project state chunkCount)) "
                   "(project state stack) : Output))))"))
    add(definition("byteOf", ["octet : UInt32"], "Bytes",
                   bits_tree("octet", 8, lambda v: f"0x{v:02x}")))
    add(definition("wordBytes", ["word : UInt32"], "Bytes",
                   "(prim append (byteOf (prim bit_and word (255 : UInt32) : UInt32)) "
                   "(prim append (byteOf (prim bit_and (shiftRight word (8 : UInt32)) (255 : UInt32) : UInt32)) "
                   "(prim append (byteOf (prim bit_and (shiftRight word (16 : UInt32)) (255 : UInt32) : UInt32)) "
                   "(byteOf (shiftRight word (24 : UInt32))) : Bytes) : Bytes) : Bytes)"))
    body = f"(wordBytes (project w v15))"
    for i in range(14, -1, -1):
        body = f"(prim append (wordBytes (project w v{i})) {body} : Bytes)"
    add(definition("blockBytes", ["w : Words16"], "Bytes", body))
    # Any output length: root output block i is the root compression with
    # counter i, all sixteen words, little-endian, truncated to `outLen`.
    add(definition("outputBytes", ["node : Output", "outLen : Nat"], "Bytes",
                   "(let blocks : Nat := (prim quotient (outLen + 63) 64 0 : Nat); "
                   "(let produced : (Nat * Bytes) := (prim iterate (fun [node] (stream : (Nat * Bytes)) => "
                   "(((first stream) + 1), (prim append (second stream) (blockBytes (rootWords node (first stream))) : Bytes))) "
                   "blocks (0, 0x) : (Nat * Bytes)); "
                   "(match (prim slice (second produced) 0 outLen : (Option Bytes))\n"
                   "    | Option.some trimmed => trimmed\n    | Option.none => (second produced))))"))
    add(definition("finalize", ["state : Hasher", "outLen : Nat"], "Bytes",
                   "(outputBytes (finalOutput state) outLen)"))
    add(definition("contextKey", ["context : Bytes"], "Cv",
                   "(cvOf (rootWords (finalOutput (update (hasherWith (iv) (flagDeriveKeyContext)) context)) 0))"))
    add(definition("start", ["mode : Mode"], "(Result Hasher Blake3Error)",
                   "(match mode\n"
                   "    | Mode.hashMode => (ctor Result.ok [Hasher, Blake3Error] (hasherWith (iv) (0 : UInt32)))\n"
                   "    | Mode.keyedMode secret => (if ((prim length secret : Nat) =? 32) then "
                   "(ctor Result.ok [Hasher, Blake3Error] (hasherWith (keyWordsOf secret) (flagKeyedHash))) else "
                   "(ctor Result.error [Hasher, Blake3Error] (ctor Blake3Error.keyLength (prim length secret : Nat))))\n"
                   "    | Mode.deriveMode context => (ctor Result.ok [Hasher, Blake3Error] "
                   "(hasherWith (contextKey context) (flagDeriveKeyMaterial))))"))
    add(definition("hexDigit", ["nibble : UInt32"], "Bytes",
                   bits_tree("nibble", 4, lambda v: "0x" + format(v, "x").encode().hex())))
    add(definition("hexStep", ["data : Bytes", "walk : (Nat * Bytes)"], "(Nat * Bytes)",
                   "(let pos : Nat := (first walk); (let octet : UInt32 := (byteAt data (prim length data : Nat) pos); "
                   "((pos + 1), (prim append (second walk) (prim append (hexDigit (shiftRight octet (4 : UInt32))) "
                   "(hexDigit (prim bit_and octet (15 : UInt32) : UInt32)) : Bytes) : Bytes))))"))
    add(definition("hex", ["data : Bytes"], "Bytes",
                   "(second (prim iterate (fun [data] (walk : (Nat * Bytes)) => (hexStep data walk)) "
                   "(prim length data : Nat) (0, 0x) : (Nat * Bytes)))"))

    # Production roots. One-shot hashing is the incremental hasher fed once,
    # so the two cannot drift apart.
    add(definition("hashPieces", ["mode : Mode", "pieces : (List Bytes)", "outLen : Nat"],
                   "(Result Bytes Blake3Error)",
                   "(match (start mode)\n"
                   "    | Result.ok initial => (ctor Result.ok [Bytes, Blake3Error] (finalize (prim list_fold "
                   "(fun (state : Hasher) (piece : Bytes) => (update state piece)) initial pieces : Hasher) outLen))\n"
                   "    | Result.error refused => (ctor Result.error [Bytes, Blake3Error] refused))",
                   production=True))
    add(definition("hash", ["message : Bytes", "outLen : Nat"], "Bytes",
                   "(finalize (update (hasherWith (iv) (0 : UInt32)) message) outLen)", production=True))
    add(definition("hashHex", ["message : Bytes"], "Bytes", "(hex (hash message 32))", production=True))
    add(definition("keyedHash", ["secret : Bytes", "message : Bytes", "outLen : Nat"], "(Result Bytes Blake3Error)",
                   "(match (start (ctor Mode.keyedMode secret))\n"
                   "    | Result.ok initial => (ctor Result.ok [Bytes, Blake3Error] (finalize (update initial message) outLen))\n"
                   "    | Result.error refused => (ctor Result.error [Bytes, Blake3Error] refused))",
                   production=True))
    add(definition("deriveKey", ["context : Bytes", "material : Bytes", "outLen : Nat"], "Bytes",
                   "(finalize (update (hasherWith (contextKey context) (flagDeriveKeyMaterial)) material) outLen)",
                   production=True))

    # The Prism image's acceptance vectors (prism.mjs reads `acceptanceVectors`
    # and runs each through the image's Bytes -> Bytes root, hashHex): every
    # official input, built in the model as the 251-byte cycle doubled ten
    # times and cut, against the official 32-byte digest as lowercase hex.
    add(structure("Vector", [("given", "Bytes"), ("expected", "Bytes")]))
    add(structure("Acceptance", [("acceptanceVectors", "(List Vector)")]))
    add(definition("officialInput", ["len : Nat"], "Bytes",
                   f"(match (prim slice (prim iterate (fun (doubled : Bytes) => (prim append doubled doubled : Bytes)) "
                   f"10 {literal(bytes(range(251)))} : Bytes) 0 len : (Option Bytes))\n"
                   "    | Option.some cut => cut\n    | Option.none => 0x)", axioms=False))
    vectors = "(nil Vector)"
    for length in sorted(cases, reverse=True):
        expected = literal(cases[length]["hash"][:32].hex().encode())
        vectors = (f"(cons (record Vector {{given := (officialInput {length}), expected := {expected}}}) "
                   f"{vectors})")
    add(definition("acceptance", [], "Acceptance", f"(record Acceptance {{acceptanceVectors := {vectors}}})",
                   axioms=False, production=True))

    emit("\n".join(decls))
    emit("\\end{semanticmodule}\n\\end{lexlean}\n")
    modules = {MODULE.name: "".join(out)}

    # Reflexivity theorems against the official vectors, each at most two
    # compressions (a sixteen-compression rfl exhausts 8 GB). Bigger inputs
    # are held by the differential in crates/blake3-check. One theorem per
    # module: Lean checks a module's theorems on parallel threads, and eleven
    # of them at once exceed the 6 GB a verify may use, while modules are
    # checked one process at a time under LEXLEAN_VERIFY_WIDTH=1.
    def q(name):
        return f"Blake3::{name}"

    def block(output, index):
        values = words_le(output[64 * index:64 * index + 64])
        return record(q("Words16"), [(f"v{i}", u32(v)) for i, v in enumerate(values)])

    def root(state, index=0):
        return f"({q('rootWords')} ({q('finalOutput')} {state}) {index})"

    def fed(start_state, data):
        return f"({q('update')} {start_state} {literal(data)})"

    plain = f"({q('hasherWith')} ({q('iv')}) (0 : UInt32))"
    keyed = f"({q('hasherWith')} ({q('keyWordsOf')} {literal(key)}) ({q('flagKeyedHash')}))"
    derived = f"({q('hasherWith')} ({q('contextKey')} {literal(context)}) ({q('flagDeriveKeyMaterial')}))"
    theorems = []
    for length in (0, 1, 63, 64, 65):
        theorems.append((f"official_hash_{length}",
                         f"({root(fed(plain, message(length)))} = {block(cases[length]['hash'], 0)})"))
    theorems.append(("official_hash_0_block_1",
                     f"({root(fed(plain, b''), 1)} = {block(cases[0]['hash'], 1)})"))
    for length in (0, 1):
        theorems.append((f"official_keyed_hash_{length}",
                         f"({root(fed(keyed, message(length)))} = {block(cases[length]['keyed_hash'], 0)})"))
        theorems.append((f"official_derive_key_{length}",
                         f"({root(fed(derived, message(length)))} = {block(cases[length]['derive_key'], 0)})"))
    theorems.append(("split_update_equals_one_shot",
                     f"({root(fed(fed(plain, bytes([0, 1])), bytes([2])))} = {root(fed(plain, bytes([0, 1, 2])))})"))
    for name, statement in theorems:
        module = "Blake3Kernel" + "".join(part.capitalize() for part in name.split("_"))
        modules[f"{module}.lex.tex"] = (
            f"\\begin{{lexlean}}{{{module}}}\n\\useglossary{{lexlean.std.nat@1.2.0}}\n"
            f"\\importmodule{{Blake3}}\n\\title{{Natural number addition}}\n\\begin{{semanticmodule}}\n"
            f"{theorem(name, statement)}\\end{{semanticmodule}}\n\\end{{lexlean}}\n")
    return modules


def canonical(modules, lexlean):
    """The bytes `lexlean fmt` makes of each module: the formatter owns layout."""
    with tempfile.TemporaryDirectory() as scratch:
        project = pathlib.Path(scratch)
        for name in ("lexlean.toml", "lexlean.lock", "lakefile.toml", "lake-manifest.json", "lean-toolchain"):
            shutil.copy(SOURCES.parent / name, project / name)
        (project / "src").mkdir()
        for name, text in modules.items():
            (project / "src" / name).write_bytes(text.encode())
        ran = subprocess.run([lexlean, "fmt", "--color", "never"], cwd=project, capture_output=True, text=True)
        if ran.returncode != 0:
            raise SystemExit(f"lexlean fmt failed:\n{ran.stderr}")
        return {name: (project / "src" / name).read_bytes() for name in modules}


def main():
    lexlean = os.environ.get("LEXLEAN", "lexlean")
    modules = canonical(render(), lexlean)
    committed = {path.name for path in SOURCES.glob("*.lex.tex")}
    if "--check" in sys.argv[1:]:
        stale = sorted(name for name, text in modules.items()
                       if name not in committed or (SOURCES / name).read_bytes() != text)
        extra = sorted(committed - set(modules))
        if stale or extra:
            raise SystemExit(f"stale {stale}, not generated {extra}: run python3 -I tools/gen_blake3.py")
        print(f"gen_blake3: {len(modules)} modules equal their generator")
        return
    SOURCES.mkdir(parents=True, exist_ok=True)
    for name in committed - set(modules):
        (SOURCES / name).unlink()
    for name, text in modules.items():
        (SOURCES / name).write_bytes(text)
    entrypoints = ", ".join(f'"src/{name}"' for name in sorted(modules))
    print(f"gen_blake3: wrote {len(modules)} modules; entrypoints = [{entrypoints}]")


if __name__ == "__main__":
    main()
