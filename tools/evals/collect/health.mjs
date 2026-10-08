// SUITE nightly + live · HEALTH: each venue's own gauges from GET /api/owner/health, and the
// platform's from GET /api/platform/{health,errors}. Aggregates only: counts, ages, per mille.
// No order, no person, no error text beyond its `what` kind leaves this file (§B.1.1).
//
// MEMORY. A venue's "memory" is its images: usedCells of ceilingCells per image. Growth per
// day needs two readings, so it is derived from the previous run's JSON when one is given.
import { getJson, ownerToken, slugOf, key } from './net.mjs';
import { ind, unverified } from '../rules.mjs';

export const DAY_MS = 86_400_000;
export const OUTBOX_MAX_MS = 10 * 60_000;
export const PERMILLE_MAX = 800;
export const HEADROOM_DAYS_MIN = 30;

/** `errors` records grouped by their `what` kind (never their text). */
export function byKind(errors = []) {
  const out = {};
  for (const e of errors) {
    const k = key(String(e.what || e.kind || 'unknown'));
    out[k] = (out[k] || 0) + 1;
  }
  return out;
}

/**
 * Days until the image fills its ceiling at the growth seen since the previous reading;
 * null when there is no previous reading or it did not grow.
 */
export function headroomDays(img, prevCells, prevAt, at) {
  if (prevCells === undefined || prevCells === null || !(at > prevAt)) return null;
  const perDay = ((img.usedCells - prevCells) * DAY_MS) / (at - prevAt);
  if (perDay <= 0) return null;
  return Math.floor((img.ceilingCells - img.usedCells) / perDay);
}

export function venueIndicators(v, h, prev = {}, at = Date.now()) {
  const p = `health.${v}`;
  const src = `GET /api/owner/health (${v})`;
  const out = [];
  const n = x => (Array.isArray(x) ? x.length : 0);
  out.push(ind(`${p}.verdict_ok`, h.verdict === 'ok' ? 1 : 0, 'bool', 'min', src, { limit: 1, note: `verdict ${h.verdict}` }));
  out.push(ind(`${p}.errors`, n(h.errors), 'records', 'ratchet', src, { note: JSON.stringify(byKind(h.errors)) }));
  out.push(ind(`${p}.outbox.waiting`, h.outbox?.waiting ?? null, 'messages', 'trend', src));
  out.push(ind(`${p}.outbox.oldest_ms`, h.outbox?.oldestMs ?? null, 'ms', 'max', src, { limit: OUTBOX_MAX_MS }));
  out.push(ind(`${p}.outbox.failing`, h.outbox?.failing ?? null, 'messages', 'zero', src));
  out.push(ind(`${p}.quarantined`, n(h.quarantined), 'records', 'zero', src));
  out.push(ind(`${p}.stranded`, n(h.rebuild?.stranded), 'orders', 'zero', src));
  out.push(ind(`${p}.unheld`, n(h.rebuild?.unheld), 'orders', 'zero', src));
  out.push(ind(`${p}.witness_intact`, h.rebuild?.intact ? 1 : 0, 'bool', 'min', src, { limit: 1 }));
  out.push(ind(`${p}.backup_sealed`, h.backupSeal?.sealed ? 1 : 0, 'bool', 'min', src, { limit: 1, note: h.backupSeal?.scheme || '' }));
  const open = Object.entries(h.rails || {}).filter(([, r]) => r.open).map(([k]) => k);
  out.push(ind(`${p}.rails_open`, open.length, 'rails', 'zero', src, open.length ? { note: open.join(', ') } : {}));
  out.push(ind(`${p}.ebills.failures`, h.ebills?.failures ?? 0, 'count', 'zero', src));
  out.push(ind(`${p}.kitchen_unseen`, n(h.kitchen?.unseen), 'orders', 'trend', src));
  out.push(ind(`${p}.events`, h.events ?? null, 'events', 'trend', src));
  out.push(ind(`${p}.worst_used_permille`, h.worstUsedPerMille ?? null, 'permille', 'max', src, { limit: PERMILLE_MAX }));
  // A growing image is a sawtooth that doubles near its ceiling (gauges.rs): reported, not judged.
  out.push(ind(`${p}.worst_growing_permille`, h.worstGrowingPerMille ?? null, 'permille', 'trend', src));
  for (const [name, img] of Object.entries(h.images || {})) {
    const q = `${p}.image.${key(name)}`;
    out.push(ind(`${q}.used_cells`, img.usedCells, 'cells', 'trend', src, { note: `${img.usedPerMille}‰ of ${img.ceilingCells} (${img.grows ? 'grows' : 'fixed'})` }));
    out.push(ind(`${q}.used_permille`, img.usedPerMille, 'permille', img.grows ? 'trend' : 'max', src, img.grows ? {} : { limit: PERMILLE_MAX }));
    const pr = prev[`${q}.used_cells`];
    const d = headroomDays(img, pr?.value, pr?.collected_at, at);
    if (d !== null) out.push(ind(`${q}.headroom_days`, d, 'days', 'min', `${src}, growth since ${new Date(pr.collected_at).toISOString()}`, { limit: HEADROOM_DAYS_MIN }));
  }
  return out;
}

/** The platform object's routes answer an administrator only; anyone else gets a 404. */
export async function platform(f, host, token) {
  const out = [];
  const h = await getJson(f, `${host}/api/platform/health`, token);
  if (h.status !== 200 || !h.json) {
    out.push(unverified('platform.total_bytes', 'bytes', 'ratchet', 'GET /api/platform/health',
      `answered ${h.status}: needs a platform administrator's token (PLATFORM_EMAIL/PLATFORM_PASSWORD) and the H5 route deployed`));
  } else {
    const j = h.json;
    out.push(ind('platform.total_bytes', j.totalBytes, 'bytes', 'ratchet', 'GET /api/platform/health', { note: 'registry+identity+sessions+... crossing into the Worker' }));
    for (const [name, img] of Object.entries(j.images || {})) out.push(ind(`platform.image.${key(name)}.bytes`, img.bytes, 'bytes', 'trend', 'GET /api/platform/health'));
    out.push(ind('platform.worst_used_permille', j.worstUsedPerMille, 'permille', 'max', 'GET /api/platform/health', { limit: PERMILLE_MAX }));
    out.push(ind('platform.venues', j.venues, 'venues', 'trend', 'GET /api/platform/health'));
    out.push(j.wasmMemoryBytes == null
      ? unverified('platform.isolate_wasm_bytes', 'bytes', 'trend', 'GET /api/platform/health', 'the route runs natively (no wasm32 memory)')
      : ind('platform.isolate_wasm_bytes', j.wasmMemoryBytes, 'bytes', 'trend', 'GET /api/platform/health: this isolate\'s linear memory'));
  }
  const e = await getJson(f, `${host}/api/platform/errors`, token);
  out.push(e.status === 200 && e.json
    ? ind('platform.errors', (e.json.errors || e.json).length ?? 0, 'records', 'ratchet', 'GET /api/platform/errors')
    : unverified('platform.errors', 'records', 'ratchet', 'GET /api/platform/errors', `answered ${e.status}: platform administrators only`));
  return out;
}

export async function collect(ctx) {
  const f = ctx.fetch ?? globalThis.fetch;
  const out = [];
  let anyToken = null;
  for (const host of ctx.hosts) {
    const v = key(slugOf(host));
    const { token, why } = await ownerToken(f, host, ctx.creds);
    if (!token) { out.push(unverified(`health.${v}.verdict_ok`, 'bool', 'min', `GET ${host}/api/owner/health`, why)); continue; }
    anyToken = anyToken || { host, token };
    // AX0: the NIGHTLY flushes the object's counters (cf.mjs reads them after), so summed nights
    // never count a poll twice; a `live` run only looks, or it would erase what the nightly sums.
    const h = await getJson(f, `${host}/api/owner/health${ctx.suite === 'nightly' ? '?counters=flush' : ''}`, token);
    if (h.status !== 200 || !h.json) { out.push(ind(`health.${v}.reachable`, 0, 'bool', 'min', `GET ${host}/api/owner/health`, { limit: 1, note: `answered ${h.status}` })); continue; }
    (ctx.healths ||= []).push({ venue: v, counters: h.json.counters ?? null }); // read by cf.mjs (cost runs after health)
    out.push(...venueIndicators(v, h.json, ctx.previous || {}, ctx.now ? ctx.now() : Date.now()));
  }
  const admin = ctx.platformToken ? { host: ctx.hosts[0], token: ctx.platformToken } : anyToken;
  if (admin) out.push(...await platform(f, admin.host, admin.token));
  return out;
}
