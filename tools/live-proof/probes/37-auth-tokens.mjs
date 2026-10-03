// Row 37: an owner's HMAC-SHA256 token opens owner routes; a tampered or
// missing token is refused (workers/api/src/auth.rs).
export default async function ({ lib, check, must, note }) {
  const { api, creds, LOC } = lib;
  const r = await api('/api/auth/login', { method: 'POST', body: { email: creds.OWNER_EMAIL, password: creds.OWNER_PASSWORD, location_id: LOC } });
  must(r.status === 200, `login ${r.status}: ${r.text.slice(0, 100)}`);
  const tok = check('response_schema', r.body).access_token;
  const ok = await api(`/api/owner/orders?location_id=${LOC}`, { token: tok });
  must(ok.status === 200, `owner orders with the token: ${ok.status}`);
  // One byte of the PAYLOAD flipped: the signature no longer covers it.
  const [h, p, s] = tok.split('.');
  const i = Math.floor(p.length / 2);
  const flipped = `${h}.${p.slice(0, i)}${p[i] === 'A' ? 'B' : 'A'}${p.slice(i + 1)}.${s}`;
  const bad = await api(`/api/owner/orders?location_id=${LOC}`, { token: flipped });
  must(bad.status === 401, `a tampered token answered ${bad.status}, not 401`);
  const none = await api(`/api/owner/orders?location_id=${LOC}`);
  must(none.status === 401, `no token answered ${none.status}, not 401`);
  note(`login 200; owner orders 200; flipped payload byte 401; no token 401`);
}
