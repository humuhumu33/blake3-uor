// Opens the Prism image and runs its engine.
//
// Nothing fetched is trusted: every blob must hash to the SHA-256 it is
// addressed by, and every program's id as the engine reads it must equal its
// layer digest (the same checks as prism/engine.mjs). The engine recurses
// deeper than a Web Worker's stack allows, so it runs on the page's thread.
export const IMAGE = "f20bd6c0a76c6a1020e448140791f98dfa133257ae251ed8ddbbb36be0f7da9f";
const ROOTS = ["hash", "keyedHash", "deriveKey"];

const hex = (buf) => Array.from(new Uint8Array(buf), (b) => b.toString(16).padStart(2, "0")).join("");

export async function openImage(onVerified) {
  const blob = async (digest, label) => {
    const res = await fetch(`registry/${digest}`);
    if (!res.ok) throw new Error(`${label}: HTTP ${res.status}`);
    const bytes = new Uint8Array(await res.arrayBuffer());
    const got = hex(await crypto.subtle.digest("SHA-256", bytes));
    if (got !== digest) throw new Error(`${label} hashes to ${got}, expected ${digest}`);
    onVerified(label, bytes.length);
    return bytes;
  };
  const manifest = JSON.parse(new TextDecoder().decode(await blob(IMAGE, "image manifest")));
  if (manifest.schema !== "prism/image/1") throw new Error(`not a prism/image/1 manifest: ${manifest.schema}`);
  const { instance } = await WebAssembly.instantiate(await blob(manifest.engine.digest, "engine (WebAssembly)"), {});
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
  for (const name of ROOTS) {
    const root = manifest.roots[name];
    const loaded = call("engine_load", [], new TextDecoder().decode(await blob(root.digest, `program ${name}`)));
    if (!loaded.ok) throw new Error(`${name}: ${loaded.error}`);
    if (loaded.program_id !== root.digest) throw new Error(`${name}: program id ${loaded.program_id}, layer ${root.digest}`);
    roots[name] = (args) => {
      const got = call("engine_call", [loaded.handle, root.entry], JSON.stringify(args));
      if (!got.ok || got.outcome.kind !== "value") throw new Error(`${name}: ${got.error ?? got.outcome.kind}`);
      return got.outcome.value;
    };
  }
  return { rootCount: Object.keys(manifest.roots).length, run: (job) => run(roots, job) };
}

const bytes = (h) => ({ kind: "bytes", hex: h });
const nat = (n) => ({ kind: "nat", value: String(n) });

// One call: { mode: "hash" | "keyed" | "derive", input, extra (hex), len }.
// Returns { hex } or, for a key that is not 32 bytes, the model's { refused }.
function run(roots, { mode, input, extra, len }) {
  let value;
  if (mode === "hash") value = roots.hash([bytes(input), nat(len)]);
  else if (mode === "keyed") value = roots.keyedHash([bytes(extra), bytes(input), nat(len)]);
  else value = roots.deriveKey([bytes(extra), bytes(input), nat(len)]);
  if (value.kind === "ok") value = value.value;
  if (value.kind === "bytes") return { hex: value.hex };
  if (value.kind === "error") return { refused: `keyLength ${value.value.fields[0].value}` };
  throw new Error(`unexpected value ${JSON.stringify(value).slice(0, 120)}`);
}
