// THE NIGHTLY EVALS' ZONE PROBES, FROM INSIDE CLOUDFLARE (W-EVALSCF, operator decision 2026-10-08).
//
// WHY. Bot Fight Mode on dowiz.org managed-challenged 33 probes of evals-nightly run 37766665168
// (user agent `node` = the live collector, Chromium = the ux collector) from GitHub's runners, and
// it stays ON. So every collector that touches *.dowiz.org runs HERE, once a night, and the GitHub
// job reads GET /evals/latest on workers.dev (outside the zone), as health-cron reads /status.
//
// SAME CODE, NOT A COPY. The indicators come from tools/evals/collect/{live-core,health,product}.mjs,
// imported as they stand (none of them imports node:*); only the timed GET is this runtime's own.
// Ids: the live probes are `edge.*` here (`live.*` stays the box's, with the box's baselines);
// health.* / product.* / platform.* keep their ids (counts, not clocks).
//
// THE SUBREQUEST BUDGET. Workers Free allows 50 subrequests per invocation. The probes need ~60, so
// one run is THREE Durable Object alarm invocations (PHASES), each <= 25: files 21, api <= 25,
// owner <= 12. The run is stored only when the last phase is done; /evals/latest never serves half.
//
// A Worker's clock advances only across I/O, which is exactly what a network timing needs.
import { probeFiles, probeApi } from '../../../tools/evals/collect/live-core.mjs';
import { collect as healthCollect } from '../../../tools/evals/collect/health.mjs';
import { collect as productCollect } from '../../../tools/evals/collect/product.mjs';
import { ind } from '../../../tools/evals/rules.mjs';
import { BOOT_FILES } from './boot-files.js';
import { UA } from './probes.js';

export const PHASES = ['files', 'api', 'owner'];
/** UTC HH:MM of the 5-minute tick that starts the nightly run (the GitHub job reads it at 03:41). */
export const DEFAULT_AT = '03:10';
/** A run that has not finished in this long is abandoned and may be restarted. */
export const RUN_TIMEOUT_MS = 30 * 60 * 1000;
export const FETCH_TIMEOUT_MS = 15000;

export const hostsOf = (env) => String(env.EVALS_HOSTS || 'sushi-durres dubin-sushi').split(/\s+/).filter(Boolean)
  .map((v) => `https://${v}.${env.WATCH_DOMAIN || 'dowiz.org'}`);

/** Does this 5-minute tick start the nightly run? Pure. */
export function isNightlyTick(atMs, env) {
  const d = new Date(atMs);
  const hm = `${String(d.getUTCHours()).padStart(2, '0')}:${String(d.getUTCMinutes()).padStart(2, '0')}`;
  return hm === (env.EVALS_AT || DEFAULT_AT);
}

/** Every request this module makes: the watcher's user agent and a timeout. */
export function wrapFetch(fetchFn) {
  return (url, init = {}) => fetchFn(url, {
    ...init,
    headers: { 'user-agent': UA, ...(init.headers || {}) },
    signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
  });
}

/** One timed GET. The body is kept (as text) only when it is text or JSON (the menu is the one parsed); photos are counted, not decoded. */
export async function timedGet(fetchFn, url, now = () => Date.now()) {
  const t0 = now();
  const r = await fetchFn(url, { redirect: 'manual' });
  const ttfb = now() - t0;
  const buf = new Uint8Array(await r.arrayBuffer());
  const textual = /json|text\//i.test(r.headers.get('content-type') || '');
  return {
    status: r.status,
    ttfb: Math.round(ttfb),
    total: Math.round(now() - t0),
    bytes: buf.length,
    cache: r.headers.get('cf-cache-status') || '',
    cacheControl: r.headers.get('cache-control') || '',
    body: textual ? new TextDecoder().decode(buf) : '',
  };
}

/** A collector that throws is a breach with its message (as tools/evals/run.mjs does), never a gap. */
async function guarded(name, f) {
  try {
    return await f();
  } catch (e) {
    return [ind(`${name}.collector_ok`, 0, 'bool', 'min', `dowiz-watch evals/${name}`,
      { limit: 1, note: String((e && e.message) || e).slice(0, 300) })];
  }
}

/** The previous run's private indicators as tools/evals' `previous` (for health headroom days). */
export function previousOf(latest) {
  if (!latest || !latest.private) return {};
  return Object.fromEntries(latest.private.map((i) => [i.id, { value: i.value, collected_at: latest.measuredAtMs }]));
}

/** Run ONE phase. Returns {public, private, healths}. */
export async function runPhase(phase, env, fetchFn, latest, now = () => Date.now()) {
  const f = wrapFetch(fetchFn);
  const get = (u) => timedGet(f, u, now);
  const hosts = hostsOf(env);
  if (phase === 'files') return { public: await guarded('edge', () => probeFiles(get, hosts[0], BOOT_FILES, 'edge')) };
  if (phase === 'api') return { public: await guarded('edge', () => probeApi(get, hosts[0], 'edge')) };
  if (phase === 'owner') {
    const ctx = {
      fetch: f, hosts, suite: 'nightly', now, previous: previousOf(latest),
      creds: { OWNER_EMAIL: env.EVALS_OWNER_EMAIL, OWNER_PASSWORD: env.EVALS_OWNER_PASSWORD },
    };
    const h = await guarded('health', () => healthCollect(ctx));
    const p = await guarded('product', () => productCollect(ctx));
    return { private: [...h, ...p], healths: ctx.healths || null };
  }
  throw new Error(`unknown phase ${phase}`);
}

/** A fresh run record. */
export function newRun(atMs, cause) {
  return { id: atMs, startedAtMs: atMs, cause, phase: 0, public: [], private: [], healths: null };
}

/**
 * Advance a run by one phase. Pure apart from `fetchFn`. Returns {run} while phases remain, or
 * {done} — the finished document, stamped with the time its last measurement was taken.
 */
export async function step(run, env, fetchFn, latest, now = () => Date.now()) {
  const phase = PHASES[run.phase];
  const got = await runPhase(phase, env, fetchFn, latest, now);
  const next = {
    ...run,
    phase: run.phase + 1,
    public: [...run.public, ...(got.public || [])],
    private: [...run.private, ...(got.private || [])],
    healths: got.healths !== undefined ? got.healths : run.healths,
  };
  if (next.phase < PHASES.length) return { run: next };
  return {
    done: {
      complete: true,
      runId: run.id,
      cause: run.cause,
      startedAtMs: run.startedAtMs,
      measuredAtMs: now(),
      hosts: hostsOf(env),
      bootFiles: BOOT_FILES,
      watcher: { version: env.WATCH_VERSION, commit: env.WATCH_COMMIT },
      public: next.public,
      private: next.private,
      healths: next.healths,
    },
  };
}

/** May a new run start now? Pure. A run in flight blocks it until it times out. */
export function canStart(running, nowMs) {
  return !running || nowMs - running.startedAtMs > RUN_TIMEOUT_MS;
}

/** Is this the bearer token the watcher was given? Length-checked, then every byte compared. */
export function authorized(req, env) {
  const want = env.EVALS_READ_TOKEN;
  if (!want) return { ok: false, why: 'the watcher has no EVALS_READ_TOKEN secret' };
  const got = (req.headers.get('authorization') || '').replace(/^Bearer\s+/i, '');
  let diff = got.length ^ want.length;
  for (let i = 0; i < want.length; i++) diff |= want.charCodeAt(i) ^ (got.charCodeAt(i) || 0);
  return diff === 0 ? { ok: true } : { ok: false, why: got ? 'wrong bearer token' : 'no bearer token (EVALS_READ_TOKEN)' };
}

/**
 * GET /evals/latest. Pure. The edge timings are public (anyone can time a public page); the
 * venues' aggregates (health, orders, bookings, counters) go only to the bearer of EVALS_READ_TOKEN.
 */
export function latestView(state, auth, nowMs) {
  const running = state.running ? { startedAtMs: state.running.startedAtMs, phase: PHASES[state.running.phase] || 'done', cause: state.running.cause } : null;
  if (!state.latest) return { status: 404, body: { complete: false, error: 'no finished evals run yet', running } };
  const l = state.latest;
  const body = {
    ...l,
    ageSeconds: Math.floor((nowMs - l.measuredAtMs) / 1000),
    running,
    private: auth.ok ? l.private : null,
    healths: auth.ok ? l.healths : null,
    privateWhy: auth.ok ? '' : auth.why,
  };
  return { status: 200, body };
}
