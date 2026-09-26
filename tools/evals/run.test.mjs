import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  parseArgs, wasmCollector, loadCollector, git, commitOf, previousRun, readBaselines,
  judgeAll, writeBaselines, main, SUITES, HOSTS,
} from './run.mjs';
import { ind } from './rules.mjs';

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'evals-run-'));
const fakeGit = (head, dirty) => (cmd, args) => (args[0] === 'rev-parse' ? { status: head ? 0 : 1, stdout: `${head}\n` } : { status: 0, stdout: dirty });

test('arguments: defaults, every flag, and refusals', () => {
  const d = parseArgs([]);
  assert.equal(d.suite, 'ci');
  assert.deepEqual(d.hosts, HOSTS);
  assert.equal(d.writeBaselines, true);
  const a = parseArgs(['--suite', 'nightly', '--host', 'https://a/', '--out', '/o', '--only', 'live,cost', '--no-baseline-write']);
  assert.deepEqual(a, { suite: 'nightly', hosts: ['https://a'], out: '/o', writeBaselines: false, only: ['live', 'cost'] });
  assert.throws(() => parseArgs(['--bogus']), /unknown argument --bogus/);
  assert.throws(() => parseArgs(['--suite', 'weekly']), /unknown suite weekly/);
  assert.deepEqual(SUITES.live, ['health']);
});

test('the python collector: its JSON on success, its stderr on failure', async () => {
  const ok = wasmCollector(() => ({ status: 0, stdout: '[{"id":"wasm.raw","value":1}]' }));
  assert.deepEqual(await ok.collect({ root: '/' }), [{ id: 'wasm.raw', value: 1 }]);
  const bad = wasmCollector(() => ({ status: 2, stderr: 'Traceback boom' }));
  await assert.rejects(bad.collect({ root: '/' }), /wasm.py rc=2: Traceback boom/);
});

test('collectors load by name, the python one through its wrapper', async () => {
  assert.equal(typeof (await loadCollector('cost')).collect, 'function');
  assert.equal(typeof (await loadCollector('wasm')).collect, 'function');
});

test('the commit carries -dirty when tracked files moved, and nocommit outside git', () => {
  assert.equal(commitOf('/', fakeGit('abc', '')), 'abc');
  assert.equal(commitOf('/', fakeGit('abc', ' M x')), 'abc-dirty');
  assert.equal(commitOf('/', fakeGit('', '')), 'nocommit');
  assert.equal(git('/', ['x'], () => ({ status: 1 })), '');
});

test('previous runs and baselines read from disk; absent dirs are empty', () => {
  const d = tmp();
  assert.deepEqual(previousRun(path.join(d, 'none'), 'ci'), {});
  assert.deepEqual(previousRun(d, 'ci'), {});
  fs.writeFileSync(path.join(d, '2026-09-25-a-ci.json'), '{"n":1}');
  fs.writeFileSync(path.join(d, '2026-09-26-b-ci.json'), '{"n":2}');
  fs.writeFileSync(path.join(d, '2026-09-27-b-nightly.json'), '{"n":3}');
  assert.deepEqual(previousRun(d, 'ci'), { n: 2 });
  assert.deepEqual(readBaselines(path.join(d, 'none')), {});
  fs.writeFileSync(path.join(d, 'wasm.baseline'), '# h\nwasm.raw=10\n');
  assert.deepEqual(readBaselines(d), { wasm: { 'wasm.raw': '10' } });
});

test('judgeAll keys by id, carries the baseline, and collects only new or improved values', () => {
  const { results, changes } = judgeAll([
    ind('wasm.raw', 9, 'b', 'ratchet', 's'), ind('wasm.gzip', 5, 'b', 'ratchet', 's'),
    ind('gates.failed', 1, 'g', 'zero', 's'),
  ], { wasm: { 'wasm.raw': '10' } }, 42);
  assert.equal(results['wasm.raw'].status, 'improved');
  assert.equal(results['wasm.raw'].baseline, 10);
  assert.equal(results['wasm.raw'].collected_at, 42);
  assert.equal(results['wasm.raw'].id, undefined);
  assert.equal(results['wasm.gzip'].status, 'new');
  assert.equal(results['wasm.gzip'].baseline, null);
  assert.equal(results['gates.failed'].why, '1 != 0');
  assert.deepEqual(changes, { wasm: { 'wasm.raw': 9, 'wasm.gzip': 5 } });
  const u = judgeAll([ind('p.a', null, 's', 'trend', 's', { note: '0 orders' }), ind('p.b', null, 's', 'trend', 's'),
    ind('p.c', null, 's', 'trend', 's', { unverified: 'no token' })], {}, 1).results;
  assert.equal(u['p.a'].unverified, 'no value: 0 orders');
  assert.equal(u['p.b'].unverified, 'no value');
  assert.equal(u['p.c'].unverified, 'no token');
});

test('writeBaselines merges into the group file', () => {
  const d = path.join(tmp(), 'b');
  writeBaselines(d, { wasm: { 'wasm.code': '3' } }, { wasm: { 'wasm.raw': 9 } }, 'today');
  assert.match(fs.readFileSync(path.join(d, 'wasm.baseline'), 'utf8'), /# wasm — written by tools\/evals\/run.mjs today.*\nwasm.code=3\nwasm.raw=9\n/);
});

const deps = (root, collectors, extra = {}) => ({
  root, baselines: path.join(root, 'bl'), exec: fakeGit('abc', ''), creds: {}, env: {}, log: () => {},
  now: (() => { let t = Date.UTC(2026, 8, 26); return () => (t += 1000); })(),
  load: async n => collectors[n] || { collect: async () => [] }, ...extra,
});

test('main: a clean ci run writes json + md and the first baselines, exit 0', async () => {
  const root = tmp();
  const r = await main(['--suite', 'ci'], deps(root, { static: { collect: async ctx => [ind('surfaces.x.raw', 10, 'b', 'ratchet', 's'), ind('routes.count', ctx.hosts.length, 'r', 'trend', 's')] } }));
  assert.equal(r.code, 0);
  assert.match(r.file, /docs\/measurements\/evals\/2026-09-26-abc-ci\.md$/);
  assert.ok(fs.existsSync(r.file.replace(/\.md$/, '.json')));
  assert.equal(r.doc._run.deployed_version.startsWith('UNVERIFIED'), true);
  assert.deepEqual(r.doc._run.hosts, []);
  assert.match(fs.readFileSync(path.join(root, 'bl/surfaces.baseline'), 'utf8'), /surfaces.x.raw=10/);
});

test('main: a throwing collector is a breach with its message, and a breach exits 1', async () => {
  const root = tmp();
  const logs = [];
  const r = await main(['--suite', 'nightly', '--only', 'live,cost', '--no-baseline-write'], deps(root, {
    live: { collect: async () => { throw new Error('socket hang up'); } },
    cost: { collect: async ctx => [ind('cost.seen', Object.keys(ctx.results).length, 'n', 'trend', 's')] },
  }, { log: l => logs.push(l), env: { EVALS_DEPLOYED_VERSION: 'af62ab61' } }));
  assert.equal(r.code, 1);
  assert.equal(r.doc['live.collector_ok'].note, 'socket hang up');
  assert.equal(r.doc['cost.seen'].value, 1, 'cost sees the earlier collectors');
  assert.equal(r.doc._run.deployed_version, 'af62ab61');
  assert.ok(logs.some(l => l.startsWith('BREACH live.collector_ok')));
  assert.ok(!fs.existsSync(path.join(root, 'bl')));
});

test('main: the live suite appends one line per tick', async () => {
  const root = tmp();
  const d = deps(root, { health: { collect: async () => [ind('health.v.errors', 0, 'r', 'trend', 's')] } });
  await main(['--suite', 'live', '--out', path.join(root, 'o')], d);
  const r = await main(['--suite', 'live', '--out', path.join(root, 'o')], d);
  assert.match(r.file, /docs\/measurements\/live\/2026-09-26\.jsonl$/);
  assert.equal(fs.readFileSync(r.file, 'utf8').trim().split('\n').length, 2);
});

test('main: credentials default to the owner file when none are injected', async () => {
  const root = tmp();
  const d = deps(root, { cost: { collect: async ctx => [ind('cost.has_creds', typeof ctx.creds === 'object' ? 1 : 0, 'b', 'trend', 's')] } });
  delete d.creds;
  const r = await main(['--suite', 'nightly', '--only', 'cost'], d);
  assert.equal(r.doc['cost.has_creds'].value, 1);
});
