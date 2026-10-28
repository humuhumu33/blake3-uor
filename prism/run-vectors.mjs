#!/usr/bin/env node
// Run a local Prism image of lexlean/src/Blake3.lex.tex on every official
// BLAKE3 vector, in all three modes, inside the image's own engine.
//
//   node prism/run-vectors.mjs <image sha256> --registry <dir> [--vectors <test_vectors.json>]
//
// The image and its registry come from the prism-browser-runtime tooling
// (`prism.mjs build`). This script trusts neither: every blob is checked
// against its sha256 before use, and every program's id as the engine reads
// it must equal its layer digest. The vectors file is untrusted data, parsed
// against its closed shape.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const argv = process.argv.slice(2);
const flag = (name, fallback) => {
  const at = argv.indexOf(`--${name}`);
  if (at < 0) return fallback;
  return argv.splice(at, 2)[1];
};
const here = dirname(fileURLToPath(import.meta.url));
const registry = flag("registry");
const vectorsPath = flag("vectors", join(here, "..", "oracles", "blake3-1.8.7", "test_vectors.json"));
const [image] = argv;
if (!registry || !/^[0-9a-f]{64}$/.test(image ?? "")) {
  console.error("usage: run-vectors.mjs <image sha256> --registry <dir> [--vectors <file>]");
  process.exit(2);
}

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
function blob(digest) {
  const bytes = readFileSync(join(registry, digest));
  if (sha256(bytes) !== digest) throw new Error(`blob ${digest} hashes to ${sha256(bytes)}`);
  return bytes;
}

// The engine ABI, as prism.mjs speaks it: JSON in, length-prefixed JSON out.
async function engine(bytes) {
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const x = instance.exports;
  const put = (text) => {
    const data = new TextEncoder().encode(text);
    const at = x.engine_alloc(data.length);
    new Uint8Array(x.memory.buffer, at, data.length).set(data);
    return [at, data.length];
  };
  const take = (at) => {
    const n = new DataView(x.memory.buffer).getUint32(at, true);
    const text = new TextDecoder().decode(new Uint8Array(x.memory.buffer, at + 4, n));
    x.engine_free(at, n + 4);
    return JSON.parse(text);
  };
  const call = (name, numbers, ...buffers) => {
    const placed = buffers.map(put);
    try {
      return take(x[name](...numbers, ...placed.flat()));
    } finally {
      for (const [at, len] of placed) x.engine_free(at, len);
    }
  };
  return {
    load: (program) => call("engine_load", [], new TextDecoder().decode(program)),
    call: (handle, entry, args) => call("engine_call", [handle, entry], JSON.stringify(args)),
  };
}

const manifest = JSON.parse(blob(image).toString("utf8"));
if (manifest.schema !== "prism/image/1") throw new Error(`not a prism/image/1 manifest: ${manifest.schema}`);
const box = await engine(blob(manifest.engine.digest));
const roots = {};
for (const [name, root] of Object.entries(manifest.roots)) {
  const loaded = box.load(blob(root.digest));
  if (!loaded.ok) throw new Error(`${name}: ${loaded.error}`);
  if (loaded.program_id !== root.digest) throw new Error(`${name}: program id ${loaded.program_id}, layer ${root.digest}`);
  roots[name] = (args) => {
    const got = box.call(loaded.handle, root.entry, args);
    if (!got.ok || got.outcome.kind !== "value") throw new Error(`${name}: ${got.error ?? got.outcome.kind}`);
    return got.outcome.value;
  };
}

const raw = JSON.parse(readFileSync(vectorsPath, "utf8"));
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
if (!same(Object.keys(raw).sort(), ["_comment", "cases", "context_string", "key"])) throw new Error("unexpected vectors shape");
const hexOf = (text) => Buffer.from(text, "utf8").toString("hex");
const bytes = (hex) => ({ kind: "bytes", hex });
const nat = (n) => ({ kind: "nat", value: String(n) });
const key = bytes(hexOf(raw.key));
const context = bytes(hexOf(raw.context_string));
let passed = 0;
let failed = 0;
const started = Date.now();
for (const c of raw.cases) {
  if (!same(Object.keys(c).sort(), ["derive_key", "hash", "input_len", "keyed_hash"])) throw new Error("unexpected case shape");
  for (const mode of ["hash", "keyed_hash", "derive_key"]) {
    if (!/^[0-9a-f]{262}$/.test(c[mode])) throw new Error(`input_len ${c.input_len} ${mode}: not 131 hex bytes`);
  }
  const input = bytes(Buffer.from(Array.from({ length: c.input_len }, (_, i) => i % 251)).toString("hex"));
  const outputs = {
    hash: roots.hash([input, nat(131)]),
    keyed_hash: roots.keyedHash([key, input, nat(131)]),
    derive_key: roots.deriveKey([context, input, nat(131)]),
  };
  // keyedHash returns Result Bytes Blake3Error; the official key is 32 bytes,
  // so anything but `ok` is a failure, reported as it came.
  const keyed = outputs.keyed_hash;
  outputs.keyed_hash = keyed.kind === "ok" ? keyed.value : keyed;
  for (const [mode, value] of Object.entries(outputs)) {
    if (value.kind === "bytes" && value.hex === c[mode]) passed++;
    else {
      failed++;
      console.log(`FAIL  input_len ${c.input_len} ${mode}: ${JSON.stringify(value).slice(0, 120)}`);
    }
  }
}
const seconds = ((Date.now() - started) / 1000).toFixed(1);
console.log(`${failed ? "FAIL" : "PASS"}  image sha256:${image}: ${passed}/${passed + failed} official outputs (35 cases x hash, keyed_hash, derive_key, 131 bytes each) in ${seconds} s`);
process.exit(failed ? 1 : 0);
