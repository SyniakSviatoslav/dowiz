// LIVE PROBE alarm-cadence (W-LOOP) -- main runs it AFTER the deploy, once a
// full UTC day has passed on the new build; a lane never runs it.
//
//   node tools/live-proof/probes/feature-alarm-cadence.mjs        (DAY=YYYY-MM-DD to pick the day)
//
// CLOUDFLARE'S OWN COUNT, NOT OURS. Before the fix one object's alarm fired
// 1,449 times a day (docs/research/2026-10-03-hub-cost-recomputed.md §1.3) and
// its runner took as many requests again. After it, over one complete UTC day:
//  1. the GraphQL answer validates against the contract's response_schema;
//  2. NO object's alarm fired more than `bounds.alarm_invocations_per_object_day_max`;
//  3. the nightly cron ran exactly once, with status success.
// Token: Account Analytics:Read from /root/.cf_analytics_token
// (LIVE_CF_ANALYTICS_FILE), never printed. Exit 0 held, 1 FAILED, 3 NEEDS-KEY.
import fs from 'node:fs';
import { readCfEnv, ENDPOINT, SCRIPT } from '../../evals/collect/cf.mjs';
import { contract, reporter, validate } from './stock-lib.mjs';

const c = contract('alarm-cadence');
const { step, verdict } = reporter('feature-alarm-cadence');
const env = readCfEnv(process.env.LIVE_CF_ANALYTICS_FILE || '/root/.cf_analytics_token');
if (!env.CLOUDFLARE_API_TOKEN || !env.CLOUDFLARE_ACCOUNT_ID) {
  step('Cloudflare analytics token', 'NEEDS-KEY', 'no CLOUDFLARE_API_TOKEN / CLOUDFLARE_ACCOUNT_ID in LIVE_CF_ANALYTICS_FILE');
  verdict();
} else {
  const day = process.env.DAY || new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
  const s = `${day}T00:00:00Z`, e = new Date(Date.parse(s) + 86_400_000).toISOString();
  const crons = [...fs.readFileSync(new URL('../../../workers/api/wrangler.toml', import.meta.url), 'utf8').matchAll(/^crons\s*=\s*\[([^\]]*)\]/gm)]
    .flatMap(m => [...m[1].matchAll(/"([^"]+)"/g)].map(x => x[1]));
  const query = `query($a:String!,$s:Time!,$e:Time!){viewer{accounts(filter:{accountTag:$a}){
    o: durableObjectsInvocationsAdaptiveGroups(limit:1000, filter:{datetime_geq:$s, datetime_lt:$e, scriptName:"${SCRIPT}"}){ sum{requests} dimensions{objectId type} }
    s: workersInvocationsScheduled(limit:100, filter:{datetime_geq:$s, datetime_lt:$e, scriptName:"${SCRIPT}"}){ cron datetime status } }}}`;
  const body = { query, variables: { a: env.CLOUDFLARE_ACCOUNT_ID, s, e } };
  const req = validate(body, c.request_schema);
  step('request validates against request_schema', !req.length, req.join('; '));
  const r = await fetch(ENDPOINT, { method: 'POST', headers: { authorization: `Bearer ${env.CLOUDFLARE_API_TOKEN}`, 'content-type': 'application/json' }, body: JSON.stringify(body) });
  const b = await r.json().catch(() => ({}));
  const bad = r.ok && !b.errors ? validate(b, c.response_schema) : [`HTTP ${r.status} ${JSON.stringify(b.errors || '').slice(0, 200)}`];
  step('GraphQL answer validates against response_schema', !bad.length, bad.slice(0, 5).join('; '));
  if (!bad.length) {
    const acc = b.data.viewer.accounts[0];
    const alarms = acc.o.filter(x => x.dimensions.type === 'alarm').map(x => ({ id: x.dimensions.objectId.slice(0, 8), n: x.sum.requests })).sort((x, y) => y.n - x.n);
    const bound = c.bounds.alarm_invocations_per_object_day_max;
    const total = alarms.reduce((t, x) => t + x.n, 0);
    step(`every object's alarms <= ${bound} on ${day}`, alarms.every(x => x.n <= bound),
      `top ${JSON.stringify(alarms.slice(0, 5))}; ${alarms.length} objects, ${total} alarm invocations (before the fix: one object 1,449/day)`);
    const runs = acc.s.filter(x => crons.includes(x.cron));
    step(`the nightly ran once, success (${crons.join(', ')})`, runs.length === 1 && runs[0].status === 'success', JSON.stringify(runs));
  }
  verdict();
}
