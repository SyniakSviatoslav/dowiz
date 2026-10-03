// Row 44: the timers fire on their own. Cloudflare's count, not ours: the
// nightly cron (wrangler.toml `crons`) ran within 26 h with status success,
// and the venues' Durable Object alarms (hubdo/timer.rs) fired in the window.
// Token: Account Analytics:Read from /root/.cf_analytics_token, never printed.
import fs from 'node:fs';
import { readCfEnv } from '../../evals/collect/cf.mjs';
export default async function ({ check, must, note, NeedsKey }) {
  const env = readCfEnv(process.env.LIVE_CF_ANALYTICS_FILE || '/root/.cf_analytics_token');
  if (!env.CLOUDFLARE_API_TOKEN || !env.CLOUDFLARE_ACCOUNT_ID) throw new NeedsKey('no Cloudflare analytics token (LIVE_CF_ANALYTICS_FILE)');
  const crons = [...fs.readFileSync(new URL('../../../workers/api/wrangler.toml', import.meta.url), 'utf8').matchAll(/^crons\s*=\s*\[([^\]]*)\]/gm)]
    .flatMap(m => [...m[1].matchAll(/"([^"]+)"/g)].map(x => x[1]));
  must(crons.length, 'wrangler.toml declares no crons');
  const e = new Date(), s = new Date(Date.now() - 26 * 3600e3);
  const query = `query($a:String!,$s:Time!,$e:Time!){viewer{accounts(filter:{accountTag:$a}){
    s: workersInvocationsScheduled(limit:100, filter:{datetime_geq:$s, datetime_lt:$e, scriptName:"dowiz-api"}){ cron datetime status }
    d: durableObjectsInvocationsAdaptiveGroups(limit:20, filter:{datetime_geq:$s, datetime_lt:$e, scriptName:"dowiz-api"}){ sum{requests errors} dimensions{type} } }}}`;
  const r = await fetch('https://api.cloudflare.com/client/v4/graphql', { method: 'POST',
    headers: { authorization: `Bearer ${env.CLOUDFLARE_API_TOKEN}`, 'content-type': 'application/json' },
    body: JSON.stringify({ query, variables: { a: env.CLOUDFLARE_ACCOUNT_ID, s: s.toISOString(), e: e.toISOString() } }) });
  must(r.ok, `Cloudflare GraphQL ${r.status}`);
  const b = check('response_schema', await r.json());
  must(!b.errors, `GraphQL errors: ${JSON.stringify(b.errors).slice(0, 160)}`);
  const acc = b.data.viewer.accounts[0];
  const runs = acc.s.filter(x => crons.includes(x.cron)).sort((x, y) => y.datetime.localeCompare(x.datetime));
  must(runs.length, `no scheduled run of ${crons.join(', ')} in 26 h`);
  must(runs[0].status === 'success', `last scheduled run ${runs[0].datetime}: ${runs[0].status}`);
  const stray = acc.s.filter(x => !crons.includes(x.cron)).map(x => x.cron);
  must(!stray.length, `scheduled runs of crons wrangler.toml no longer declares: ${[...new Set(stray)].join(', ')}`);
  const alarm = acc.d.find(x => x.dimensions.type === 'alarm')?.sum || { requests: 0, errors: 0 };
  must(alarm.requests > 0, 'no Durable Object alarm fired in 26 h');
  note(`cron ${runs[0].cron} ran ${runs[0].datetime} ${runs[0].status} (${runs.length} in 26 h); DO alarms ${alarm.requests}, errors ${alarm.errors}`);
}
