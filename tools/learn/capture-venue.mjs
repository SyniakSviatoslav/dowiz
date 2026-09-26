// The venue side of a recording: sign in, snapshot what is open, and close what the
// recording left behind. `fetch` is injectable so capture.test.mjs proves every branch
// without the network. Tokens stay in memory; nothing here prints one.
const UA = { 'user-agent': 'Mozilla/5.0 (dowiz learn capture)', 'content-type': 'application/json' };
export const ENDED = new Set(['REJECTED', 'CANCELLED', 'COMPENSATED_REFUND', 'REFUNDED', 'DELIVERED', 'COLLECTED', 'COMPLETED', 'SERVED']);
export const REASON = 'TEST learn capture cleanup';

/// One API call. A 503 "Worker exceeded resource limits" (the platform's flap) is retried
/// (1102 on a sign-in's argon2 is common) is retried for reads and sign-ins only, at most eight
/// tries, `wait` ms apart; a write that 503'd may have landed and is never retried.
export async function call(host, path, { method = 'GET', body, token } = {}, f = globalThis.fetch, wait = 2500) {
  for (let i = 0; ; i++) {
    const r = await f(`${host}${path}`, { method, headers: { ...UA, ...(token ? { authorization: `Bearer ${token}` } : {}) },
      body: body == null ? undefined : JSON.stringify(body) });
    const t = await r.text();
    const retry = r.status === 503 && i < 7 && (method === 'GET' || /\/login$/.test(path));
    if (retry) { await new Promise(res => setTimeout(res, wait)); continue; }
    let b; try { b = JSON.parse(t); } catch { b = t; }
    return { status: r.status, body: b };
  }
}

/// What the app keeps after its own sign-in, per role: the recording starts signed in,
/// so no credential is ever typed on camera. Answers { local, session } storage entries.
export function seed(role, who) {
  if (role === 'owner') return { local: { dw_rt: who.owner.refresh_token, dw_loc: who.owner.user?.locationId }, session: { dw_at: who.owner.access_token } };
  if (role === 'waiter') return { local: { dw_room_session: JSON.stringify(who.staffBody) }, session: {} };
  if (role === 'courier') return { local: { dw_c_jwt: who.courier }, session: {} };
  if (role === 'guest') return { local: {}, session: {} };   // the storefront needs no sign-in
  return null;
}
export const ROLES_SIGNED = ['owner', 'waiter', 'courier', 'guest'];

/// API sign-in with the owner's credentials: the console token and, as staff, the room's
/// (both needed for the before/after snapshot whatever the role); a courier lesson also signs
/// the venue's courier in (COURIER_PHONE / COURIER_PASSWORD).
export async function signIn(host, role, c, f = globalThis.fetch) {
  if (!ROLES_SIGNED.includes(role)) return { status: `no sign-in for role ${role}`, token: null };
  const o = await call(host, '/api/auth/login', { method: 'POST', body: { email: c.OWNER_EMAIL, password: c.OWNER_PASSWORD } }, f);
  const s = await call(host, '/api/staff/login', { method: 'POST', body: { email: c.OWNER_EMAIL, password: c.OWNER_PASSWORD } }, f);
  let token = o.body?.access_token || null, status = `${o.status}/${s.status}`, courier = null;
  const staff = s.body?.jwt || null;
  if (role === 'courier') {
    const k = await call(host, '/api/courier/auth/login', { method: 'POST', body: { phone: c.COURIER_PHONE, password: c.COURIER_PASSWORD } }, f);
    courier = k.body?.jwt || null; status += `/${k.status}`;
    if (!courier) token = null;
  }
  return { status, token: token && staff ? token : null, staff, courier, owner: o.body, staffBody: s.body, f };
}

/// What is open at the venue: order ids with status, room sittings.
export async function snapshot(host, loc, who) {
  const o = await call(host, `/api/owner/orders?location_id=${encodeURIComponent(loc)}`, { token: who.token }, who.f);
  const r = await call(host, `/api/staff/room?location_id=${encodeURIComponent(loc)}`, { token: who.staff }, who.f);
  if (o.status !== 200 || r.status !== 200) throw new Error(`snapshot unreadable (orders ${o.status}, room ${r.status}): nothing can be proved closed`);
  const list = Array.isArray(o.body) ? o.body : (o.body.orders || []);
  return { orders: Object.fromEntries(list.map(x => [x.id, x.status])), sittings: (r.body.sittings || []).map(s => String(s.sitting_id ?? s.id)) };
}

/// What appeared between two snapshots.
export function diff(a, b) {
  return { orders: Object.keys(b.orders).filter(id => !(id in a.orders)).map(id => ({ id, status: b.orders[id] })),
    sittings: b.sittings.filter(s => !a.sittings.includes(s)) };
}

/// End each new order (reject a PENDING one, refund anything further on); a new
/// sitting has no end route and is reported. Answers how many are still open.
export async function closeNew(host, loc, who, fresh, log = () => {}) {
  let open = 0;
  for (const o of fresh.orders) {
    if (ENDED.has(o.status)) { log(`ended  order ${o.id} ${o.status}`); continue; }
    let r;
    if (o.status === 'PENDING') r = await call(host, `/api/owner/orders/${o.id}/action`, { method: 'POST', token: who.token, body: { action: 'reject', location_id: loc, reason: REASON } }, who.f);
    else {
      await call(host, `/api/staff/orders/${o.id}/refund`, { method: 'POST', token: who.staff, body: { location_id: loc, reason: 'other', note: REASON } }, who.f);
      r = await call(host, `/api/staff/orders/${o.id}/refund`, { method: 'POST', token: who.staff, body: { location_id: loc, complete: true } }, who.f);
    }
    const ok = r.status === 200;
    if (!ok) open++;
    log(`${ok ? 'closed' : 'OPEN  '} order ${o.id} ${o.status} (${r.status})`);
  }
  for (const s of fresh.sittings) { open++; log(`OPEN   sitting ${s}: no end route; close it in the room`); }
  return open;
}
