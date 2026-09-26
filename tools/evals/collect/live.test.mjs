import test from 'node:test';
import assert from 'node:assert/strict';
import { tree } from './fixture.mjs';
import { bootPaths, apiPaths, photoRows, probe, collect, RUNS } from './live.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));

/** A fake edge: routes by path, counts calls. */
export function edge(routes) {
  const calls = [];
  const f = async url => {
    const u = new URL(url);
    calls.push(u.pathname + u.search);
    const r = routes[u.pathname + u.search] ?? routes[u.pathname] ?? { status: 404, body: '' };
    return new Response(r.body ?? '', { status: r.status ?? 200, headers: r.headers ?? {} });
  };
  f.calls = calls;
  return f;
}

test('boot paths are the served store boot files, largest first, capped', () => {
  const p = bootPaths(tree(), 3);
  assert.equal(p.length, 3);
  assert.ok(p.every(x => x.startsWith('/') && !x.endsWith('index.html')));
});

test('api paths name the slug', () => {
  assert.equal(apiPaths('v').menu, '/api/public/locations/v/menu?locale=en');
});

test('photo rows are found at any depth', () => {
  const j = { a: [{ imageUrl: 'x', sub: { imageUrl: null } }], b: 1, c: null };
  assert.equal(photoRows(j).length, 2);
});

test('probe takes the median of three and the last response', async () => {
  const f = edge({ '/x': { body: 'abc', headers: { 'cf-cache-status': 'MISS' } } });
  const p = await probe(f, 'https://h/x');
  assert.equal(f.calls.length, RUNS);
  assert.equal(p.bytes, 3);
  assert.equal(p.cache, 'MISS');
});

test('the live collector: timings, cache, revalidating boot files, photos', async () => {
  const menu = { location: { id: 'L' }, items: [{ imageUrl: '/media/a', imageUrlSmall: '/media/a-s' }, { imageUrl: '/media/b', imageUrlSmall: null }, { imageUrl: null }] };
  const f = edge({
    '/': { body: '<html>' },
    '/api/public/locations/v/menu?locale=en': { body: JSON.stringify(menu), headers: { 'cf-cache-status': 'HIT' } },
    '/media/a-s': { body: 'x'.repeat(10) }, '/media/b': { body: 'x'.repeat(30) },
    '/store/app.js': { body: 'js', headers: { 'cache-control': 'public, max-age=0, must-revalidate' } },
  });
  const r = byId(await collect({ root: tree(), host: 'https://v.dowiz.org', fetch: f }));
  assert.equal(r['live.root.status_ok'].value, 1);
  assert.equal(r['live.api.rates.status_ok'].value, 0);
  assert.equal(r['live.api.rates.status_ok'].note, 'status 404');
  assert.equal(r['live.boot_files_revalidating'].value, 1);
  assert.equal(r['live.api.menu.edge_hit'].value, 1);
  assert.equal(r['live.photos.products'].value, 2);
  assert.equal(r['live.photos.small_permille'].value, 500);
  assert.equal(r['live.photos.mean_bytes'].value, 20);
  assert.equal(r['live.root.ttfb_ms'].rule, 'plus25');
});

test('a menu that is not json, has no photos, and is not cached', async () => {
  const f = edge({ '/api/public/locations/v/menu?locale=en': { body: 'oops' } });
  const r = byId(await collect({ root: tree(), host: 'https://v.dowiz.org', fetch: f }));
  assert.equal(r['live.api.menu.edge_hit'].value, 0);
  assert.equal(r['live.api.menu.edge_hit'].note, 'cf-cache-status absent');
  assert.equal(r['live.photos.small_permille'].value, null);
  assert.equal(r['live.photos.mean_bytes'].value, null);
  assert.match(r['live.root.ttfb_ms'].note, /cf-cache-status -/);
});

test('the live and product collectors fall back to the global fetch', async () => {
  const { collect: product } = await import('./product.mjs');
  const saved = globalThis.fetch;
  globalThis.fetch = edge({ '/api/auth/login': { body: '{"access_token":"T"}' } });
  try {
    const r = byId(await collect({ root: tree(), host: 'https://v.dowiz.org' }));
    assert.equal(r['live.root.status_ok'].value, 0);
    const p = byId(await product({ hosts: ['https://v.dowiz.org'], creds: { OWNER_EMAIL: 'e', OWNER_PASSWORD: 'p' }, now: () => 0 }));
    assert.equal(p['product.v.orders_7d'].value, 0);
  } finally { globalThis.fetch = saved; }
});
