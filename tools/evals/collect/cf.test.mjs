import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { readCfEnv, fold, analytics, measure, QUERY, ENDPOINT, WHY_NO_TOKEN, FILE, SCHEDULED_LIMIT, NO_ORDER_DAY_DO_TARGET } from './cf.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));
const ACC = {
  w: [
    { dimensions: { scriptName: 'dowiz-api', status: 'success' }, sum: { requests: 10, errors: 0, subrequests: 4 }, quantiles: { cpuTimeP50: 7, cpuTimeP99: 40 } },
    { dimensions: { scriptName: 'dowiz-api', status: 'scriptThrewException' }, sum: { requests: 2, errors: 2 }, quantiles: { cpuTimeP50: 9, cpuTimeP99: 30 } },
    { dimensions: { scriptName: 'dowiz-api', status: 'clientDisconnected' }, sum: { requests: 0 } },
    { dimensions: { scriptName: 'other', status: 'success' }, sum: { requests: 999 } },
  ],
  d: [
    { dimensions: { scriptName: 'dowiz-api', type: 'http' }, sum: { requests: 26, errors: 1, responseBodySize: 500 } },
    { dimensions: { scriptName: 'dowiz-api', type: 'alarm' }, sum: { requests: 4, errors: 0, responseBodySize: 0 } },
    { dimensions: { scriptName: 'x', type: 'alarm' }, sum: { requests: 1 } },
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

test('fold sums one script over its status rows and takes the worst quantile', () => {
  assert.deepEqual(fold(ACC), { workerRequests: 12, workerErrors: 2, subrequests: 4, cpuP50Us: 9, cpuP99Us: 40, doRequests: 30, doErrors: 1, doResponseBytes: 500, doAlarms: 4, cronRuns: 3, cronMinute: 2 });
  assert.deepEqual(fold({}), { workerRequests: 0, workerErrors: 0, subrequests: 0, cpuP50Us: 0, cpuP99Us: 0, doRequests: 0, doErrors: 0, doResponseBytes: 0, doAlarms: 0, cronRuns: 0, cronMinute: 0 });
});

test('the query asks for the object type and every scheduled firing, bounded', () => {
  assert.match(QUERY, /dimensions\{scriptName type\}/);
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
  assert.equal(r.out.length, 4);
  assert.ok(seen.length <= 1, 'one query at most, whether the default file exists or not');
});
