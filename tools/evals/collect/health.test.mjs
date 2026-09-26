import test from 'node:test';
import assert from 'node:assert/strict';
import { byKind, headroomDays, venueIndicators, platform, collect, DAY_MS } from './health.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));
const J = (body, status = 200) => new Response(typeof body === 'string' ? body : JSON.stringify(body), { status });

const H = {
  verdict: 'ok', errors: [{ what: 'outbox.abandoned', text: 'secret person' }, { kind: 'x' }, {}],
  outbox: { waiting: 2, oldestMs: 1000, failing: 0 }, quarantined: [], rebuild: { stranded: [], unheld: [1], intact: true },
  backupSeal: { sealed: true, scheme: 'age' }, rails: { fiscal: { open: true }, stripe: { open: false } }, ebills: {},
  kitchen: { unseen: [] }, events: 9, worstUsedPerMille: 700, worstGrowingPerMille: 967,
  images: { log: { usedCells: 200, ceilingCells: 1200, usedPerMille: 166, grows: true }, 'hub-ops': { usedCells: 900, ceilingCells: 1000, usedPerMille: 900, grows: false } },
};

test('errors group by kind and never by text', () => {
  assert.deepEqual(byKind(H.errors), { outbox_abandoned: 1, x: 1, unknown: 1 });
  assert.deepEqual(byKind(), {});
});

test('headroom days from growth since the last reading; none without growth', () => {
  const img = { usedCells: 200, ceilingCells: 1200 };
  assert.equal(headroomDays(img, 100, 0, DAY_MS), 10);
  assert.equal(headroomDays(img, 200, 0, DAY_MS), null);
  assert.equal(headroomDays(img, undefined, 0, DAY_MS), null);
  assert.equal(headroomDays(img, null, 0, DAY_MS), null);
  assert.equal(headroomDays(img, 100, DAY_MS, DAY_MS), null);
});

test('a venue: every gauge, growing images reported not judged, a fixed image over 800 judged', () => {
  const prev = { 'health.v.image.log.used_cells': { value: 100, collected_at: 0 } };
  const r = byId(venueIndicators('v', H, prev, DAY_MS));
  assert.equal(r['health.v.verdict_ok'].value, 1);
  assert.equal(r['health.v.errors'].value, 3);
  assert.doesNotMatch(r['health.v.errors'].note, /secret/);
  assert.equal(r['health.v.unheld'].value, 1);
  assert.equal(r['health.v.rails_open'].note, 'fiscal');
  assert.equal(r['health.v.backup_sealed'].note, 'age');
  assert.equal(r['health.v.worst_growing_permille'].rule, 'trend');
  assert.equal(r['health.v.image.log.used_permille'].rule, 'trend');
  assert.equal(r['health.v.image.hub_ops.used_permille'].limit, 800);
  assert.match(r['health.v.image.hub_ops.used_cells'].note, /fixed/);
  assert.equal(r['health.v.image.log.headroom_days'].value, 10);
  assert.equal(r['health.v.image.hub_ops.headroom_days'], undefined);
});

test('a sparse health answer: nulls stay null, absent lists count zero', () => {
  const r = byId(venueIndicators('v', { verdict: 'watch' }));
  assert.equal(r['health.v.verdict_ok'].value, 0);
  assert.equal(r['health.v.outbox.oldest_ms'].value, null);
  assert.equal(r['health.v.quarantined'].value, 0);
  assert.equal(r['health.v.rails_open'].note, undefined);
  assert.equal(r['health.v.backup_sealed'].note, '');
  assert.equal(r['health.v.worst_used_permille'].value, null);
  assert.equal(r['health.v.events'].value, null);
});

const router = routes => async (url, o) => {
  const p = new URL(url).pathname;
  if (p === '/api/auth/login') return routes.login ? J({ access_token: 'T' }) : J({}, 401);
  return routes[p] ? routes[p]() : J('no', 404);
};

test('the platform route: measured, and UNVERIFIED with the reason when refused', async () => {
  const ok = router({
    '/api/platform/health': () => J({ totalBytes: 99, images: { registry: { bytes: 90 } }, worstUsedPerMille: 10, venues: 2, wasmMemoryBytes: 65536 }),
    '/api/platform/errors': () => J({ errors: [1, 2] }),
  });
  const r = byId(await platform(ok, 'https://h', 'T'));
  assert.equal(r['platform.total_bytes'].value, 99);
  assert.equal(r['platform.image.registry.bytes'].value, 90);
  assert.equal(r['platform.isolate_wasm_bytes'].value, 65536);
  assert.equal(r['platform.errors'].value, 2);
  const native = byId(await platform(router({ '/api/platform/health': () => J({ totalBytes: 1 }), '/api/platform/errors': () => J([1]) }), 'https://h', 'T'));
  assert.match(native['platform.isolate_wasm_bytes'].unverified, /natively/);
  assert.equal(native['platform.errors'].value, 1);
  const no = byId(await platform(router({}), 'https://h', 'T'));
  assert.match(no['platform.total_bytes'].unverified, /answered 404/);
  assert.match(no['platform.errors'].unverified, /answered 404/);
});

test('the health collector: per venue, the refused login, the unreachable route, the platform token', async () => {
  const f = router({ login: true, '/api/owner/health': () => J(H), '/api/platform/health': () => J('x', 404) });
  const r = byId(await collect({ hosts: ['https://a.dowiz.org'], creds: {}, fetch: f }));
  assert.match(r['health.a.verdict_ok'].unverified, /no owner credentials/);
  const creds = { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'p' };
  const g = byId(await collect({ hosts: ['https://a.dowiz.org'], creds, fetch: f, now: () => 5, previous: {} }));
  assert.equal(g['health.a.verdict_ok'].value, 1);
  assert.ok(g['platform.total_bytes'].unverified);
  const down = byId(await collect({ hosts: ['https://b.dowiz.org'], creds, fetch: router({ login: true }), platformToken: 'P' }));
  assert.equal(down['health.b.reachable'].value, 0);
  assert.equal(down['health.b.reachable'].note, 'answered 404');
  assert.ok(down['platform.errors']);
});

test('the platform error list may be a bare object; the global fetch is the default', async () => {
  const e = byId(await platform(router({ '/api/platform/health': () => J('x', 500), '/api/platform/errors': () => J({}) }), 'https://h', 'T'));
  assert.equal(e['platform.errors'].value, 0);
  const saved = globalThis.fetch;
  globalThis.fetch = router({ login: true, '/api/owner/health': () => J(H) });
  try {
    const r = byId(await collect({ hosts: ['https://a.dowiz.org'], creds: { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'p' } }));
    assert.equal(r['health.a.verdict_ok'].value, 1);
    assert.equal(r['health.a.image.log.headroom_days'], undefined, 'no previous reading, no growth');
  } finally { globalThis.fetch = saved; }
});
