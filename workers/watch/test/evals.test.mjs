// dowiz-watch's nightly evals (src/evals.js), driven with a fake edge. The Durable Object half
// (state.js: one alarm per phase) imports cloudflare:workers and is covered by the local replay and
// the live proof; everything it calls is here.
//   node --test workers/watch/test/
import { test } from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  PHASES, isNightlyTick, wrapFetch, timedGet, runPhase, step, newRun, canStart, authorized, latestView, previousOf, hostsOf, RUN_TIMEOUT_MS,
} from '../src/evals.js';
import { BOOT_FILES } from '../src/boot-files.js';
import { UA } from '../src/probes.js';
import { bootPaths, collect as liveCollect } from '../../../tools/evals/collect/live.mjs';

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const ENV = { WATCH_DOMAIN: 'dowiz.org', EVALS_HOSTS: 'v w', WATCH_VERSION: '0.1.0', WATCH_COMMIT: 'abc' };
const byId = (xs) => Object.fromEntries(xs.map((x) => [x.id, x]));

/** A fake edge: routes by path(+query), records every call with its init. */
function edge(routes) {
  const calls = [];
  const f = async (url, init = {}) => {
    const u = new URL(url);
    calls.push({ path: u.pathname + u.search, host: u.hostname, init });
    const r = routes[u.pathname + u.search] ?? routes[u.pathname] ?? { status: 404, body: '' };
    if (r.throw) throw new Error(r.throw);
    return new Response(r.body ?? '', { status: r.status ?? 200, headers: r.headers ?? {} });
  };
  f.calls = calls;
  return f;
}

const photos = Array.from({ length: 14 }, (_, i) => ({ imageUrl: `/media/p${i}`, imageUrlSmall: i % 2 ? `/media/p${i}-s` : null }));
const MENU = JSON.stringify({ categories: [{ products: photos }] });
const site = () => {
  const routes = {
    '/': { body: '<html>', headers: { 'content-type': 'text/html' } },
    '/api/public/locations/v/menu?locale=en': { body: MENU, headers: { 'content-type': 'application/json', 'cf-cache-status': 'HIT' } },
  };
  for (const p of photos) { routes[p.imageUrl] = { body: 'x'.repeat(100) }; routes[`${p.imageUrl}-s`] = { body: 'x'.repeat(10) }; }
  for (const b of BOOT_FILES) routes[b] = { body: 'b', headers: { 'cache-control': 'public, max-age=0' } };
  return edge(routes);
};

test('the deployed boot-file list is the repo\'s own bootPaths (regenerate: scripts/boot-files.mjs)', () => {
  assert.deepEqual(BOOT_FILES, bootPaths(ROOT));
});

test('the nightly tick is EVALS_AT UTC, default 03:10, and only that minute', () => {
  assert.equal(isNightlyTick(Date.UTC(2026, 9, 8, 3, 10), {}), true);
  assert.equal(isNightlyTick(Date.UTC(2026, 9, 8, 3, 15), {}), false);
  assert.equal(isNightlyTick(Date.UTC(2026, 9, 8, 4, 5), { EVALS_AT: '04:05' }), true);
});

test('hosts from EVALS_HOSTS, default the two venues', () => {
  assert.deepEqual(hostsOf(ENV), ['https://v.dowiz.org', 'https://w.dowiz.org']);
  assert.deepEqual(hostsOf({}), ['https://sushi-durres.dowiz.org', 'https://dubin-sushi.dowiz.org']);
});

test('every request carries the watcher UA and a timeout; a caller header survives', async () => {
  const f = edge({ '/x': { body: 'ok' } });
  await wrapFetch(f)('https://h/x', { headers: { authorization: 'Bearer t' } });
  assert.equal(f.calls[0].init.headers['user-agent'], UA);
  assert.equal(f.calls[0].init.headers.authorization, 'Bearer t');
  assert.ok(f.calls[0].init.signal instanceof AbortSignal);
});

test('timedGet: bytes always, body only for text/json, cache headers', async () => {
  let t = 0;
  const now = () => (t += 5);
  const f = edge({ '/j': { body: '{"a":1}', headers: { 'content-type': 'application/json', 'cf-cache-status': 'HIT' } }, '/img': { body: 'xxxx', headers: { 'content-type': 'image/webp' } } });
  const j = await timedGet(f, 'https://h/j', now);
  assert.deepEqual([j.status, j.bytes, j.body, j.cache, j.ttfb, j.total], [200, 7, '{"a":1}', 'HIT', 5, 10]);
  const i = await timedGet(f, 'https://h/img', now);
  assert.deepEqual([i.bytes, i.body], [4, '']);
});

test('SAME CODE: the watcher\'s phases give the box collector\'s indicators, live.* -> edge.*', async () => {
  const w = [...(await runPhase('files', ENV, site(), null)).public, ...(await runPhase('api', ENV, site(), null)).public];
  const box = await liveCollect({ root: ROOT, host: 'https://v.dowiz.org', fetch: site() });
  const strip = (i, from) => ({ ...i, id: i.id.replace(from, ''), ...(/_ms$/.test(i.id) ? { value: 'clock' } : {}), source: '' });
  assert.deepEqual(w.map((i) => strip(i, 'edge.')), box.map((i) => strip(i, 'live.')));
  const r = byId(w);
  assert.equal(r['edge.boot_files_revalidating'].value, 6);
  assert.equal(r['edge.api.menu.edge_hit'].value, 1);
  assert.equal(r['edge.photos.products'].value, 14);
  assert.equal(r['edge.photos.small_permille'].value, 500);
});

test('THE SUBREQUEST BUDGET: every phase stays <= 25 (Workers Free allows 50 per invocation)', async () => {
  for (const phase of ['files', 'api']) {
    const f = site();
    await runPhase(phase, ENV, f, null);
    assert.ok(f.calls.length <= 25, `${phase}: ${f.calls.length} subrequests`);
  }
  const f = edge({ '/api/auth/login': { body: '{"access_token":"T"}' } });
  await runPhase('owner', { ...ENV, EVALS_OWNER_EMAIL: 'e', EVALS_OWNER_PASSWORD: 'p' }, f, null);
  assert.ok(f.calls.length <= 25, `owner: ${f.calls.length} subrequests`);
});

test('owner phase: the SAME health + product collectors, the nightly flush, counters stashed', async () => {
  const H = { verdict: 'ok', errors: [], outbox: {}, rebuild: { intact: true }, backupSeal: { sealed: true }, images: { log: { usedCells: 10, ceilingCells: 100, usedPerMille: 100, grows: false } }, counters: { since_total: 3 } };
  const f = edge({
    '/api/auth/login': { body: '{"access_token":"T"}' },
    '/api/owner/health?counters=flush': { body: JSON.stringify(H) },
    '/api/owner/orders?since=0': { body: '{"orders":[]}' },
  });
  const latest = { measuredAtMs: 1, private: [{ id: 'health.v.image.log.used_cells', value: 5 }] };
  const got = await runPhase('owner', { ...ENV, EVALS_OWNER_EMAIL: 'e', EVALS_OWNER_PASSWORD: 'p' }, f, latest, () => 86_400_001);
  const r = byId(got.private);
  assert.equal(r['health.v.verdict_ok'].value, 1);
  assert.equal(r['health.v.image.log.headroom_days'].value, 18); // (100-10)/(5 cells a day), from the previous run
  assert.equal(r['product.v.orders_7d'].value, 0);
  assert.deepEqual(got.healths.map((h) => h.venue), ['v', 'w']);
  assert.ok(f.calls.some((c) => c.path === '/api/owner/health?counters=flush'));
  assert.ok(f.calls.every((c) => c.init.headers['user-agent'] === UA));
});

test('owner phase without the secrets: UNVERIFIED rows, never green', async () => {
  const got = await runPhase('owner', ENV, edge({}), null);
  const r = byId(got.private);
  assert.equal(r['health.v.verdict_ok'].value, null);
  assert.equal(r['health.v.verdict_ok'].unverified, 'no owner credentials');
  assert.equal(r['product.w.orders_7d'].unverified, 'no owner credentials');
});

test('a collector that throws is a collector_ok=0 row (a breach), never a gap', async () => {
  const boom = async () => { throw new Error('network gone'); };
  const files = byId((await runPhase('files', ENV, boom, null)).public);
  assert.equal(files['edge.collector_ok'].value, 0);
  assert.match(files['edge.collector_ok'].note, /network gone/);
  const owner = byId((await runPhase('owner', { ...ENV, EVALS_OWNER_EMAIL: 'e', EVALS_OWNER_PASSWORD: 'p' }, boom, null)).private);
  assert.equal(owner['health.collector_ok'].value, 0);
  assert.equal(owner['product.collector_ok'].value, 0);
  await assert.rejects(runPhase('nope', ENV, boom, null), /unknown phase/);
});

test('a run is three steps; only the last returns the finished, time-stamped document', async () => {
  let run = newRun(1000, 'cron');
  const seen = [];
  for (let i = 0; i < PHASES.length - 1; i++) {
    const r = await step(run, ENV, site(), null, () => 5000 + i);
    assert.ok(r.run && !r.done);
    run = r.run;
    seen.push(run.phase);
  }
  assert.deepEqual(seen, [1, 2]);
  const r = await step(run, ENV, edge({}), null, () => 9999);
  assert.equal(r.run, undefined);
  const d = r.done;
  assert.equal(d.complete, true);
  assert.equal(d.runId, 1000);
  assert.equal(d.startedAtMs, 1000);
  assert.equal(d.measuredAtMs, 9999);
  assert.deepEqual(d.bootFiles, BOOT_FILES);
  assert.deepEqual(d.watcher, { version: '0.1.0', commit: 'abc' });
  assert.ok(d.public.length > 40 && d.public.every((i) => i.id.startsWith('edge.')));
  assert.ok(d.private.length >= 4);
});

test('a run in flight blocks another until it times out', () => {
  assert.equal(canStart(null, 0), true);
  assert.equal(canStart({ startedAtMs: 0 }, RUN_TIMEOUT_MS), false);
  assert.equal(canStart({ startedAtMs: 0 }, RUN_TIMEOUT_MS + 1), true);
});

test('the bearer token: none configured, none sent, wrong, right', () => {
  const req = (h) => new Request('https://w/evals/latest', { headers: h });
  assert.deepEqual(authorized(req({ authorization: 'Bearer x' }), {}), { ok: false, why: 'the watcher has no EVALS_READ_TOKEN secret' });
  assert.equal(authorized(req({}), { EVALS_READ_TOKEN: 'sekret' }).why, 'no bearer token (EVALS_READ_TOKEN)');
  assert.equal(authorized(req({ authorization: 'Bearer sekreT' }), { EVALS_READ_TOKEN: 'sekret' }).why, 'wrong bearer token');
  assert.equal(authorized(req({ authorization: 'Bearer sekret!' }), { EVALS_READ_TOKEN: 'sekret' }).ok, false);
  assert.equal(authorized(req({ authorization: 'Bearer sekret' }), { EVALS_READ_TOKEN: 'sekret' }).ok, true);
});

test('/evals/latest: 404 before the first run; aggregates only to the token; age and the run in flight', () => {
  const none = latestView({ latest: null, running: { startedAtMs: 5, phase: 1, cause: 'cron' } }, { ok: true }, 10);
  assert.equal(none.status, 404);
  assert.equal(none.body.complete, false);
  assert.deepEqual(none.body.running, { startedAtMs: 5, phase: 'api', cause: 'cron' });
  const latest = { complete: true, measuredAtMs: 1000, public: [{ id: 'edge.x' }], private: [{ id: 'health.v.errors' }], healths: [{ venue: 'v' }] };
  const anon = latestView({ latest, running: null }, { ok: false, why: 'no bearer token' }, 61_000);
  assert.equal(anon.status, 200);
  assert.equal(anon.body.ageSeconds, 60);
  assert.equal(anon.body.private, null);
  assert.equal(anon.body.healths, null);
  assert.equal(anon.body.privateWhy, 'no bearer token');
  assert.deepEqual(anon.body.public, [{ id: 'edge.x' }]);
  const owner = latestView({ latest, running: null }, { ok: true }, 61_000);
  assert.deepEqual(owner.body.private, [{ id: 'health.v.errors' }]);
  assert.equal(owner.body.privateWhy, '');
});

test('previousOf: the last run\'s private values, stamped with when they were measured', () => {
  assert.deepEqual(previousOf(null), {});
  assert.deepEqual(previousOf({ measuredAtMs: 7, private: [{ id: 'a', value: 3 }] }), { a: { value: 3, collected_at: 7 } });
});
