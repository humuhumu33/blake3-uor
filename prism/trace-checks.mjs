#!/usr/bin/env node
// `checkTrace` in the Prism image's engine: it accepts real traces at every
// level, and rejects each planted defect with the error that names it.
//
//   node prism/trace-checks.mjs <image sha256> --registry <dir> [--blake3 <PrismPM blake3.mjs>]
//
// The checker is a verified LexLean definition run by the LexLean calculus
// engine the image carries; its rust-std export is blocked (docs/trace.md).
// With --blake3, every DigestSet is also recomputed by PrismPM's own BLAKE3
// and node's SHA-256, independently of the model. Exit status 0 means every
// check passed.
import { createHash } from "node:crypto";
import { openImage, bytes, nat, list, ctor } from "./engine.mjs";

const argv = process.argv.slice(2);
const flag = (name) => {
  const at = argv.indexOf(`--${name}`);
  return at < 0 ? undefined : argv.splice(at, 2)[1];
};
const registry = flag("registry");
const blake3Path = flag("blake3");
const [image] = argv;
if (!registry || !/^[0-9a-f]{64}$/.test(image ?? "")) {
  console.error("usage: trace-checks.mjs <image sha256> --registry <dir> [--blake3 <blake3.mjs>]");
  process.exit(2);
}
const { roots } = await openImage(registry, image);
const blake3 = blake3Path ? (await import(blake3Path)).blake3 : undefined;

const ERRORS = ["malformed", "digestSet", "unknownLabel", "load", "provenance", "permutation", "step", "bit", "link",
  "output", "digest", "rootMismatch"];
const KEY = Buffer.from("whats the Elvish word for friend");
const CONTEXT = Buffer.from("BLAKE3 2019-12-27 16:29:52 test vectors context");
const MODES = { hash: ctor(0), keyed: ctor(1, [bytes(KEY)]), derive: ctor(2, [bytes(CONTEXT)]) };
const input = (len) => Buffer.from(Array.from({ length: len }, (_, i) => i % 251));

let failures = 0;
const report = (ok, text) => {
  console.log(`${ok ? "PASS" : "FAIL"}  ${text}`);
  if (!ok) failures++;
};
const traceFile = (mode, data, outLen, level) => {
  const got = roots.traceFile([MODES[mode], list([bytes(data)]), nat(outLen), nat(level)]);
  return Buffer.from(got.value.hex, "hex");
};
const traceAddress = (mode, data, outLen) => Buffer.from(roots.traceAddress([MODES[mode], list([bytes(data)]), nat(outLen)]).value.hex, "hex");
const check = (file) => {
  const got = roots.checkTrace([bytes(file)]);
  if (got.kind === "ok") return { ok: Buffer.from(got.value.hex, "hex") };
  return { error: ERRORS[got.value.constructor], at: Number(got.value.fields[0].value) };
};

// `--check-files f...`: only run checkTrace on each file, one line per file,
// `OK <root DigestSet hex>` or `ERROR <TraceError> <offset>`. The Rust tests
// use this to put traces from the certified traceFile through the checker.
const files = process.argv.indexOf("--check-files");
if (files >= 0) {
  const { readFileSync } = await import("node:fs");
  for (const path of process.argv.slice(files + 1)) {
    const got = check(readFileSync(path));
    console.log(got.ok ? `OK ${got.ok.toString("hex")}` : `ERROR ${got.error} ${got.at}`);
  }
  process.exit(0);
}

// The documented layout (docs/trace.md), read independently of the model.
const WORD_SECTION = 8 + 1 + 64 + 16 + 7 * (16 + 112 * 18) + 16 * 18 + 64;
const BIT_SECTION = WORD_SECTION + (7 * 8 * 10 + 16) * 32;
function parse(file) {
  if (file.subarray(0, 8).toString() !== "B3TRACE1") throw new Error("magic");
  const level = file[8];
  const count = Number(file.readBigUInt64LE(9));
  const entries = [];
  let at = 17;
  for (let k = 0; k < count; k++) {
    const len = file.readUInt32LE(at);
    const blob = file.subarray(at + 4, at + 4 + len);
    const set = file.subarray(at + 4 + len, at + 68 + len);
    let next = at + 68 + len;
    const section = blob[0] === 1 && level > 0 ? next : undefined;
    if (section !== undefined) next += level === 1 ? WORD_SECTION : BIT_SECTION;
    entries.push({ at, blob, set, section });
    at = next;
  }
  if (at !== file.length) throw new Error("trailing bytes");
  return { level, entries };
}
const digestSet = (blob) => Buffer.concat([Buffer.from(blake3(blob, 32)), createHash("sha256").update(blob).digest()]);
const relabel = (file, entry) => {
  const len = entry.blob.length;
  digestSet(file.subarray(entry.at + 4, entry.at + 4 + len)).copy(file, entry.at + 4 + len);
};

// Real traces are accepted at every level, and their address is the model's.
for (const [len, levels] of [[0, [0, 1, 2]], [64, [0, 1, 2]], [1025, [0, 1]]]) {
  for (const mode of Object.keys(MODES)) {
    const address = traceAddress(mode, input(len), 40);
    for (const level of levels) {
      const file = traceFile(mode, input(len), 40, level);
      const got = check(file);
      report(got.ok && got.ok.equals(address.subarray(0, 64)),
        `${mode} ${len} B level ${level}: checkTrace accepts, root = traceAddress (${file.length} bytes)`);
      if (blake3) {
        const { entries } = parse(file);
        const bad = entries.filter((e) => !digestSet(e.blob).equals(e.set));
        report(bad.length === 0, `${mode} ${len} B level ${level}: ${entries.length} DigestSets = PrismPM blake3.mjs + node sha256`);
      }
    }
  }
}

// Each planted defect is rejected, by the check that names it.
const expect = (file, error, at, what) => {
  const got = check(file);
  report(got.error === error && (at === undefined || got.at === at),
    `${what}: rejected as ${got.error ?? "accepted"}${got.at !== undefined ? ` at ${got.at}` : ""} (want ${error}${at !== undefined ? ` at ${at}` : ""})`);
};
{
  const word = traceFile("hash", input(64), 32, 1);
  const bit = traceFile("hash", input(64), 32, 2);
  const section = parse(word).entries[0].section;
  const step0 = section + 89 + 16;
  let planted = Buffer.from(word);
  planted[step0 + 14] ^= 1;
  expect(planted, "step", step0, "a flipped operation result");
  planted = Buffer.from(bit);
  const bits = parse(bit).entries[0].section + 89 + 16 + 18;
  planted[bits] ^= 1 << 4;
  expect(planted, "bit", bits, "a flipped carry bit");
  planted = Buffer.from(word);
  planted[section + 89 + 16 + 112 * 18] ^= 1;
  expect(planted, "permutation", section + 89 + 16 + 112 * 18, "a wrong permutation index");
  planted = Buffer.from(word);
  const a = Buffer.from(planted.subarray(step0, step0 + 18));
  planted.copy(planted, step0, step0 + 18, step0 + 36);
  a.copy(planted, step0 + 18);
  expect(planted, "step", step0, "a reordered step");
}
{
  const tree = traceFile("hash", input(1025), 32, 0);
  const { entries } = parse(tree);
  const checkpoint = entries[0];
  let planted = Buffer.from(tree);
  planted[checkpoint.at + 4 + 34 + 60] ^= 2;
  expect(planted, "digestSet", checkpoint.at, "a wrong flag, label kept");
  planted = Buffer.from(tree);
  planted[checkpoint.at + 4 + 98] ^= 1;
  expect(planted, "digestSet", checkpoint.at, "a checkpoint whose message changed, label kept");
  if (blake3) {
    planted = Buffer.from(tree);
    planted[checkpoint.at + 4 + 34 + 60] ^= 2;
    relabel(planted, checkpoint);
    expect(planted, "unknownLabel", undefined, "a wrong flag, relabelled");
  }
}
if (blake3) {
  const tree = traceFile("hash", input(2049), 32, 0);
  const { entries } = parse(tree);
  const parent = entries.find((e) => e.blob[0] === 3 && e.blob[81] === 1);
  const chunks = entries.filter((e) => e.blob[0] === 2);
  const planted = Buffer.from(tree);
  chunks[1].set.subarray(0, 32).copy(planted, parent.at + 4 + 17);
  relabel(planted, parent);
  expect(planted, "link", parent.at, "a forged parent link");
}
{
  const other = traceFile("hash", input(64).subarray(1), 32, 0);
  const got = check(other);
  report(got.ok && !got.ok.equals(traceAddress("hash", input(64), 32).subarray(0, 64)),
    "another input's real trace is accepted but its root is not this input's address");
}
console.log(failures ? `${failures} check(s) failed` : "every check passed");
process.exit(failures ? 1 : 0);
