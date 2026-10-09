// CLOUDFLARE'S OWN COUNT of what the platform served: GraphQL analytics, read-only.
//
// The token (Account Analytics:Read, nothing else) is read from $CF_ANALYTICS_FILE, default
// /root/.cf_analytics_token (`export CLOUDFLARE_ACCOUNT_ID=…`, `export CLOUDFLARE_API_TOKEN=…`).
// It is never printed, logged or written to a report. Window: the 24 hours before the run.
import fs from 'node:fs';
import { ind, unverified } from '../rules.mjs';

export const FILE = '/root/.cf_analytics_token';
export const SCRIPT = 'dowiz-api';
export const DAY_MS = 86_400_000;
export const ENDPOINT = 'https://api.cloudflare.com/client/v4/graphql';
/** Scheduled invocations are rows, one per firing: 1,441 a day with the minute cron, 1 without. */
export const SCHEDULED_LIMIT = 2000;
/** DAG Phase 2 / FT2's gate: object requests on a day with no orders, once the minute cron is gone. */
export const NO_ORDER_DAY_DO_TARGET = 1000;
/** Object groups (script x name x type x status) one query returns; a day that fills it is said (`doRowsCapped`). */
export const DO_ROWS = 1000;
/** The object the platform keeps its registry in (`platform_store::PLATFORM`): not a venue. */
export const PLATFORM_OBJECT = '__platform';
/** An HTTP invocation whose CALLER went away: a console socket or a stream the browser closed (W-EVALFIX
 * 2026-10-08: every one of the day's 11 was on dubin-durres, the object holding the console's sockets, after
 * 812-2,111 s of wall time on average). Not an error of ours; counted apart as `cf.do_disconnects_day`. */
export const DISCONNECTS = ['clientDisconnected', 'responseStreamDisconnected'];
// CPU QUANTILES OVER EVERY INVOCATION (`q`, W-EVALFIX): grouped by status, a quantile is the
// quantile of that status's rows only, and "the worst p50 of the groups" read ten stream
// disconnects' 12,406 µs as the median of 1,077 invocations whose true p50 was 3,625 µs.
export const QUERY = `query($a:String!,$s:Time!,$e:Time!){viewer{accounts(filter:{accountTag:$a}){
 w: workersInvocationsAdaptive(limit:100, filter:{datetime_geq:$s, datetime_lt:$e}){ sum{requests errors subrequests cpuTimeUs} dimensions{scriptName status} }
 q: workersInvocationsAdaptive(limit:100, filter:{datetime_geq:$s, datetime_lt:$e}){ quantiles{cpuTimeP50 cpuTimeP90 cpuTimeP99} dimensions{scriptName} }
 d: durableObjectsInvocationsAdaptiveGroups(limit:${DO_ROWS}, filter:{datetime_geq:$s, datetime_lt:$e}){ sum{requests errors wallTime responseBodySize} dimensions{scriptName name type status} }
 s: workersInvocationsScheduled(limit:${SCHEDULED_LIMIT}, filter:{datetime_geq:$s, datetime_lt:$e}){ cron scriptName }
}}}`;

/** `export KEY=value` lines (quotes stripped); {} when the file is absent. */
export function readCfEnv(file) {
  if (!fs.existsSync(file)) return {};
  const out = {};
  for (const raw of fs.readFileSync(file, 'utf8').split('\n')) {
    const l = raw.replace(/^export\s+/, '');
    const i = l.indexOf('=');
    if (i > 0) out[l.slice(0, i).trim()] = l.slice(i + 1).trim().replace(/^['"]|['"]$/g, '');
  }
  return out;
}

/** The groups for one script, summed over their status (and, for objects, name/type) rows.
 * The CPU quantiles are the script's own row of `q` (every invocation), null when absent.
 * `doAlarms` is the object invocations of type `alarm` (each venue's own timer, DAG Phase 2);
 * `doErrors` leaves out an HTTP caller that went away (`DISCONNECTS`), counted in `doDisconnects`;
 * `venueObjects` counts the venue objects invoked (every registered venue is, by its nightly);
 * `cronRuns` counts scheduled firings, `cronMinute` the ones of the removed `* * * * *`. */
export function fold(account, script = SCRIPT) {
  const w = (account.w || []).filter(r => r.dimensions.scriptName === script);
  const d = (account.d || []).filter(r => r.dimensions.scriptName === script);
  const q = (account.q || []).find(r => r.dimensions.scriptName === script)?.quantiles || {};
  const sum = (rows, k) => rows.reduce((n, r) => n + (r.sum[k] || 0), 0);
  const s = (account.s || []).filter(r => r.scriptName === script);
  const gone = r => r.dimensions.type === 'http' && DISCONNECTS.includes(r.dimensions.status);
  const cpu = k => (Number.isFinite(q[k]) ? q[k] : null);
  return {
    workerRequests: sum(w, 'requests'), workerErrors: sum(w, 'errors'), subrequests: sum(w, 'subrequests'),
    cpuP50Us: cpu('cpuTimeP50'), cpuP90Us: cpu('cpuTimeP90'), cpuP99Us: cpu('cpuTimeP99'),
    doRequests: sum(d, 'requests'), doErrors: sum(d.filter(r => !gone(r)), 'errors'), doDisconnects: sum(d.filter(gone), 'errors'),
    doResponseBytes: sum(d, 'responseBodySize'),
    doAlarms: sum(d.filter(r => r.dimensions.type === 'alarm'), 'requests'),
    venueObjects: new Set(d.map(r => r.dimensions.name).filter(n => n && n !== PLATFORM_OBJECT)).size,
    doRowsCapped: (account.d || []).length >= DO_ROWS,
    cronRuns: s.length, cronMinute: s.filter(r => r.cron === '* * * * *').length,
  };
}

/** One query for the day before `end`; returns the folded counts or throws with the answer. */
export async function analytics(fetchFn, env, end) {
  const r = await fetchFn(ENDPOINT, {
    method: 'POST',
    headers: { authorization: `Bearer ${env.CLOUDFLARE_API_TOKEN}`, 'content-type': 'application/json' },
    body: JSON.stringify({ query: QUERY, variables: { a: env.CLOUDFLARE_ACCOUNT_ID, s: new Date(end - DAY_MS).toISOString(), e: new Date(end).toISOString() } }),
  });
  const j = await r.json().catch(() => ({}));
  const acc = j.data?.viewer?.accounts?.[0];
  if (r.status !== 200 || !acc) throw new Error(`graphql answered ${r.status}: ${JSON.stringify(j.errors || j).slice(0, 200)}`);
  return fold(acc);
}

export const WHY_NO_TOKEN = 'no Account Analytics:Read token (set CF_ANALYTICS_FILE or create /root/.cf_analytics_token)';

export const WHY_NO_HEALTH = 'no venue health read this run (the nightly health collector stashes counters; owner credentials /root/.dowiz_owner)';
export const COUNTER_IDS = [['cf.since_none_share', 'permille'], ['cf.wakes_readonly_share', 'permille'], ['cf.cold_fold_us_p50', 'µs'],
  ['cf.proj_rows_per_write', 'rows'], ['cf.catalog_write_us', 'µs'], ['cf.catalog_write_journal_bytes', 'bytes']];

/** AX0-COUNTERS (plan §A0): the objects' own counts, from each venue's `health.counters` window.
 * Hibernation erases an unflushed window, so `samples=` and the windows that began at a WAKE are in
 * every note: a share over one sample, or over windows that lost their start, is visibly weak. */
export function counters(healths, fallback) {
  const src = `GET /api/owner/health -> counters (the object's own window, flushed nightly)${fallback ? `; FALLBACK, Analytics Engine: ${fallback}` : ''}`;
  if (!healths?.length) return COUNTER_IDS.map(([id, unit]) => unverified(id, unit, 'trend', src, WHY_NO_HEALTH));
  const ok = healths.filter(h => h.counters && !h.counters.error);
  const bad = healths.length - ok.length;
  const sum = k => ok.reduce((n, h) => n + (Number(h.counters[k]) || 0), 0);
  const woke = ok.filter(h => h.counters.window?.cause === 'wake').length;
  const note = `samples=${ok.length} venue window(s), ${woke} began at a wake (counts before it are lost)${bad ? `, ${bad} unreadable` : ''}`;
  const share = (id, num, den, what) => (den > 0
    ? ind(id, Math.round((1000 * num) / den), 'permille', 'trend', src, { note: `${num}/${den}; ${note}` })
    : unverified(id, 'permille', 'trend', src, `no ${what} in the window; ${note}`));
  const samples = ok.flatMap(h => h.counters.cold_fold_samples || []).sort((x, y) => x - y);
  const writes = sum('writes');
  const cat = sum('cat_writes');
  const clock = [...new Set(ok.map(h => h.counters.clock))].join(',');
  return [
    share('cf.since_none_share', sum('since_none'), sum('since_total'), '?since= poll'),
    share('cf.wakes_readonly_share', sum('wakes_readonly'), sum('wakes_total'), 'wake'),
    samples.length ? ind('cf.cold_fold_us_p50', samples[Math.floor(samples.length / 2)], 'µs', 'trend', src, { note: `${samples.length} fold(s); clock ${clock}; ${note}` })
      : unverified('cf.cold_fold_us_p50', 'µs', 'trend', src, `no fold from bytes in the window; ${note}`),
    writes ? ind('cf.proj_rows_per_write', Math.round((100 * sum('proj_rows')) / writes) / 100, 'rows', 'trend', src, { note: `${sum('proj_rows')} chunks / ${writes} writes; ${note}` })
      : unverified('cf.proj_rows_per_write', 'rows', 'trend', src, `no write in the window; ${note}`),
    cat ? ind('cf.catalog_write_us', Math.round((sum('cat_decode_us') + sum('journal_us')) / cat), 'µs', 'trend', src,
      { note: `per catalogue write: decode ${Math.round(sum('cat_decode_us') / cat)} µs over ${Math.round(sum('cat_decoded_bytes') / cat)} B + journal ${Math.round(sum('journal_us') / cat)} µs; clock ${clock} (live: advances on I/O only); ${note}` })
      : unverified('cf.catalog_write_us', 'µs', 'trend', src, `no catalogue write in the window; ${note}`),
    cat ? ind('cf.catalog_write_journal_bytes', Math.round(sum('journal_bytes') / cat), 'bytes', 'trend', src, { note: `journal bytes loaded per catalogue write; ${note}` })
      : unverified('cf.catalog_write_journal_bytes', 'bytes', 'trend', src, `no catalogue write in the window; ${note}`),
  ];
}

// W-AE: THE SAME SIX from Workers Analytics Engine, which keeps what a hibernation erases. Each
// venue's object writes one point per window (workers/api/src/hubdo/counters/ae.rs): at the nightly
// flush and every 16th stored write. A point is a DELTA since the previous one, so SUM over a day
// counts every event once. SQL API (analytics-engine/sql-api, read 2026-10-07): POST the query as
// the body, `Authorization: Bearer`, the token needs Account Analytics Read; sampled rows are
// weighted by `_sample_interval`. The health windows stay the FALLBACK, and say so in `source`.
export const AE_DATASET = 'dowiz_counters';
/** double1..double15, the order `ae.rs` DOUBLES writes. */
export const AE_DOUBLES = ['since_total', 'since_none', 'wakes_total', 'wakes_writing', 'reads', 'writes', 'proj_rows',
  'cold_folds', 'cold_fold_us', 'cold_fold_p50_us', 'cat_writes', 'cat_decode_us', 'cat_decoded_bytes', 'journal_us', 'journal_bytes'];
export const aeEndpoint = account => `https://api.cloudflare.com/client/v4/accounts/${account}/analytics_engine/sql`;
const col = n => `double${AE_DOUBLES.indexOf(n) + 1}`;
/** One query, the 24 h before it: the sums, the cold-fold p50 (each point's own p50, weighted by its folds), venues, points. */
export const AE_SQL = `SELECT ${AE_DOUBLES.filter(n => n !== 'cold_fold_p50_us').map(n => `SUM(_sample_interval * ${col(n)}) AS ${n}`).join(', ')}, `
  + `quantileWeighted(0.5, ${col('cold_fold_p50_us')}, ${col('cold_folds')}) AS cold_fold_p50_us, count(DISTINCT index1) AS venues, `
  + `SUM(_sample_interval) AS points FROM ${AE_DATASET} WHERE timestamp > NOW() - INTERVAL '1' DAY FORMAT JSON`;

/** The day's sums, or a throw with the answer. */
export async function aeSums(fetchFn, env) {
  const r = await fetchFn(aeEndpoint(env.CLOUDFLARE_ACCOUNT_ID), { method: 'POST', headers: { authorization: `Bearer ${env.CLOUDFLARE_API_TOKEN}` }, body: AE_SQL });
  const text = await r.text().catch(() => '');
  let j = null;
  try { j = JSON.parse(text); } catch { /* said below */ }
  const row = j?.data?.[0];
  if (r.status !== 200 || !row) throw new Error(`analytics engine answered ${r.status}: ${text.slice(0, 200)}`);
  return Object.fromEntries(Object.entries(row).map(([k, v]) => [k, Number(v) || 0]));
}

/** The six counters from the day's sums; null when no point was written (nothing to say yet). */
export function aeCounters(t) {
  if (!t.points) return null;
  const src = `Workers Analytics Engine ${AE_DATASET} (SQL API), the 24 h before the run`;
  const note = `points=${t.points} from ${t.venues} venue object(s); a point is a delta, written at each flush and every 16th write`;
  const share = (id, num, den, what) => (den > 0 ? ind(id, Math.round((1000 * num) / den), 'permille', 'trend', src, { note: `${num}/${den}; ${note}` })
    : unverified(id, 'permille', 'trend', src, `no ${what} in the day; ${note}`));
  const per = (id, num, den, unit, what, scale = 1) => (den > 0 ? ind(id, Math.round((scale * num) / den) / scale, unit, 'trend', src, { note: `${num} / ${den}; ${note}` })
    : unverified(id, unit, 'trend', src, `no ${what} in the day; ${note}`));
  return [
    share('cf.since_none_share', t.since_none, t.since_total, '?since= poll'),
    share('cf.wakes_readonly_share', t.wakes_total - t.wakes_writing, t.wakes_total, 'wake'),
    t.cold_folds > 0 ? ind('cf.cold_fold_us_p50', Math.round(t.cold_fold_p50_us), 'µs', 'trend', src, { note: `${t.cold_folds} fold(s), p50 of each point's own p50 weighted by its folds; ${note}` })
      : unverified('cf.cold_fold_us_p50', 'µs', 'trend', src, `no fold from bytes in the day; ${note}`),
    per('cf.proj_rows_per_write', t.proj_rows, t.writes, 'rows', 'write', 100),
    per('cf.catalog_write_us', t.cat_decode_us + t.journal_us, t.cat_writes, 'µs', 'catalogue write'),
    per('cf.catalog_write_journal_bytes', t.journal_bytes, t.cat_writes, 'bytes', 'catalogue write'),
  ];
}

/** Analytics Engine first; the health windows when it cannot answer, with the reason in `source`. */
async function countersMeasure(ctx, env) {
  if (!(env.CLOUDFLARE_API_TOKEN && env.CLOUDFLARE_ACCOUNT_ID)) return counters(ctx.healths, WHY_NO_TOKEN);
  try {
    return aeCounters(await aeSums(ctx.fetch ?? globalThis.fetch, env)) ?? counters(ctx.healths, 'no point written in the 24 h');
  } catch (e) {
    return counters(ctx.healths, e.message);
  }
}

/** The measured indicators, or each one UNVERIFIED with the same reason; the AX0 counters beside them. */
export async function measure(ctx) {
  const env = readCfEnv(ctx.env?.CF_ANALYTICS_FILE || FILE);
  const r = await analyticsMeasure(ctx, env);
  return { m: r.m, out: [...r.out, ...(await countersMeasure(ctx, env))] };
}

async function analyticsMeasure(ctx, env) {
  const src = `Cloudflare GraphQL analytics, ${SCRIPT}, the 24 h before the run`;
  const ids = [['cf.worker_requests_day', 'requests'], ['cf.worker_errors_day', 'errors'], ['cf.cpu_p99_us', 'µs'], ['cf.do_requests_day', 'requests']];
  let m = null;
  let why = WHY_NO_TOKEN;
  if (env.CLOUDFLARE_API_TOKEN && env.CLOUDFLARE_ACCOUNT_ID) {
    try { m = await analytics(ctx.fetch ?? globalThis.fetch, env, ctx.now()); } catch (e) { why = e.message; }
  }
  if (!m) return { m, out: ids.map(([id, unit]) => unverified(id, unit, 'trend', src, why)) };
  return {
    m,
    out: [
      ind('cf.worker_requests_day', m.workerRequests, 'requests', 'trend', src, { note: `${m.subrequests} subrequests` }),
      ind('cf.worker_errors_day', m.workerErrors, 'errors', 'zero', src),
      ind('cf.cpu_p50_us', m.cpuP50Us, 'µs', 'plus25', src, { note: `over every invocation, all statuses; p90 ${m.cpuP90Us} µs` }),
      ind('cf.cpu_p99_us', m.cpuP99Us, 'µs', 'plus25', src, { note: 'CPU per invocation, all statuses (memory: workers report it as a quantile, not judged here)' }),
      ind('cf.do_requests_day', m.doRequests, 'requests', 'trend', src,
        { note: `target on a day with no orders: < ${NO_ORDER_DAY_DO_TARGET} (DAG Phase 2; 19,839 on 2026-09-27 with the minute cron)` }),
      ind('cf.do_errors_day', m.doErrors, 'errors', 'zero', src,
        { note: `every status but success, except an HTTP caller that went away (${DISCONNECTS.join(', ')}): those are cf.do_disconnects_day${m.doRowsCapped ? `; CAPPED at ${DO_ROWS} groups` : ''}` }),
      ind('cf.do_disconnects_day', m.doDisconnects, 'invocations', 'trend', `${src}: HTTP object invocations whose caller went away (a closed console socket or stream)`),
      ind('cf.venue_objects_day', m.venueObjects, 'objects', 'trend', `${src}: venue objects invoked (every registered venue is, by its nightly)`),
      ind('cf.do_alarms_day', m.doAlarms, 'invocations', 'trend', `${src}: object invocations of type alarm (each venue's timer)`),
      ind('cf.cron_runs_day', m.cronRuns, 'firings', 'trend', `${src}: scheduled firings (1 a day: the nightly)`,
        { note: `${m.cronMinute} of the removed * * * * *${m.cronRuns >= SCHEDULED_LIMIT ? `; capped at ${SCHEDULED_LIMIT} rows` : ''}` }),
      ind('cf.do_response_bytes_day', m.doResponseBytes, 'bytes', 'trend', `${src}: bytes the object sent the Worker`),
    ],
  };
}
