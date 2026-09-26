import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { tree } from './fixture.mjs';
import * as T from './tests.mjs';
import * as G from './gates.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));

test('files skips build and vendored directories; a missing dir is empty', () => {
  const root = tree();
  assert.deepEqual(T.files(path.join(root, 'nope'), /x/), []);
  const rs = T.files(path.join(root, 'crates/dowiz-core'), /\.rs$/);
  assert.equal(rs.length, 1);
  assert.equal(T.countMatches(rs, /#\[test\]/g), 1);
  assert.equal(T.countMatches([path.join(root, 'e2e/tests/one.mjs')], /x/g), 0);
});

test('mutation score is per mille of caught over caught + missed', () => {
  assert.equal(T.mutationScore({ caught: 3, missed: 1 }), 750);
  assert.equal(T.mutationScore({ caught: 0, missed: 2 }), 0);
  assert.equal(T.mutationScore({ caught: 0 }), null);
  assert.equal(T.mutationScore({}), null);
});

test('the tests collector counts per crate, js cases, e2e files; no mutants output is UNVERIFIED', async () => {
  const r = byId(await T.collect({ root: tree() }));
  assert.equal(r['tests.rust.kernel'].value, 2);
  assert.equal(r['tests.rust.dowiz_core'].value, 1);
  assert.equal(r['tests.rust.worker'].value, 0);
  assert.equal(r['tests.js.files'].value, 1, 'spikes/ is not counted');
  assert.equal(r['tests.js.cases'].value, 2);
  assert.equal(r['tests.e2e.files'].value, 1);
  assert.match(r['mutants.kernel_permille'].unverified, /no cargo-mutants output/);
});

test('a mutants outcomes.json becomes the score and the missed count', async () => {
  const o = { caught: 8, missed: 2, total_mutants: 12, start_time: '2026-09-20T01:02:03Z',
    outcomes: [{ scenario: { Mutant: { file: 'src/money.rs' } } }, { scenario: 'Baseline' }] };
  const root = tree({ [T.MUTANTS]: JSON.stringify(o) });
  const r = byId(await T.collect({ root }));
  assert.equal(r['mutants.kernel_permille'].value, 800);
  assert.equal(r['mutants.kernel_permille'].note, '8 caught, 2 missed of 12; run 2026-09-20 over src/money.rs');
  assert.equal(r['mutants.kernel_missed'].value, 2);
  const none = byId(await T.collect({ root: tree({ [T.MUTANTS]: '{"caught":1}' }) }));
  assert.equal(none['mutants.kernel_missed'].value, 0);
});

test('run-all table parsing', () => {
  const rows = G.parseTable('ok   file-size rc=0 fine\nFAIL clock rc=1 bad\nskip cargo-x  needs cargo\nnoise');
  assert.deepEqual(rows, [{ mark: 'ok', name: 'file-size', rc: 0 }, { mark: 'FAIL', name: 'clock', rc: 1 }, { mark: 'skip', name: 'cargo-x', rc: null }]);
});

test('run spawns for real and reports the exit code and output', () => {
  const r = G.run(process.cwd(), 'sh', ['-c', 'echo out; echo err >&2; exit 3']);
  assert.equal(r.rc, 3);
  assert.equal(r.out, 'out\nerr\n');
  assert.equal(r.err, '');
  assert.match(G.run(process.cwd(), '/no/such/binary', []).err, /ENOENT/);
});

test('the gates collector: green, failed, skipped, baselines — and a table with no rows breaches', async () => {
  const root = tree();
  const exec = () => ({ rc: 1, out: 'ok a rc=0\nok b rc=0\nFAIL c rc=2 x\nskip d\n', err: '' });
  const r = byId(await G.collect({ root, exec }));
  assert.equal(r['gates.ran'].value, 3);
  assert.equal(r['gates.green'].value, 2);
  assert.equal(r['gates.failed'].value, 1);
  assert.equal(r['gates.failed'].note, 'c rc=2');
  assert.equal(r['gates.skipped'].note, 'd');
  assert.equal(r['gates.baselines'].value, 1);
  const g = byId(await G.collect({ root, exec: () => ({ rc: 0, out: 'ok a rc=0\n', err: '' }) }));
  assert.equal(g['gates.failed'].note, undefined);
  const e = await G.collect({ root, exec: () => ({ rc: null, out: '', err: 'timed out' }) });
  assert.equal(e[0].id, 'gates.table_rows');
  assert.match(e[0].note, /rc=null timed out/);
  fs.rmSync(root, { recursive: true });
});

test('the gates collector runs run-all.sh for real when no runner is injected', async () => {
  const e = await G.collect({ root: tree() });
  assert.equal(e[0].id, 'gates.table_rows');
  assert.match(e[0].note, /rc=\d+/);
});
