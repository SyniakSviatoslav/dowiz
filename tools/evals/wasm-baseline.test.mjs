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

// THE STRIP IS WIRED (lane W-STRIP, 2026-09-27). The baseline below assumes the post-glue strip
// runs on every build; if the `[build] command` loses it, the name section (~640 KB) comes back and
// the ratchet goes red for a reason nobody would find in the Rust. This names the cause instead.
test('wrangler builds with the post-glue strip and keeps the unstripped copy', () => {
  const toml = fs.readFileSync(path.join(HERE, '../../workers/api/wrangler.toml'), 'utf8');
  const cmd = toml.match(/^\[build\][\s\S]*?^command\s*=\s*"([^"]*)"/m)?.[1] ?? '';
  assert.match(cmd, /^worker-build --release && node scripts\/strip-wasm\.mjs build\/index_bg\.wasm --keep build\/unstripped$/);
  assert.equal(Number(base['wasm.section.name']), 0, 'a stripped bundle has no name section');
});
