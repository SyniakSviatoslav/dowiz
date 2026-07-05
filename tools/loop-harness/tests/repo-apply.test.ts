import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { evaluate } from '../src/oracle.js';
import { makeRepoHooks } from '../src/repo-apply.js';

// A throwaway git repo with a script that prints METRIC=<n>; the candidate patch
// lowers n (a deterministic, parseable "speedup"). This exercises the FULL
// auto-apply path: apply → benchmark → green → security → KEEP (left on disk) or
// atomic ROLLBACK (git checkout restores exact bytes).
function repo(metric = 100): { dir: string; file: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'repoperf-'));
  const file = path.join(dir, 'metric.js');
  fs.writeFileSync(file, `console.log('METRIC=${metric}');\n`);
  const g = (...a: string[]) => execFileSync('git', ['-C', dir, ...a], { stdio: 'ignore', env: { ...process.env, GIT_AUTHOR_NAME: 't', GIT_AUTHOR_EMAIL: 't@t', GIT_COMMITTER_NAME: 't', GIT_COMMITTER_EMAIL: 't@t' } });
  g('init', '-q');
  g('add', 'metric.js');
  g('commit', '-qm', 'init');
  return { dir, file };
}
const read = (f: string) => fs.readFileSync(f, 'utf8');
const bench = (dir: string) => ({ cmd: 'node', args: ['metric.js'], cwd: dir, metric: { parse: /METRIC=(\d+)/ } } as const);

test('repo-apply — faster + green + secure → KEPT (patch stays on disk)', async () => {
  const { dir, file } = repo(100);
  const v = await evaluate(makeRepoHooks({
    repoDir: dir, paths: ['metric.js'],
    applyPatch: () => fs.writeFileSync(file, `console.log('METRIC=80');\n`), // 20% lower
    benchmark: bench(dir), green: () => true, security: () => true,
  }));
  assert.equal(v.decision, 'kept');
  assert.equal(v.speedup_pct, 20);
  assert.match(read(file), /METRIC=80/, 'kept change remains on disk');
});

test('repo-apply — tests RED → ATOMIC ROLLBACK (git restores exact bytes)', async () => {
  const { dir, file } = repo(100);
  const v = await evaluate(makeRepoHooks({
    repoDir: dir, paths: ['metric.js'],
    applyPatch: () => fs.writeFileSync(file, `console.log('METRIC=80');\n`),
    benchmark: bench(dir), green: () => false, security: () => true,
  }));
  assert.equal(v.decision, 'rolled_back');
  assert.match(v.reason, /RED/);
  assert.match(read(file), /METRIC=100/, 'main restored to original by git checkout');
});

test('repo-apply — security regression → ATOMIC ROLLBACK', async () => {
  const { dir, file } = repo(100);
  const v = await evaluate(makeRepoHooks({
    repoDir: dir, paths: ['metric.js'],
    applyPatch: () => fs.writeFileSync(file, `console.log('METRIC=80');\n`),
    benchmark: bench(dir), green: () => true, security: () => false,
  }));
  assert.equal(v.decision, 'rolled_back');
  assert.match(v.reason, /security/i);
  assert.match(read(file), /METRIC=100/);
});

test('repo-apply — no speedup (2% < 5%) → ROLLBACK (added risk for nothing)', async () => {
  const { dir, file } = repo(100);
  const v = await evaluate(makeRepoHooks({
    repoDir: dir, paths: ['metric.js'],
    applyPatch: () => fs.writeFileSync(file, `console.log('METRIC=98');\n`),
    benchmark: bench(dir), green: () => true, security: () => true,
  }));
  assert.equal(v.decision, 'rolled_back');
  assert.match(v.reason, /no proven speedup/);
  assert.match(read(file), /METRIC=100/);
});

test('repo-apply — dirty path → NOT reversible → refuse to apply', async () => {
  const { dir, file } = repo(100);
  fs.writeFileSync(file, `console.log('METRIC=999');\n`); // uncommitted change → dirty
  let applied = false;
  const v = await evaluate(makeRepoHooks({
    repoDir: dir, paths: ['metric.js'],
    applyPatch: () => { applied = true; },
    benchmark: bench(dir), green: () => true, security: () => true,
  }));
  assert.equal(v.decision, 'rolled_back');
  assert.match(v.reason, /reversible/i);
  assert.equal(applied, false, 'must not apply when the tree is dirty (revert could not restore exactly)');
});
