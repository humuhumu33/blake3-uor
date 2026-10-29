// Load a local Prism image and call its roots in the image's own engine.
//
// Every blob is checked against its sha256 before use, and every program's
// id as the engine reads it must equal its layer digest, so neither the
// registry nor the image is trusted.
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { join } from "node:path";

export const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

export async function openImage(registry, image) {
  const blob = (digest) => {
    const bytes = readFileSync(join(registry, digest));
    if (sha256(bytes) !== digest) throw new Error(`blob ${digest} hashes to ${sha256(bytes)}`);
    return bytes;
  };
  const manifest = JSON.parse(blob(image).toString("utf8"));
  if (manifest.schema !== "prism/image/1") throw new Error(`not a prism/image/1 manifest: ${manifest.schema}`);
  const { instance } = await WebAssembly.instantiate(blob(manifest.engine.digest), {});
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
  const roots = {};
  for (const [name, root] of Object.entries(manifest.roots)) {
    const loaded = call("engine_load", [], new TextDecoder().decode(blob(root.digest)));
    if (!loaded.ok) throw new Error(`${name}: ${loaded.error}`);
    if (loaded.program_id !== root.digest) throw new Error(`${name}: program id ${loaded.program_id}, layer ${root.digest}`);
    roots[name] = (args) => {
      const got = call("engine_call", [loaded.handle, root.entry], JSON.stringify(args));
      if (!got.ok || got.outcome.kind !== "value") throw new Error(`${name}: ${got.error ?? got.outcome.kind}`);
      return got.outcome.value;
    };
  }
  return { manifest, roots };
}

// Values as the engine reads and writes them.
export const bytes = (data) => ({ kind: "bytes", hex: Buffer.from(data).toString("hex") });
export const nat = (n) => ({ kind: "nat", value: String(n) });
export const list = (items) => ({ kind: "list", items });
export const ctor = (index, fields = []) => ({ kind: "adt", constructor: index, fields });
