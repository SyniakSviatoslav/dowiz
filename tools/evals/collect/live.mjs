// SUITE nightly · LIVE: GET-only probes of one venue host. Median of three for every clock
// (§B.1.4); cache status and cache-control per file; the photo story from the live menu.
// The probing itself is live-core.mjs, which dowiz-watch also runs from inside Cloudflare (edge.*).
import path from 'node:path';
import { bootGraph, weigh } from './graph.mjs';
import { timedGet } from './net.mjs';
import { RUNS, BOOT_FILES, PHOTOS, apiPaths, photoRows, probeWith, probeFiles, probeApi } from './live-core.mjs';

export { RUNS, BOOT_FILES, PHOTOS, apiPaths, photoRows };

/** The storefront's boot files as served paths, the largest first, capped. */
export function bootPaths(root, n = BOOT_FILES) {
  const pub = path.join(root, 'workers/api/public');
  const files = bootGraph(pub, 'store/index.html').files.filter(f => !f.endsWith('index.html'));
  return files.map(f => ({ f, w: weigh([f]).raw })).sort((a, b) => b.w - a.w).slice(0, n)
    .map(x => '/' + path.relative(pub, x.f));
}

export const probe = (fetchFn, url, runs = RUNS) => probeWith(u => timedGet(fetchFn, u), url, runs);

export async function collect(ctx) {
  const f = ctx.fetch ?? globalThis.fetch;
  const get = u => timedGet(f, u);
  return [...await probeFiles(get, ctx.host, bootPaths(ctx.root)), ...await probeApi(get, ctx.host)];
}
