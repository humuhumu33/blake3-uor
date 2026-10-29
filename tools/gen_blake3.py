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
# Five LexLean projects in one workspace, each with only its own roots:
# a production certificate imports a shared lemma library built from every
# root of its project, and the hash and trace roots together exceed the 8 GB
# a verify may use, while each project alone fits.
PROJECTS = ("lexlean", "lexlean-trace", "lexlean-reverse", "lexlean-slice", "lexlean-analysis")
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
# Step kinds, numbered 0-4 in the serialised trace (docs/trace.md).
OPS = ["opAdd", "opAddMsg", "opXor", "opRor", "opXorCv"]
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


def theorem(name, statement, axioms=True, parameters=()):
    text = (f"theorem{{\n  name: {name},\n  parameters: [{', '.join(parameters)}],\n  statement: {statement},\n"
            "  proof: reflexivity{}")
    if axioms:
        text += ",\n" + AX.rstrip("\n")
    return text + "\n}\n"


def record(name, values):
    return f"(record {name} {{{', '.join(f'{k} := {v}' for k, v in values)}}})"


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


def lets(bindings, body):
    """Nested lets: [(name, type, expr), ...] around body, parentheses counted."""
    text = "".join(f"(let {n} : {t} := {e}; " for n, t, e in bindings)
    return text + body + ")" * len(bindings)


def iff(cond, then, other):
    return f"(if {cond} then {then} else {other})"


def eq(a, b):
    return f"(prim equal {a} {b} : Bool)"


def conj(*terms):
    """Right-nested Boolean conjunction."""
    body = terms[-1]
    for term in reversed(terms[:-1]):
        body = f"({term} && {body})"
    return body


def cat(*pieces):
    """Bytes concatenation of a fixed list of pieces, right-nested."""
    body = pieces[-1]
    for piece in reversed(pieces[:-1]):
        body = f"(prim append {piece} {body} : Bytes)"
    return body


def blist(items, t="Bytes"):
    body = f"(nil {t})"
    for item in reversed(items):
        body = f"(cons {item} {body})"
    return body


def sigma(r):
    """Message slot j of round r holds block word sigma(r)[j]."""
    order = list(range(16))
    for _ in range(r):
        order = [order[p] for p in PERMUTATION]
    return order


def render_trace(core="Blake3Uor.Blake3::", sha=lambda n: f"LexLeanStd.Sha256::{n}"):
    """The trace module: canonical blobs, ADR-015 DigestSets, the serialised
    trace at three levels, the Merkle structure, and the replaying checker.
    `core` qualifies the core module's names and `sha` the stdlib's; the
    flattened Prism image passes its own."""
    q = lambda n: f"{core}{n}"
    W16, CV, NODE = q("Words16"), q("Cv"), q("Node")
    decls = []
    add = decls.append
    ok = lambda t, v: f"(ctor Result.ok [{t}, TraceError] {v})"
    bad = lambda t, kind, at: f"(ctor Result.error [{t}, TraceError] (ctor TraceError.{kind} {at}))"

    add(inductive("TraceError", [("malformed", ["Nat"]), ("digestSet", ["Nat"]), ("unknownLabel", ["Nat"]),
                                 ("load", ["Nat"]), ("provenance", ["Nat"]), ("permutation", ["Nat"]),
                                 ("step", ["Nat"]), ("bit", ["Nat"]), ("link", ["Nat"]), ("output", ["Nat"]),
                                 ("digest", ["Nat"]), ("rootMismatch", ["Nat"])]))

    # Encoding.
    add(definition("u8", ["n : Nat"], "Bytes", f"({q('byteOf')} ({q('wordOfNat')} n 8))"))
    add(definition("u32", ["n : Nat"], "Bytes", f"({q('wordBytes')} ({q('wordOfNat')} n 32))"))
    add(definition("u64", ["n : Nat"], "Bytes",
                   f"(prim append ({q('wordBytes')} ({q('counterLow')} n)) ({q('wordBytes')} ({q('counterHigh')} n)) : Bytes)"))
    add(definition("wordBE", ["w : UInt32"], "Bytes",
                   cat(*[f"({q('byteOf')} (prim bit_and ({q('shiftRight')} w {u32(s)}) {u32(255)} : UInt32))"
                         for s in (24, 16, 8, 0)])))
    add(definition("shaBytes", ["blob : Bytes"], "Bytes",
                   lets([("d", sha("Digest"), f"({sha('sha256')} blob)")],
                        cat(*[f"(wordBE (project d h{i}))" for i in range(8)]))))
    # ADR-015: BLAKE3 (default mode, 32 bytes) then SHA-256, over the same bytes.
    add(definition("digestSetOf", ["blob : Bytes"], "Bytes",
                   f"(prim append ({q('hash')} blob 32) (shaBytes blob) : Bytes)"))
    add(definition("labelOf", ["ds : Bytes"], "Bytes",
                   "(match (prim slice ds 0 32 : (Option Bytes))\n    | Option.some head => head\n    | Option.none => ds)",
                   axioms=False))
    add(definition("cvBytes", ["c : " + CV], "Bytes", cat(*[f"({q('wordBytes')} (project c h{i}))" for i in range(8)])))
    add(definition("nodeState", ["n : " + NODE], W16,
                   f"({q('initialState')} (project n chain) (project n counterLo) (project n counterHi) "
                   "(project n blockLen) (project n flags))", axioms=False))
    add(definition("checkpointBlob", ["n : " + NODE], "Bytes",
                   f"({q('concatAll')} {blist(['0x01', '(u8 (project n role))', '(u64 (project n origin))', '(u64 (project n span))', '(u64 (project n index))', '(u64 (project n offset))', '(' + q('blockBytes') + ' (nodeState n))', '(' + q('blockBytes') + ' (project n block))'])})"))
    add(definition("entryOf", ["blob : Bytes", "ds : Bytes"], "Bytes",
                   cat("(u32 (prim length blob : Nat))", "blob", "ds")))

    # Bits. An add's 32 adder records and an xor's 32 bit records, bit 0 first.
    add(definition("bitAt", ["w : UInt32", "i : UInt32"], "UInt32",
                   f"(prim bit_and ({q('shiftRight')} w i) {u32(1)} : UInt32)"))
    sl = lambda v, k: f"({q('shiftLeft')} {v} {u32(k)})"
    add(definition("adderByte", ["a : UInt32", "b : UInt32", "c : UInt32"], "UInt32",
                   lets([("s", "UInt32", "(prim bit_xor (prim bit_xor a b : UInt32) c : UInt32)"),
                         ("co", "UInt32", "(prim bit_or (prim bit_or (prim bit_and a b : UInt32) (prim bit_and a c : UInt32) : UInt32) (prim bit_and b c : UInt32) : UInt32)")],
                        f"(prim bit_or a (prim bit_or {sl('b', 1)} (prim bit_or {sl('c', 2)} (prim bit_or {sl('s', 3)} {sl('co', 4)} : UInt32) : UInt32) : UInt32) : UInt32)")))
    add(definition("carryOut", ["a : UInt32", "b : UInt32", "c : UInt32"], "UInt32",
                   "(prim bit_or (prim bit_or (prim bit_and a b : UInt32) (prim bit_and a c : UInt32) : UInt32) (prim bit_and b c : UInt32) : UInt32)"))
    add(definition("xorByte", ["a : UInt32", "b : UInt32"], "UInt32",
                   f"(prim bit_or a (prim bit_or {sl('b', 1)} {sl('(prim bit_xor a b : UInt32)', 2)} : UInt32) : UInt32)"))
    # A bit record is below 32, so it has its own 32-leaf lookup: the 256-leaf
    # byteOf inside these loops makes their certificate exceed 8 GB.
    add(definition("bitByte", ["v : UInt32"], "Bytes", bits_tree("v", 5, lambda n: f"0x{n:02x}")))
    add(definition("addBits", ["x : UInt32", "y : UInt32"], "Bytes",
                   "(second (second (prim iterate (fun [x, y] (walk : (UInt32 * (UInt32 * Bytes))) => "
                   + lets([("i", "UInt32", "(first walk)"), ("a", "UInt32", "(bitAt x i)"), ("b", "UInt32", "(bitAt y i)"),
                           ("c", "UInt32", "(first (second walk))")],
                          f"(({q('add')} i {u32(1)}), ((carryOut a b c), (prim append (second (second walk)) "
                          "(bitByte (adderByte a b c)) : Bytes)))")
                   + f") 32 ({u32(0)}, ({u32(0)}, 0x)) : (UInt32 * (UInt32 * Bytes)))))"))
    add(definition("xorBits", ["x : UInt32", "y : UInt32"], "Bytes",
                   "(second (prim iterate (fun [x, y] (walk : (UInt32 * Bytes)) => "
                   + lets([("i", "UInt32", "(first walk)")],
                          f"(({q('add')} i {u32(1)}), (prim append (second walk) (bitByte (xorByte (bitAt x i) (bitAt y i))) : Bytes))")
                   + f") 32 ({u32(0)}, 0x) : (UInt32 * Bytes)))"))
    add(definition("kindCode", ["kind : " + q("Op")], "Nat",
                   "(match kind\n" + "\n".join(f"    | {q('Op')}.{o} => {i}" for i, o in enumerate(OPS)) + ")",
                   axioms=False))
    add(definition("stepBits", ["level : Nat", "s : " + q("Step")], "Bytes",
                   iff("(level <? 2)", "0x",
                       "(match (project s op)\n"
                       f"    | {q('Op')}.opAdd => (addBits (project s left) (project s right))\n"
                       f"    | {q('Op')}.opAddMsg => (addBits (project s left) (project s right))\n"
                       f"    | {q('Op')}.opXor => (xorBits (project s left) (project s right))\n"
                       f"    | {q('Op')}.opRor => 0x\n"
                       f"    | {q('Op')}.opXorCv => (xorBits (project s left) (project s right)))")))
    add(definition("stepPiece", ["level : Nat", "s : " + q("Step")], "Bytes",
                   f"({q('concatAll')} {blist(['(u8 (project s round))', '(u8 (project s g))', '(u8 (project s index))', '(u8 (kindCode (project s op)))', '(u8 (project s dest))', '(u8 (project s source))', '(' + q('wordBytes') + ' (project s left))', '(' + q('wordBytes') + ' (project s right))', '(' + q('wordBytes') + ' (project s result))', '(stepBits level s)'])})"))
    tree = lambda lo, hi: (f"0x{bytes(sigma(lo)).hex()}" if lo == hi
                           else iff(f"(r <? {(lo + hi + 1) // 2})", tree(lo, (lo + hi + 1) // 2 - 1), tree((lo + hi + 1) // 2, hi)))
    add(definition("mapBytes", ["r : Nat"], "Bytes", tree(0, ROUNDS - 1), axioms=False))
    add(definition("roundStart", ["s : " + q("Step")], "Bool",
                   "(((project s round) <? 7) && (((project s g) =? 0) && ((project s index) =? 0)))", axioms=False))
    # Folding the newest-first log onto the output puts the steps in order.
    add(definition("sectionBytes", ["level : Nat", "n : " + NODE], "Bytes",
                   lets([("t", q("CompressionTrace"),
                          f"({q('compressTrace')} (project n chain) (project n block) (project n counterLo) "
                          "(project n counterHi) (project n blockLen) (project n flags))")],
                        f"({q('concatAll')} (cons (u64 (project n offset)) (cons ({q('byteOf')} (project n blockLen)) "
                        f"(cons ({q('blockBytes')} (project n block)) (cons 0x{bytes(range(16)).hex()} "
                        f"(prim list_fold (fun [level] (acc : (List Bytes)) (s : {q('Step')}) => (cons (if (roundStart s) then "
                        "(prim append (mapBytes (project s round)) (stepPiece level s) : Bytes) else (stepPiece level s)) acc)) "
                        f"(cons ({q('blockBytes')} ({q('compressOut')} t)) (nil Bytes)) (project t log) : (List Bytes)))))))")))

    # The Merkle structure, built by replaying the event log. Each blob goes
    # into the file as it is made: length, bytes, DigestSet, and for a
    # checkpoint at level 1 or 2 its word section.
    add(structure("Build", [("level", "Nat"), ("entries", "(List Bytes)"), ("count", "Nat"),
                            ("stack", "(List Bytes)"), ("chunkLabels", "(List Bytes)"), ("chunkSize", "Nat"),
                            ("waiting", "(Option Bytes)"), ("current", "Bytes"), ("outputs", "(List Bytes)"),
                            ("events", "(List Bytes)"), ("eventCount", "Nat")]))
    build_fields = ["level", "entries", "count", "stack", "chunkLabels", "chunkSize", "waiting", "current",
                    "outputs", "events", "eventCount"]

    def build_with(src, **changes):
        return record("Build", [(f, changes.get(f, f"(project {src} {f})")) for f in build_fields])

    add(definition("emit", ["b : Build", "blob : Bytes", "extra : Bytes"], "(Build * Bytes)",
                   lets([("ds", "Bytes", "(digestSetOf blob)")],
                        "(" + build_with("b", entries="(cons (prim append (entryOf blob ds) extra : Bytes) (project b entries))",
                                         count="((project b count) + 1)") + ", (labelOf ds))")))
    add(definition("joinLabels", ["labels : (List Bytes)"], "Bytes",
                   "(" + q("concatAll") + " (prim list_fold (fun (reversed : (List Bytes)) (piece : Bytes) => (cons piece reversed)) "
                   "(nil Bytes) labels : (List Bytes)))", axioms=False))
    add(definition("chunkBlob", ["chunk : Nat", "b : Build", "tail : Bytes"], "Bytes",
                   cat("0x02", "(u64 chunk)", "(u8 (project b chunkSize))", "(joinLabels (project b chunkLabels))", "tail")))
    is_end = f"(prim equal (prim bit_and (project n flags) ({q('flagChunkEnd')}) : UInt32) ({q('flagChunkEnd')}) : Bool)"
    more = "((project nb1 eventCount) + 1)"
    on_compressed = lets(
        [("cp", "Bytes", "(checkpointBlob n)"),
         ("made", "(Build * Bytes)", "(emit b cp (if ((project b level) =? 0) then 0x else (sectionBytes (project b level) n)))"),
         ("nb1", "Build", "(first made)"), ("cl", "Bytes", "(second made)"),
         ("ev", "(List Bytes)", "(cons (prim append 0x00 cl : Bytes) (project nb1 events))")],
        iff("((project n role) =? 0)",
            iff(is_end,
                lets([("closed", "(Build * Bytes)",
                       "(emit nb1 (chunkBlob (project n origin) " + build_with("nb1", chunkLabels="(cons cl (project nb1 chunkLabels))",
                                                                          chunkSize="((project nb1 chunkSize) + 1)")
                       + f" (prim append 0x01 (cvBytes ({q('cvOf')} (project n output))) : Bytes)) 0x)"),
                      ("nb2", "Build", "(first closed)")],
                     build_with("nb2", chunkLabels="(nil Bytes)", chunkSize="0", current="(second closed)", events="ev",
                                eventCount=more)),
                build_with("nb1", chunkLabels="(cons cl (project nb1 chunkLabels))", chunkSize="((project nb1 chunkSize) + 1)",
                           events="ev", eventCount=more)),
            iff("((project n role) =? 1)",
                "(match (project nb1 waiting)\n"
                "    | Option.some held => " + lets(
                    [("joined", "(Build * Bytes)",
                      "(emit nb1 " + cat("0x03", "(u64 (project n origin))", "(u64 (project n span))", "held", "(project nb1 current)",
                                         "0x01", "cl", f"(cvBytes ({q('cvOf')} (project n output)))") + " 0x)"),
                     ("nb2", "Build", "(first joined)")],
                    build_with("nb2", waiting="(ctor Option.none [Bytes])", current="(second joined)", events="ev", eventCount=more))
                + "\n    | Option.none => " + build_with("nb1", events="ev", eventCount=more) + ")",
                lets([("put", "(Build * Bytes)",
                       "(emit nb1 " + cat("0x04", "(u64 (project n index))", "cl", f"({q('blockBytes')} (project n output))") + " 0x)"),
                      ("nb2", "Build", "(first put)")],
                     build_with("nb2", outputs="(cons (second put) (project nb2 outputs))", events="ev", eventCount=more)))))
    stack_event = lambda tag: (f"(cons {cat(tag, '(u8 depth)', '(u64 (project e origin))', '(u64 (project e span))', '(cvBytes (project e cv))')} "
                               "(project b events))")
    add(definition("buildStep", ["b : Build", "event : " + q("Event")], "Build",
                   f"(match event\n    | {q('Event')}.compressed n => {on_compressed}\n"
                   f"    | {q('Event')}.pushed e depth => "
                   + build_with("b", stack="(cons (project b current) (project b stack))", events=stack_event("0x01"),
                                eventCount="((project b eventCount) + 1)")
                   + f"\n    | {q('Event')}.popped e depth => (match (project b stack)\n"
                   "      | List.cons top rest => "
                   + build_with("b", stack="rest", waiting="(ctor Option.some [Bytes] top)", events=stack_event("0x02"),
                                eventCount="((project b eventCount) + 1)")
                   + "\n      | List.nil => " + build_with("b", events=stack_event("0x02"), eventCount="((project b eventCount) + 1)")
                   + "))"))
    add(definition("emptyBuild", ["level : Nat", "entries : (List Bytes)", "count : Nat"], "Build",
                   "(record Build {level := level, entries := entries, count := count, stack := (nil Bytes), "
                   "chunkLabels := (nil Bytes), chunkSize := 0, waiting := (ctor Option.none [Bytes]), current := 0x, "
                   "outputs := (nil Bytes), events := (nil Bytes), eventCount := 0})", axioms=False))
    # After the log: the root's tree node (a parent if a pop is waiting, else
    # the root chunk's non-final blocks), the events blob, then the root
    # blob. The root covers chunk 0 onwards, every chunk of the input.
    add(definition("finishBuild", ["b : Build", "trace : " + q("Trace"), "inputSet : Bytes", "context : Bytes"],
                   "(Build * Bytes)",
                   lets([("chunks", "Nat", "(if ((project trace inputLen) =? 0) then 1 else "
                                           "(prim quotient ((project trace inputLen) + 1023) 1024 0 : Nat))"),
                         ("nodeBlob", "Bytes",
                          "(match (project b waiting)\n"
                          "    | Option.some held => " + cat("0x03", "(u64 0)", "(u64 chunks)", "held", "(project b current)", "0x00")
                          + "\n    | Option.none => (chunkBlob 0 b 0x00))"),
                         ("node", "(Build * Bytes)", "(emit b nodeBlob 0x)"),
                         ("log", "(Build * Bytes)",
                          "(emit (first node) (prim append 0x05 (prim append (u64 (project b eventCount)) "
                          "(joinLabels (project b events)) : Bytes) : Bytes) 0x)"),
                         ("mode", "Nat", "(project trace mode)")],
                        "(emit (first log) (" + q("concatAll") + " " + blist(
                            ["0x0601", "(u8 mode)", "(u64 (project trace inputLen))", "(u64 (project trace outLen))", "inputSet",
                             "(second node)", "(second log)", "(if (mode =? 2) then context else 0x)",
                             "(u64 (prim length (project b outputs) : Nat))", "(joinLabels (project b outputs))",
                             f"({q('digestOf')} trace)"]) + ") 0x)")))
    add(definition("buildTrace", ["prior : Build", "trace : " + q("Trace"), "inputSet : Bytes", "context : Bytes"],
                   "(Build * Bytes)",
                   "(finishBuild (prim list_fold (fun (b : Build) (event : " + q("Event") + ") => (buildStep b event)) "
                   "(emptyBuild (project prior level) (project prior entries) (project prior count)) "
                   "(project trace events) : Build) trace inputSet context)"))
    # A derive-key trace is the context's whole trace (mode 3) and then the
    # key material's, whose root names the context root.
    add(definition("traceBuild", ["mode : " + q("Mode"), "pieces : (List Bytes)", "outLen : Nat", "level : Nat"],
                   "(Result (Build * Bytes) " + q("Blake3Error") + ")",
                   lets([("input", "Bytes", f"({q('concatAll')} pieces)")],
                        f"(match ({q('traceOf')} mode pieces outLen)\n"
                        f"    | Result.ok trace => (ctor Result.ok [(Build * Bytes), {q('Blake3Error')}] (match mode\n"
                        f"      | {q('Mode')}.deriveMode context => (let earlier : (Build * Bytes) := (buildTrace (emptyBuild level (nil Bytes) 0) "
                        f"(record {q('Trace')} {{mode := 3, inputLen := (prim length context : Nat), outLen := 32, "
                        f"keyWords := ({q('iv')}), modeFlags := ({q('flagDeriveKeyContext')}), prelude := (nil {q('Event')}), "
                        f"events := (project trace prelude)}}) (digestSetOf context) 0x); "
                        f"(buildTrace (first earlier) trace (digestSetOf input) (second earlier)))\n"
                        f"      | {q('Mode')}.hashMode => (buildTrace (emptyBuild level (nil Bytes) 0) trace (digestSetOf input) 0x)\n"
                        f"      | {q('Mode')}.keyedMode secret => (buildTrace (emptyBuild level (nil Bytes) 0) trace (digestSetOf input) 0x)))\n"
                        f"    | Result.error refused => (ctor Result.error [(Build * Bytes), {q('Blake3Error')}] refused))")))
    add(definition("traceFile", ["mode : " + q("Mode"), "pieces : (List Bytes)", "outLen : Nat", "level : Nat"],
                   f"(Result Bytes {q('Blake3Error')})",
                   f"(match (traceBuild mode pieces outLen level)\n"
                   f"    | Result.ok built => (ctor Result.ok [Bytes, {q('Blake3Error')}] "
                   + cat("0x4233545241434531", "(u8 level)", "(u64 (project (first built) count))",
                         "(joinLabels (project (first built) entries))")
                   + f")\n    | Result.error refused => (ctor Result.error [Bytes, {q('Blake3Error')}] refused))",
                   production=True))
    # The trace's address: the root blob's DigestSet. The root blob itself
    # records the input's DigestSet and the digest beside it.
    add(definition("traceAddress", ["mode : " + q("Mode"), "pieces : (List Bytes)", "outLen : Nat"],
                   f"(Result Bytes {q('Blake3Error')})",
                   f"(match (traceBuild mode pieces outLen 0)\n"
                   f"    | Result.ok built => (ctor Result.ok [Bytes, {q('Blake3Error')}] (match (project (first built) entries)\n"
                   "      | List.cons last rest => (match (prim slice last (prim subtract (prim length last : Nat) 64 : Nat) 64 : (Option Bytes))\n"
                   "        | Option.some set => set\n        | Option.none => 0x)\n      | List.nil => 0x))\n"
                   f"    | Result.error refused => (ctor Result.error [Bytes, {q('Blake3Error')}] refused))",
                   production=True))

    # Reading the file back.
    # A word's value as a Nat. The checker is not exported to Rust: its
    # rust-std certificate exceeds 11 GB whenever a Nat is computed from the
    # file's bytes, by this loop or any other conversion tried (docs/trace.md).
    add(definition("natOfWord", ["w : UInt32"], "Nat",
                   "(second (second (prim iterate (fun [w] (walk : (UInt32 * (Nat * Nat))) => "
                   + lets([("i", "UInt32", "(first walk)")],
                          f"(({q('add')} i {u32(1)}), (((first (second walk)) + (first (second walk))), "
                          f"(if (prim equal (bitAt w i) {u32(1)} : Bool) then ((second (second walk)) + (first (second walk))) "
                          "else (second (second walk)))))")
                   + f") 32 ({u32(0)}, (1, 0)) : (UInt32 * (Nat * Nat)))))"))
    add(definition("rd8", ["data : Bytes", "pos : Nat"], "Nat",
                   f"(natOfWord ({q('byteAt')} data (prim length data : Nat) pos))"))
    add(definition("rd32", ["data : Bytes", "pos : Nat"], "UInt32",
                   f"({q('blockWord')} data (prim length data : Nat) pos)"))
    add(definition("rd64", ["data : Bytes", "pos : Nat"], "Nat",
                   "((natOfWord (rd32 data pos)) + (prim multiply (natOfWord (rd32 data (pos + 4))) 4294967296 : Nat))"))
    add(definition("cutAt", ["data : Bytes", "pos : Nat", "len : Nat"], "(Option Bytes)",
                   "(prim slice data pos len : (Option Bytes))", axioms=False))
    add(definition("words16At", ["data : Bytes", "pos : Nat"], W16,
                   record(W16, [(f"v{i}", f"(rd32 data (pos + {4 * i}))") for i in range(16)])))
    add(definition("cvAt", ["data : Bytes", "pos : Nat"], CV,
                   record(CV, [(f"h{i}", f"(rd32 data (pos + {4 * i}))") for i in range(8)])))

    add(definition("wordsEq", ["a : " + W16, "b : " + W16], "Bool",
                   conj(*[f"(prim equal (project a v{i}) (project b v{i}) : Bool)" for i in range(16)]), axioms=False))
    add(definition("cvEq", ["a : " + CV, "b : " + CV], "Bool",
                   conj(*[f"(prim equal (project a h{i}) (project b h{i}) : Bool)" for i in range(8)]), axioms=False))
    add(definition("sameBytes", ["a : (Option Bytes)", "b : (Option Bytes)"], "Bool",
                   "(match a\n    | Option.some x => (match b\n      | Option.some y => (prim equal x y : Bool)\n"
                   "      | Option.none => false)\n    | Option.none => false)", axioms=False))

    # The 32 bit records of a step, replayed from its operands.
    add(definition("bitsHold", ["data : Bytes", "pos : Nat", "kind : Nat", "x : UInt32", "y : UInt32", "z : UInt32"], "Bool",
                   "(second (prim iterate (fun [data, kind, pos, x, y, z] (walk : ((UInt32 * (UInt32 * Nat)) * Bool)) => "
                   + lets([("i", "UInt32", "(first (first walk))"), ("c", "UInt32", "(first (second (first walk)))"),
                           ("k", "Nat", "(second (second (first walk)))"),
                           ("a", "UInt32", "(bitAt x i)"), ("b", "UInt32", "(bitAt y i)"),
                           ("want", "UInt32", "(if ((kind =? 0) || (kind =? 1)) then (adderByte a b c) else (xorByte a b))"),
                           ("sum", "UInt32", "(if ((kind =? 0) || (kind =? 1)) then (prim bit_xor (prim bit_xor a b : UInt32) c : UInt32) else (prim bit_xor a b : UInt32))")],
                          f"((({q('add')} i {u32(1)}), ((carryOut a b c), (k + 1))), (if (second walk) then "
                          f"((prim equal ({q('byteAt')} data (prim length data : Nat) (pos + k)) want : Bool) && "
                          "(prim equal sum (bitAt z i) : Bool)) else false))")
                   + f") 32 (({u32(0)}, ({u32(0)}, 0)), true) : ((UInt32 * (UInt32 * Nat)) * Bool)))"))
    add(definition("hasBits", ["level : Nat", "kind : Nat"], "Bool",
                   "(if (level <? 2) then false else (if (kind =? 3) then false else true))", axioms=False))
    # A recorded step against the machine's own: address, kind, wiring,
    # operands and result, then its bit records.
    add(definition("stepHolds", ["data : Bytes", "pos : Nat", "s : " + q("Step")], "Bool",
                   conj("((rd8 data pos) =? (project s round))", "((rd8 data (pos + 1)) =? (project s g))",
                        "((rd8 data (pos + 2)) =? (project s index))", "((rd8 data (pos + 3)) =? (kindCode (project s op)))",
                        "((rd8 data (pos + 4)) =? (project s dest))", "((rd8 data (pos + 5)) =? (project s source))",
                        "(prim equal (rd32 data (pos + 6)) (project s left) : Bool)",
                        "(prim equal (rd32 data (pos + 10)) (project s right) : Bool)",
                        "(prim equal (rd32 data (pos + 14)) (project s result) : Bool)")))
    mach = q("Machine")
    racc = f"(Result (Nat * {mach}) TraceError)"
    replay = (
        "(match (second walk)\n"
        "      | Result.ok at => " + lets(
            [("k", "Nat", "(first walk)"), ("p", "Nat", "(first at)"),
             ("mapped", "(Result Nat TraceError)",
              iff("((k <? 784) && ((prim remainder k 112 0 : Nat) =? 0))",
                  iff("(sameBytes (cutAt data p 16) (ctor Option.some [Bytes] (mapBytes (prim quotient k 112 0 : Nat))))",
                      ok("Nat", "(p + 16)"), bad("Nat", "permutation", "p")),
                  ok("Nat", "p"))),
             ("moved", mach, f"({q('machineStep')} cv m (record {mach} {{state := (project (second at) state), "
                             f"out := (project (second at) out), log := (nil {q('Step')})}}) k)")],
            "(match mapped\n        | Result.ok here => (match (project moved log)\n"
            "          | List.cons s rest => "
            + iff("(stepHolds data here s)",
                  iff("(if (hasBits level (kindCode (project s op))) then (bitsHold data (here + 18) (kindCode (project s op)) "
                      "(project s left) (project s right) (project s result)) else true)",
                      ok(f"(Nat * {mach})", "((here + (if (hasBits level (kindCode (project s op))) then 50 else 18)), moved)"),
                      bad(f"(Nat * {mach})", "bit", "(here + 18)")),
                  bad(f"(Nat * {mach})", "step", "here"))
            + "\n          | List.nil => " + bad(f"(Nat * {mach})", "step", "here") + ")\n"
            f"        | Result.error failed => (ctor Result.error [(Nat * {mach}), TraceError] failed))")
        + f"\n      | Result.error failed => (ctor Result.error [(Nat * {mach}), TraceError] failed))")
    loads_zero = ("(second (prim iterate (fun [data, len, pos] (walk : (Nat * Bool)) => (((first walk) + 1), "
                  "(if (second walk) then (if ((first walk) <? len) then true else ((rd8 data ((pos + 9) + (first walk))) =? 0)) "
                  "else false))) 64 (0, true) : (Nat * Bool)))")
    ivs = conj(*[f"(prim equal (project s0 v{8 + i}) {u32(IV[i])} : Bool)" for i in range(4)])
    add(definition("ivHolds", ["s0 : " + W16], "Bool", ivs, axioms=False))
    # A checkpoint's word section, replayed from the checkpoint alone.
    add(definition("checkSection", ["data : Bytes", "pos : Nat", "level : Nat", "s0 : " + W16, "m : " + W16],
                   "(Result Nat TraceError)",
                   lets([("len", "Nat", "(rd8 data (pos + 8))"),
                         ("cv", CV, record(CV, [(f"h{i}", f"(project s0 v{i})") for i in range(8)]))],
                        iff(f"((len =? (natOfWord (project s0 v14))) && ((wordsEq (words16At data (pos + 9)) m) && {loads_zero}))",
                            iff(f"((sameBytes (cutAt data (pos + 73) 16) (ctor Option.some [Bytes] 0x{bytes(range(16)).hex()})) && (ivHolds s0))",
                                "(match (second (prim iterate (fun [cv, data, level, m] (walk : (Nat * " + racc + ")) => "
                                "(((first walk) + 1), " + replay + ")) 800 (0, " + ok(f"(Nat * {mach})", f"((pos + 89), (record {mach} {{state := s0, "
                                f"out := {record(W16, [(f'v{i}', u32(0)) for i in range(16)])}, log := (nil {q('Step')})}}))") + ") : (Nat * " + racc + ")))\n"
                                "    | Result.ok done => (if (wordsEq (words16At data (first done)) (project (second done) out)) then "
                                + ok("Nat", "((first done) + 64)") + " else " + bad("Nat", "output", "(first done)") + ")\n"
                                "    | Result.error failed => (ctor Result.error [Nat, TraceError] failed))",
                                bad("Nat", "provenance", "(pos + 73)")),
                            bad("Nat", "load", "pos")))))

    # The whole file: every blob against its DigestSet (both keys), every
    # label it names against the blobs before it, every checkpoint's word
    # section by replay, every output against its checkpoint, the digest
    # against the outputs. Lookups are linear: checking is meant for small
    # and medium traces, building for any size.
    zeros16 = record(W16, [(f"v{i}", u32(0)) for i in range(16)])
    zeros8 = record(CV, [(f"h{i}", u32(0)) for i in range(8)])
    # What the checker remembers of a blob, as bytes only: a loop whose state
    # nests records inside a list inside a record makes the export
    # certificate exceed 8 GB.
    add(structure("Seen", [("label", "Bytes"), ("kind", "Nat"), ("cv", "Bytes"), ("state", "Bytes"), ("block", "Bytes")]))
    add(structure("Check", [("pos", "Nat"), ("level", "Nat"), ("seen", "(List Seen)"), ("root", "Bytes"),
                            ("last", "Nat")]))
    add(definition("lookup", ["seen : (List Seen)", "wanted : Bytes"], "(Option Seen)",
                   "(prim list_fold (fun [wanted] (found : (Option Seen)) (s : Seen) => "
                   "(if (prim equal (project s label) wanted : Bool) then (ctor Option.some [Seen] s) else found)) "
                   "(ctor Option.none [Seen]) seen : (Option Seen))", axioms=False))
    add(definition("known", ["seen : (List Seen)", "blob : Bytes", "at : Nat", "kinds : (List Nat)"], "(Option Seen)",
                   "(match (cutAt blob at 32)\n    | Option.some wanted => (match (lookup seen wanted)\n"
                   "      | Option.some s => (if (prim list_fold (fun [s] (hit : Bool) (k : Nat) => (hit || ((project s kind) =? k))) "
                   "false kinds : Bool) then (ctor Option.some [Seen] s) else (ctor Option.none [Seen]))\n"
                   "      | Option.none => (ctor Option.none [Seen]))\n    | Option.none => (ctor Option.none [Seen]))",
                   axioms=False))
    add(definition("replayOut", ["s0 : " + W16, "m : " + W16], W16,
                   f"({q('compress')} ({q('cvOf')} s0) m (project s0 v12) (project s0 v13) (project s0 v14) (project s0 v15))"))
    add(definition("bytesAt", ["blob : Bytes", "at : Nat", "len : Nat"], "Bytes",
                   "(match (cutAt blob at len)\n    | Option.some got => got\n    | Option.none => 0x)", axioms=False))
    # A checkpoint's 16 output words, replayed, as bytes.
    add(definition("replayBytes", ["state : Bytes", "block : Bytes"], "Bytes",
                   f"({q('blockBytes')} (replayOut (words16At state 0) (words16At block 0)))"))
    seen_row = lambda kind, cv, state, block: (
        f"(cons (record Seen {{label := label, kind := {kind}, cv := {cv}, state := {state}, block := {block}}}) (project ck seen))")
    after = lambda pos, seen, root="(project ck root)": (
        ok("Check", f"(record Check {{pos := {pos}, level := (project ck level), seen := {seen}, root := {root}, last := kind}})"))
    allknown = lambda count, start, kinds, caps="blob, seen": (
        f"(second (prim iterate (fun [{caps}] (walk : (Nat * Bool)) => (((first walk) + 1), ((second walk) && "
        f"(match (known seen blob ({start} + (prim multiply 32 (first walk) : Nat)) {blist(kinds, 'Nat')})\n"
        "      | Option.some found => true\n      | Option.none => false)))) "
        f"{count} (0, true) : (Nat * Bool)))")
    checkpoint_case = lets(
        [("s0", W16, "(words16At blob 34)"), ("m", W16, "(words16At blob 98)")],
        iff("((len =? 162) && (ivHolds s0))",
            iff("((project ck level) =? 0)", after("next", seen_row(1, "0x", "(bytesAt blob 34 64)", "(bytesAt blob 98 64)")),
                f"(match (checkSection data next (project ck level) s0 m)\n"
                f"      | Result.ok moved => {after('moved', seen_row(1, '0x', '(bytesAt blob 34 64)', '(bytesAt blob 98 64)'))}\n"
                "      | Result.error failed => (ctor Result.error [Check, TraceError] failed))"),
            bad("Check", "malformed", "pos")))
    chunk_case = lets(
        [("n", "Nat", "(rd8 blob 9)"), ("tail", "Nat", "(10 + (prim multiply 32 n : Nat))"), ("seen", "(List Seen)", "(project ck seen)")],
        iff(allknown("n", "10", ["1"]),
            iff("((rd8 blob tail) =? 1)",
                f"(match (known seen blob (prim subtract tail 32 : Nat) {blist(['1'], 'Nat')})\n"
                f"      | Option.some last => (if (sameBytes (cutAt blob (tail + 1) 32) (cutAt (replayBytes (project last state) (project last block)) 0 32)) "
                f"then {after('next', seen_row(2, '(bytesAt blob (tail + 1) 32)', '0x', '0x'))} else {bad('Check', 'link', 'pos')})\n"
                f"      | Option.none => {bad('Check', 'unknownLabel', 'pos')})",
                after("next", seen_row(2, "0x", "0x", "0x"))),
            bad("Check", "unknownLabel", "pos")))
    two = blist(["2", "3"], "Nat")
    parent_case = (
        f"(match (known (project ck seen) blob 17 {two})\n      | Option.some l => (match (known (project ck seen) blob 49 {two})\n"
        "        | Option.some rr => "
        + iff("((rd8 blob 81) =? 1)",
              f"(match (known (project ck seen) blob 82 {blist(['1'], 'Nat')})\n"
              "          | Option.some cp => "
              + iff(conj("(prim equal (project cp block) (prim append (project l cv) (project rr cv) : Bytes) : Bool)",
                         "(sameBytes (cutAt blob 114 32) (cutAt (replayBytes (project cp state) (project cp block)) 0 32))"),
                    after("next", seen_row(3, "(bytesAt blob 114 32)", "0x", "0x")), bad("Check", "link", "pos"))
              + f"\n          | Option.none => {bad('Check', 'unknownLabel', 'pos')})",
              after("next", seen_row(3, "0x", "0x", "0x")))
        + f"\n        | Option.none => {bad('Check', 'unknownLabel', 'pos')})\n      | Option.none => {bad('Check', 'unknownLabel', 'pos')})")
    output_case = (
        f"(match (known (project ck seen) blob 9 {blist(['1'], 'Nat')})\n      | Option.some cp => "
        + iff("(prim equal (bytesAt blob 41 64) (replayBytes (project cp state) (project cp block)) : Bool)",
              after("next", seen_row(4, "0x", "0x", "(bytesAt blob 41 64)")), bad("Check", "output", "pos"))
        + f"\n      | Option.none => {bad('Check', 'unknownLabel', 'pos')})")
    events_case = lets(
        [("walked", "(Nat * Bool)",
          "(second (prim iterate (fun [blob, ck] (walk : (Nat * (Nat * Bool))) => (let at : Nat := (first (second walk)); "
          "(((first walk) + 1), (if ((rd8 blob at) =? 0) then ((at + 33), ((second (second walk)) && (match (known (project ck seen) blob (at + 1) "
          + blist(["1"], "Nat") + ")\n      | Option.some found => true\n      | Option.none => false))) "
          "else ((at + 50), (second (second walk))))))) (rd64 blob 1) (0, (9, true)) : (Nat * (Nat * Bool))))")],
        iff("((second walked) && ((first walked) =? len))", after("next", seen_row(5, "0x", "0x", "0x")),
            bad("Check", "unknownLabel", "pos")))
    root_case = lets(
        [("mode", "Nat", "(rd8 blob 2)"), ("outLen", "Nat", "(rd64 blob 11)"),
         ("at", "Nat", "(if (mode =? 2) then 179 else 147)"), ("count", "Nat", "(rd64 blob at)"),
         ("seen", "(List Seen)", "(project ck seen)"),
         ("outputs", "Bytes", f"({q('concatAll')} (second (prim iterate (fun [at, blob, seen] (walk : (Nat * (List Bytes))) => "
                              "(((first walk) + 1), (match (known seen blob ((at + 8) + (prim multiply 32 (first walk) : Nat)) "
                              + blist(["4"], "Nat") + ")\n      | Option.some o => (prim append (second walk) (cons "
                              "(project o block) (nil Bytes)) : (List Bytes))\n      | Option.none => (second walk)))) "
                              "count (0, (nil Bytes)) : (Nat * (List Bytes)))))"),
         ("digestAt", "Nat", "((at + 8) + (prim multiply 32 count : Nat))")],
        iff(conj(f"(match (known seen blob 83 {two})\n      | Option.some found => true\n      | Option.none => false)",
                 f"(match (known seen blob 115 {blist(['5'], 'Nat')})\n      | Option.some found => true\n      | Option.none => false)",
                 f"(if (mode =? 2) then (match (known seen blob 147 {blist(['6'], 'Nat')})\n      | Option.some found => true\n      | Option.none => false) else true)",
                 allknown("count", "(at + 8)", ["4"], "at, blob, seen")),
            iff(conj("((digestAt + outLen) =? len)",
                     "(sameBytes (cutAt blob digestAt outLen) (cutAt outputs 0 outLen))"),
                after("next", seen_row(6, "0x", "0x", "0x"), "ds"), bad("Check", "digest", "pos")),
            bad("Check", "unknownLabel", "pos")))
    # One function per blob kind: the loop over entries calls a small
    # dispatcher, since a large function called inside a loop closure makes
    # the export certificate exceed 8 GB.
    case_params = ["data : Bytes", "ck : Check", "pos : Nat", "len : Nat", "next : Nat", "blob : Bytes", "label : Bytes",
                   "kind : Nat", "ds : Bytes"]
    call = lambda name: f"({name} data ck pos len next blob label kind ds)"
    for name, body in [("checkpointCase", checkpoint_case), ("chunkCase", chunk_case), ("parentCase", parent_case),
                       ("outputCase", output_case), ("eventsCase", events_case), ("rootCase", root_case)]:
        add(definition(name, case_params, "(Result Check TraceError)", body))
    add(definition("checkEntry", ["data : Bytes", "ck : Check"], "(Result Check TraceError)",
                   lets([("pos", "Nat", "(project ck pos)"), ("len", "Nat", "(natOfWord (rd32 data pos))"),
                         ("next", "Nat", "((pos + 68) + len)")],
                        "(match (cutAt data (pos + 4) len)\n    | Option.some blob => (match (cutAt data ((pos + 4) + len) 64)\n"
                        "      | Option.some ds => "
                        + iff("(prim equal ds (digestSetOf blob) : Bool)",
                              lets([("label", "Bytes", "(labelOf ds)"), ("kind", "Nat", "(rd8 blob 0)")],
                                   iff("(kind =? 1)", call("checkpointCase"), iff("(kind =? 2)", call("chunkCase"),
                                       iff("(kind =? 3)", call("parentCase"), iff("(kind =? 4)", call("outputCase"),
                                           iff("(kind =? 5)", call("eventsCase"), iff("(kind =? 6)", call("rootCase"),
                                               bad("Check", "malformed", "pos")))))))),
                              bad("Check", "digestSet", "pos"))
                        + f"\n      | Option.none => {bad('Check', 'malformed', 'pos')})\n    | Option.none => {bad('Check', 'malformed', 'pos')})")))
    add(definition("checkTrace", ["claimed : Bytes"], "(Result Bytes TraceError)",
                   lets([("level", "Nat", "(rd8 claimed 8)"), ("count", "Nat", "(rd64 claimed 9)")],
                        iff(conj("(sameBytes (cutAt claimed 0 8) (ctor Option.some [Bytes] 0x4233545241434531))", "(level <=? 2)",
                                 "((prim multiply count 68 : Nat) <=? (prim length claimed : Nat))"),
                            "(match (prim iterate (fun [claimed] (acc : (Result Check TraceError)) => (match acc\n"
                            "      | Result.ok ck => (checkEntry claimed ck)\n      | Result.error failed => (ctor Result.error [Check, TraceError] failed))) "
                            "count " + ok("Check", "(record Check {pos := 17, level := level, seen := (nil Seen), root := 0x, last := 0})")
                            + " : (Result Check TraceError))\n"
                            "      | Result.ok ck => " + iff("(((project ck last) =? 6) && ((project ck pos) =? (prim length claimed : Nat)))",
                                                            ok("Bytes", "(project ck root)"), bad("Bytes", "malformed", "(project ck pos)"))
                            + "\n      | Result.error failed => (ctor Result.error [Bytes, TraceError] failed))",
                            bad("Bytes", "malformed", "0")))))
    return decls


def render_reverse(core="Blake3Uor.Blake3::"):
    """Phase 1: the step machine run backwards, the cursor, and seek. Every
    name of the core is qualified; nothing here changes the core. Its roots
    run in the Prism image's engine: in the reverse project they are
    ordinary definitions (see docs/trace.md), and the image marks them."""
    q = lambda n: f"{core}{n}"
    W16, CV, OP, ROW, STEP, MACH = q("Words16"), q("Cv"), q("Op"), q("Row"), q("Step"), q("Machine")
    decls = []
    add = decls.append
    zeros = record(W16, [(f"v{i}", u32(0)) for i in range(16)])

    # Inverses. add: subtract modulo 2^32; xor: the same xor; rotate right by
    # n: rotate left by n.
    add(definition("sub32", ["left : UInt32", "right : UInt32"], "UInt32",
                   f"(match (prim checked_add ({q('widen')} left) (4294967296 : UInt64) : (Option UInt64))\n"
                   f"    | Option.some big => (match (prim checked_subtract big ({q('widen')} right) : (Option UInt64))\n"
                   f"      | Option.some diff => ({q('narrow')} diff)\n      | Option.none => (0 : UInt32))\n"
                   "    | Option.none => (0 : UInt32))"))
    add(definition("rotl", ["value : UInt32", "amount : UInt32"], "UInt32",
                   "(match (prim checked_subtract (32 : UInt32) amount : (Option UInt32))\n"
                   f"    | Option.some complement => ({q('rotateRight')} value complement amount)\n    | Option.none => value)"))
    add(definition("undo", ["kind : " + OP, "result : UInt32", "right : UInt32"], "UInt32",
                   f"(match kind\n    | {OP}.opAdd => (sub32 result right)\n    | {OP}.opAddMsg => (sub32 result right)\n"
                   f"    | {OP}.opXor => ({q('xor')} result right)\n    | {OP}.opRor => (rotl result right)\n"
                   f"    | {OP}.opXorCv => ({q('xor')} result right))"))
    add(definition("roundOf", ["k : Nat"], "Nat", "(if (k <? 784) then (prim quotient k 112 0 : Nat) else 7)", axioms=False))
    add(definition("rowFor", ["k : Nat"], ROW,
                   f"({q('rowAt')} (if (k <? 784) then (prim remainder k 112 0 : Nat) else (prim subtract k 672 : Nat)))",
                   axioms=False))
    # A step's second operand, as machineStep reads it. No step writes its
    # own source, so the operand is the same before and after the step.
    add(definition("operand", ["cv : " + CV, "m : " + W16, "st : " + W16, "k : Nat"], "UInt32",
                   lets([("row", ROW, "(rowFor k)")],
                        f"(match (project row kind)\n    | {OP}.opAdd => ({q('getWord')} st (project row source))\n"
                        f"    | {OP}.opAddMsg => ({q('getWord')} m ({q('slotWord')} (roundOf k) (project row source)))\n"
                        f"    | {OP}.opXor => ({q('getWord')} st (project row source))\n"
                        f"    | {OP}.opRor => ({q('amountWord')} (project row source))\n"
                        f"    | {OP}.opXorCv => ({q('getCv')} cv (project row source)))"), axioms=False))
    add(definition("stepForward", ["cv : " + CV, "m : " + W16, "st : " + W16, "k : Nat"], W16,
                   f"(project ({q('machineStep')} cv m (record {MACH} {{state := st, out := {zeros}, log := (nil {STEP})}}) k) state)"))
    # The state before step k, from the state after it. The feed-forward
    # steps (k >= 784) write output words, not state.
    add(definition("stepBack", ["cv : " + CV, "m : " + W16, "st : " + W16, "k : Nat"], W16,
                   lets([("row", ROW, "(rowFor k)")],
                        iff("((roundOf k) =? 7)", "st",
                            f"({q('setWord')} st (project row dest) (undo (project row kind) ({q('getWord')} st (project row dest)) "
                            "(operand cv m st k)))"))))
    # The state after round 6, from the 16 output words and the CV: the
    # upper half is out[i+8] ^ cv[i], the lower out[i] ^ that.
    upper = {i: f"({q('xor')} (project out v{8 + i}) (project cv h{i}))" for i in range(8)}
    add(definition("unfeed", ["out : " + W16, "cv : " + CV], W16,
                   record(W16, [(f"v{i}", f"({q('xor')} (project out v{i}) {upper[i]})") for i in range(8)]
                          + [(f"v{8 + i}", upper[i]) for i in range(8)])))
    add(definition("mixBack", ["cv : " + CV, "m : " + W16, "s : " + W16], W16,
                   "(second (prim iterate (fun [cv, m] (walk : (Nat * " + W16 + ")) => (((first walk) + 1), "
                   "(stepBack cv m (second walk) (prim subtract 783 (first walk) : Nat)))) 784 (0, s) : (Nat * " + W16 + ")))"))
    # One compression run backwards: from its 16 output words, its message
    # and its CV to its initial state (CV, IV words, counter, length, flags).
    add(definition("compressBack", ["out : " + W16, "cv : " + CV, "m : " + W16], W16, "(mixBack cv m (unfeed out cv))"))
    # Reversible (0) or lossy (1): a round step needs the message only for
    # addMsg; a feed-forward step needs all 16 output words.
    add(definition("stepTag", ["s : " + STEP, "kept : Nat", "messageKnown : Bool"], "Nat",
                   iff("((project s round) <? 7)",
                       f"(match (project s op)\n    | {OP}.opAddMsg => (if messageKnown then 0 else 1)\n"
                       f"    | {OP}.opAdd => 0\n    | {OP}.opXor => 0\n    | {OP}.opRor => 0\n    | {OP}.opXorCv => 0)",
                       "(if (kept <? 16) then 1 else 0)"), axioms=False))
    add(definition("wordsSame", ["a : " + W16, "b : " + W16], "Bool",
                   conj(*[f"(prim equal (project a v{i}) (project b v{i}) : Bool)" for i in range(16)]), axioms=False))

    # The cursor: an address is (compression, step, bit); a move goes to the
    # next or previous unit of a granularity (0 bit, 1 op, 2 G, 3 round,
    # 4 compression), staying put at either end of the trace.
    add(structure("Address", [("node", "Nat"), ("step", "Nat"), ("bit", "Nat")]))
    addr = lambda n, s, b: f"(record Address {{node := {n}, step := {s}, bit := {b}}})"
    here = "a"
    add(definition("stepAhead", ["a : Address", "target : Nat", "nodes : Nat"], "Address",
                   iff("(target <? 800)", addr("(project a node)", "target", "0"),
                       iff("(((project a node) + 1) <? nodes)", addr("((project a node) + 1)", "0", "0"), here)), axioms=False))
    add(definition("stepBehind", ["a : Address", "target : Nat"], "Address",
                   iff("(target <? (project a step))", addr("(project a node)", "target", "0"),
                       iff("(0 <? (project a node))", addr("(prim subtract (project a node) 1 : Nat)", "784", "0"), here)),
                   axioms=False))

    forward = {
        "0": iff("(((project a bit) + 1) <? 32)", addr("(project a node)", "(project a step)", "((project a bit) + 1)"),
                 "(stepAhead a ((project a step) + 1) nodes)"),
        "1": "(stepAhead a ((project a step) + 1) nodes)",
        "2": "(stepAhead a " + iff("((project a step) <? 784)",
                                   "(prim multiply ((prim quotient (project a step) 14 0 : Nat) + 1) 14 : Nat)", "800") + " nodes)",
        "3": "(stepAhead a " + iff("((project a step) <? 784)",
                                   "(prim multiply ((prim quotient (project a step) 112 0 : Nat) + 1) 112 : Nat)", "800") + " nodes)",
        "4": "(stepAhead a 800 nodes)",
    }
    backward = {
        "0": iff("(0 <? (project a bit))", addr("(project a node)", "(project a step)", "(prim subtract (project a bit) 1 : Nat)"),
                 iff("(0 <? (project a step))", addr("(project a node)", "(prim subtract (project a step) 1 : Nat)", "31"),
                     iff("(0 <? (project a node))", addr("(prim subtract (project a node) 1 : Nat)", "799", "31"), here))),
        "1": iff("(0 <? (project a step))", addr("(project a node)", "(prim subtract (project a step) 1 : Nat)", "0"),
                 iff("(0 <? (project a node))", addr("(prim subtract (project a node) 1 : Nat)", "799", "0"), here)),
        "2": f"(stepBehind a (groupStart a 14))",
        "3": f"(stepBehind a (groupStart a 112))",
        "4": iff("(0 <? (project a node))", addr("(prim subtract (project a node) 1 : Nat)", "0", "0"), here),
    }
    # The start of the unit before the one holding `a` (a G is 14 steps, a
    # round 112; the feed-forward, steps 784 to 799, is one unit of each).
    add(definition("groupStart", ["a : Address", "size : Nat"], "Nat",
                   lets([("own", "Nat", f"(if ((project a step) <? 784) then (prim multiply (prim quotient (project a step) size 0 : Nat) size : Nat) else 784)")],
                        iff("(own <? (project a step))", "own",
                            iff("(own =? 784)", "(prim subtract 784 size : Nat)",
                                iff("(own <? size)", "own", "(prim subtract own size : Nat)")))), axioms=False))
    body = forward["4"]
    for g in ("3", "2", "1", "0"):
        body = iff(f"(granularity =? {g})", forward[g], body)
    back_body = backward["4"]
    for g in ("3", "2", "1", "0"):
        back_body = iff(f"(granularity =? {g})", backward[g], back_body)
    add(definition("cursorMove", ["a : Address", "granularity : Nat", "backwards : Bool", "nodes : Nat"], "Address",
                   iff("backwards", back_body, body), axioms=False))

    # Roots for the tests.
    add(definition("opFromCode", ["code : Nat"], OP,
                   iff("(code =? 0)", f"(ctor {OP}.opAdd)", iff("(code =? 1)", f"(ctor {OP}.opAddMsg)",
                       iff("(code =? 2)", f"(ctor {OP}.opXor)", iff("(code =? 3)", f"(ctor {OP}.opRor)", f"(ctor {OP}.opXorCv)")))),
                   axioms=False))
    add(definition("kindCodeOf", ["kind : " + OP], "Nat",
                   "(match kind\n" + "\n".join(f"    | {OP}.{o} => {i}" for i, o in enumerate(OPS)) + ")", axioms=False))
    add(definition("stepRoundTrip", ["kind : Nat", "left : UInt32", "right : UInt32"], "Bool",
                   lets([("op", OP, "(opFromCode kind)")],
                        f"(prim equal (undo op ({q('perform')} op left right) right) left : Bool)")))
    words = lambda b: f"({q('loadBlock')} {b} 64 0)"
    add(definition("unmix", ["chain : Bytes", "block : Bytes", "out : Bytes"], "Bytes",
                   f"({q('blockBytes')} (compressBack {words('out')} ({q('keyWordsOf')} chain) {words('block')}))"))
    # One step of the agreement check, a named function: the loop closure
    # only calls it, since large calls written inside a loop closure inflate
    # the export certificate.
    add(definition("agreeStep", ["cv : " + CV, "m : " + W16, "walk : (Nat * (" + W16 + " * Bool))"], "(Nat * (" + W16 + " * Bool))",
                   lets([("k", "Nat", "(first walk)"), ("before", W16, "(first (second walk))"),
                         ("after", W16, "(stepForward cv m before k)")],
                        "((k + 1), (after, ((second (second walk)) && (wordsSame (stepBack cv m after k) before))))")))
    # Moving back one step by stepBack agrees with replaying forward from the
    # checkpoint, at every step of the compression.
    add(definition("prevAgrees", ["chain : Bytes", "block : Bytes", "lo : UInt32", "hi : UInt32", "len : UInt32", "fl : UInt32"],
                   "Bool",
                   lets([("cv", CV, f"({q('keyWordsOf')} chain)"), ("m", W16, words("block"))],
                        "(second (second (prim iterate (fun [cv, m] (walk : (Nat * (" + W16 + " * Bool))) => (agreeStep cv m walk)) "
                        f"800 (0, (({q('initialState')} cv lo hi len fl), true)) : (Nat * ({W16} * Bool)))))")))
    u8 = lambda n: f"({q('byteOf')} ({q('wordOfNat')} {n} 8))"
    add(definition("stepRecord", ["s : " + STEP, "tag : Nat"], "Bytes",
                   f"({q('concatAll')} {blist([u8('(project s round)'), u8('(project s g)'), u8('(project s index)'), u8('(kindCodeOf (project s op))'), u8('(project s dest)'), u8('(project s source)'), '(' + q('wordBytes') + ' (project s left))', '(' + q('wordBytes') + ' (project s right))', '(' + q('wordBytes') + ' (project s result))', u8('tag')])})"))
    add(definition("compressionsOf", ["trace : " + q("Trace")], f"(List {q('Node')})",
                   f"(prim list_fold (fun (acc : (List {q('Node')})) (event : {q('Event')}) => (match event\n"
                   f"    | {q('Event')}.compressed n => (cons n acc)\n    | {q('Event')}.pushed e depth => acc\n"
                   f"    | {q('Event')}.popped e depth => acc)) (nil {q('Node')}) "
                   f"(prim append (project trace prelude) (project trace events) : (List {q('Event')})) : (List {q('Node')}))",
                   axioms=False))
    # Seek: the step at (compression, step) of the trace of (mode, pieces,
    # outLen), as its 18-byte record and a reversibility tag, replayed from
    # the compression's checkpoint; empty when there is no such step.
    add(definition("seekStep", ["mode : " + q("Mode"), "pieces : (List Bytes)", "outLen : Nat", "node : Nat", "step : Nat"],
                   f"(Result Bytes {q('Blake3Error')})",
                   f"(match ({q('traceOf')} mode pieces outLen)\n"
                   f"    | Result.ok trace => (ctor Result.ok [Bytes, {q('Blake3Error')}] "
                   + lets([("nodes", f"(List {q('Node')})", "(compressionsOf trace)"), ("count", "Nat", "(prim length nodes : Nat)")],
                          iff("((node <? count) && (step <? 800))",
                              f"(match (prim index nodes (prim subtract (prim subtract count 1 : Nat) node : Nat) : (Option {q('Node')}))\n"
                              "      | Option.some n => "
                              + lets([("t", q("CompressionTrace"),
                                       f"({q('compressTrace')} (project n chain) (project n block) (project n counterLo) "
                                       "(project n counterHi) (project n blockLen) (project n flags))")],
                                     f"(match (prim index (project t log) (prim subtract 799 step : Nat) : (Option {STEP}))\n"
                                     "        | Option.some s => (stepRecord s (stepTag s (if ((project n role) =? 2) then 16 else 8) true))\n"
                                     "        | Option.none => 0x)")
                              + "\n      | Option.none => 0x)", "0x"))
                   + f")\n    | Result.error refused => (ctor Result.error [Bytes, {q('Blake3Error')}] refused))"))
    return decls


def lookup(var, lo, hi, leaf):
    """A balanced lookup of the Nat `var` over lo..hi, as render's schedule tables."""
    if lo == hi:
        return leaf(lo)
    mid = (lo + hi + 1) // 2
    return f"(if ({var} <? {mid}) then {lookup(var, lo, mid - 1, leaf)} else {lookup(var, mid, hi, leaf)})"


def schedule_rows():
    """The 128 rows of the step machine: (G, op index, kind, dest, source)."""
    (r1, _), (r2, _), (r3, _), (r4, _) = ROTATIONS
    rows = []
    for g, (a, b, c, d) in enumerate(G_LANES):
        x, y = 2 * g, 2 * g + 1
        # G: a += b; a += mx; d ^= a; d >>>= 16; c += d; b ^= c; b >>>= 12;
        # then the same with my, 8 and 7.
        for i, (kind, dest, source) in enumerate(
                [("opAdd", a, b), ("opAddMsg", a, x), ("opXor", d, a), ("opRor", d, r1), ("opAdd", c, d),
                 ("opXor", b, c), ("opRor", b, r2), ("opAdd", a, b), ("opAddMsg", a, y), ("opXor", d, a),
                 ("opRor", d, r3), ("opAdd", c, d), ("opXor", b, c), ("opRor", b, r4)]):
            rows.append((g, i, kind, dest, source))
    # The feed-forward: output word i is v[i] ^ v[i+8], or v[i] ^ cv[i-8].
    return rows + [(0, i, "opXor", i, i + 8) for i in range(8)] + [(0, 8 + i, "opXorCv", 8 + i, i) for i in range(8)]


# The inputs of one compression, as the taint map numbers them: message bits
# 0-511 (word w bit i is 32w + i), then the initial state's bits 512-1023.
# State words 8-11 are the IV, constants, so no input bit is theirs.
FULL_INIT = [0 if 8 <= i <= 11 else 0xFFFFFFFF for i in range(16)]
# Word level: input words 0-15 the message, 16-31 the initial state.
FULL_MASK = 0xFFFFFFFF & ~(0xF << 24)


def word_diffusion():
    """The first step after which every state word depends on every input
    word, by the word-level taint rule; the kernel theorem checks the
    LexLean definition against this number."""
    masks = [0 if 8 <= i <= 11 else 1 << (16 + i) for i in range(16)]
    rows = schedule_rows()
    for k in range(7 * 112):
        r, (_, _, kind, dest, source) = k // 112, rows[k % 112]
        if kind in ("opAdd", "opXor"):
            masks[dest] |= masks[source]
        elif kind == "opAddMsg":
            masks[dest] |= 1 << sigma(r)[source]
        if all(m == FULL_MASK for m in masks):
            return k
    return 800


# Chunk 0's and chunk 1's chaining values of the official 1025-byte input,
# from blake3 1.8.7's hazmat API: data for the slice theorems, which check
# them against the official hash, so this table is not trusted.
SLICE_CVS = ("5c9e654411e393d1f4bec710ccd5bc5669ab177d610a0eb691fcfee92fb4e8b1",
             "2f008e8a6979718c0c2f2fd7deb6e7fe4c1a3d8e90139c60cc163eb610ad8411")


def render_slice(core="Blake3Uor.Blake3::"):
    """Phase 2, verified slices. A slice of chunk N is the Bao layout for one
    chunk: the input length (8 bytes, little-endian), the parent nodes on
    the chunk's path from the root (64 bytes each, left CV then right CV),
    then the chunk. sliceOf reads the CVs from the trace; sliceCheck hashes
    only the chunk and the path, from the root down."""
    q = lambda n: f"{core}{n}"
    W16, CV, NODE, ERR = q("Words16"), q("Cv"), q("Node"), q("Blake3Error")
    decls = []
    add = decls.append
    zeros = record(W16, [(f"v{i}", u32(0)) for i in range(16)])
    add(inductive("SliceError", [("keyLength", ["Nat"]), ("outOfRange", []), ("header", []), ("rootLength", []),
                                 ("parent", ["Nat"]), ("size", []), ("chunk", [])]))
    add(definition("chunkCount", ["len : Nat"], "Nat",
                   "(if (len =? 0) then 1 else ((prim quotient (prim subtract len 1 : Nat) 1024 0 : Nat) + 1))", axioms=False))
    add(definition("chunkSize", ["len : Nat", "chunk : Nat"], "Nat",
                   lets([("begin", "Nat", "(prim multiply chunk 1024 : Nat)")],
                        iff("((begin + 1024) <=? len)", "1024", "(prim subtract len begin : Nat)")), axioms=False))
    # The left subtree of `size` > 1 chunks: the largest power of two below it.
    add(definition("leftSpan", ["size : Nat"], "Nat",
                   "(first (prim iterate_until (fun [size] (p : Nat) => (if ((prim multiply p 2 : Nat) <? size) then "
                   "(ctor Option.some [Nat] (prim multiply p 2 : Nat)) else (ctor Option.none [Nat]))) 64 1 : (Nat * Bool)))",
                   axioms=False))
    add(definition("chainBytes", ["c : " + CV], "Bytes", cat(*[f"({q('wordBytes')} (project c h{i}))" for i in range(8)])))
    add(definition("lengthBytes", ["len : Nat"], "Bytes",
                   cat(f"({q('wordBytes')} ({q('counterLow')} len))", f"({q('wordBytes')} ({q('counterHigh')} len))")))
    add(definition("chainsSame", ["a : " + CV, "b : " + CV], "Bool",
                   conj(*[eq(f"(project a h{i})", f"(project b h{i})") for i in range(8)]), axioms=False))
    add(definition("upperChain", ["w : " + W16], CV, record(CV, [(f"h{i}", f"(project w v{8 + i})") for i in range(8)]),
                   axioms=False))
    add(definition("treeNodes", ["trace : " + q("Trace")], f"(List {NODE})",
                   f"(prim list_fold (fun (acc : (List {NODE})) (event : {q('Event')}) => (match event\n"
                   f"    | {q('Event')}.compressed n => (cons n acc)\n    | {q('Event')}.pushed e depth => acc\n"
                   f"    | {q('Event')}.popped e depth => acc)) (nil {NODE}) (project trace events) : (List {NODE}))",
                   axioms=False))
    # Whether a node's output is the CV of the subtree of `span` chunks from
    # chunk `origin`: a chunk's last block, or a non-root parent.
    add(definition("covers", ["n : " + NODE, "origin : Nat", "span : Nat"], "Bool",
                   iff("(span =? 1)",
                       "(((project n role) =? 0) && (((project n origin) =? origin) && "
                       f"(if (prim equal (prim bit_and (project n flags) ({q('flagChunkEnd')}) : UInt32) (0 : UInt32) : Bool) "
                       "then false else true)))",
                       "(((project n role) =? 1) && (((project n origin) =? origin) && ((project n span) =? span)))")))
    add(definition("subtreeChain", [f"nodes : (List {NODE})", "origin : Nat", "span : Nat"], CV,
                   f"({q('cvOf')} (prim list_fold (fun [origin, span] (acc : {W16}) (n : {NODE}) => "
                   f"(if (covers n origin span) then (project n output) else acc)) {zeros} nodes : {W16}))"))
    # One level down the path: the parent node of the subtree (lo, size),
    # then the child holding the chunk.
    path = "(Nat * (Nat * (List Bytes)))"
    add(definition("descend", [f"nodes : (List {NODE})", "chunk : Nat", "st : " + path], f"(Option {path})",
                   lets([("lo", "Nat", "(first st)"), ("size", "Nat", "(first (second st))"),
                         ("acc", "(List Bytes)", "(second (second st))")],
                        iff("(size <=? 1)", f"(ctor Option.none [{path}])",
                            lets([("left", "Nat", "(leftSpan size)"),
                                  ("node", "Bytes", "(prim append (chainBytes (subtreeChain nodes lo left)) (chainBytes "
                                                    "(subtreeChain nodes (lo + left) (prim subtract size left : Nat))) : Bytes)")],
                                 iff("(chunk <? (lo + left))",
                                     f"(ctor Option.some [{path}] (lo, (left, (cons node acc))))",
                                     f"(ctor Option.some [{path}] ((lo + left), ((prim subtract size left : Nat), (cons node acc))))"))))))
    # The slice of chunk `chunk` of `input`; empty when there is no such chunk.
    add(definition("sliceOf", ["mode : " + q("Mode"), "input : Bytes", "chunk : Nat"], f"(Result Bytes {ERR})",
                   f"(match ({q('traceOf')} mode (cons input (nil Bytes)) 32)\n"
                   f"    | Result.ok trace => (ctor Result.ok [Bytes, {ERR}] "
                   + lets([("len", "Nat", "(prim length input : Nat)"), ("n", "Nat", "(chunkCount len)"),
                           ("nodes", f"(List {NODE})", "(treeNodes trace)")],
                          iff("(chunk <? n)",
                              lets([("parents", "(List Bytes)",
                                     f"(second (second (first (prim iterate_until (fun [chunk, nodes] (st : {path}) => "
                                     f"(descend nodes chunk st)) 64 (0, (n, (nil Bytes))) : ({path} * Bool)))))"),
                                    ("data", "Bytes", "(match (prim slice input (prim multiply chunk 1024 : Nat) (chunkSize len chunk) : "
                                                      "(Option Bytes))\n      | Option.some d => d\n      | Option.none => 0x)")],
                                   f"({q('concatAll')} (cons (lengthBytes len) (prim list_fold (fun (acc : (List Bytes)) "
                                   "(node : Bytes) => (cons node acc)) (cons data (nil Bytes)) parents : (List Bytes))))"),
                              "0x"))
                   + f")\n    | Result.error refused => (ctor Result.error [Bytes, {ERR}] refused))", production=core != ""))

    fields = ("lo", "size", "pos", "expected", "top", "level", "failed")
    add(structure("Walk", [("lo", "Nat"), ("size", "Nat"), ("pos", "Nat"), ("expected", CV), ("top", "Bool"),
                           ("level", "Nat"), ("failed", "Bool")]))
    walk = lambda **given: "(ctor Option.some [Walk] " + record("Walk", [(f, given.get(f, f"(project w {f})")) for f in fields]) + ")"
    down = dict(pos="((project w pos) + 64)", top="false", level="((project w level) + 1)")
    # One parent node, checked against the CV expected for its subtree (the
    # root hash at the top), then the child on the chunk's side.
    add(definition("checkLevel", ["key : " + CV, "base : UInt32", "slice : Bytes", "chunk : Nat", "w : Walk"], "(Option Walk)",
                   iff("(project w failed)", "(ctor Option.none [Walk])", iff("((project w size) <=? 1)", "(ctor Option.none [Walk])",
                       iff("(((project w pos) + 64) <=? (prim length slice : Nat))",
                           lets([("block", W16, f"({q('loadBlock')} slice (prim length slice : Nat) (project w pos))"),
                                 ("flags", "UInt32", f"(prim bit_or base (if (project w top) then (prim bit_or ({q('flagParent')}) "
                                                     f"({q('flagRoot')}) : UInt32) else ({q('flagParent')})) : UInt32)"),
                                 ("out", W16, f"({q('compress')} key block (0 : UInt32) (0 : UInt32) (64 : UInt32) flags)")],
                                iff(f"(chainsSame ({q('cvOf')} out) (project w expected))",
                                    lets([("left", "Nat", "(leftSpan (project w size))")],
                                         iff("(chunk <? ((project w lo) + left))",
                                             walk(size="left", expected=f"({q('cvOf')} block)", **down),
                                             walk(lo="((project w lo) + left)", size="(prim subtract (project w size) left : Nat)",
                                                  expected="(upperChain block)", **down))),
                                    walk(failed="true"))),
                           walk(failed="true"))))))
    start_walk = record("Walk", [("lo", "0"), ("size", "(chunkCount len)"), ("pos", "8"), ("expected", f"({q('keyWordsOf')} root)"),
                                 ("top", "true"), ("level", "0"), ("failed", "false")])
    fail = lambda e: f"(ctor Result.error [Bytes, SliceError] (ctor SliceError.{e}))"
    leaf = lets([("size", "Nat", "(chunkSize len chunk)"),
                 ("data", "Bytes", "(match (prim slice slice (project walked pos) (prim subtract (prim length slice : Nat) "
                                   "(project walked pos) : Nat) : (Option Bytes))\n      | Option.some d => d\n      | Option.none => 0x)")],
                iff("((prim length data : Nat) =? size)",
                    lets([("pending", q("Pending"), f"(first ({q('chunkPending')} key base data 0 size chunk (nil {q('Event')})))"),
                          ("node", NODE, f"({q('run')} (if (project walked top) then ({q('rootPending')} pending 0) else pending))")],
                         iff(f"(chainsSame ({q('cvOf')} (project node output)) (project walked expected))",
                             "(ctor Result.ok [Bytes, SliceError] data)", fail("chunk"))),
                    fail("size")))
    add(definition("checkWith", ["key : " + CV, "base : UInt32", "slice : Bytes", "chunk : Nat", "len : Nat", "root : Bytes"],
                   "(Result Bytes SliceError)",
                   iff("(chunk <? (chunkCount len))",
                       iff("((prim length root : Nat) =? 32)",
                           # The header word by word: the kernel reduces
                           # UInt32 equality, not ByteArray equality.
                           iff(conj("(8 <=? (prim length slice : Nat))",
                                    eq(f"({q('blockWord')} slice 8 0)", f"({q('counterLow')} len)"),
                                    eq(f"({q('blockWord')} slice 8 4)", f"({q('counterHigh')} len)")),
                               lets([("walked", "Walk", "(first (prim iterate_until (fun [base, chunk, key, slice] (w : Walk) => "
                                                        f"(checkLevel key base slice chunk w)) 64 {start_walk} : (Walk * Bool)))")],
                                    iff("(project walked failed)",
                                        "(ctor Result.error [Bytes, SliceError] (ctor SliceError.parent (project walked level)))", leaf)),
                               fail("header")), fail("rootLength")),
                       fail("outOfRange"))))
    # The chunk, if `slice` proves it is chunk `chunk` of an input of `len`
    # bytes whose 32-byte hash in `mode` is `root`.
    add(definition("sliceCheck", ["mode : " + q("Mode"), "slice : Bytes", "chunk : Nat", "len : Nat", "root : Bytes"],
                   "(Result Bytes SliceError)",
                   f"(match ({q('start')} mode)\n"
                   "    | Result.ok h => (checkWith (project h keyWords) (project h modeFlags) slice chunk len root)\n"
                   f"    | Result.error refused => (match refused\n"
                   f"      | {ERR}.keyLength n => (ctor Result.error [Bytes, SliceError] (ctor SliceError.keyLength n))))",
                   production=core != ""))
    return decls


def render_analysis(core="Blake3Uor.Blake3::", rev="Blake3UorReverse.Blake3Reverse::"):
    """Phase 2, difference traces and the dependency map of one compression."""
    q = lambda n: f"{core}{n}"
    b = lambda n: f"{rev}{n}"
    W16, CV, OP, ROW, STEP = q("Words16"), q("Cv"), q("Op"), q("Row"), q("Step")
    decls = []
    add = decls.append
    zeros = record(W16, [(f"v{i}", u32(0)) for i in range(16)])
    reverse = lambda t, items: (f"(prim list_fold (fun (acc : (List {t})) (x : {t}) => (cons x acc)) (nil {t}) {items} : (List {t}))")

    # Difference traces. An input is a compression's (CV, block, counter,
    # length, flags); a difference trace of two inputs is, for each of the
    # 800 steps, the xor of their left operands, right operands and results
    # (12 bytes a step, 9,600 in all).
    add(structure("Input", [("chain", "Bytes"), ("block", "Bytes"), ("lo", "UInt32"), ("hi", "UInt32"), ("len", "UInt32"),
                            ("fl", "UInt32")]))
    add(structure("Triple", [("left", "UInt32"), ("right", "UInt32"), ("result", "UInt32")]))
    chain = f"({q('keyWordsOf')} (project x chain))"
    block = f"({q('loadBlock')} (project x block) 64 0)"
    add(definition("tripleDiff", ["a : Triple", "p : Triple"], "Bytes",
                   cat(*[f"({q('wordBytes')} ({q('xor')} (project a {f}) (project p {f})))" for f in ("left", "right", "result")])))
    # Forwards: the records of the compression run forwards, oldest first.
    add(definition("forwardTriples", ["x : Input"], "(List Triple)",
                   f"(prim list_fold (fun (acc : (List Triple)) (s : {STEP}) => (cons (record Triple {{left := (project s left), "
                   "right := (project s right), result := (project s result)}) acc)) (nil Triple) "
                   f"(project ({q('compressTrace')} {chain} {block} (project x lo) (project x hi) (project x len) (project x fl)) log) "
                   ": (List Triple))"))
    # Backwards: the same records rebuilt from the 16 output words, the
    # message and the CV alone. The feed-forward reads the state after round
    # 6, which unfeed recovers; each round step is then undone from the end.
    add(definition("feedTriple", ["cv : " + CV, "m : " + W16, "s : " + W16, "k : Nat"], "Triple",
                   lets([("row", ROW, f"({b('rowFor')} k)"), ("left", "UInt32", f"({q('getWord')} s (project row dest))"),
                         ("right", "UInt32", f"({b('operand')} cv m s k)")],
                        f"(record Triple {{left := left, right := right, result := ({q('perform')} (project row kind) left right)}})")))
    walk = f"(Nat * ({W16} * (List Triple)))"
    add(definition("backStep", ["cv : " + CV, "m : " + W16, "walk : " + walk], walk,
                   lets([("k", "Nat", "(prim subtract 783 (first walk) : Nat)"), ("s", W16, "(first (second walk))"),
                         ("row", ROW, f"({b('rowFor')} k)"), ("result", "UInt32", f"({q('getWord')} s (project row dest))"),
                         ("right", "UInt32", f"({b('operand')} cv m s k)"),
                         ("left", "UInt32", f"({b('undo')} (project row kind) result right)")],
                        f"(((first walk) + 1), (({q('setWord')} s (project row dest) left), "
                        "(cons (record Triple {left := left, right := right, result := result}) (second (second walk)))))")))
    add(definition("backwardTriples", ["x : Input"], "(List Triple)",
                   lets([("cv", CV, chain), ("m", W16, block),
                         ("s", W16, f"({b('unfeed')} ({q('compress')} cv m (project x lo) (project x hi) (project x len) (project x fl)) cv)"),
                         ("feed", "(List Triple)", "(second (prim iterate (fun [cv, m, s] (walk : (Nat * (List Triple))) => "
                                                   "(((first walk) + 1), (cons (feedTriple cv m s (prim subtract 799 (first walk) : Nat)) "
                                                   "(second walk)))) 16 (0, (nil Triple)) : (Nat * (List Triple))))")],
                        f"(second (second (prim iterate (fun [cv, m] (walk : {walk}) => (backStep cv m walk)) 784 (0, (s, feed)) : {walk})))")))
    zipped = "((List Triple) * (List Bytes))"
    add(definition("tripleDiffs", ["a : (List Triple)", "p : (List Triple)"], "Bytes",
                   f"({q('concatAll')} " + reverse("Bytes", f"(second (prim list_fold (fun (acc : {zipped}) (x : Triple) => (match (first acc)\n"
                                                            "    | List.cons y rest => (rest, (cons (tripleDiff x y) (second acc)))\n"
                                                            f"    | List.nil => ((nil Triple), (second acc)))) (p, (nil Bytes)) a : {zipped}))") + ")"))
    add(definition("diffTrace", ["a : Input", "p : Input"], "Bytes", "(tripleDiffs (forwardTriples a) (forwardTriples p))"))
    add(definition("diffTraceBack", ["a : Input", "p : Input"], "Bytes", "(tripleDiffs (backwardTriples a) (backwardTriples p))"))

    # Reach: for each state word, the first step after which the two runs
    # differ in it (0 when the initial states already do; 800: never).
    add(structure("Reach", [(f"n{i}", "Nat") for i in range(16)]))
    add(definition("reachAt", ["r : Reach", "i : Nat"], "Nat", lookup("i", 0, 15, lambda n: f"(project r n{n})"), axioms=False))
    add(definition("reachSet", ["r : Reach", "i : Nat", "value : Nat"], "Reach",
                   lookup("i", 0, 15, lambda n: record("Reach", [(f"n{k}", "value" if k == n else f"(project r n{k})")
                                                                 for k in range(16)])), axioms=False))
    add(definition("reachList", ["r : Reach"], "(List Nat)", blist([f"(project r n{i})" for i in range(16)], "Nat"), axioms=False))
    rwalk = "(Nat * ((List Triple) * Reach))"
    add(definition("reachStep", ["walk : " + rwalk, "x : Triple"], rwalk,
                   lets([("k", "Nat", "(first walk)"), ("dest", "Nat", f"(project ({b('rowFor')} k) dest)"),
                         ("reach", "Reach", "(second (second walk))")],
                        "(match (first (second walk))\n    | List.cons y rest => (((first walk) + 1), (rest, "
                        + iff("((k <? 784) && (((reachAt reach dest) =? 800) && (if (prim equal (project x result) (project y result) : Bool) "
                              "then false else true)))", "(reachSet reach dest k)", "reach")
                        + "))\n    | List.nil => (((first walk) + 1), ((nil Triple), reach)))"), axioms=False))
    initial = lambda x: (f"({q('initialState')} ({q('keyWordsOf')} (project {x} chain)) (project {x} lo) (project {x} hi) "
                         f"(project {x} len) (project {x} fl))")
    add(definition("diffReach", ["a : Input", "p : Input"], "(List Nat)",
                   lets([("sa", W16, initial("a")), ("sp", W16, initial("p")),
                         ("begin", "Reach", record("Reach", [(f"n{i}", iff(eq(f"(project sa v{i})", f"(project sp v{i})"), "800", "0"))
                                                             for i in range(16)]))],
                        f"(reachList (second (second (prim list_fold (fun (walk : {rwalk}) (x : Triple) => (reachStep walk x)) "
                        f"(0, ((forwardTriples p), begin)) (forwardTriples a) : {rwalk}))))")))

    # The dependency map: for each bit of each step's result, the set of
    # input bits with a path to it in the bit-level circuit (structural
    # taint). The rules: bit i of a sum depends on bits 0..i of both
    # operands (the carry chain); of an xor, on bit i of both; of a
    # right-rotation by n, on bit i+n mod 32. A set is 1024 bits: Bits.msg
    # holds message bit 32w+i at word w bit i, Bits.init the initial state's.
    add(structure("Bits", [("msg", W16), ("init", W16)]))
    T = "(List Bits)"
    add(structure("Taint16", [(f"t{i}", T) for i in range(16)]))
    add(structure("TaintState", [("state", "Taint16"), ("out", "Taint16")]))
    add(definition("or16", ["a : " + W16, "p : " + W16], W16,
                   record(W16, [(f"v{i}", f"(prim bit_or (project a v{i}) (project p v{i}) : UInt32)") for i in range(16)])))
    add(definition("bitsUnion", ["a : Bits", "p : Bits"], "Bits",
                   "(record Bits {msg := (or16 (project a msg) (project p msg)), init := (or16 (project a init) (project p init))})"))
    add(definition("noBits", [], "Bits", f"(record Bits {{msg := {zeros}, init := {zeros}}})", axioms=False))
    full = record(W16, [(f"v{i}", u32(v)) for i, v in enumerate(FULL_INIT)])
    ones = record(W16, [(f"v{i}", u32(0xFFFFFFFF)) for i in range(16)])
    add(definition("allBits", [], "Bits", f"(record Bits {{msg := {ones}, init := {full}}})", axioms=False))
    add(definition("bitsSame", ["a : Bits", "p : Bits"], "Bool",
                   conj(*[eq(f"(project (project a {h}) v{i})", f"(project (project p {h}) v{i})") for h in ("msg", "init")
                          for i in range(16)]), axioms=False))
    add(definition("oneBit", ["inMessage : Bool", "w : Nat", "i : Nat"], "Bits",
                   lets([("single", W16, f"({q('setWord')} {zeros} w ({q('shiftLeft')} (1 : UInt32) ({q('wordOfNat')} i 5)))")],
                        iff("inMessage", f"(record Bits {{msg := single, init := {zeros}}})",
                            f"(record Bits {{msg := {zeros}, init := single}})"))))
    # The 32 singleton sets of input word w (bit 0 first).
    add(definition("inputWord", ["inMessage : Bool", "w : Nat"], T,
                   f"(second (prim iterate (fun [inMessage, w] (walk : (Nat * {T})) => (((first walk) + 1), (cons (oneBit inMessage w "
                   f"(prim subtract 31 (first walk) : Nat)) (second walk)))) 32 (0, (nil Bits)) : (Nat * {T})))"))
    add(definition("blankWord", [], T,
                   f"(second (prim iterate (fun (walk : (Nat * {T})) => (((first walk) + 1), (cons (noBits) (second walk)))) 32 "
                   f"(0, (nil Bits)) : (Nat * {T})))", axioms=False))
    pair = f"({T} * {T})"
    add(definition("taintXor", ["a : " + T, "p : " + T], T,
                   reverse("Bits", f"(second (prim list_fold (fun (acc : {pair}) (x : Bits) => (match (first acc)\n"
                                   "    | List.cons y rest => (rest, (cons (bitsUnion x y) (second acc)))\n"
                                   f"    | List.nil => ((nil Bits), (second acc)))) (p, (nil Bits)) a : {pair}))")))
    carry = f"({T} * (Bits * {T}))"
    add(definition("taintAdd", ["a : " + T, "p : " + T], T,
                   reverse("Bits", f"(second (second (prim list_fold (fun (acc : {carry}) (x : Bits) => (match (first acc)\n"
                                   "    | List.cons y rest => (let carried : Bits := (bitsUnion (first (second acc)) (bitsUnion x y)); "
                                   "(rest, (carried, (cons carried (second (second acc))))))\n"
                                   f"    | List.nil => ((nil Bits), (second acc)))) (p, ((noBits), (nil Bits))) a : {carry})))")))
    add(definition("taintRor", ["a : " + T, "n : Nat"], T,
                   f"(second (prim iterate (fun [a, n] (walk : (Nat * {T})) => (((first walk) + 1), (cons (match (prim index a "
                   "(prim remainder ((prim subtract 31 (first walk) : Nat) + n) 32 0 : Nat) : (Option Bits))\n"
                   f"    | Option.some x => x\n    | Option.none => (noBits)) (second walk)))) 32 (0, (nil Bits)) : (Nat * {T})))",
                   axioms=False))
    add(definition("amountNat", ["source : Nat"], "Nat", lookup("source", 7, 16, str), axioms=False))
    add(definition("taintAt", ["t : Taint16", "i : Nat"], T, lookup("i", 0, 15, lambda n: f"(project t t{n})"), axioms=False))
    add(definition("taintSet", ["t : Taint16", "i : Nat", "value : " + T], "Taint16",
                   lookup("i", 0, 15, lambda n: record("Taint16", [(f"t{k}", "value" if k == n else f"(project t t{k})")
                                                                   for k in range(16)])), axioms=False))
    add(definition("taintResult", ["ts : TaintState", "k : Nat"], T,
                   lets([("row", ROW, f"({b('rowFor')} k)"), ("st", "Taint16", "(project ts state)"),
                         ("left", T, "(taintAt st (project row dest))")],
                        f"(match (project row kind)\n    | {OP}.opAdd => (taintAdd left (taintAt st (project row source)))\n"
                        f"    | {OP}.opAddMsg => (taintAdd left (inputWord true ({q('slotWord')} ({b('roundOf')} k) (project row source))))\n"
                        f"    | {OP}.opXor => (taintXor left (taintAt st (project row source)))\n"
                        f"    | {OP}.opRor => (taintRor left (amountNat (project row source)))\n"
                        f"    | {OP}.opXorCv => (taintXor left (inputWord false (project row source))))")))
    add(definition("taintApply", ["ts : TaintState", "k : Nat", "result : " + T], "TaintState",
                   lets([("dest", "Nat", f"(project ({b('rowFor')} k) dest)")],
                        iff(f"(({b('roundOf')} k) =? 7)",
                            "(record TaintState {state := (project ts state), out := (taintSet (project ts out) dest result)})",
                            "(record TaintState {state := (taintSet (project ts state) dest result), out := (project ts out)})")),
                   axioms=False))
    add(definition("taintStart", [], "TaintState",
                   lets([("blank", T, "(blankWord)")],
                        "(record TaintState {state := " + record("Taint16", [(f"t{i}", "blank" if 8 <= i <= 11 else f"(inputWord false {i})")
                                                                             for i in range(16)])
                        + ", out := " + record("Taint16", [(f"t{i}", "blank") for i in range(16)]) + "})")))
    add(definition("bitsBytes", ["x : Bits"], "Bytes",
                   f"(prim append ({q('blockBytes')} (project x msg)) ({q('blockBytes')} (project x init)) : Bytes)"))
    add(definition("taintBytes", ["t : " + T], "Bytes",
                   f"({q('concatAll')} " + reverse("Bytes", "(prim list_fold (fun (acc : (List Bytes)) (x : Bits) => "
                                                            "(cons (bitsBytes x) acc)) (nil Bytes) t : (List Bytes))") + ")"))
    twalk = "(Nat * (TaintState * (List Bytes)))"
    add(definition("taintWalk", ["begin : Nat", "walk : " + twalk], twalk,
                   lets([("k", "Nat", "(first walk)"), ("ts", "TaintState", "(first (second walk))"),
                         ("result", T, "(taintResult ts k)")],
                        "(((first walk) + 1), ((taintApply ts k result), (if (begin <=? k) then (cons (taintBytes result) "
                        "(second (second walk))) else (second (second walk)))))")))
    # The map of round r (0-6; 7 is the feed-forward): for each of its steps
    # in order, the 32 sets of the step's result, bit 0 first, 128 bytes a
    # set (the message words, then the initial state's, little-endian).
    add(definition("taintRound", ["r : Nat"], "Bytes",
                   lets([("begin", "Nat", "(if (r <? 7) then (prim multiply r 112 : Nat) else 784)"),
                         ("finish", "Nat", "(if (r <? 7) then (prim multiply (r + 1) 112 : Nat) else 800)")],
                        f"({q('concatAll')} " + reverse("Bytes", f"(second (second (prim iterate (fun [begin] (walk : {twalk}) => "
                                                                 f"(taintWalk begin walk)) finish (0, ((taintStart), (nil Bytes))) : {twalk})))")
                        + ")")))
    add(definition("wordFull", ["t : " + T], "Bool",
                   "(prim list_fold (fun (ok : Bool) (x : Bits) => (ok && (bitsSame x (allBits)))) true t : Bool)", axioms=False))
    fwalk = "(Nat * (TaintState * Reach))"
    add(definition("fullWalk", ["walk : " + fwalk], fwalk,
                   lets([("k", "Nat", "(first walk)"), ("ts", "TaintState", "(first (second walk))"),
                         ("reach", "Reach", "(second (second walk))"), ("result", T, "(taintResult ts k)"),
                         ("dest", "Nat", f"(project ({b('rowFor')} k) dest)")],
                        "(((first walk) + 1), ((taintApply ts k result), "
                        + iff("(((reachAt reach dest) =? 800) && (wordFull result))", "(reachSet reach dest k)", "reach") + "))")))
    # For each state word, the first round step after which every bit of it
    # depends on every input bit (800: never). A full word stays full, since
    # every rule keeps all of a full operand.
    add(definition("fullTaintSteps", [], "(List Nat)",
                   f"(reachList (second (second (prim iterate (fun (walk : {fwalk}) => (fullWalk walk)) 784 (0, ((taintStart), "
                   + record("Reach", [(f"n{i}", "800") for i in range(16)]) + f")) : {fwalk}))))"))

    # The same map at word level: a state word's set of input words (bit w:
    # message word w; bit 16 + w: initial state word w) as one UInt32.
    add(definition("maskOf", ["w : Nat"], "UInt32", f"({q('shiftLeft')} (1 : UInt32) ({q('wordOfNat')} w 5))"))
    add(definition("maskStep", ["masks : " + W16, "k : Nat"], W16,
                   lets([("row", ROW, f"({b('rowFor')} k)")],
                        f"({q('setWord')} masks (project row dest) (prim bit_or ({q('getWord')} masks (project row dest)) (match (project row kind)\n"
                        f"    | {OP}.opAdd => ({q('getWord')} masks (project row source))\n"
                        f"    | {OP}.opAddMsg => (maskOf ({q('slotWord')} ({b('roundOf')} k) (project row source)))\n"
                        f"    | {OP}.opXor => ({q('getWord')} masks (project row source))\n    | {OP}.opRor => (0 : UInt32)\n"
                        f"    | {OP}.opXorCv => (maskOf ((project row source) + 16))) : UInt32))")))
    add(definition("masksFull", ["masks : " + W16], "Bool",
                   conj(*[eq(f"(project masks v{i})", u32(FULL_MASK)) for i in range(16)]), axioms=False))
    mwalk = f"(Nat * ({W16} * Nat))"
    add(definition("maskWalk", ["walk : " + mwalk], mwalk,
                   lets([("k", "Nat", "(first walk)"), ("masks", W16, "(maskStep (first (second walk)) k)")],
                        "(((first walk) + 1), (masks, " + iff("(((second (second walk)) =? 800) && (masksFull masks))", "k",
                                                              "(second (second walk))") + "))")))
    start = record(W16, [(f"v{i}", u32(0 if 8 <= i <= 11 else 1 << (16 + i))) for i in range(16)])
    # The first round step after which every state word depends on every
    # input word (800: never).
    add(definition("wordDiffusion", [], "Nat",
                   f"(second (second (prim iterate (fun (walk : {mwalk}) => (maskWalk walk)) 784 (0, ({start}, 800)) : {mwalk})))"))
    return decls


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

    # The word trace. A compression is a step machine run over a fixed
    # schedule: 7 rounds x 8 G x 14 steps, then the 16 feed-forward xors.
    # Every step is a Step record with its address (round, G, op index), its
    # kind, where it writes, where its second operand comes from, both
    # operands and its result. A compression's output is read back from the
    # feed-forward steps of its trace, so the trace is the computation, not a
    # log beside it. Kinds are the 2026-10-23 HashSmash ledger's (add, addMsg,
    # xor, ror, xorCv). The schedule is a balanced lookup, so the program, and
    # every certificate over it, stays small.
    add(inductive("Op", [(o, []) for o in OPS]))
    add(structure("Step", [("round", "Nat"), ("g", "Nat"), ("index", "Nat"), ("op", "Op"), ("dest", "Nat"),
                           ("source", "Nat"), ("left", "UInt32"), ("right", "UInt32"), ("result", "UInt32")]))
    add(structure("Row", [("g", "Nat"), ("index", "Nat"), ("kind", "Op"), ("dest", "Nat"), ("source", "Nat")]))
    add(structure("Machine", [("state", "Words16"), ("out", "Words16"), ("log", "(List Step)")]))
    add(structure("CompressionTrace", [("chain", "Cv"), ("block", "Words16"), ("counterLo", "UInt32"),
                                       ("counterHi", "UInt32"), ("blockLen", "UInt32"), ("flags", "UInt32"),
                                       ("log", "(List Step)")]))
    add(definition("rotr", ["value : UInt32", "amount : UInt32"], "UInt32",
                   "(match (prim checked_subtract (32 : UInt32) amount : (Option UInt32))\n"
                   "    | Option.some complement => (rotateRight value amount complement)\n"
                   "    | Option.none => value)"))
    add(definition("perform", ["kind : Op", "left : UInt32", "right : UInt32"], "UInt32",
                   "(match kind\n    | Op.opAdd => (add left right)\n    | Op.opAddMsg => (add left right)\n"
                   "    | Op.opXor => (xor left right)\n    | Op.opRor => (rotr left right)\n"
                   "    | Op.opXorCv => (xor left right))"))

    def tree(var, lo, hi, leaf):
        if lo == hi:
            return leaf(lo)
        mid = (lo + hi + 1) // 2
        return f"(if ({var} <? {mid}) then {tree(var, lo, mid - 1, leaf)} else {tree(var, mid, hi, leaf)})"

    schedule = schedule_rows()
    add(definition("rowAt", ["k : Nat"], "Row",
                   tree("k", 0, len(schedule) - 1, lambda n: (
                       f"(record Row {{g := {schedule[n][0]}, index := {schedule[n][1]}, kind := (ctor Op.{schedule[n][2]}), "
                       f"dest := {schedule[n][3]}, source := {schedule[n][4]}}})")), axioms=False))
    add(definition("getWord", ["w : Words16", "i : Nat"], "UInt32",
                   tree("i", 0, 15, lambda n: f"(project w v{n})"), axioms=False))
    add(definition("setWord", ["w : Words16", "i : Nat", "value : UInt32"], "Words16",
                   tree("i", 0, 15, lambda n: record("Words16", [(f"v{k}", "value" if k == n else f"(project w v{k})")
                                                                for k in range(16)])), axioms=False))
    add(definition("getCv", ["c : Cv", "i : Nat"], "UInt32", tree("i", 0, 7, lambda n: f"(project c h{n})"), axioms=False))

    # Message slot `slot` of round r holds block word sigma_r(slot): the
    # permutation applied r times, as one table.
    add(definition("slotWord", ["r : Nat", "slot : Nat"], "Nat",
                   "(let at : Nat := ((prim multiply r 16 : Nat) + slot); "
                   + tree("at", 0, 16 * ROUNDS - 1, lambda n: str(sigma(n // 16)[n % 16])) + ")", axioms=False))
    add(definition("amountWord", ["source : Nat"], "UInt32",
                   tree("source", 7, 16, lambda n: u32(n)), axioms=False))
    # One step of the machine: step k of the schedule (k < 784 a round step,
    # round k / 112; from 784 the feed-forward, round 7).
    add(definition("machineStep", ["cv : Cv", "m : Words16", "mach : Machine", "k : Nat"], "Machine",
                   "(let r : Nat := (if (k <? 784) then (prim quotient k 112 0 : Nat) else 7); "
                   "(let row : Row := (rowAt (if (k <? 784) then (prim remainder k 112 0 : Nat) else (prim subtract k 672 : Nat))); "
                   "(let st : Words16 := (project mach state); (let left : UInt32 := (getWord st (project row dest)); "
                   "(let right : UInt32 := (match (project row kind)\n"
                   "    | Op.opAdd => (getWord st (project row source))\n"
                   "    | Op.opAddMsg => (getWord m (slotWord r (project row source)))\n"
                   "    | Op.opXor => (getWord st (project row source))\n"
                   "    | Op.opRor => (amountWord (project row source))\n"
                   "    | Op.opXorCv => (getCv cv (project row source))); "
                   "(let result : UInt32 := (perform (project row kind) left right); "
                   "(record Machine {state := (if (r =? 7) then st else (setWord st (project row dest) result)), "
                   "out := (if (r =? 7) then (setWord (project mach out) (project row dest) result) else (project mach out)), "
                   "log := (cons (record Step {round := r, g := (project row g), index := (project row index), op := (project row kind), "
                   "dest := (project row dest), source := (project row source), left := left, right := right, result := result}) "
                   "(project mach log))})))))))"))
    add(definition("permute", ["m : Words16"], "Words16",
                   record("Words16", [(f"v{i}", f"(project m v{p})") for i, p in enumerate(PERMUTATION)]),
                   axioms=False))
    add(definition("iv", [], "Cv", cv_literal(IV), axioms=False))
    for name, value in FLAGS:
        add(definition(name, [], "UInt32", u32(value), axioms=False))
    start = record("Words16", [(f"v{i}", f"(project cv h{i})") for i in range(8)]
                   + [(f"v{8 + i}", u32(IV[i])) for i in range(4)]
                   + [("v12", "counterLo"), ("v13", "counterHi"), ("v14", "blockLen"), ("v15", "flags")])
    zeros = record("Words16", [(f"v{i}", u32(0)) for i in range(16)])
    add(definition("initialState", ["cv : Cv", "counterLo : UInt32", "counterHi : UInt32", "blockLen : UInt32",
                                    "flags : UInt32"], "Words16", start, axioms=False))
    # The log is newest first: the feed-forward's last step at its head.
    add(definition("compressTrace", ["cv : Cv", "m : Words16", "counterLo : UInt32", "counterHi : UInt32",
                                     "blockLen : UInt32", "flags : UInt32"], "CompressionTrace",
                   "(let ran : (Nat * Machine) := (prim iterate (fun [cv, m] (walk : (Nat * Machine)) => "
                   "(((first walk) + 1), (machineStep cv m (second walk) (first walk)))) "
                   f"{ROUNDS * 112 + 16} (0, (record Machine {{state := (initialState cv counterLo counterHi blockLen flags), "
                   f"out := {zeros}, log := (nil Step)}})) : (Nat * Machine)); "
                   "(record CompressionTrace {chain := cv, block := m, counterLo := counterLo, counterHi := counterHi, "
                   "blockLen := blockLen, flags := flags, log := (project (second ran) log)}))"))
    # The output: each feed-forward step's result, at its output word.
    add(definition("compressOut", ["t : CompressionTrace"], "Words16",
                   "(prim list_fold (fun (out : Words16) (s : Step) => (if ((project s round) =? 7) then "
                   f"(setWord out (project s dest) (project s result)) else out)) {zeros} (project t log) : Words16)",
                   axioms=False))
    add(definition("compress", ["cv : Cv", "m : Words16", "counterLo : UInt32", "counterHi : UInt32",
                                "blockLen : UInt32", "flags : UInt32"], "Words16",
                   "(compressOut (compressTrace cv m counterLo counterHi blockLen flags))"))
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

    # The tree trace. A Pending is a compression waiting to run (the reference
    # implementation's Output); running it gives a Node, which records the
    # compression's inputs, its place in the tree and its output. Roles: 0 a
    # chunk block, 1 a parent, 2 a root output block. An Entry is a CV on the
    # stack with the chunks it covers. Events are the hasher's log, in order:
    # every compression and every stack push and pop.
    add(structure("Entry", [("cv", "Cv"), ("origin", "Nat"), ("span", "Nat")]))
    place = [("role", "Nat"), ("origin", "Nat"), ("span", "Nat"), ("index", "Nat"), ("offset", "Nat")]
    add(structure("Pending", place + [("chain", "Cv"), ("block", "Words16"), ("counter", "Nat"),
                                      ("blockLen", "UInt32"), ("flags", "UInt32")]))
    add(structure("Node", place + [("chain", "Cv"), ("block", "Words16"), ("counterLo", "UInt32"),
                                   ("counterHi", "UInt32"), ("blockLen", "UInt32"), ("flags", "UInt32"),
                                   ("output", "Words16")]))
    add(inductive("Event", [("compressed", ["Node"]), ("pushed", ["Entry", "Nat"]), ("popped", ["Entry", "Nat"])]))
    # `log` is newest first; `prelude` is a finished trace's events in order
    # (the context hash of derive_key) that precede this hasher's own.
    add(structure("Hasher", [("keyWords", "Cv"), ("modeFlags", "UInt32"), ("chunkCount", "Nat"),
                             ("pending", "Bytes"), ("stack", "(List Entry)"), ("log", "(List Event)"),
                             ("prelude", "(List Event)")]))
    add(structure("Trace", [("mode", "Nat"), ("inputLen", "Nat"), ("outLen", "Nat"), ("keyWords", "Cv"),
                            ("modeFlags", "UInt32"), ("prelude", "(List Event)"), ("events", "(List Event)")]))
    add(structure("Merge", [("fresh", "Entry"), ("total", "Nat"), ("stack", "(List Entry)"),
                            ("log", "(List Event)")]))
    add(inductive("Mode", [("hashMode", []), ("keyedMode", ["Bytes"]), ("deriveMode", ["Bytes"])]))
    add(inductive("Blake3Error", [("keyLength", ["Nat"])]))

    add(definition("run", ["p : Pending"], "Node",
                   "(let lo : UInt32 := (counterLow (project p counter)); "
                   "(let hi : UInt32 := (counterHigh (project p counter)); "
                   "(record Node {role := (project p role), origin := (project p origin), span := (project p span), "
                   "index := (project p index), offset := (project p offset), chain := (project p chain), "
                   "block := (project p block), counterLo := lo, counterHi := hi, blockLen := (project p blockLen), "
                   "flags := (project p flags), output := (compress (project p chain) (project p block) lo hi "
                   "(project p blockLen) (project p flags))})))"))
    add(definition("rootPending", ["p : Pending", "index : Nat"], "Pending",
                   "(record Pending {role := 2, origin := (project p origin), span := (project p span), "
                   "index := index, offset := (project p offset), chain := (project p chain), block := (project p block), "
                   "counter := index, blockLen := (project p blockLen), "
                   "flags := (prim bit_or (project p flags) (flagRoot) : UInt32)})"))
    add(definition("blockPending", ["key : Cv", "base : UInt32", "message : Bytes", "limit : Nat", "begin : Nat",
                                    "chunk : Nat", "pos : Nat", "chain : Cv", "len : Nat", "flags : UInt32"],
                   "Pending",
                   "(record Pending {role := 0, origin := chunk, span := 1, index := (prim quotient pos 64 0 : Nat), "
                   "offset := ((prim multiply chunk 1024 : Nat) + pos), chain := chain, "
                   "block := (loadBlock message limit (begin + pos)), counter := chunk, "
                   "blockLen := (wordOfNat len 7), flags := (prim bit_or base flags : UInt32)})"))
    # One chunk of `len` bytes (at most 1024) at `begin` of `message`: every
    # block but the last runs and is logged; the last is returned pending.
    add(definition("chunkPending", ["key : Cv", "base : UInt32", "message : Bytes", "begin : Nat",
                                    "len : Nat", "chunk : Nat", "log : (List Event)"], "(Pending * (List Event))",
                   "(let limit : Nat := (begin + len); (let last : Nat := (lastOffset len); "
                   "(let walked : (Nat * (Cv * (List Event))) := (prim iterate (fun [base, begin, chunk, key, limit, message] "
                   "(walk : (Nat * (Cv * (List Event)))) => (let pos : Nat := (first walk); "
                   "(let node : Node := (run (blockPending key base message limit begin chunk pos (first (second walk)) 64 "
                   "(startFlag pos))); ((pos + 64), ((cvOf (project node output)), "
                   "(cons (ctor Event.compressed node) (second (second walk)))))))) "
                   "(prim quotient last 64 0 : Nat) (0, (key, log)) : (Nat * (Cv * (List Event)))); "
                   "((blockPending key base message limit begin chunk last (first (second walked)) "
                   "(prim subtract len last : Nat) (prim bit_or (flagChunkEnd) (startFlag last) : UInt32)), "
                   "(second (second walked))))))"))
    parent_block = record("Words16", [(f"v{i}", f"(project (project left cv) h{i})") for i in range(8)]
                          + [(f"v{8 + i}", f"(project (project right cv) h{i})") for i in range(8)])
    add(definition("parentPending", ["left : Entry", "right : Entry", "key : Cv", "base : UInt32"], "Pending",
                   "(record Pending {role := 1, origin := (project left origin), "
                   "span := ((project left span) + (project right span)), index := 0, offset := 0, chain := key, "
                   f"block := {parent_block}, counter := 0, blockLen := (64 : UInt32), "
                   "flags := (prim bit_or base (flagParent) : UInt32)})"))
    # The reference stack merge: after chunk `total` completes, merge once per
    # trailing zero bit of `total`, logging each pop and parent compression.
    # A Nat below 2^64 has at most 64 trailing zeros.
    add(definition("mergeStep", ["key : Cv", "base : UInt32", "merging : Merge"], "(Option Merge)",
                   "(if ((prim remainder (project merging total) 2 0 : Nat) =? 0) then "
                   "(match (project merging stack)\n"
                   "    | List.cons top rest => (let node : Node := (run (parentPending top (project merging fresh) key base)); "
                   "(ctor Option.some [Merge] (record Merge {fresh := (record Entry {cv := (cvOf (project node output)), "
                   "origin := (project top origin), span := ((project top span) + (project (project merging fresh) span))}), "
                   "total := (prim quotient (project merging total) 2 0 : Nat), stack := rest, "
                   "log := (cons (ctor Event.compressed node) (cons (ctor Event.popped top "
                   "(prim length (project merging stack) : Nat)) (project merging log)))})))\n"
                   "    | List.nil => (ctor Option.none [Merge])) else (ctor Option.none [Merge]))"))
    add(definition("pushChunk", ["key : Cv", "base : UInt32", "merging : Merge"], "((List Entry) * (List Event))",
                   "(let merged : Merge := (first (prim iterate_until (fun [base, key] (mg : Merge) => "
                   "(mergeStep key base mg)) 64 merging : (Merge * Bool))); "
                   "(let grown : (List Entry) := (cons (project merged fresh) (project merged stack)); "
                   "(grown, (cons (ctor Event.pushed (project merged fresh) (prim length grown : Nat)) "
                   "(project merged log)))))"))
    add(definition("hasherWith", ["key : Cv", "base : UInt32"], "Hasher",
                   "(record Hasher {keyWords := key, modeFlags := base, chunkCount := 0, pending := 0x, "
                   "stack := (nil Entry), log := (nil Event), prelude := (nil Event)})", axioms=False))
    # Commit every chunk except the last 1 to 1024 bytes, which stay pending.
    add(definition("update", ["state : Hasher", "input : Bytes"], "Hasher",
                   "(let joined : Bytes := (prim append (project state pending) input : Bytes); "
                   "(let size : Nat := (prim length joined : Nat); "
                   "(let full : Nat := (if (size =? 0) then 0 else (prim quotient (prim subtract size 1 : Nat) 1024 0 : Nat)); "
                   "(let base : UInt32 := (project state modeFlags); (let key : Cv := (project state keyWords); "
                   "(let committed : (Nat * ((List Entry) * (List Event))) := (prim iterate (fun [base, joined, key, state] "
                   "(progress : (Nat * ((List Entry) * (List Event)))) => "
                   "(let chunk : Nat := ((project state chunkCount) + (first progress)); "
                   "(let walked : (Pending * (List Event)) := (chunkPending key base joined "
                   "(prim multiply (first progress) 1024 : Nat) 1024 chunk (second (second progress))); "
                   "(let node : Node := (run (first walked)); "
                   "(((first progress) + 1), (pushChunk key base (record Merge {fresh := (record Entry "
                   "{cv := (cvOf (project node output)), origin := chunk, span := 1}), total := (chunk + 1), "
                   "stack := (first (second progress)), log := (cons (ctor Event.compressed node) (second walked))}))))))) "
                   "full (0, ((project state stack), (project state log))) : (Nat * ((List Entry) * (List Event)))); "
                   "(let kept : Nat := (prim multiply full 1024 : Nat); "
                   "(record Hasher {keyWords := key, modeFlags := base, chunkCount := ((project state chunkCount) + full), "
                   "pending := (match (prim slice joined kept (prim subtract size kept : Nat) : (Option Bytes))\n"
                   "    | Option.some rest => rest\n    | Option.none => 0x), stack := (first (second committed)), "
                   "log := (second (second committed)), prelude := (project state prelude)}))))))))"))
    add(definition("reverseEvents", ["events : (List Event)"], "(List Event)",
                   "(prim list_fold (fun (reversed : (List Event)) (event : Event) => (cons event reversed)) "
                   "(nil Event) events : (List Event))", axioms=False))
    add(definition("modeOf", ["base : UInt32"], "Nat",
                   "(if (prim equal base (flagKeyedHash) : Bool) then 1 else (if (prim equal base (flagDeriveKeyMaterial) : Bool) "
                   "then 2 else (if (prim equal base (flagDeriveKeyContext) : Bool) then 3 else 0)))", axioms=False))
    # Fold the stack into the pending node, top first (each fold runs the
    # pending node without ROOT, pops, and makes the parent pending), then run
    # the root node once per output block with ROOT and counter = block.
    add(definition("finalTrace", ["state : Hasher", "outLen : Nat"], "Trace",
                   "(let key : Cv := (project state keyWords); (let base : UInt32 := (project state modeFlags); "
                   "(let tail : Bytes := (project state pending); "
                   "(let walked : (Pending * (List Event)) := (chunkPending key base tail 0 (prim length tail : Nat) "
                   "(project state chunkCount) (project state log)); "
                   "(let folded : (Pending * ((List Event) * Nat)) := (prim list_fold (fun [base, key] "
                   "(acc : (Pending * ((List Event) * Nat))) (entry : Entry) => (let node : Node := (run (first acc)); "
                   "((parentPending entry (record Entry {cv := (cvOf (project node output)), origin := (project (first acc) origin), "
                   "span := (project (first acc) span)}) key base), ((cons (ctor Event.popped entry (second (second acc))) "
                   "(cons (ctor Event.compressed node) (first (second acc)))), (prim subtract (second (second acc)) 1 : Nat))))) "
                   "((first walked), ((second walked), (prim length (project state stack) : Nat))) (project state stack) "
                   ": (Pending * ((List Event) * Nat))); "
                   "(let produced : (Nat * (List Event)) := (prim iterate (fun [folded] (emitted : (Nat * (List Event))) => "
                   "(((first emitted) + 1), (cons (ctor Event.compressed (run (rootPending (first folded) (first emitted)))) "
                   "(second emitted)))) (prim quotient (outLen + 63) 64 0 : Nat) (0, (first (second folded))) "
                   ": (Nat * (List Event))); "
                   "(record Trace {mode := (modeOf base), inputLen := ((prim multiply (project state chunkCount) 1024 : Nat) "
                   "+ (prim length tail : Nat)), outLen := outLen, keyWords := key, modeFlags := base, "
                   "prelude := (project state prelude), events := (reverseEvents (second produced))})))))))"))
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
    # Concatenation by pairs, so n pieces cost n log n copied bytes, not n^2.
    add(definition("pairUp", ["pieces : (List Bytes)"], "(List Bytes)",
                   "(let paired : ((Option Bytes) * (List Bytes)) := (prim list_fold (fun (acc : ((Option Bytes) * (List Bytes))) "
                   "(piece : Bytes) => (match (first acc)\n"
                   "    | Option.some held => ((ctor Option.none [Bytes]), (cons (prim append held piece : Bytes) (second acc)))\n"
                   "    | Option.none => ((ctor Option.some [Bytes] piece), (second acc)))) "
                   "((ctor Option.none [Bytes]), (nil Bytes)) pieces : ((Option Bytes) * (List Bytes))); "
                   "(let whole : (List Bytes) := (match (first paired)\n"
                   "    | Option.some held => (cons held (second paired))\n    | Option.none => (second paired)); "
                   "(prim list_fold (fun (reversed : (List Bytes)) (piece : Bytes) => (cons piece reversed)) "
                   "(nil Bytes) whole : (List Bytes))))", axioms=False))
    add(definition("concatAll", ["pieces : (List Bytes)"], "Bytes",
                   "(match (first (prim iterate_until (fun (level : (List Bytes)) => "
                   "(if ((prim length level : Nat) <=? 1) then (ctor Option.none [(List Bytes)]) else "
                   "(ctor Option.some [(List Bytes)] (pairUp level)))) 64 pieces : ((List Bytes) * Bool)))\n"
                   "    | List.cons whole rest => whole\n    | List.nil => 0x)", axioms=False))
    # The digest is the trace's root output blocks, in order, cut to outLen.
    add(definition("digestOf", ["trace : Trace"], "Bytes",
                   "(let blocks : (List Bytes) := (prim list_fold (fun (acc : (List Bytes)) (event : Event) => "
                   "(match event\n    | Event.compressed node => (if ((project node role) =? 2) then "
                   "(cons (blockBytes (project node output)) acc) else acc)\n"
                   "    | Event.pushed entry depth => acc\n    | Event.popped entry depth => acc)) "
                   "(nil Bytes) (project trace events) : (List Bytes)); "
                   "(let whole : Bytes := (concatAll (prim list_fold (fun (reversed : (List Bytes)) (piece : Bytes) => "
                   "(cons piece reversed)) (nil Bytes) blocks : (List Bytes))); "
                   "(match (prim slice whole 0 (project trace outLen) : (Option Bytes))\n"
                   "    | Option.some trimmed => trimmed\n    | Option.none => whole)))"))
    # Output block `index` of a trace, as words: the root node with that index.
    add(definition("rootWordsAt", ["trace : Trace", "index : Nat"], "Words16",
                   "(prim list_fold (fun [index] (acc : Words16) (event : Event) => (match event\n"
                   "    | Event.compressed node => (if ((project node role) =? 2) then (if ((project node index) =? index) "
                   "then (project node output) else acc) else acc)\n"
                   "    | Event.pushed entry depth => acc\n    | Event.popped entry depth => acc)) "
                   f"{record('Words16', [(f'v{i}', u32(0)) for i in range(16)])} (project trace events) : Words16)", axioms=False))
    add(definition("contextTrace", ["context : Bytes"], "Trace",
                   "(finalTrace (update (hasherWith (iv) (flagDeriveKeyContext)) context) 32)"))
    # derive_key's context key is a projection of the context trace, and that
    # trace is the prelude of the key-material trace.
    add(definition("deriveHasher", ["context : Bytes"], "Hasher",
                   "(let derived : Trace := (contextTrace context); "
                   "(record Hasher {keyWords := (cvOf (rootWordsAt derived 0)), modeFlags := (flagDeriveKeyMaterial), "
                   "chunkCount := 0, pending := 0x, stack := (nil Entry), log := (nil Event), "
                   "prelude := (project derived events)}))"))
    add(definition("start", ["mode : Mode"], "(Result Hasher Blake3Error)",
                   "(match mode\n"
                   "    | Mode.hashMode => (ctor Result.ok [Hasher, Blake3Error] (hasherWith (iv) (0 : UInt32)))\n"
                   "    | Mode.keyedMode secret => (if ((prim length secret : Nat) =? 32) then "
                   "(ctor Result.ok [Hasher, Blake3Error] (hasherWith (keyWordsOf secret) (flagKeyedHash))) else "
                   "(ctor Result.error [Hasher, Blake3Error] (ctor Blake3Error.keyLength (prim length secret : Nat))))\n"
                   "    | Mode.deriveMode context => (ctor Result.ok [Hasher, Blake3Error] (deriveHasher context)))"))
    add(definition("traceOf", ["mode : Mode", "pieces : (List Bytes)", "outLen : Nat"], "(Result Trace Blake3Error)",
                   "(match (start mode)\n"
                   "    | Result.ok initial => (ctor Result.ok [Trace, Blake3Error] (finalTrace (prim list_fold "
                   "(fun (state : Hasher) (piece : Bytes) => (update state piece)) initial pieces : Hasher) outLen))\n"
                   "    | Result.error refused => (ctor Result.error [Trace, Blake3Error] refused))"))
    add(definition("hexDigit", ["nibble : UInt32"], "Bytes",
                   bits_tree("nibble", 4, lambda v: "0x" + format(v, "x").encode().hex())))
    add(definition("hexStep", ["data : Bytes", "walk : (Nat * Bytes)"], "(Nat * Bytes)",
                   "(let pos : Nat := (first walk); (let octet : UInt32 := (byteAt data (prim length data : Nat) pos); "
                   "((pos + 1), (prim append (second walk) (prim append (hexDigit (shiftRight octet (4 : UInt32))) "
                   "(hexDigit (prim bit_and octet (15 : UInt32) : UInt32)) : Bytes) : Bytes))))"))
    add(definition("hex", ["data : Bytes"], "Bytes",
                   "(second (prim iterate (fun [data] (walk : (Nat * Bytes)) => (hexStep data walk)) "
                   "(prim length data : Nat) (0, 0x) : (Nat * Bytes)))"))

    # Production roots: every digest is a projection of a trace.
    add(definition("hashPieces", ["mode : Mode", "pieces : (List Bytes)", "outLen : Nat"],
                   "(Result Bytes Blake3Error)",
                   "(match (traceOf mode pieces outLen)\n"
                   "    | Result.ok trace => (ctor Result.ok [Bytes, Blake3Error] (digestOf trace))\n"
                   "    | Result.error refused => (ctor Result.error [Bytes, Blake3Error] refused))",
                   production=True))
    add(definition("hash", ["message : Bytes", "outLen : Nat"], "Bytes",
                   "(digestOf (finalTrace (update (hasherWith (iv) (0 : UInt32)) message) outLen))", production=True))
    add(definition("hashHex", ["message : Bytes"], "Bytes", "(hex (hash message 32))", production=True))
    add(definition("keyedHash", ["secret : Bytes", "message : Bytes", "outLen : Nat"], "(Result Bytes Blake3Error)",
                   "(match (traceOf (ctor Mode.keyedMode secret) (cons message (nil Bytes)) outLen)\n"
                   "    | Result.ok trace => (ctor Result.ok [Bytes, Blake3Error] (digestOf trace))\n"
                   "    | Result.error refused => (ctor Result.error [Bytes, Blake3Error] refused))",
                   production=True))
    add(definition("deriveKey", ["context : Bytes", "material : Bytes", "outLen : Nat"], "Bytes",
                   "(digestOf (finalTrace (update (deriveHasher context) material) outLen))",
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
    modules = {"lexlean/src/Blake3.lex.tex": "".join(out)}
    modules["lexlean-trace/src/Blake3Trace.lex.tex"] = (
        "\\begin{lexlean}{Blake3Trace}\n\\useglossary{lexlean.std.nat@1.2.0}\n\\importmodule{Blake3Uor.Blake3}\n"
        "\\importmodule{LexLeanStd.Sha256}\n\\title{Natural number addition}\n\\begin{semanticmodule}\n"
        + "\n".join(render_trace()) + "\\end{semanticmodule}\n\\end{lexlean}\n")

    # Phase 1: the reverse project, with its own roots so the core and
    # trace exports stay as they are.
    modules["lexlean-reverse/src/Blake3Reverse.lex.tex"] = (
        "\\begin{lexlean}{Blake3Reverse}\n\\useglossary{lexlean.std.nat@1.2.0}\n\\importmodule{Blake3Uor.Blake3}\n"
        "\\title{Natural number addition}\n\\begin{semanticmodule}\n"
        + "\n".join(render_reverse()) + "\\end{semanticmodule}\n\\end{lexlean}\n")
    zero = record("Blake3Uor.Blake3::Words16", [(f"v{i}", u32(0)) for i in range(16)])
    runs_back = theorem("compression_runs_back_to_its_start",
                        f"((Blake3Reverse::compressBack (Blake3Uor.Blake3::compress (Blake3Uor.Blake3::iv) {zero} {u32(0)} {u32(0)} "
                        f"{u32(0)} {u32(11)}) (Blake3Uor.Blake3::iv) {zero}) = (Blake3Uor.Blake3::initialState (Blake3Uor.Blake3::iv) "
                        f"{u32(0)} {u32(0)} {u32(0)} {u32(11)}))")
    modules["lexlean-reverse/src/Blake3ReverseKernelRunsBack.lex.tex"] = (
        "\\begin{lexlean}{Blake3ReverseKernelRunsBack}\n\\useglossary{lexlean.std.nat@1.2.0}\n"
        "\\importmodule{Blake3Uor.Blake3}\n\\importmodule{Blake3Reverse}\n\\title{Natural number addition}\n"
        "\\begin{semanticmodule}\n" + runs_back + "\\end{semanticmodule}\n\\end{lexlean}\n")

    # Phase 2: slices, and the difference and dependency analysis, each a
    # project of its own for the same reason as the reverse project.
    def module(name, imports, body):
        return (f"\\begin{{lexlean}}{{{name}}}\n\\useglossary{{lexlean.std.nat@1.2.0}}\n"
                + "".join(f"\\importmodule{{{m}}}\n" for m in imports)
                + "\\title{Natural number addition}\n\\begin{semanticmodule}\n" + body + "\\end{semanticmodule}\n\\end{lexlean}\n")
    modules["lexlean-slice/src/Blake3Slice.lex.tex"] = module("Blake3Slice", ["Blake3Uor.Blake3"], "\n".join(render_slice()))
    # The slice of chunk 1 of the official 1025-byte input against its
    # official hash: accepted; with one bit of the sibling CV (chunk 0's)
    # flipped, refused at the root; with the chunk's byte changed, refused at
    # the chunk. Two compressions each, so the kernel checks them.
    _, _, cases = load_vectors()
    cv0, cv1 = (bytes.fromhex(c) for c in SLICE_CVS)
    header = (1025).to_bytes(8, "little")
    chunk = message(1025)[1024:]
    root = cases[1025]["hash"][:32]
    wrong = bytes([cv0[0] ^ 1]) + cv0[1:]
    hashing = "(ctor Blake3Uor.Blake3::Mode.hashMode)"
    result = lambda v: f"(ctor Result.{v} [Bytes, Blake3Slice::SliceError]"
    for name, data, expected in [
            ("official_slice_is_accepted", header + cv0 + cv1 + chunk, f"{result('ok')} {literal(chunk)})"),
            ("wrong_sibling_is_refused", header + wrong + cv1 + chunk, f"{result('error')} (ctor Blake3Slice::SliceError.parent 0))"),
            ("wrong_chunk_is_refused", header + cv0 + cv1 + bytes([chunk[0] ^ 1]), f"{result('error')} (ctor Blake3Slice::SliceError.chunk))")]:
        name_module = "Blake3SliceKernel" + "".join(part.capitalize() for part in name.split("_"))
        statement = f"((Blake3Slice::sliceCheck {hashing} {literal(data)} 1 1025 {literal(root)}) = {expected})"
        modules[f"lexlean-slice/src/{name_module}.lex.tex"] = module(name_module, ["Blake3Uor.Blake3", "Blake3Slice"],
                                                                     theorem(name, statement))
    modules["lexlean-analysis/src/Blake3Analysis.lex.tex"] = module(
        "Blake3Analysis", ["Blake3Uor.Blake3", "Blake3UorReverse.Blake3Reverse"], "\n".join(render_analysis()))
    modules["lexlean-analysis/src/Blake3AnalysisKernelWordDiffusion.lex.tex"] = module(
        "Blake3AnalysisKernelWordDiffusion", ["Blake3Uor.Blake3", "Blake3UorReverse.Blake3Reverse", "Blake3Analysis"],
        theorem("word_diffusion_step", f"((Blake3Analysis::wordDiffusion) = {word_diffusion()})"))

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

    def root(state, index=0, out=64):
        return f"({q('rootWordsAt')} ({q('finalTrace')} {state} {out}) {index})"

    def fed(start_state, data):
        return f"({q('update')} {start_state} {literal(data)})"

    plain = f"({q('hasherWith')} ({q('iv')}) (0 : UInt32))"
    keyed = f"({q('hasherWith')} ({q('keyWordsOf')} {literal(key)}) ({q('flagKeyedHash')}))"
    derived = f"({q('deriveHasher')} {literal(context)})"
    theorems = []
    for length in (0, 1, 63, 64, 65):
        theorems.append((f"official_hash_{length}",
                         f"({root(fed(plain, message(length)))} = {block(cases[length]['hash'], 0)})", ()))
    theorems.append(("official_hash_0_block_1",
                     f"({root(fed(plain, b''), 1, 128)} = {block(cases[0]['hash'], 1)})", ()))
    for length in (0, 1):
        theorems.append((f"official_keyed_hash_{length}",
                         f"({root(fed(keyed, message(length)))} = {block(cases[length]['keyed_hash'], 0)})", ()))
        theorems.append((f"official_derive_key_{length}",
                         f"({root(fed(derived, message(length)))} = {block(cases[length]['derive_key'], 0)})", ()))
    theorems.append(("split_update_equals_one_shot",
                     f"({root(fed(fed(plain, bytes([0, 1])), bytes([2])))} = {root(fed(plain, bytes([0, 1, 2])))})", ()))
    # For every input, by definition: each digest is a projection of its
    # trace, and each compression's output is a projection of its word
    # trace. The kernel checks these by unfolding, with every variable free.
    words = ["cv : Blake3::Cv", "m : Blake3::Words16", "lo : UInt32", "hi : UInt32", "len : UInt32", "fl : UInt32"]
    theorems.append(("compression_is_its_word_trace",
                     f"(({q('compress')} cv m lo hi len fl) = ({q('compressOut')} ({q('compressTrace')} cv m lo hi len fl)))",
                     words))
    theorems.append(("node_output_is_its_word_trace",
                     f"((project ({q('run')} p) output) = ({q('compressOut')} ({q('compressTrace')} (project p chain) "
                     f"(project p block) ({q('counterLow')} (project p counter)) ({q('counterHigh')} (project p counter)) "
                     f"(project p blockLen) (project p flags))))", ["p : Blake3::Pending"]))
    theorems.append(("hash_is_its_trace_digest",
                     f"(({q('hash')} message outLen) = ({q('digestOf')} ({q('finalTrace')} ({q('update')} {plain} message) outLen)))",
                     ["message : Bytes", "outLen : Nat"]))
    theorems.append(("derive_key_is_its_trace_digest",
                     f"(({q('deriveKey')} context material outLen) = ({q('digestOf')} ({q('finalTrace')} "
                     f"({q('update')} ({q('deriveHasher')} context) material) outLen)))",
                     ["context : Bytes", "material : Bytes", "outLen : Nat"]))
    theorems.append(("pieces_are_their_trace_digest",
                     f"(({q('hashPieces')} mode pieces outLen) = (match ({q('traceOf')} mode pieces outLen)\n"
                     f"    | Result.ok trace => (ctor Result.ok [Bytes, Blake3::Blake3Error] ({q('digestOf')} trace))\n"
                     f"    | Result.error refused => (ctor Result.error [Bytes, Blake3::Blake3Error] refused)))",
                     ["mode : Blake3::Mode", "pieces : (List Bytes)", "outLen : Nat"]))
    for name, statement, parameters in theorems:
        module = "Blake3Kernel" + "".join(part.capitalize() for part in name.split("_"))
        modules[f"lexlean/src/{module}.lex.tex"] = (
            f"\\begin{{lexlean}}{{{module}}}\n\\useglossary{{lexlean.std.nat@1.2.0}}\n"
            f"\\importmodule{{Blake3}}\n\\title{{Natural number addition}}\n\\begin{{semanticmodule}}\n"
            f"{theorem(name, statement, parameters=parameters)}\\end{{semanticmodule}}\n\\end{{lexlean}}\n")
    return modules


IMAGE = ROOT / "prism" / "Blake3Image.lex.tex"
IMAGE_MAX_LEN = 8193
IMAGE_ROOTS = {"hash", "hashHex", "keyedHash", "deriveKey", "hashPieces", "acceptance", "traceFile", "traceAddress",
               "checkTrace", "cursorMove", "prevAgrees", "seekStep", "stepRoundTrip", "unmix",
               "sliceOf", "sliceCheck", "diffTrace", "diffTraceBack", "diffReach", "taintRound", "fullTaintSteps", "wordDiffusion"}


def blocks(module):
    """The declarations of a semantic module, as text blocks."""
    body = module.split("\\begin{semanticmodule}\n", 1)[1].split("\\end{semanticmodule}", 1)[0]
    return [b.strip("\n") + "\n" for b in re.split(r"\n(?=(?:structure|definition|inductive|theorem)\{)", "\n" + body)
            if b.strip()]


def render_image(core_module):
    """One module for the Prism image builder, which reads a single file: the
    core, the stdlib's Sha256 under `sha` names, and the trace module, with
    the image's roots marked production and every theorem left out."""
    stdlib = (ROOT / "vendor" / "lexlean-stdlib" / "src" / "Sha256.lex.tex").read_text()
    ours = (blocks(core_module) + render_trace(core="", sha=lambda n: "sha" + n[0].upper() + n[1:]) + render_reverse(core="")
            + render_slice(core="") + render_analysis(core="", rev=""))
    names = [m.group(1) for b in ours for m in [re.match(r"\w+\{\s*name: (\w+),", b)] if m]
    if len(names) != len(set(names)):
        raise SystemExit(f"the image would declare twice: {sorted({n for n in names if names.count(n) > 1})}")
    taken = set(names)
    sha_blocks = [b for b in blocks(stdlib) if not b.startswith("theorem{")]
    own = {re.match(r"\w+\{\s*name: (\w+),", b).group(1) for b in sha_blocks}
    tokens = set(re.findall(r"[A-Za-z_][A-Za-z0-9_]*", "".join(sha_blocks)))
    renamed = own | (tokens & taken)
    sha_text = "".join(sha_blocks)
    sha_text = re.sub(r"\b(" + "|".join(sorted(renamed, key=len, reverse=True)) + r")\b",
                      lambda m: "sha" + m.group(1)[0].upper() + m.group(1)[1:], sha_text)
    sha_text = re.sub(r",\s*production: \{[^}]*\}", "", sha_text)
    # The image's acceptance vectors stop at IMAGE_MAX_LEN bytes: the engine
    # keeps every allocation of a call, and the traces of the 16 KiB to
    # 100 KiB official inputs exhaust its memory. The Rust export runs every
    # case (B3-01).
    _, _, cases = load_vectors()
    vectors = "(nil Vector)"
    for length in sorted((n for n in cases if n <= IMAGE_MAX_LEN), reverse=True):
        expected = literal(cases[length]["hash"][:32].hex().encode())
        vectors = f"(cons (record Vector {{given := (officialInput {length}), expected := {expected}}}) {vectors})"
    capped = definition("acceptance", [], "Acceptance", f"(record Acceptance {{acceptanceVectors := {vectors}}})",
                        axioms=False, production=True)
    marked = []
    for b in [blk for blk in ours if not blk.startswith("theorem{")]:
        name = re.match(r"\w+\{\s*name: (\w+),", b).group(1)
        if name == "acceptance":
            marked.append(capped)
            continue
        b = re.sub(r",\s*production: \{[^}]*\}", "", b)
        if name in IMAGE_ROOTS and b.startswith("definition{"):
            b = b.rstrip("\n").rstrip("}").rstrip("\n") + ",\n" + PRODUCTION.rstrip("\n") + "\n}\n"
        marked.append(b)
    return ("\\begin{lexlean}{Blake3Image}\n\\useglossary{lexlean.std.nat@1.2.0}\n\\title{Natural number addition}\n"
            "\\begin{semanticmodule}\n" + sha_text + "\n" + "\n".join(marked) + "\\end{semanticmodule}\n\\end{lexlean}\n")


def canonical(modules, lexlean):
    """The bytes `lexlean fmt` makes of each module: the formatter owns layout."""
    with tempfile.TemporaryDirectory() as scratch:
        # The workspace as it is in the repository: the three projects beside
        # the vendored stdlib the trace project imports Sha256 from.
        stage = pathlib.Path(scratch)
        shutil.copy(ROOT / "lexlean-workspace.toml", stage / "lexlean-workspace.toml")
        shutil.copytree(ROOT / "vendor", stage / "vendor")
        for project in PROJECTS:
            (stage / project / "src").mkdir(parents=True)
            for name in ("lexlean.toml", "lexlean.lock", "lakefile.toml", "lake-manifest.json", "lean-toolchain"):
                shutil.copy(ROOT / project / name, stage / project / name)
        for path, text in modules.items():
            (stage / path).write_bytes(text.encode())
        # A lock pins the bytes of the packages it imports, and formatting
        # rewrites a project's bytes, so the staged projects are locked and
        # formatted one at a time in import order.
        for project in PROJECTS:
            for command in ("lock", "fmt"):
                ran = subprocess.run([lexlean, command, "--color", "never"], cwd=stage / project,
                                     capture_output=True, text=True)
                if ran.returncode != 0:
                    raise SystemExit(f"lexlean {command} failed in {project}:\n{ran.stderr}")
        return {path: (stage / path).read_bytes() for path in modules}


def main():
    lexlean = os.environ.get("LEXLEAN", "lexlean")
    modules = canonical(render(), lexlean)
    image = render_image(modules["lexlean/src/Blake3.lex.tex"].decode()).encode()
    committed = {str(path.relative_to(ROOT)) for project in PROJECTS for path in (ROOT / project / "src").glob("*.lex.tex")}
    if "--check" in sys.argv[1:]:
        stale = sorted(path for path, text in modules.items()
                       if path not in committed or (ROOT / path).read_bytes() != text)
        extra = sorted(committed - set(modules))
        if not IMAGE.exists() or IMAGE.read_bytes() != image:
            stale.append(str(IMAGE.relative_to(ROOT)))
        if stale or extra:
            raise SystemExit(f"stale {stale}, not generated {extra}: run python3 -I tools/gen_blake3.py")
        print(f"gen_blake3: {len(modules)} modules equal their generator")
        return
    for path in committed - set(modules):
        (ROOT / path).unlink()
    for path, text in modules.items():
        (ROOT / path).parent.mkdir(parents=True, exist_ok=True)
        (ROOT / path).write_bytes(text)
    IMAGE.write_bytes(image)
    for project in PROJECTS:
        names = sorted(path.split("/", 1)[1] for path in modules if path.startswith(project + "/"))
        print(f"gen_blake3: {project}: entrypoints = [{', '.join(chr(34) + n + chr(34) for n in names)}]")


if __name__ == "__main__":
    main()
