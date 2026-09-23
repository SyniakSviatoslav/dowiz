// The wasm32 reader under node: instantiate the module with NO imports, copy
// the image into its memory, ask the two readers, print what they said.
//
// Usage: node harness.mjs <module.wasm> <image> [kv|log]
// Prints one line per reader: `<family> status=<n> n=<count> root=<fold>` and
// exits non-zero when the status is not OK, so a refusal is a red line and not
// a quietly printed zero.
import { readFileSync } from "node:fs";

const [, , modulePath, imagePath, family = "kv"] = process.argv;
if (!modulePath || !imagePath) {
  console.error("usage: node harness.mjs <module.wasm> <image> [kv|log]");
  process.exit(2);
}

const { instance } = await WebAssembly.instantiate(readFileSync(modulePath), {});
const { memory, bw_alloc, bw_free, bw_kv, bw_log, bw_abi_version } = instance.exports;
if (bw_abi_version() !== 1) {
  console.error(`abi version ${bw_abi_version()} is not the 1 this harness was written for`);
  process.exit(3);
}

const image = readFileSync(imagePath);
const ptr = bw_alloc(image.length);
new Uint8Array(memory.buffer, ptr, image.length).set(image);
// Two i64 cells for the answer, inside the module's memory as well.
const out = bw_alloc(16);
const read = family === "log" ? bw_log : bw_kv;
const status = read(ptr, image.length, out);
// Re-read the view after the calls: memory.buffer is detached by growth.
const cells = new BigInt64Array(memory.buffer, out, 2);
console.log(`${family} status=${status} n=${cells[0]} root=${cells[1]}`);
bw_free(out, 16);
bw_free(ptr, image.length);
process.exit(status === 0 ? 0 : 1);
