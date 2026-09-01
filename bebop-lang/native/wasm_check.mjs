/* wasm_check.mjs — B3-5: validate + execute an emitted wasm module in a real
 * runtime (V8 via node), asserting main() returns the expected i64.
 * Usage: node wasm_check.mjs <file.wasm> <expected-i64>
 * Part of the release gate; pairs with `make wasm-check`. */
import { readFile } from 'node:fs/promises';

const [file, wantStr] = process.argv.slice(2);
if (!file || !wantStr) {
  console.error('usage: node wasm_check.mjs <file.wasm> <expected-i64>');
  process.exit(2);
}
const want = BigInt(wantStr);

const bytes = await readFile(file);
const mod = await WebAssembly.instantiate(bytes);
const got = BigInt(mod.instance.exports.main());
if (got !== want) {
  console.error(`FAIL ${file}: main() = ${got}, want ${want}`);
  process.exit(1);
}
console.log(`ok ${file}: main() = ${got}`);
