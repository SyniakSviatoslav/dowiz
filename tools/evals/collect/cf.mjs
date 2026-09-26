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
export const QUERY = `query($a:String!,$s:Time!,$e:Time!){viewer{accounts(filter:{accountTag:$a}){
 w: workersInvocationsAdaptive(limit:100, filter:{datetime_geq:$s, datetime_lt:$e}){ sum{requests errors subrequests cpuTimeUs} quantiles{cpuTimeP50 cpuTimeP99} dimensions{scriptName status} }
 d: durableObjectsInvocationsAdaptiveGroups(limit:100, filter:{datetime_geq:$s, datetime_lt:$e}){ sum{requests errors wallTime responseBodySize} dimensions{scriptName} }
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

/** The two groups for one script, summed over their status rows. */
export function fold(account, script = SCRIPT) {
  const w = (account.w || []).filter(r => r.dimensions.scriptName === script);
  const d = (account.d || []).filter(r => r.dimensions.scriptName === script);
  const sum = (rows, k) => rows.reduce((n, r) => n + (r.sum[k] || 0), 0);
  const worst = (rows, k) => rows.reduce((n, r) => Math.max(n, r.quantiles?.[k] || 0), 0);
  return {
    workerRequests: sum(w, 'requests'), workerErrors: sum(w, 'errors'), subrequests: sum(w, 'subrequests'),
    cpuP50Us: worst(w, 'cpuTimeP50'), cpuP99Us: worst(w, 'cpuTimeP99'),
    doRequests: sum(d, 'requests'), doErrors: sum(d, 'errors'), doResponseBytes: sum(d, 'responseBodySize'),
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

/** The measured indicators, or each one UNVERIFIED with the same reason. */
export async function measure(ctx) {
  const env = readCfEnv(ctx.env?.CF_ANALYTICS_FILE || FILE);
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
      ind('cf.cpu_p50_us', m.cpuP50Us, 'µs', 'plus25', src),
      ind('cf.cpu_p99_us', m.cpuP99Us, 'µs', 'plus25', src, { note: 'CPU per invocation (not memory: no analytics dataset reports isolate memory)' }),
      ind('cf.do_requests_day', m.doRequests, 'requests', 'trend', src),
      ind('cf.do_errors_day', m.doErrors, 'errors', 'zero', src),
      ind('cf.do_response_bytes_day', m.doResponseBytes, 'bytes', 'trend', `${src}: bytes the object sent the Worker`),
    ],
  };
}
