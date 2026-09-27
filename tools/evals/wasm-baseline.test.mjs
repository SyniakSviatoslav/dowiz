// node --test tools/evals/wasm-baseline.test.mjs
//
// THE WASM RATCHET HAS SOMETHING TO RATCHET AGAINST (W-PERF P4). The bundle
// doubled 2,426,605 -> 4,852,622 B between 2026-09-22 and 09-27 and no gate
// said so, because `baselines/wasm.baseline` did not exist: every `wasm.*`
// indicator judged `new`. This holds the file present and each of the three
// size indicators RED one byte above it.
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { judge, parseBaseline } from './rules.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const base = parseBaseline(fs.readFileSync(path.join(HERE, 'baselines/wasm.baseline'), 'utf8'));

for (const id of ['wasm.raw', 'wasm.gzip', 'wasm.section.name']) {
  test(`${id} is red on growth and green at its baseline`, () => {
    assert.ok(base[id] !== undefined, `${id} has a baseline`);
    const b = Number(base[id]);
    assert.ok(Number.isInteger(b) && b >= 0);
    assert.equal(judge({ id, value: b + 1, rule: 'ratchet' }, base[id]).status, 'breach');
    assert.equal(judge({ id, value: b, rule: 'ratchet' }, base[id]).status, 'ok');
  });
}

test('the raw bundle baseline is a real build, not a placeholder', () => {
  assert.ok(Number(base['wasm.raw']) > 1_000_000, 'a Worker bundle is megabytes');
  assert.ok(Number(base['wasm.gzip']) < Number(base['wasm.raw']));
});
