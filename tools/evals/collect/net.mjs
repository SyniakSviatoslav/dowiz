// THE NETWORK SIDE OF EVERY NIGHTLY COLLECTOR: credentials, sign-in, one timed GET.
//
// GET-ONLY. Nothing here can write to a venue: the one POST is the sign-in, which creates a
// session in the platform's session table and nothing in any venue's hub. Credentials are read
// from /root/.dowiz_owner (or $DOWIZ_OWNER_FILE) and never printed, logged or written to a report.
import fs from 'node:fs';

export const CREDS = process.env.DOWIZ_OWNER_FILE || '/root/.dowiz_owner';

/** `export KEY=value` lines. A missing file is an empty object, never a throw. */
export function readCreds(file = CREDS, env = process.env) {
  const out = {};
  if (env.OWNER_EMAIL) out.OWNER_EMAIL = env.OWNER_EMAIL;
  if (env.OWNER_PASSWORD) out.OWNER_PASSWORD = env.OWNER_PASSWORD;
  if (!fs.existsSync(file)) return out;
  for (const line of fs.readFileSync(file, 'utf8').split('\n')) {
    if (!line.startsWith('export ')) continue;
    const eq = line.indexOf('=');
    out[line.slice(7, eq)] = line.slice(eq + 1);
  }
  return out;
}

/**
 * One GET, timed. `ttfb` = until the response headers arrived; `total` = until the last body
 * byte. Both include this client's DNS/TLS; the probe runs from wherever the suite runs, and the
 * report names the host it ran from rather than pretending to be the edge.
 */
export async function timedGet(fetchFn, url, headers = {}, now = () => performance.now()) {
  const t0 = now();
  const r = await fetchFn(url, { headers, redirect: 'manual' });
  const ttfb = now() - t0;
  const body = Buffer.from(await r.arrayBuffer());
  return {
    status: r.status,
    ttfb: Math.round(ttfb),
    total: Math.round(now() - t0),
    bytes: body.length,
    cache: r.headers.get('cf-cache-status') || '',
    cacheControl: r.headers.get('cache-control') || '',
    body,
  };
}

export async function getJson(fetchFn, url, token) {
  const r = await fetchFn(url, { headers: token ? { authorization: `Bearer ${token}` } : {} });
  const text = await r.text();
  let json = null;
  try { json = JSON.parse(text); } catch { /* not json: the status says why */ }
  return { status: r.status, json, text: text.slice(0, 200) };
}

/** The owner's access token for one venue host, or null with the status that refused it. */
export async function ownerToken(fetchFn, host, creds) {
  if (!creds.OWNER_EMAIL || !creds.OWNER_PASSWORD) return { token: null, why: 'no owner credentials' };
  const r = await fetchFn(`${host}/api/auth/login`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD }),
  });
  const b = await r.json().catch(() => ({}));
  return b.access_token ? { token: b.access_token, why: '' } : { token: null, why: `login answered ${r.status}` };
}

/** `sushi-durres` from `https://sushi-durres.dowiz.org`. */
export const slugOf = host => new URL(host).hostname.split('.')[0];
/** An indicator id segment from a slug. */
export const key = s => s.replace(/[^a-z0-9]+/gi, '_');
