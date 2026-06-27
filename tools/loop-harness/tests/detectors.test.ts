import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { configTuneDetector, loadTunables, type Tunable } from '../src/detectors.js';
import { classify, buildHooks } from '../src/autoupgrade.js';
import { evaluate } from '../src/oracle.js';

// A throwaway git repo whose bench.js prints METRIC=<N> (lower = fewer resources).
// Tuning N down is a deterministic, mechanical, reversible "speedup".
function repo(n = 100): { dir: string; file: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'detect-'));
  const file = path.join(dir, 'bench.js');
  fs.writeFileSync(file, `const N = ${n};\nconsole.log('METRIC=' + N);\n`);
  const g = (...a: string[]) => execFileSync('git', ['-C', dir, ...a], { stdio: 'ignore', env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t' } });
  g('init', '-q'); g('add', 'bench.js'); g('commit', '-qm', 'init');
  return { dir, file };
}
const read = (f: string) => fs.readFileSync(f, 'utf8');
const tunable = (candidates: string[]): Tunable => ({
  id: 'N', file: 'bench.js', find: 'N = (\\d+)', candidates,
  benchmark: { cmd: 'node', args: ['bench.js'], metric: { parse: 'METRIC=(\\d+)' } },
});

test('configTuneDetector — emits a repo-perf candidate per non-current value; current is skipped', () => {
  const { dir } = repo(100);
  const cands = configTuneDetector(dir, [tunable(['50', '100', '200'])]);
  const ids = cands.map((c) => c.id).sort();
  assert.deepEqual(ids, ['repo-perf:tune:N:200', 'repo-perf:tune:N:50']); // 100 == current → skipped
});

test('full pipeline — detector → classify A → oracle KEEPS a faster value (file tuned on disk)', async () => {
  const { dir, file } = repo(100);
  const [cand] = configTuneDetector(dir, [tunable(['50'])]);
  assert.equal(classify(cand!).class, 'A', 'a config tune is Class A');
  const built = buildHooks(cand!);
  assert.ok('hooks' in built, 'repo-perf candidate yields oracle hooks');
  const v = await evaluate((built as { hooks: any }).hooks);
  assert.equal(v.decision, 'kept');
  assert.equal(v.speedup_pct, 50); // 100 → 50
  assert.match(read(file), /N = 50/, 'tuned value kept on disk');
});

test('full pipeline — a SLOWER value is atomically rolled back (file restored)', async () => {
  const { dir, file } = repo(100);
  const [cand] = configTuneDetector(dir, [tunable(['200'])]); // 100 → 200 = slower
  const built = buildHooks(cand!);
  const v = await evaluate((built as { hooks: any }).hooks);
  assert.equal(v.decision, 'rolled_back');
  assert.ok(v.speedup_pct! < 0);
  assert.match(read(file), /N = 100/, 'git restored the original value');
});

test('loadTunables — absent declaration → [] (safe default; no auto-tuning without an operator declaration)', () => {
  const { dir } = repo();
  assert.deepEqual(loadTunables(dir), []);
});

test('loadTunables — reads loops/autoupgrade.tunables.json', () => {
  const { dir } = repo();
  fs.mkdirSync(path.join(dir, 'loops'), { recursive: true });
  fs.writeFileSync(path.join(dir, 'loops', 'autoupgrade.tunables.json'), JSON.stringify({ tunables: [tunable(['50'])] }));
  assert.equal(loadTunables(dir).length, 1);
});
