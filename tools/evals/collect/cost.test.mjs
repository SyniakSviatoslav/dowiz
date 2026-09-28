import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { inputs, model, price, collect, FIXED_MONTH, REQUESTS_PER_ORDER_BUDGET, USD, TIMER_DO_PER_ORDER } from './cost.mjs';

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
});

test('the model: integer micro-dollars; timers\' share of object requests (no minute cron)', () => {
  const m = model({ venues: 2, ordersPerDay: 10, perOrder: 100 });
  assert.equal(TIMER_DO_PER_ORDER, 10);
  assert.equal(m.durable, 10 * 10 + 2 + 1000);
  assert.equal(m.worker, 1 + 1000);
  assert.equal(m.cronShare, Math.round((1000 * 102) / 1102));
  assert.equal(m.marginal, Math.round(Math.round((30 * (1001 * 300_000 + 1102 * 150_000)) / 1e6) / 2));
  // AN IDLE PLATFORM: the nightly's one check per venue, nothing else.
  assert.equal(model({ venues: 4, ordersPerDay: 0, perOrder: 100 }).durable, 4);
  assert.equal(m.allIn, m.marginal + Math.round(FIXED_MONTH / 2));
  for (const v of Object.values(m)) assert.ok(Number.isInteger(v));
  assert.equal(FIXED_MONTH, 5 * USD + 933_333);
});

test('without a token: modelled prices, the cf numbers UNVERIFIED with the reason', async () => {
  const r = byId(await collect({ results: {}, hosts: ['a'], env: { CF_ANALYTICS_FILE: '/no/such' }, now: () => 0 }));
  assert.equal(r['cost.marginal_venue_month_micro_usd'].limit, USD);
  assert.match(r['cost.marginal_venue_month_micro_usd'].source, /^modelled/);
  assert.equal(r['cost.logs_month'].value, Math.round((1 * 30 * 100) / 1000));
  assert.equal(r['cost.cron_share_permille'].value, 1000);
  assert.equal(r['cost.cron_share_permille'].limit, 500);
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
    d: [{ dimensions: { scriptName: 'dowiz-api' }, sum: { requests: 6000, errors: 0, wallTime: 1, responseBodySize: 5 } }] }] } } }));
  const r = byId(await collect({ results: {}, hosts: ['a', 'b'], env: { CF_ANALYTICS_FILE: file }, fetch, now: () => Date.UTC(2026, 8, 26) }));
  assert.equal(r['cf.worker_requests_day'].value, 2000);
  assert.match(r['cost.marginal_venue_month_micro_usd'].source, /^MEASURED/);
  assert.equal(r['cost.marginal_venue_month_micro_usd'].value, price(2000, 6000, 2).marginal);
  assert.equal(r['cost.logs_month'].value, 6000);
  assert.equal(r['cost.do_measured_over_modelled_permille'].value, Math.round((1000 * 6000) / 2));
});
