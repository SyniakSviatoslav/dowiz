import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { parseArgs, derive, main, MIN_RUNS, RUN_TIMEOUT_MS, POLL_MS } from './edge-baseline.mjs';
import { parseBaseline } from './rules.mjs';

const run = (n, ttfb, extra = []) => ({
  complete: true, runId: n, measuredAtMs: Date.UTC(2026, 9, 8, 3, 10 + n), watcher: { commit: 'abc' },
  public: [
    { id: 'edge.root.ttfb_ms', value: ttfb, rule: 'plus25' },
    { id: 'edge.photos.mean_bytes', value: 1000 + n, rule: 'ratchet' },
    { id: 'edge.photos.small_permille', value: 500 + n, rule: 'floor' },
    { id: 'edge.root.status_ok', value: 1, rule: 'min', limit: 1 },
    { id: 'edge.photos.products', value: 9, rule: 'trend' },
    ...extra,
  ],
});

test('args: at least seven runs', () => {
  assert.equal(parseArgs([]).runs, MIN_RUNS);
  assert.throws(() => parseArgs(['--runs', '3']), /--runs must be >= 7/);
  assert.throws(() => parseArgs(['--x']), /unknown argument/);
  assert.equal(parseArgs(['--url', 'https://w/', '--replace', '--out', 'o', '--runs', '9']).url, 'https://w');
});

test('derive: plus25 median, ratchet max, floor min; no own run breaches its baseline; nulls are named not written', () => {
  const runs = [10, 12, 11, 30, 9, 13, 12].map((t, i) => run(i, t, [{ id: 'edge.api.rates.ttfb_ms', value: i === 3 ? null : 5, rule: 'plus25' }]));
  const { baseline, skipped, check } = derive(runs);
  assert.deepEqual(baseline, { 'edge.root.ttfb_ms': 12, 'edge.photos.mean_bytes': 1006, 'edge.photos.small_permille': 500 });
  assert.equal(skipped['edge.api.rates.ttfb_ms'], '6 of 7 runs had a value');
  assert.equal(check['edge.root.ttfb_ms'].breaches, 1); // the 30 ms outlier: shown, not hidden
  assert.equal(check['edge.photos.mean_bytes'].breaches, 0);
  assert.equal(check['edge.photos.small_permille'].breaches, 0);
});

const tmpOut = () => path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'edge-bl-')), 'edge.baseline');

/** A fake watcher: POST /evals/run starts a run that is finished at the next poll. */
function watcher({ refuse = false, never = false } = {}) {
  let n = 0;
  let latest = { complete: true, runId: 0, measuredAtMs: 0, public: [] };
  const calls = [];
  const f = async (url, init = {}) => {
    calls.push(`${init.method || 'GET'} ${new URL(url).pathname} ${init.headers?.authorization || ''}`);
    if (init.method === 'POST') {
      if (refuse) return new Response('{"error":"wrong bearer token"}', { status: 403 });
      n += 1;
      if (!never) latest = run(n, 10 + n);
      return new Response('{"started":true}', { status: 202 });
    }
    return new Response(JSON.stringify(latest));
  };
  f.calls = calls;
  return f;
}

test('main: seven fresh runs -> the file, with every run named in its header', async () => {
  const out = tmpOut();
  const f = watcher();
  const slept = [];
  const log = [];
  await main(['--out', out], { fetch: f, token: 'tok', sleep: async ms => { slept.push(ms); }, log: l => log.push(l), now: () => 0 });
  const text = fs.readFileSync(out, 'utf8');
  assert.deepEqual(parseBaseline(text), { 'edge.photos.mean_bytes': '1007', 'edge.photos.small_permille': '501', 'edge.root.ttfb_ms': '14' });
  assert.match(text, /from 7 runs: 2026-10-08T03:11:00.000Z@abc, .*2026-10-08T03:17:00.000Z@abc; plus25 = median/);
  assert.equal(f.calls.filter(c => c.startsWith('POST /evals/run Bearer tok')).length, 7);
  assert.ok(slept.every(ms => ms === POLL_MS));
  assert.ok(log.some(l => /^edge.root.ttfb_ms plus25 = 14 {2}runs 11 12 13 14 15 16 17 {2}breaches-of-own-runs 0$/.test(l)));
});

test('main refuses: no token, an existing file without --replace, a refused trigger, a run that never finishes', async () => {
  const deps = { sleep: async () => {}, log: () => {}, now: () => 0 };
  await assert.rejects(main(['--out', tmpOut()], { ...deps, fetch: watcher(), token: '' }), /no EVALS_READ_TOKEN/);
  const out = tmpOut();
  fs.writeFileSync(out, 'edge.x=1\n');
  await assert.rejects(main(['--out', out], { ...deps, fetch: watcher(), token: 't' }), /exists; pass --replace/);
  await assert.rejects(main(['--out', tmpOut()], { ...deps, fetch: watcher({ refuse: true }), token: 't' }), /answered 403/);
  let t = 0;
  await assert.rejects(main(['--out', tmpOut()], { ...deps, now: () => (t += RUN_TIMEOUT_MS), fetch: watcher({ never: true }), token: 't' }), /no finished run within 600 s/);
  await main(['--out', out, '--replace'], { ...deps, fetch: watcher(), token: 't' });
  assert.match(fs.readFileSync(out, 'utf8'), /edge.root.ttfb_ms=14/);
});

test('main refuses to write an empty baseline', async () => {
  let n = 0;
  const f = async (url, init = {}) => (init.method === 'POST' ? (n++, new Response('{}', { status: 202 }))
    : new Response(JSON.stringify({ complete: true, runId: n, measuredAtMs: 1, public: [{ id: 'edge.root.ttfb_ms', value: null, rule: 'plus25' }] })));
  await assert.rejects(main(['--out', tmpOut()], { fetch: f, token: 't', sleep: async () => {}, log: () => {}, now: () => 0 }), /nothing written/);
});
