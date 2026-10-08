// LIVE PROBE ae-counters (W-AE: AX0's counters -> Workers Analytics Engine) on qa-durres -- main
// runs it AFTER the deploy; a lane never runs it against production. WRITTEN, NOT RUN by the lane.
//
//   node tools/live-proof/probes/feature-ae-counters.mjs      (FLOWS_HOST may name another qa- hub)
//
// 1. GET /api/owner/health?counters=flush answers `counters.ae` (binding COUNTERS) and the flush's
//    point was ACCEPTED: `ae.points` grew by one, or this is a fresh wake whose first flush wrote it
//    (points >= 1). `ae.errors > 0` with a lastError naming the binding is a FAIL: the binding is
//    absent from the deployed Worker.
// 2. THE PLATFORM HAS IT: the SQL API (Account Analytics Read token, /root/.cf_analytics_token or
//    $CF_ANALYTICS_FILE) returns at least one `flush` point for this venue written after step 1,
//    with 15 doubles. Analytics Engine is not read-your-writes: the probe asks up to 12 times,
//    10 s apart (2 minutes), and says how long it waited. No token = NEEDS-KEY, not a pass.
//    403 "Authorization error" = the token lacks Account Analytics Read for Analytics Engine
//    (2026-10-07: the analytics token answered GraphQL 200 and the AE SQL API 403).
// Every step that cannot run is a FAIL or NEEDS-KEY, never a skip.
import * as lib from '../../../e2e/flows/lib.mjs';
import { contract, reporter } from './stock-lib.mjs';
import { readCfEnv, FILE, aeEndpoint, AE_DATASET } from '../../evals/collect/cf.mjs';

const C = contract('ae-counters');
const { step, verdict } = reporter('feature-ae-counters');
const must = (ok, why) => { if (!ok) throw new Error(why); };
const TRIES = 12;
const GAP_MS = 10_000;

const health = async (q = '') => {
  const r = await lib.own(`/api/owner/health?location_id=${lib.LOC}${q}`);
  must(r.status === 200 && r.body?.counters, `health answered ${r.status} ${r.text?.slice(0, 160)}`);
  must(!r.body.counters.error, `the object's counters are unreadable: ${r.body.counters.error}`);
  return r.body.counters;
};

/** The venue's flush points since `sinceSec` (unix seconds), as the SQL API answers them. */
async function flushPoints(env, sinceSec) {
  const venue = lib.LOC.replace(/'/g, '');
  const sql = `SELECT timestamp, index1, blob1, blob2, blob3, ${C.doubles.map((_, i) => `double${i + 1}`).join(', ')} `
    + `FROM ${AE_DATASET} WHERE index1 = '${venue}' AND blob2 = 'flush' AND timestamp >= toDateTime(${sinceSec}) ORDER BY timestamp DESC LIMIT 5 FORMAT JSON`;
  const r = await fetch(aeEndpoint(env.CLOUDFLARE_ACCOUNT_ID), { method: 'POST', headers: { authorization: `Bearer ${env.CLOUDFLARE_API_TOKEN}` }, body: sql });
  const text = await r.text();
  must(r.status === 200, `the SQL API answered ${r.status}: ${text.slice(0, 200)}`);
  return JSON.parse(text).data || [];
}

try {
  const before = await health();
  const t0 = Math.floor(Date.now() / 1000) - 5;
  const f = await health('&counters=flush');
  const ae = f.ae;
  step('health counters carry `ae` with binding COUNTERS', ae?.binding === 'COUNTERS', JSON.stringify(ae));
  step('no refusal: the binding is live', ae?.errors === 0, `errors=${ae?.errors} lastError=${ae?.lastError}`);
  const fresh = f.window.cause === 'wake' && f.window.wokeAtMs > before.window.wokeAtMs;
  step('the flush wrote one point (points +1, or >= 1 on a fresh wake)',
    fresh ? ae?.points >= 1 : ae?.points === (before.ae?.points ?? 0) + 1, `${before.ae?.points} -> ${ae?.points}${fresh ? ' (woke between)' : ''}`);

  const env = readCfEnv(process.env.CF_ANALYTICS_FILE || FILE);
  if (!(env.CLOUDFLARE_API_TOKEN && env.CLOUDFLARE_ACCOUNT_ID)) {
    step(`the platform has the point (SQL API)`, 'NEEDS-KEY', `no token in ${process.env.CF_ANALYTICS_FILE || FILE}`);
  } else {
    let rows = [];
    let waited = 0;
    for (let i = 0; i < TRIES && rows.length === 0; i++) {
      if (i) { await new Promise(r => setTimeout(r, GAP_MS)); waited += GAP_MS; }
      rows = await flushPoints(env, t0);
    }
    step(`the SQL API returns a flush point for ${lib.LOC} written after the flush`, rows.length > 0, `waited ${waited / 1000} s; ${rows.length} row(s)`);
    if (rows.length) {
      const p = rows[0];
      step('blob1 is the venue, blob3 the platform clock', p.blob1 === lib.LOC && p.blob3 === 'platform-ms-advances-on-io-only', JSON.stringify([p.blob1, p.blob2, p.blob3]));
      const doubles = C.doubles.map((_, i) => Number(p[`double${i + 1}`]));
      step('15 doubles, all counts >= 0', doubles.length === 15 && doubles.every(n => Number.isFinite(n) && n >= 0), JSON.stringify(doubles));
      step('the flush GET is in its own point (reads >= 1)', doubles[C.doubles.indexOf('reads')] >= 1, `reads=${doubles[C.doubles.indexOf('reads')]}`);
    }
  }
} catch (e) {
  step('the probe ran to the end', false, e.message);
} finally {
  verdict();
}
