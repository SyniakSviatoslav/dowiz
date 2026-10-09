import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { collect, MAX_AGE_MS, DEFAULT_URL } from './watch.mjs';
import { bootPaths } from './live.mjs';
import { main, SUITES, UX_ELSEWHERE } from '../run.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));
const NOW = Date.UTC(2026, 9, 8, 3, 41);

export const doc = (over = {}) => ({
  complete: true, runId: 1, measuredAtMs: NOW - 31 * 60_000, watcher: { version: '0.1.0', commit: 'abc' },
  hosts: ['https://sushi-durres.dowiz.org', 'https://dubin-sushi.dowiz.org'], bootFiles: bootPaths(ROOT),
  public: [{ id: 'edge.root.ttfb_ms', value: 12, unit: 'ms', rule: 'plus25', source: 'GET /' }],
  private: [{ id: 'product.sushi_durres.orders_7d', value: 14, unit: 'orders', rule: 'trend', source: 'GET /api/owner/orders' }],
  healths: [{ venue: 'sushi_durres', counters: { since_total: 1 } }],
  ...over,
});
const serve = (status, body) => {
  const f = async (url, init) => { f.url = url; f.init = init; return new Response(typeof body === 'string' ? body : JSON.stringify(body), { status }); };
  return f;
};
const ctx = (f, env = {}) => ({ root: ROOT, fetch: f, env, now: () => NOW });

test('fresh: the watcher\'s rows, stamped with when they were measured; counters handed to cf.mjs', async () => {
  const f = serve(200, doc());
  const c = ctx(f, { WATCH_URL: 'https://w.example/', EVALS_READ_TOKEN: 'tok' });
  const r = byId(await collect(c));
  assert.equal(f.url, 'https://w.example/evals/latest');
  assert.equal(f.init.headers.authorization, 'Bearer tok');
  assert.equal(r['watch.evals_fresh'].value, 1);
  assert.match(r['watch.evals_fresh'].note, /measured 2026-10-08T03:10:00.000Z by dowiz-watch 0.1.0@abc, 1860 s before this run/);
  assert.equal(r['watch.boot_files_match'].value, 1);
  assert.equal(r['edge.root.ttfb_ms'].value, 12);
  assert.equal(r['edge.root.ttfb_ms'].measured_at, NOW - 31 * 60_000);
  assert.match(r['edge.root.ttfb_ms'].source, /^dowiz-watch: GET \//);
  assert.equal(r['product.sushi_durres.orders_7d'].value, 14);
  assert.deepEqual(c.healths, [{ venue: 'sushi_durres', counters: { since_total: 1 } }]);
});

test('the default URL is the deployed watcher; no token sends no header', async () => {
  const f = serve(200, doc());
  await collect({ root: ROOT, fetch: f, now: () => NOW });
  assert.equal(f.url, `${DEFAULT_URL}/evals/latest`);
  assert.deepEqual(f.init.headers, {});
});

test('RED: stale, missing, unfinished, unreadable -> watch.evals_fresh = 0 and NOTHING else is judged', async () => {
  const cases = [
    [serve(200, doc({ measuredAtMs: NOW - MAX_AGE_MS - 1 })), /STALE: measured/],
    [serve(200, doc({ measuredAtMs: NOW + 60_000 })), /STALE/], // from the future: a broken clock is not fresh
    [serve(404, { complete: false, error: 'no finished evals run yet' }), /answered 404/],
    [serve(200, doc({ complete: false })), /answered 200/],
    [serve(200, doc({ measuredAtMs: undefined })), /answered 200/],
    [serve(200, 'error code: 1042'), /unreadable/],
    [async () => { throw new Error('ECONNREFUSED'); }, /unreadable: ECONNREFUSED/],
  ];
  for (const [f, why] of cases) {
    const got = await collect(ctx(f));
    assert.equal(got.length, 1);
    assert.equal(got[0].id, 'watch.evals_fresh');
    assert.equal(got[0].value, 0);
    assert.match(got[0].note, why);
  }
  // exactly at the limit is still fresh
  assert.equal(byId(await collect(ctx(serve(200, doc({ measuredAtMs: NOW - MAX_AGE_MS })))))['watch.evals_fresh'].value, 1);
});

test('a deployed watcher probing other boot files is a breach that names both lists', async () => {
  const r = byId(await collect(ctx(serve(200, doc({ bootFiles: ['/old.css'] })))));
  assert.equal(r['watch.boot_files_match'].value, 0);
  assert.match(r['watch.boot_files_match'].note, /deployed \["\/old.css"\] != repo .*redeploy dowiz-watch/);
  assert.equal(byId(await collect(ctx(serve(200, doc({ bootFiles: undefined })))))['watch.boot_files_match'].value, 0);
});

test('without the token the venues\' aggregates are UNVERIFIED with the watcher\'s reason', async () => {
  const c = ctx(serve(200, doc({ private: null, healths: null, privateWhy: 'no bearer token (EVALS_READ_TOKEN)' })));
  const r = byId(await collect(c));
  assert.equal(r['health.sushi_durres.verdict_ok'].value, null);
  assert.equal(r['health.dubin_sushi.verdict_ok'].unverified, 'held by dowiz-watch: no bearer token (EVALS_READ_TOKEN)');
  assert.equal(r['product.dubin_sushi.orders_7d'].value, null);
  assert.equal(c.healths, undefined);
  const bare = byId(await collect(ctx(serve(200, doc({ private: null, hosts: ['https://v.dowiz.org'], public: undefined })))));
  assert.equal(bare['health.v.verdict_ok'].unverified, 'held by dowiz-watch: not served');
});

const tmp = () => fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cf-'));
const suiteRun = async (f) => main(['--suite', 'nightly-cf', '--out', tmp(), '--no-baseline-write'],
  { root: ROOT, fetch: f, env: { WATCH_URL: 'https://w.example', CF_ANALYTICS_FILE: '/nonexistent/cf' }, creds: {}, now: () => NOW, log: () => {}, exec: () => ({ status: 1, stdout: '' }) });

test('the GitHub suite never names a zone collector', () => {
  assert.deepEqual(SUITES['nightly-cf'], ['watch', 'order', 'ux-elsewhere', 'cost']);
  for (const z of ['live', 'health', 'product', 'ux']) assert.ok(!SUITES['nightly-cf'].includes(z), z);
});

test('RED: a STALE watcher result FAILS the job (exit 1, watch.evals_fresh in the breaches)', async () => {
  const r = await suiteRun(serve(200, doc({ measuredAtMs: NOW - MAX_AGE_MS - 1 })));
  assert.equal(r.code, 1);
  assert.equal(r.doc['watch.evals_fresh'].status, 'breach');
  assert.equal(r.doc['edge.root.ttfb_ms'], undefined); // stale numbers are not judged
  const missing = await suiteRun(serve(404, { complete: false }));
  assert.equal(missing.code, 1);
  assert.equal(missing.doc['watch.evals_fresh'].status, 'breach');
});

test('fresh: watch.evals_fresh ok, product counts reach the cost model, ux is one loud UNVERIFIED row', async () => {
  const r = await suiteRun(serve(200, doc()));
  assert.equal(r.doc['watch.evals_fresh'].status, 'ok');
  assert.equal(r.doc['watch.boot_files_match'].status, 'ok');
  assert.equal(r.doc['ux.measured_here'].status, 'unverified');
  assert.equal(r.doc['ux.measured_here'].unverified, UX_ELSEWHERE);
  assert.match(r.doc['cost.modelled_worker_requests_day'].source, /2 orders\/day \(1 venues' orders_7d \/ 7\)/);
  assert.match(fs.readFileSync(r.file, 'utf8'), /ux\.measured_here` — NOT MEASURED HERE/);
});
