// THE PURE NETWORK HALF of the nightly collectors: JSON GET, the owner sign-in, a host's slug.
//
// NO node:* IMPORT, on purpose: dowiz-watch (workers/watch/src/evals.js) imports health.mjs and
// product.mjs through this file and runs the SAME code from inside Cloudflare (W-EVALSCF,
// 2026-10-08: Bot Fight Mode challenges GitHub's runners). net.mjs re-exports all of it.
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
