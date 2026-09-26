// SUITE nightly · LIVE: GET-only probes of one venue host. Median of three for every clock
// (§B.1.4); cache status and cache-control per file; the photo story from the live menu.
import path from 'node:path';
import { bootGraph, weigh } from './graph.mjs';
import { timedGet, slugOf, key } from './net.mjs';
import { ind, median } from '../rules.mjs';

export const RUNS = 3;
export const BOOT_FILES = 6;
export const PHOTOS = 10;

/** The storefront's boot files as served paths, the largest first, capped. */
export function bootPaths(root, n = BOOT_FILES) {
  const pub = path.join(root, 'workers/api/public');
  const files = bootGraph(pub, 'store/index.html').files.filter(f => !f.endsWith('index.html'));
  return files.map(f => ({ f, w: weigh([f]).raw })).sort((a, b) => b.w - a.w).slice(0, n)
    .map(x => '/' + path.relative(pub, x.f));
}

export function apiPaths(slug) {
  return {
    menu: `/api/public/locations/${slug}/menu?locale=en`,
    menu_fresh: `/api/public/locations/${slug}/menu?locale=en&fresh=1`,
    manifest: '/manifest.webmanifest',
    consent: '/api/public/consent/wordings',
    rates: '/api/public/rates',
  };
}

/** Every object in a JSON tree that carries an `imageUrl` key. */
export function photoRows(j, out = []) {
  if (Array.isArray(j)) for (const x of j) photoRows(x, out);
  else if (j && typeof j === 'object') {
    if ('imageUrl' in j) out.push(j);
    for (const v of Object.values(j)) if (v && typeof v === 'object') photoRows(v, out);
  }
  return out;
}

export async function probe(fetchFn, url, runs = RUNS) {
  const rs = [];
  for (let i = 0; i < runs; i++) rs.push(await timedGet(fetchFn, url));
  const last = rs[rs.length - 1];
  return { status: last.status, ttfb: median(rs.map(r => r.ttfb)), total: median(rs.map(r => r.total)),
    bytes: last.bytes, cache: last.cache, cacheControl: last.cacheControl, body: last.body };
}

export async function collect(ctx) {
  const host = ctx.host;
  const f = ctx.fetch ?? globalThis.fetch;
  const out = [];
  const src = u => `GET ${host}${u}, median of ${RUNS}`;
  const timed = async (id, u) => {
    const p = await probe(f, host + u);
    out.push(ind(`live.${id}.ttfb_ms`, p.ttfb, 'ms', 'plus25', src(u), { note: `status ${p.status}; cf-cache-status ${p.cache || '-'}` }));
    out.push(ind(`live.${id}.total_ms`, p.total, 'ms', 'plus25', src(u)));
    out.push(ind(`live.${id}.status_ok`, p.status >= 200 && p.status < 400 ? 1 : 0, 'bool', 'min', src(u), { limit: 1, note: `status ${p.status}` }));
    return p;
  };
  await timed('root', '/');
  let revalidating = 0;
  for (const u of bootPaths(ctx.root)) {
    const p = await timed(`file.${key(path.basename(u))}`, u);
    if (/max-age=0\b|no-cache/.test(p.cacheControl)) revalidating += 1;
  }
  out.push(ind('live.boot_files_revalidating', revalidating, 'files', 'ratchet',
    `boot files answering max-age=0 (O2 closes it), of the ${BOOT_FILES} largest`));
  const slug = slugOf(host);
  let menu = null;
  for (const [id, u] of Object.entries(apiPaths(slug))) {
    const p = await timed(`api.${id}`, u);
    if (id === 'menu') menu = p;
  }
  out.push(ind('live.api.menu.edge_hit', menu.cache === 'HIT' ? 1 : 0, 'bool', 'min', src(apiPaths(slug).menu), { limit: 1, note: `cf-cache-status ${menu.cache || 'absent'}` }));
  let rows = [];
  try { rows = photoRows(JSON.parse(menu.body.toString('utf8'))); } catch { rows = []; }
  const withPhoto = rows.filter(r => r.imageUrl);
  const small = withPhoto.filter(r => r.imageUrlSmall).length;
  out.push(ind('live.photos.products', withPhoto.length, 'products', 'trend', src(apiPaths(slug).menu)));
  out.push(ind('live.photos.small_permille', withPhoto.length ? Math.round((1000 * small) / withPhoto.length) : null,
    'permille', 'floor', 'imageUrlSmall non-null / imageUrl non-null (O1 target 1000)'));
  const uniq = [...new Set(withPhoto.map(r => r.imageUrlSmall || r.imageUrl))].slice(0, PHOTOS);
  const sizes = [];
  for (const u of uniq) sizes.push((await timedGet(f, new URL(u, host).href)).bytes);
  out.push(ind('live.photos.mean_bytes', sizes.length ? Math.round(sizes.reduce((a, b) => a + b, 0) / sizes.length) : null,
    'bytes', 'ratchet', `${sizes.length} photos as the storefront draws them (small when it exists)`));
  return out;
}
