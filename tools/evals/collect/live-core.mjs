// THE PURE HALF OF THE LIVE COLLECTOR: what is probed and how it becomes indicators.
//
// NO node:* IMPORT, on purpose: live.mjs runs it from the box (ids `live.*`), and dowiz-watch
// (workers/watch/src/evals.js) runs the SAME code from inside Cloudflare (ids `edge.*`, W-EVALSCF
// 2026-10-08: Bot Fight Mode challenges GitHub's runners, so the GitHub nightly reads the watcher).
// `get(url)` is one timed GET: {status, ttfb, total, bytes, cache, cacheControl, body} where body
// is a string or a Buffer.
import { slugOf, key } from './http.mjs';
import { ind, median } from '../rules.mjs';

export const RUNS = 3;
export const BOOT_FILES = 6;
export const PHOTOS = 10;

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

/** Median of `runs` timed GETs; status, size and headers of the last. */
export async function probeWith(get, url, runs = RUNS) {
  const rs = [];
  for (let i = 0; i < runs; i++) rs.push(await get(url));
  const last = rs[rs.length - 1];
  return { status: last.status, ttfb: median(rs.map(r => r.ttfb)), total: median(rs.map(r => r.total)),
    bytes: last.bytes, cache: last.cache, cacheControl: last.cacheControl, body: last.body };
}

const text = b => (typeof b === 'string' ? b : b.toString('utf8'));

/** Root + the boot files: timings per file, and how many boot files revalidate. */
export async function probeFiles(get, host, boot, prefix = 'live') {
  const out = [];
  const src = u => `GET ${host}${u}, median of ${RUNS}`;
  const timed = async (id, u) => {
    const p = await probeWith(get, host + u);
    out.push(ind(`${prefix}.${id}.ttfb_ms`, p.ttfb, 'ms', 'plus25', src(u), { note: `status ${p.status}; cf-cache-status ${p.cache || '-'}` }));
    out.push(ind(`${prefix}.${id}.total_ms`, p.total, 'ms', 'plus25', src(u)));
    out.push(ind(`${prefix}.${id}.status_ok`, p.status >= 200 && p.status < 400 ? 1 : 0, 'bool', 'min', src(u), { limit: 1, note: `status ${p.status}` }));
    return p;
  };
  await timed('root', '/');
  let revalidating = 0;
  for (const u of boot) {
    const p = await timed(`file.${key(u.split('/').pop())}`, u);
    if (/max-age=0\b|no-cache/.test(p.cacheControl)) revalidating += 1;
  }
  out.push(ind(`${prefix}.boot_files_revalidating`, revalidating, 'files', 'ratchet',
    `boot files answering max-age=0 (O2 closes it), of the ${BOOT_FILES} largest`));
  return out;
}

/** The public API reads, the menu's edge cache, and the photo story. */
export async function probeApi(get, host, prefix = 'live') {
  const out = [];
  const src = u => `GET ${host}${u}, median of ${RUNS}`;
  const slug = slugOf(host);
  let menu = null;
  for (const [id, u] of Object.entries(apiPaths(slug))) {
    const p = await probeWith(get, host + u);
    out.push(ind(`${prefix}.api.${id}.ttfb_ms`, p.ttfb, 'ms', 'plus25', src(u), { note: `status ${p.status}; cf-cache-status ${p.cache || '-'}` }));
    out.push(ind(`${prefix}.api.${id}.total_ms`, p.total, 'ms', 'plus25', src(u)));
    out.push(ind(`${prefix}.api.${id}.status_ok`, p.status >= 200 && p.status < 400 ? 1 : 0, 'bool', 'min', src(u), { limit: 1, note: `status ${p.status}` }));
    if (id === 'menu') menu = p;
  }
  out.push(ind(`${prefix}.api.menu.edge_hit`, menu.cache === 'HIT' ? 1 : 0, 'bool', 'min', src(apiPaths(slug).menu), { limit: 1, note: `cf-cache-status ${menu.cache || 'absent'}` }));
  let rows = [];
  try { rows = photoRows(JSON.parse(text(menu.body))); } catch { rows = []; }
  const withPhoto = rows.filter(r => r.imageUrl);
  const small = withPhoto.filter(r => r.imageUrlSmall).length;
  out.push(ind(`${prefix}.photos.products`, withPhoto.length, 'products', 'trend', src(apiPaths(slug).menu)));
  out.push(ind(`${prefix}.photos.small_permille`, withPhoto.length ? Math.round((1000 * small) / withPhoto.length) : null,
    'permille', 'floor', 'imageUrlSmall non-null / imageUrl non-null (O1 target 1000)'));
  const uniq = [...new Set(withPhoto.map(r => r.imageUrlSmall || r.imageUrl))].slice(0, PHOTOS);
  const sizes = [];
  for (const u of uniq) sizes.push((await get(new URL(u, host).href)).bytes);
  out.push(ind(`${prefix}.photos.mean_bytes`, sizes.length ? Math.round(sizes.reduce((a, b) => a + b, 0) / sizes.length) : null,
    'bytes', 'ratchet', `${sizes.length} photos as the storefront draws them (small when it exists)`));
  return out;
}
