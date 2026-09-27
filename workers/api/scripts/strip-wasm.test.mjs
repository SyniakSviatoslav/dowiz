// node --test workers/api/scripts/strip-wasm.test.mjs
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { strip, sections, main, keepName, functionNames } from './strip-wasm.mjs';

const SCRIPT = fileURLToPath(new URL('./strip-wasm.mjs', import.meta.url));
const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'strip-wasm-'));
const quiet = () => { const lines = []; return [lines, l => lines.push(l)]; };

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
  assert.throws(() => strip(Buffer.concat([core, Buffer.from([0x00, 0x80])])), /truncated LEB128/);
});

test('--keep writes the unstripped bytes under the digest of the stripped file, then strips in place', () => {
  const d = tmp();
  const file = path.join(d, 'index_bg.wasm');
  fs.writeFileSync(file, full);
  const [out, log] = quiet();
  assert.equal(main([file, '--keep', path.join(d, 'unstripped')], log, log), 0);
  const shipped = fs.readFileSync(file);
  assert.deepEqual(shipped, strip(full).out);
  const digest = crypto.createHash('sha256').update(shipped).digest('hex').slice(0, 16);
  assert.equal(keepName(shipped), `index_bg.${digest}.wasm`);
  assert.deepEqual(fs.readFileSync(path.join(d, 'unstripped', `index_bg.${digest}.wasm`)), full);
  assert.match(out[0], /-> \d+ B \(dropped name \d+, producers \d+\); unstripped kept at /);
});

test('a second run over a stripped build keeps nothing, so the kept dir never holds a stripped file', () => {
  const d = tmp();
  const file = path.join(d, 'index_bg.wasm');
  fs.writeFileSync(file, core);
  const [out, log] = quiet();
  assert.equal(main([file, '--keep', path.join(d, 'k')], log, log), 0);
  assert.equal(fs.existsSync(path.join(d, 'k')), false);
  assert.deepEqual(fs.readFileSync(file), core);
  assert.match(out[0], /dropped nothing\)$/);
});

test('without --keep it strips in place and keeps nothing', () => {
  const d = tmp();
  const file = path.join(d, 'm.wasm');
  fs.writeFileSync(file, full);
  const [, log] = quiet();
  assert.equal(main([file], log, log), 0);
  assert.deepEqual(fs.readFileSync(file), strip(full).out);
  assert.deepEqual(fs.readdirSync(d), ['m.wasm']);
});

test('refuses a bad command line with 2 and a broken module with 1, writing nothing', () => {
  const d = tmp();
  const file = path.join(d, 'bad.wasm');
  fs.writeFileSync(file, 'hello world!');
  for (const argv of [[], [file, '--keep'], [file, '--kep', d], [file, '--keep', d, 'extra']]) {
    const [out, log] = quiet();
    assert.equal(main(argv, log, log), 2, argv.join(' '));
    assert.match(out[0], /^usage:/);
  }
  const [out, log] = quiet();
  assert.equal(main([file, '--keep', path.join(d, 'k')], log, log), 1);
  assert.match(out[0], /not a wasm module/);
  assert.equal(fs.readFileSync(file, 'utf8'), 'hello world!');
  assert.equal(fs.existsSync(path.join(d, 'k')), false);
});

test('the command line wrangler runs: exit codes reach the shell', () => {
  const d = tmp();
  const file = path.join(d, 'index_bg.wasm');
  fs.writeFileSync(file, full);
  const ok = spawnSync(process.execPath, [SCRIPT, file, '--keep', path.join(d, 'u')], { encoding: 'utf8' });
  assert.equal(ok.status, 0, ok.stderr);
  assert.match(ok.stdout, /unstripped kept at/);
  const bad = spawnSync(process.execPath, [SCRIPT, path.join(d, 'missing.wasm')], { encoding: 'utf8' });
  assert.equal(bad.status, 1);
  assert.match(bad.stderr, /ENOENT/);
});

// A real `name` section: subsection 0 (module name "m"), then subsection 1 (function 0 = "answer").
const fnames = Buffer.from([1, 9, 1, 0, 6, ...Buffer.from('answer')]);
const named = Buffer.concat([core, custom('name', [0, 2, 1, 0x6d, ...fnames])]);

test('functionNames reads the function-name subsection and skips the others', () => {
  assert.deepEqual([...functionNames(named)], [[0, 'answer']]);
  assert.equal(functionNames(core).size, 0);
  assert.equal(functionNames(strip(named).out).size, 0);
});

test('--name symbolises a stripped trace against the kept copy', () => {
  const d = tmp();
  const kept = path.join(d, 'kept.wasm');
  fs.writeFileSync(kept, named);
  const [out, log] = quiet();
  assert.equal(main(['--name', kept, '0', '7'], log, log), 0);
  assert.deepEqual(out, ['wasm-function[0] answer', 'wasm-function[7] (no name)']);
  for (const argv of [['--name'], ['--name', kept], ['--name', kept, 'x1']]) {
    const [o, l] = quiet();
    assert.equal(main(argv, l, l), 2);
    assert.match(o[0], /^usage: strip-wasm.mjs --name/);
  }
  const [o2, l2] = quiet();
  assert.equal(main(['--name', path.join(d, 'missing.wasm'), '0'], l2, l2), 1);
  assert.match(o2[0], /ENOENT/);
});
