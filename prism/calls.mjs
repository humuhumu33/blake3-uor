#!/usr/bin/env node
// Call roots of a local Prism image in its engine, many calls per process.
//
//   node prism/calls.mjs <image sha256> --registry <dir> < calls.json
//
// stdin is a JSON array of [root, args] pairs, args in the engine's value
// encoding (see engine.mjs); stdout is a JSON array of the values returned,
// in order. A call the engine refuses ends the run with an error.
import { readFileSync } from "node:fs";
import { openImage } from "./engine.mjs";

const argv = process.argv.slice(2);
const at = argv.indexOf("--registry");
const registry = at < 0 ? undefined : argv.splice(at, 2)[1];
const [image] = argv;
if (!registry || !/^[0-9a-f]{64}$/.test(image ?? "")) {
  console.error("usage: calls.mjs <image sha256> --registry <dir> < calls.json");
  process.exit(2);
}
const { roots } = await openImage(registry, image);
const calls = JSON.parse(readFileSync(0, "utf8"));
const out = calls.map(([root, args]) => {
  if (!(root in roots)) throw new Error(`${root} is not a root of this image`);
  return roots[root](args);
});
process.stdout.write(JSON.stringify(out));
