// SUITE nightly-cf · WATCH: the zone-touching collectors' numbers, read from dowiz-watch.
//
// WHY (operator decision 2026-10-08, W-EVALSCF). Bot Fight Mode on dowiz.org managed-challenges
// every request from GitHub's runners (33 in evals-nightly run 37766665168) and stays ON. So the
// live/health/product probes run inside Cloudflare (workers/watch/src/evals.js, the SAME collector
// code) at 03:10 UTC, and this collector reads GET <WATCH_URL>/evals/latest on workers.dev.
//
// A NUMBER WITHOUT A FRESH MEASUREMENT IS NOT A NUMBER. The document carries the time its last
// probe was taken; older than MAX_AGE_MS, missing, half-done or unreadable is `watch.evals_fresh`
// = 0, a BREACH (the job fails), and its numbers are not judged at all. A deployed watcher whose
// boot-file list differs from this checkout's is `watch.boot_files_match` = 0, also a breach.
// The venues' aggregates come only with EVALS_READ_TOKEN; without it they are UNVERIFIED.
import { bootPaths } from './live.mjs';
import { slugOf, key } from './http.mjs';
import { ind, unverified } from '../rules.mjs';

export const DEFAULT_URL = 'https://dowiz-watch.sviatoslavsyniak.workers.dev';
/** The watcher measures at 03:10 UTC, this job reads at 03:41; six hours absorbs GitHub's cron lag, not a missed night. */
export const MAX_AGE_MS = 6 * 3600_000;

const fail = (id, src, note) => ind(id, 0, 'bool', 'min', src, { limit: 1, note });

export async function collect(ctx) {
  const f = ctx.fetch ?? globalThis.fetch;
  const base = String(ctx.env?.WATCH_URL || DEFAULT_URL).replace(/\/$/, '');
  const url = `${base}/evals/latest`;
  const src = `GET ${url}`;
  const token = ctx.env?.EVALS_READ_TOKEN;
  let r;
  let doc = null;
  try {
    r = await f(url, { headers: token ? { authorization: `Bearer ${token}` } : {} });
    doc = JSON.parse(await r.text());
  } catch (e) {
    return [fail('watch.evals_fresh', src, `unreadable: ${String(e.message || e).slice(0, 200)}`)];
  }
  if (r.status !== 200 || !doc || doc.complete !== true || !Number.isFinite(doc.measuredAtMs)) {
    return [fail('watch.evals_fresh', src, `answered ${r.status}: ${JSON.stringify(doc).slice(0, 200)}`)];
  }
  const now = ctx.now ? ctx.now() : Date.now();
  const age = now - doc.measuredAtMs;
  const when = `measured ${new Date(doc.measuredAtMs).toISOString()} by dowiz-watch ${doc.watcher?.version}@${doc.watcher?.commit}, ${Math.round(age / 1000)} s before this run`;
  if (!(age >= 0 && age <= MAX_AGE_MS)) {
    return [fail('watch.evals_fresh', src, `STALE: ${when} (limit ${MAX_AGE_MS / 1000} s)`)];
  }
  const out = [ind('watch.evals_fresh', 1, 'bool', 'min', src, { limit: 1, note: when })];
  const want = bootPaths(ctx.root);
  const got = doc.bootFiles || [];
  const same = want.length === got.length && want.every((p, i) => p === got[i]);
  out.push(ind('watch.boot_files_match', same ? 1 : 0, 'bool', 'min', `${src} bootFiles vs bootPaths(this checkout)`,
    { limit: 1, note: same ? '' : `deployed ${JSON.stringify(got)} != repo ${JSON.stringify(want)}: redeploy dowiz-watch` }));
  const stamp = i => ({ ...i, measured_at: doc.measuredAtMs, source: `dowiz-watch: ${i.source}` });
  out.push(...(doc.public || []).map(stamp));
  if (Array.isArray(doc.private)) {
    out.push(...doc.private.map(stamp));
    if (doc.healths) ctx.healths = doc.healths; // read by cf.mjs (the AX0 counters), as collect/health.mjs does
  } else {
    for (const h of doc.hosts || []) {
      const v = key(slugOf(h));
      out.push(unverified(`health.${v}.verdict_ok`, 'bool', 'min', src, `held by dowiz-watch: ${doc.privateWhy || 'not served'}`));
      out.push(unverified(`product.${v}.orders_7d`, 'orders', 'trend', src, `held by dowiz-watch: ${doc.privateWhy || 'not served'}`));
    }
  }
  return out;
}
