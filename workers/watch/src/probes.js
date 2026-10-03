// What one tick asks of production. The rows are the ones tools/live-checks/health.sh runs; that
// script stays the box-side copy. Pure except for `fetch`, which is passed in so tests can drive it.
//
// "UP" IS ONE RULE FOR EVERY TARGET: the expected status, the expected body, and NO `cf-mitigated`
// header. A Cloudflare challenge is CHALLENGED, never up: a monitor that could not see the service
// has not seen it up (the old heartbeat called a 403 challenge page healthy for six runs).

export const UA = 'dowiz-watch/0.1 (+https://github.com/SyniakSviatoslav/dowiz)';

/// The targets of one tick, from the Worker's vars.
export function targets(env) {
  const domain = env.WATCH_DOMAIN || 'dowiz.org';
  const venues = String(env.WATCH_VENUES || '').split(/\s+/).filter(Boolean);
  const out = [{ name: 'platform healthz', kind: 'healthz', url: `https://${domain}/healthz` }];
  for (const v of venues) {
    const base = `https://${v}.${domain}`;
    out.push({ name: `${v} healthz`, kind: 'healthz', url: `${base}/healthz` });
    out.push({ name: `${v} storefront`, kind: 'store', url: `${base}/` });
    out.push({ name: `${v} menu`, kind: 'menu', url: `${base}/api/public/locations/${v}/menu` });
    out.push({ name: `${v} quote`, kind: 'quote', url: `${base}/api/public/locations/${v}/eta` });
  }
  for (const pair of String(env.EXTRA_TARGETS || '').split(/\s+/).filter(Boolean)) {
    const i = pair.indexOf('=');
    if (i > 0) out.push({ name: pair.slice(0, i), kind: 'healthz', url: pair.slice(i + 1) });
  }
  return out;
}

/// The menu's dish count, or -1 when the body is not a menu.
export function dishCount(text) {
  try {
    const d = JSON.parse(text);
    return (d.categories || []).reduce((n, c) => n + ((c && c.products) || []).length, 0);
  } catch {
    return -1;
  }
}

/// The verdict on one answer: {status: 'up'|'down'|'challenged', detail}. Pure.
export function judge(kind, code, headers, body) {
  const mitigated = headers.get('cf-mitigated');
  if (mitigated) return { status: 'challenged', detail: `${code} cf-mitigated: ${mitigated}` };
  const snip = String(body || '').slice(0, 80).replace(/\s+/g, ' ');
  if (code !== 200) return { status: 'down', detail: `${code} ${snip}` };
  switch (kind) {
    case 'healthz':
      return body.trim() === 'ok' ? { status: 'up', detail: '200 ok' } : { status: 'down', detail: `200 body "${snip}"` };
    case 'store': {
      const html = /text\/html/i.test(headers.get('content-type') || '');
      const csp = !!headers.get('content-security-policy');
      return html && csp ? { status: 'up', detail: '200 html, CSP present' } : { status: 'down', detail: `200 html=${html} csp=${csp}` };
    }
    case 'menu': {
      const n = dishCount(body);
      return n > 0 ? { status: 'up', detail: `200, ${n} dishes` } : { status: 'down', detail: `200, dishes=${n}` };
    }
    case 'quote':
      return /"range"/.test(body) ? { status: 'up', detail: '200 range' } : { status: 'down', detail: `200 no range: ${snip}` };
    default:
      return { status: 'down', detail: `unknown kind ${kind}` };
  }
}

/// One request. The ETA quote is the one POST; it computes and stores nothing (eta.rs::quote).
function request(t) {
  const init = { headers: { 'user-agent': UA }, redirect: 'manual', signal: AbortSignal.timeout(10000) };
  if (t.kind === 'quote') {
    init.method = 'POST';
    init.headers['content-type'] = 'application/json';
    init.body = JSON.stringify({ items: [{ quantity: 1 }], pickup: true });
  }
  return init;
}

/// Probe one target, asking once more when the first answer is not up: one lost packet must not
/// page anybody. Returns {name, url, status, detail, code, ms}.
export async function probe(t, fetchFn = fetch) {
  let last;
  for (let attempt = 0; attempt < 2; attempt++) {
    const t0 = Date.now();
    try {
      const r = await fetchFn(t.url, request(t));
      const body = await r.text();
      last = { ...judge(t.kind, r.status, r.headers, body), code: r.status, ms: Date.now() - t0 };
    } catch (e) {
      last = { status: 'down', detail: `unreachable: ${String(e && e.message || e).slice(0, 80)}`, code: 0, ms: Date.now() - t0 };
    }
    if (last.status === 'up') break;
  }
  return { name: t.name, url: t.url, ...last };
}
