import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { readCfEnv, fold, analytics, measure, counters, aeSums, aeCounters, aeEndpoint, AE_SQL, AE_DOUBLES, AE_DATASET, QUERY, ENDPOINT, WHY_NO_TOKEN, WHY_NO_HEALTH, COUNTER_IDS, FILE, SCHEDULED_LIMIT, NO_ORDER_DAY_DO_TARGET, DO_ROWS } from './cf.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));
const ACC = {
  w: [
    { dimensions: { scriptName: 'dowiz-api', status: 'success' }, sum: { requests: 10, errors: 0, subrequests: 4 } },
    { dimensions: { scriptName: 'dowiz-api', status: 'scriptThrewException' }, sum: { requests: 2, errors: 2 } },
    { dimensions: { scriptName: 'dowiz-api', status: 'clientDisconnected' }, sum: { requests: 0 } },
    { dimensions: { scriptName: 'other', status: 'success' }, sum: { requests: 999 } },
  ],
  // The quantiles over EVERY invocation of a script (no status dimension).
  q: [
    { dimensions: { scriptName: 'dowiz-api' }, quantiles: { cpuTimeP50: 7, cpuTimeP90: 20, cpuTimeP99: 40 } },
    { dimensions: { scriptName: 'other' }, quantiles: { cpuTimeP50: 999, cpuTimeP90: 999, cpuTimeP99: 999 } },
  ],
  d: [
    { dimensions: { scriptName: 'dowiz-api', name: 'alpha', type: 'http', status: 'success' }, sum: { requests: 20, errors: 0, responseBodySize: 500 } },
    { dimensions: { scriptName: 'dowiz-api', name: 'alpha', type: 'http', status: 'scriptThrewException' }, sum: { requests: 1, errors: 1, responseBodySize: 0 } },
    { dimensions: { scriptName: 'dowiz-api', name: 'beta', type: 'http', status: 'clientDisconnected' }, sum: { requests: 3, errors: 3 } },
    { dimensions: { scriptName: 'dowiz-api', name: 'beta', type: 'http', status: 'responseStreamDisconnected' }, sum: { requests: 2, errors: 2 } },
    { dimensions: { scriptName: 'dowiz-api', name: 'beta', type: 'alarm', status: 'success' }, sum: { requests: 3, errors: 0, responseBodySize: 0 } },
    { dimensions: { scriptName: 'dowiz-api', name: 'beta', type: 'alarm', status: 'clientDisconnected' }, sum: { requests: 1, errors: 1 } },
    { dimensions: { scriptName: 'dowiz-api', name: '__platform', type: 'http', status: 'success' }, sum: { requests: 0, errors: 0 } },
    { dimensions: { scriptName: 'x', name: 'gamma', type: 'alarm', status: 'success' }, sum: { requests: 1 } },
  ],
  s: [
    { scriptName: 'dowiz-api', cron: '17 3 * * *' },
    { scriptName: 'dowiz-api', cron: '* * * * *' },
    { scriptName: 'dowiz-api', cron: '* * * * *' },
    { scriptName: 'x', cron: '* * * * *' },
  ],
};

test('the token file: export lines, quotes stripped; absent is empty', () => {
  const f = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cf-')), 't');
  fs.writeFileSync(f, "export CLOUDFLARE_ACCOUNT_ID='a'\nCLOUDFLARE_API_TOKEN=\"b\"\n# c\nnoequals\n");
  assert.deepEqual(readCfEnv(f), { CLOUDFLARE_ACCOUNT_ID: 'a', CLOUDFLARE_API_TOKEN: 'b' });
  assert.deepEqual(readCfEnv('/no/such'), {});
  assert.equal(FILE, '/root/.cf_analytics_token');
});

test('fold sums one script over its rows; CPU quantiles over every invocation; a caller gone is not an error', () => {
  assert.deepEqual(fold(ACC), { workerRequests: 12, workerErrors: 2, subrequests: 4, cpuP50Us: 7, cpuP90Us: 20, cpuP99Us: 40,
    doRequests: 30, doErrors: 2, doDisconnects: 5, doResponseBytes: 500, doAlarms: 4, venueObjects: 2, doRowsCapped: false, cronRuns: 3, cronMinute: 2 });
  assert.deepEqual(fold({}), { workerRequests: 0, workerErrors: 0, subrequests: 0, cpuP50Us: null, cpuP90Us: null, cpuP99Us: null,
    doRequests: 0, doErrors: 0, doDisconnects: 0, doResponseBytes: 0, doAlarms: 0, venueObjects: 0, doRowsCapped: false, cronRuns: 0, cronMinute: 0 });
});

// W-EVALFIX, LIVE 2026-10-08: the worst per-status p50 read 12,406 µs (ten stream disconnects)
// as the median of 1,077 invocations whose p50 over all of them was 3,625 µs.
test('the median is not the worst group\'s median: a slow minority status does not move it', () => {
  const live = {
    w: [{ dimensions: { scriptName: 'dowiz-api', status: 'success' }, sum: { requests: 1059 } },
      { dimensions: { scriptName: 'dowiz-api', status: 'responseStreamDisconnected' }, sum: { requests: 10 } }],
    q: [{ dimensions: { scriptName: 'dowiz-api' }, quantiles: { cpuTimeP50: 3625, cpuTimeP90: 13452, cpuTimeP99: 24365 } }],
  };
  assert.equal(fold(live).cpuP50Us, 3625);
  assert.equal(fold(live).cpuP99Us, 24365);
});

test('the query asks for the object type and every scheduled firing, bounded', () => {
  assert.match(QUERY, /dimensions\{scriptName name type status\}/);
  assert.match(QUERY, /q: workersInvocationsAdaptive\(limit:100, filter:\{datetime_geq:\$s, datetime_lt:\$e\}\)\{ quantiles\{cpuTimeP50 cpuTimeP90 cpuTimeP99\} dimensions\{scriptName\} \}/);
  assert.doesNotMatch(QUERY, /quantiles[^}]*\}[^}]*dimensions\{scriptName status\}/, 'no quantile grouped by status');
  assert.match(QUERY, new RegExp(`durableObjectsInvocationsAdaptiveGroups\\(limit:${DO_ROWS},`));
  assert.match(QUERY, new RegExp(`workersInvocationsScheduled\\(limit:${SCHEDULED_LIMIT},`));
  assert.ok(SCHEDULED_LIMIT > 1441, 'a whole day of the old minute cron fits, so its disappearance is measured');
  assert.equal(NO_ORDER_DAY_DO_TARGET, 1000);
});

test('one POST with the bearer and the day window; a refusal throws with the answer', async () => {
  const seen = [];
  const ok = async (u, o) => { seen.push([u, o]); return new Response(JSON.stringify({ data: { viewer: { accounts: [ACC] } } })); };
  const end = Date.UTC(2026, 8, 26);
  assert.equal((await analytics(ok, { CLOUDFLARE_API_TOKEN: 'T', CLOUDFLARE_ACCOUNT_ID: 'A' }, end)).workerRequests, 12);
  const [u, o] = seen[0];
  assert.equal(u, ENDPOINT);
  assert.equal(o.headers.authorization, 'Bearer T');
  const body = JSON.parse(o.body);
  assert.equal(body.query, QUERY);
  assert.deepEqual(body.variables, { a: 'A', s: '2026-09-25T00:00:00.000Z', e: '2026-09-26T00:00:00.000Z' });
  await assert.rejects(analytics(async () => new Response('{"errors":[{"message":"denied"}]}', { status: 403 }), {}, end), /graphql answered 403: \[\{"message":"denied"\}\]/);
  await assert.rejects(analytics(async () => new Response('not json'), {}, end), /graphql answered 200: \{\}/);
});

test('measure: the indicators, an error as the reason, and no token as the reason', async () => {
  const f = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cf-')), 't');
  fs.writeFileSync(f, 'CLOUDFLARE_ACCOUNT_ID=a\nCLOUDFLARE_API_TOKEN=b\n');
  const env = { CF_ANALYTICS_FILE: f };
  const good = await measure({ env, now: () => 0, fetch: async () => new Response(JSON.stringify({ data: { viewer: { accounts: [ACC] } } })) });
  const r = byId(good.out);
  assert.equal(r['cf.worker_errors_day'].value, 2);
  assert.equal(r['cf.worker_errors_day'].rule, 'zero');
  assert.equal(r['cf.do_response_bytes_day'].value, 500);
  assert.equal(r['cf.worker_requests_day'].note, '4 subrequests');
  assert.equal(r['cf.do_alarms_day'].value, 4);
  assert.equal(r['cf.do_errors_day'].value, 2, 'the thrown exception and the alarm whose run was cut; not the closed sockets');
  assert.equal(r['cf.do_disconnects_day'].value, 5);
  assert.equal(r['cf.venue_objects_day'].value, 2);
  assert.equal(r['cf.cpu_p50_us'].value, 7);
  assert.equal(r['cf.cron_runs_day'].value, 3);
  assert.equal(r['cf.cron_runs_day'].note, '2 of the removed * * * * *');
  assert.match(r['cf.do_requests_day'].note, /no orders: < 1000/);
  const full = { ...ACC, s: Array.from({ length: SCHEDULED_LIMIT }, () => ({ scriptName: 'dowiz-api', cron: '* * * * *' })) };
  const capped = byId((await measure({ env, now: () => 0, fetch: async () => new Response(JSON.stringify({ data: { viewer: { accounts: [full] } } })) })).out);
  assert.match(capped['cf.cron_runs_day'].note, /capped at 2000 rows/);
  const bad = byId((await measure({ env, now: () => 0, fetch: async () => new Response('{}', { status: 500 }) })).out);
  assert.match(bad['cf.worker_requests_day'].unverified, /graphql answered 500/);
  const none = await measure({ env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0 });
  assert.equal(none.m, null);
  assert.equal(byId(none.out)['cf.cpu_p99_us'].unverified, WHY_NO_TOKEN);
  const saved = globalThis.fetch;
  globalThis.fetch = async () => new Response(JSON.stringify({ data: { viewer: { accounts: [ACC] } } }));
  try { assert.equal((await measure({ env, now: () => 0 })).m.doRequests, 30); } finally { globalThis.fetch = saved; }
});

test('the token path defaults to /root/.cf_analytics_token when the environment names none', async () => {
  const seen = [];
  const r = await measure({ now: () => 0, fetch: async () => { seen.push(1); return new Response('{}', { status: 500 }); } });
  assert.equal(r.m, null);
  assert.equal(r.out.length, 4 + COUNTER_IDS.length, 'the four analytics ids and the AX0 counters, each said');
  assert.ok(seen.length <= 2, 'GraphQL + Analytics Engine at most, whether the default file exists or not');
});

// AX0-COUNTERS (plan §A0): the four indicators the plan names, plus the catalogue write's cost.
const W = (o = {}) => ({ since_total: 0, since_none: 0, wakes_total: 0, wakes_readonly: 0, writes: 0, proj_rows: 0, cat_writes: 0,
  cat_decode_us: 0, cat_decoded_bytes: 0, journal_us: 0, journal_bytes: 0, cold_fold_samples: [], clock: 'platform-ms-advances-on-io-only',
  window: { cause: 'flush' }, ...o });

test('counters: every indicator prints a number from the venues\' windows, with samples= and the wake windows', () => {
  const r = byId(counters([
    { venue: 'a', counters: W({ since_total: 8, since_none: 2, wakes_total: 1, wakes_readonly: 1, writes: 2, proj_rows: 5, cold_fold_samples: [3000, 1000],
      cat_writes: 1, cat_decode_us: 0, cat_decoded_bytes: 1096208, journal_us: 4000, journal_bytes: 2155928, window: { cause: 'wake' } }) },
    { venue: 'b', counters: W({ since_total: 2, since_none: 0, wakes_total: 1, wakes_readonly: 0, writes: 2, proj_rows: 3, cold_fold_samples: [2000] }) },
    { venue: 'c', counters: { error: 'the object answered 500' } },
  ]));
  assert.equal(r['cf.since_none_share'].value, 200);
  assert.equal(r['cf.wakes_readonly_share'].value, 500);
  assert.equal(r['cf.cold_fold_us_p50'].value, 2000);
  assert.equal(r['cf.proj_rows_per_write'].value, 2);
  assert.equal(r['cf.catalog_write_us'].value, 4000);
  assert.equal(r['cf.catalog_write_journal_bytes'].value, 2155928);
  for (const [id] of COUNTER_IDS) {
    assert.match(r[id].note, /samples=2 venue window\(s\), 1 began at a wake .*1 unreadable/, id);
    assert.equal(r[id].rule, 'trend');
  }
  assert.match(r['cf.catalog_write_us'].note, /advances on I\/O only/);
});

test('counters: no health is UNVERIFIED with the reason, an empty window names what it lacked (fail loud, never 0)', () => {
  for (const x of [counters(undefined), counters([])]) {
    assert.equal(x.length, COUNTER_IDS.length);
    for (const i of x) assert.equal(i.unverified, WHY_NO_HEALTH);
  }
  const quiet = byId(counters([{ venue: 'a', counters: W() }]));
  assert.match(quiet['cf.since_none_share'].unverified, /no \?since= poll in the window; samples=1/);
  assert.match(quiet['cf.wakes_readonly_share'].unverified, /no wake/);
  assert.match(quiet['cf.cold_fold_us_p50'].unverified, /no fold from bytes/);
  assert.match(quiet['cf.proj_rows_per_write'].unverified, /no write/);
  assert.match(quiet['cf.catalog_write_us'].unverified, /no catalogue write/);
  for (const i of Object.values(quiet)) assert.equal(i.value, null, i.id);
});

test('measure carries the counters from ctx.healths beside the analytics', async () => {
  const got = byId((await measure({ env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0, healths: [{ venue: 'a', counters: W({ since_total: 4, since_none: 1 }) }] })).out);
  assert.equal(got['cf.since_none_share'].value, 250);
  assert.equal(got['cf.cpu_p99_us'].unverified, WHY_NO_TOKEN);
});

// W-AE: the six counters from Workers Analytics Engine, the health windows as the labelled fallback.
const SUMS = { since_total: 10, since_none: 1, wakes_total: 4, wakes_writing: 1, reads: 50, writes: 20, proj_rows: 30, cold_folds: 3,
  cold_fold_us: 9000, cold_fold_p50_us: 2500, cat_writes: 2, cat_decode_us: 100, cat_decoded_bytes: 4000, journal_us: 300, journal_bytes: 8000, venues: 2, points: 5 };

test('AE: the SQL reads double1..15 in the order ae.rs writes them, weighted by _sample_interval, over one day', () => {
  assert.equal(AE_DOUBLES.length, 15);
  assert.deepEqual([AE_DOUBLES[0], AE_DOUBLES[3], AE_DOUBLES[9], AE_DOUBLES[14]], ['since_total', 'wakes_writing', 'cold_fold_p50_us', 'journal_bytes']);
  assert.match(AE_SQL, /SUM\(_sample_interval \* double1\) AS since_total/);
  assert.match(AE_SQL, /SUM\(_sample_interval \* double15\) AS journal_bytes/);
  assert.match(AE_SQL, /quantileWeighted\(0\.5, double10, double8\) AS cold_fold_p50_us/);
  assert.match(AE_SQL, new RegExp(`FROM ${AE_DATASET} WHERE timestamp > NOW\\(\\) - INTERVAL '1' DAY FORMAT JSON$`));
  assert.equal(aeEndpoint('A'), 'https://api.cloudflare.com/client/v4/accounts/A/analytics_engine/sql');
});

test('AE: one POST with the bearer and the SQL as the body; a refusal throws with the answer', async () => {
  const seen = [];
  const ok = async (u, o) => { seen.push([u, o]); return new Response(JSON.stringify({ meta: [], data: [{ ...SUMS, writes: '20' }], rows: 1 })); };
  const t = await aeSums(ok, { CLOUDFLARE_API_TOKEN: 'T', CLOUDFLARE_ACCOUNT_ID: 'A' });
  assert.equal(t.writes, 20, 'numbers that come back as strings are numbers');
  assert.equal(seen[0][0], aeEndpoint('A'));
  assert.equal(seen[0][1].headers.authorization, 'Bearer T');
  assert.equal(seen[0][1].body, AE_SQL);
  await assert.rejects(aeSums(async () => new Response('Authorization error', { status: 403 }), {}), /analytics engine answered 403: Authorization error/);
  await assert.rejects(aeSums(async () => new Response('{"data":[]}'), {}), /analytics engine answered 200/);
});

test('AE: the six indicators from the sums, sourced to Analytics Engine; no point at all is null (not zeros)', () => {
  const r = byId(aeCounters(SUMS));
  assert.equal(r['cf.since_none_share'].value, 100);
  assert.equal(r['cf.wakes_readonly_share'].value, 750, '(4 - 1) / 4 wakes wrote nothing');
  assert.equal(r['cf.cold_fold_us_p50'].value, 2500);
  assert.equal(r['cf.proj_rows_per_write'].value, 1.5);
  assert.equal(r['cf.catalog_write_us'].value, 200);
  assert.equal(r['cf.catalog_write_journal_bytes'].value, 4000);
  for (const [id] of COUNTER_IDS) {
    assert.match(r[id].source, /^Workers Analytics Engine dowiz_counters/, id);
    assert.match(r[id].note, /points=5 from 2 venue object\(s\)/, id);
  }
  assert.equal(aeCounters({ ...SUMS, points: 0 }), null);
  const quiet = byId(aeCounters({ ...SUMS, since_total: 0, wakes_total: 0, cold_folds: 0, writes: 0, cat_writes: 0 }));
  for (const i of Object.values(quiet)) assert.equal(i.value, null, i.id);
});

test('AE first, health as the LABELLED fallback: a refusal, no point, and no token each name themselves in source', async () => {
  const f = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cf-')), 't');
  fs.writeFileSync(f, 'CLOUDFLARE_ACCOUNT_ID=a\nCLOUDFLARE_API_TOKEN=b\n');
  const healths = [{ venue: 'a', counters: W({ since_total: 4, since_none: 1 }) }];
  const route = ae => async u => (u.endsWith('/analytics_engine/sql') ? ae() : new Response(JSON.stringify({ data: { viewer: { accounts: [ACC] } } })));
  const live = byId((await measure({ env: { CF_ANALYTICS_FILE: f }, now: () => 0, healths, fetch: route(() => new Response(JSON.stringify({ data: [SUMS] }))) })).out);
  assert.equal(live['cf.since_none_share'].value, 100, 'AE wins over the health window');
  assert.match(live['cf.since_none_share'].source, /Analytics Engine/);
  const refused = byId((await measure({ env: { CF_ANALYTICS_FILE: f }, now: () => 0, healths, fetch: route(() => new Response('Authorization error', { status: 403 })) })).out);
  assert.equal(refused['cf.since_none_share'].value, 250);
  assert.match(refused['cf.since_none_share'].source, /^GET \/api\/owner\/health .*FALLBACK, Analytics Engine: analytics engine answered 403/);
  const empty = byId((await measure({ env: { CF_ANALYTICS_FILE: f }, now: () => 0, healths, fetch: route(() => new Response(JSON.stringify({ data: [{ ...SUMS, points: 0 }] }))) })).out);
  assert.match(empty['cf.since_none_share'].source, /FALLBACK, Analytics Engine: no point written in the 24 h/);
  const none = byId((await measure({ env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0, healths })).out);
  assert.match(none['cf.since_none_share'].source, /FALLBACK, Analytics Engine: no Account Analytics:Read token/);
});
