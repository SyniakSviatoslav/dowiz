// The deciders in wasm32 under node (`--features decide`): instantiate with
// NO imports, copy three byte strings in, call `bw_amend` or `bw_pay`, write
// the answer's bytes to stdout and the status to stderr.
//
// Usage: node decide.mjs <module.wasm> amend <log> <stock> <input.json>
//        node decide.mjs <module.wasm> pay   <log> <room.json> <input.json>
// Exit 0 only when the decider answered OK (status 0); a refusal exits 1 with
// its words on stdout, so `cmp` against a `.refusal` file still reads them.
import { readFileSync } from "node:fs";

const [, , modulePath, what, a, b, c] = process.argv;
if (!modulePath || !["amend", "pay"].includes(what) || !a || !b || !c) {
  console.error("usage: node decide.mjs <module.wasm> amend|pay <log> <stock|room> <input>");
  process.exit(2);
}
const { instance } = await WebAssembly.instantiate(readFileSync(modulePath), {});
const { memory, bw_alloc, bw_free, bw_amend, bw_pay, bw_abi_version } = instance.exports;
if (bw_abi_version() !== 1 || !bw_amend || !bw_pay) {
  console.error("this module has no deciders: build it with --features decide");
  process.exit(3);
}
// Copy one file into the module's memory; an empty one is (0, 0).
const put = (path) => {
  const bytes = readFileSync(path);
  const ptr = bytes.length ? bw_alloc(bytes.length) : 0;
  if (bytes.length) new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
  return [ptr, bytes.length];
};
const args = [put(a), put(b), put(c)];
const out = bw_alloc(8); // two u32 cells: the answer's pointer and length
const status = (what === "amend" ? bw_amend : bw_pay)(...args.flat(), out);
// Re-read the views after the call: memory.buffer is detached by growth.
const [ptr, len] = new Uint32Array(memory.buffer, out, 2);
process.stdout.write(Buffer.from(new Uint8Array(memory.buffer, ptr, len)));
console.error(`${what} status=${status} bytes=${len}`);
bw_free(ptr, len);
bw_free(out, 8);
for (const [p, n] of args) bw_free(p, n);
process.exit(status === 0 ? 0 : 1);
