import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { inputs, model, price, collect, FIXED_MONTH, REQUESTS_PER_ORDER_BUDGET, USD, TIMER_DO_PER_ORDER, FAN_BATCH } from './cost.mjs';

const byId = xs => Object.fromEntries(xs.map(x => [x.id, x]));
const I = v => ({ value: v });

test('inputs: measured where the run measured them, modelled and labelled otherwise', () => {
  const m = inputs({ 'platform.venues': I(4), 'product.a.orders_7d': I(70), 'product.b.orders_7d': I(0), 'product.c.orders_7d': I(null), 'order.requests_total': I(90) }, ['x']);
  assert.deepEqual(m, { venues: 4, venuesFrom: 'GET /api/platform/health', ordersPerDay: 10, ordersFrom: "2 venues' orders_7d / 7",
    perOrder: 90, perOrderFrom: 'order.requests_total' });
  const d = inputs({}, ['a', 'b']);
  assert.equal(d.venues, 2);
  assert.equal(d.venuesFrom, 'the 2 hosts probed');
  assert.equal(d.ordersPerDay, 0);
  assert.equal(d.perOrder, REQUESTS_PER_ORDER_BUDGET);
  assert.match(d.perOrderFrom, /MODELLED/);
  assert.equal(inputs({}, []).venues, 1, 'never divides by zero venues');
  // The venue objects Cloudflare saw invoked come before the hosts this run probed.
  const c = inputs({}, ['a', 'b'], { venueObjects: 3 });
  assert.equal(c.venues, 3);
  assert.match(c.venuesFrom, /cf\.venue_objects_day/);
  assert.equal(inputs({ 'platform.venues': I(5) }, ['a'], { venueObjects: 3 }).venues, 5, 'the registry outranks a count');
});

test('the model: integer micro-dollars; timers\' share of object requests (no minute cron)', () => {
  const m = model({ venues: 2, ordersPerDay: 10, perOrder: 100 });
  assert.equal(TIMER_DO_PER_ORDER, 10);
  assert.equal(FAN_BATCH, 25);
  assert.equal(m.cronDo, 10 * 10 + 2 + 1, 'orders\' alarm runs + each venue\'s night + one fan-out turn');
  assert.equal(m.durable, 103 + 1000);
  assert.equal(m.worker, 1 + 1000);
  assert.equal(m.cronShare, Math.round((1000 * 103) / 1103));
  assert.equal(m.marginal, Math.round(Math.round((30 * (1001 * 300_000 + 1103 * 150_000)) / 1e6) / 2));
  // AN IDLE PLATFORM: each venue's night and the platform's fan-out turns, nothing else; and a
  // timer share over no user request is not a number (it would be 1000 by definition).
  const idle = model({ venues: 26, ordersPerDay: 0, perOrder: 100 });
  assert.equal(idle.durable, 26 + 2);
  assert.equal(idle.cronShare, null);
  assert.equal(m.allIn, m.marginal + Math.round(FIXED_MONTH / 2));
  for (const v of Object.values(m)) assert.ok(Number.isInteger(v));
  assert.equal(FIXED_MONTH, 5 * USD + 933_333);
});

test('without a token: modelled prices, the cf numbers UNVERIFIED with the reason', async () => {
  const r = byId(await collect({ results: {}, hosts: ['a'], env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0 }));
  assert.equal(r['cost.marginal_venue_month_micro_usd'].limit, USD);
  assert.match(r['cost.marginal_venue_month_micro_usd'].source, /^modelled/);
  assert.equal(r['cost.logs_month'].value, Math.round((1 * 30 * 100) / 1000));
  assert.match(r['cost.cron_share_permille'].unverified, /1000 by definition/);
  const busy = byId(await collect({ results: { 'product.a.orders_7d': I(70) }, hosts: ['a'], env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0 }));
  assert.equal(busy['cost.cron_share_permille'].value, Math.round((1000 * (100 + 1 + 1)) / (102 + 10 * REQUESTS_PER_ORDER_BUDGET)));
  assert.equal(busy['cost.cron_share_permille'].limit, 500);
  assert.match(r['cf.do_requests_day'].unverified, /no Account Analytics:Read token/);
  assert.ok(r['cost.plan'].unverified);
  assert.equal(r['cost.do_measured_over_modelled_permille'], undefined);
  assert.ok(byId(await collect({ env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0 }))['cost.modelled_do_requests_day'].value > 0);
});

test('with a token: the measured day prices the month and the gap to the model is judged', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cost-'));
  const file = path.join(dir, 'tok');
  fs.writeFileSync(file, 'export CLOUDFLARE_ACCOUNT_ID=acc\nexport CLOUDFLARE_API_TOKEN="tok"\n');
  const fetch = async () => new Response(JSON.stringify({ data: { viewer: { accounts: [{
    w: [{ dimensions: { scriptName: 'dowiz-api', status: 'success' }, sum: { requests: 2000, errors: 0, subrequests: 9, cpuTimeUs: 1 }, quantiles: { cpuTimeP50: 7000, cpuTimeP99: 40000 } }],
    d: [{ dimensions: { scriptName: 'dowiz-api', name: 'v1', type: 'http', status: 'success' }, sum: { requests: 5996, errors: 0, wallTime: 1, responseBodySize: 5 } },
      { dimensions: { scriptName: 'dowiz-api', name: 'v1', type: 'alarm', status: 'success' }, sum: { requests: 1, errors: 0 } },
      { dimensions: { scriptName: 'dowiz-api', name: 'v2', type: 'alarm', status: 'success' }, sum: { requests: 2, errors: 0 } },
      { dimensions: { scriptName: 'dowiz-api', name: '__platform', type: 'alarm', status: 'success' }, sum: { requests: 1, errors: 0 } }] }] } } }));
  const r = byId(await collect({ results: {}, hosts: ['a', 'b', 'c'], env: { CF_ANALYTICS_FILE: file }, fetch, now: () => Date.UTC(2026, 8, 26) }));
  assert.equal(r['cf.worker_requests_day'].value, 2000);
  assert.match(r['cost.marginal_venue_month_micro_usd'].source, /^MEASURED/);
  assert.equal(r['cost.marginal_venue_month_micro_usd'].value, price(2000, 6000, 2).marginal, 'two venue objects, not three hosts');
  assert.equal(r['cost.logs_month'].value, 6000);
  // THE TIMERS ARE JUDGED: 4 alarms over 2 nights + 1 fan-out turn.
  assert.equal(r['cost.do_measured_over_modelled_permille'].value, Math.round((1000 * 4) / 3));
  assert.equal(r['cost.do_measured_over_modelled_permille'].limit, 1200);
  assert.equal(r['cost.do_people_requests_day'].value, 5996);
  assert.equal(r['cost.cron_share_permille'].value, Math.round((1000 * 4) / 6000));
  assert.match(r['cost.cron_share_permille'].source, /^MEASURED/);
});

// W-EVALFIX, LIVE 2026-10-08 (0 orders in 7 days): qa-durres' alarm fired every minute. The judged
// gap is the alarms', so a minute-cadence timer is a breach on ITS number, not on the people's.
test('a minute-cadence alarm breaches the timer gap; people\'s requests never do', async () => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'evals-cost-'));
  const file = path.join(dir, 'tok');
  fs.writeFileSync(file, 'export CLOUDFLARE_ACCOUNT_ID=acc\nexport CLOUDFLARE_API_TOKEN=tok\n');
  const day = (alarmsQa) => async () => new Response(JSON.stringify({ data: { viewer: { accounts: [{
    w: [{ dimensions: { scriptName: 'dowiz-api', status: 'success' }, sum: { requests: 1077 } }],
    d: [
      { dimensions: { scriptName: 'dowiz-api', name: 'qa-durres', type: 'alarm', status: 'success' }, sum: { requests: alarmsQa } },
      ...['sushi-durres', 'dubin-durres', '__platform'].map(n => ({ dimensions: { scriptName: 'dowiz-api', name: n, type: 'alarm', status: 'success' }, sum: { requests: 1 } })),
      { dimensions: { scriptName: 'dowiz-api', name: 'dubin-durres', type: 'hibernation', status: 'success' }, sum: { requests: 534 } },
      { dimensions: { scriptName: 'dowiz-api', name: '__platform', type: 'http', status: 'success' }, sum: { requests: 1031 } }] }] } } }));
  const run = async a => byId(await collect({ results: { 'product.sushi_durres.orders_7d': I(0) }, hosts: ['a', 'b'], env: { CF_ANALYTICS_FILE: file }, fetch: day(a), now: () => 0 }));
  const before = await run(1450);
  assert.equal(before['cost.do_measured_over_modelled_permille'].value, Math.round((1000 * 1453) / 4), '3 nights + 1 fan-out turn');
  assert.ok(before['cost.do_measured_over_modelled_permille'].value > before['cost.do_measured_over_modelled_permille'].limit);
  const after = await run(1);
  assert.equal(after['cost.do_measured_over_modelled_permille'].value, 1000);
  assert.equal(after['cost.do_people_requests_day'].value, 1565);
  assert.equal(after['cost.do_people_requests_day'].rule, 'trend');
  assert.ok(after['cost.cron_share_permille'].value <= 500);
});
