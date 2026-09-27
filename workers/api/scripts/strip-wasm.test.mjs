// node --test workers/api/scripts/strip-wasm.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import { strip, sections } from './strip-wasm.mjs';

const custom = (name, payload) => {
  const n = Buffer.from(name);
  const body = Buffer.concat([Buffer.from([n.length]), n, Buffer.from(payload)]);
  return Buffer.concat([Buffer.from([0, body.length]), body]);
};
// A real module: one function `answer` returning 42, exported.
const core = Buffer.from([
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
  0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type () -> i32
  0x03, 0x02, 0x01, 0x00, // func 0 : type 0
  0x07, 0x0a, 0x01, 0x06, 0x61, 0x6e, 0x73, 0x77, 0x65, 0x72, 0x00, 0x00, // export "answer"
  0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b, // body: i32.const 42
]);
const full = Buffer.concat([core, custom('name', [1, 2, 3]), custom('producers', [9]), custom('target_features', [7])]);

test('drops name and producers, keeps every other byte, and the module still runs', async () => {
  const { out, dropped } = strip(full);
  assert.deepEqual(dropped.map(d => d.name), ['name', 'producers']);
  assert.deepEqual(sections(out).map(s => s.name || s.id), [1, 3, 7, 10, 'target_features']);
  assert.deepEqual(out, Buffer.concat([core, custom('target_features', [7])]));
  const a = await WebAssembly.instantiate(full);
  const b = await WebAssembly.instantiate(out);
  assert.equal(b.instance.exports.answer(), a.instance.exports.answer());
  assert.deepEqual(WebAssembly.Module.exports(b.module), WebAssembly.Module.exports(a.module));
  assert.deepEqual(WebAssembly.Module.customSections(b.module, 'name'), []);
});

test('a module with nothing to drop is returned unchanged', () => {
  assert.deepEqual(strip(core).out, core);
});

test('refuses what is not a module, and a section that runs past the end', () => {
  assert.throws(() => strip(Buffer.from('hello world!')), /not a wasm module/);
  assert.throws(() => strip(core.subarray(0, core.length - 2)), /runs past the end/);
});
