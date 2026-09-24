// SWEEP -- every TEST-marked artefact on the live venue, and (with `close`) its end.
//   node sweep.mjs          list: open TEST orders, TEST bookings not ended, the floor plan,
//                           active TEST staff, pending TEST invites, TEST campaigns, open room sittings
//   node sweep.mjs close    end each one through the owner API (reject/cancel/refund, booking cancel,
//                           plan emptied, staff suspended; a pending invite is claimed with the code in
//                           suite state and suspended -- the product has no revoke route)
// Run it FIRST (a killed run leaves things open) and LAST (the proof). Exit code = artefacts still open.
import { api, ownerToken, staffToken, LOC, sst, staffLogin } from './_lib.mjs';

const CLOSE = process.argv[2] === 'close';
const tok = await ownerToken();
const stok = await staffToken();   // the refund route is a staff route; the owner signs in there too
const S = sst();
const T = /TEST/i;
const ENDED = new Set(['REJECTED', 'CANCELLED', 'COMPENSATED_REFUND', 'REFUNDED', 'DELIVERED', 'COLLECTED', 'COMPLETED', 'SERVED']);
const RSV_ENDED = new Set(['CANCELLED_BY_GUEST', 'CANCELLED_BY_VENUE', 'DECLINED', 'NO_SHOW', 'COMPLETED', 'EXPIRED']);
let open = 0;
const line = (state, what, detail = '') => console.log(`${state.padEnd(6)} ${what}${detail ? ' :: ' + detail : ''}`);
const act = (id, action, reason) => api(`/api/owner/orders/${id}/action`, { method: 'POST', token: tok, body: { action, location_id: LOC, reason } });
const ord = async id => ((await api(`/api/owner/orders?location_id=${LOC}`, { token: tok })).body.orders || []).find(o => o.id === id);

// ── orders ──
const ol = await api(`/api/owner/orders?location_id=${LOC}`, { token: tok });
if (!tok || !stok || ol.status !== 200) { console.log(`cannot read the venue (owner token ${!!tok}, staff token ${!!stok}, orders ${ol.status}): nothing is proved`); process.exit(99); }
// A waiter's round carries no name (contact is empty): it is TEST by its id in suite state (r1, r2...).
const mine = new Set([S.r1, S.r2, ...(S.extra || [])].filter(Boolean));
const orders = (Array.isArray(ol.body) ? ol.body : ol.body.orders || []).filter(o => mine.has(o.id) || T.test(JSON.stringify([o.contact, o.fulfilment, o.id, o.note])));
for (const o of orders) {
  if (ENDED.has(o.status)) { line('ended', `order ${o.id}`, `${o.status} ${o.contact?.name || o.fulfilment?.table || ''}`); continue; }
  if (!CLOSE) { open++; line('OPEN', `order ${o.id}`, `${o.status} ${o.fulfilment?.kind} ${o.contact?.name || o.fulfilment?.table || ''}`); continue; }
  let r;
  if (o.status === 'PENDING') r = await act(o.id, 'reject', 'TEST walk cleanup');
  else {
    if (o.status !== 'REFUNDING') r = await api(`/api/staff/orders/${o.id}/refund`, { method: 'POST', token: stok, body: { location_id: LOC, reason: 'other', note: 'TEST walk cleanup' } });
    r = await api(`/api/staff/orders/${o.id}/refund`, { method: 'POST', token: stok, body: { location_id: LOC, complete: true } });
  }
  const after = (await ord(o.id))?.status;
  if (!ENDED.has(after)) open++;
  line(ENDED.has(after) ? 'closed' : 'OPEN', `order ${o.id}`, `${o.status} -> ${after} (${r.status} ${JSON.stringify(r.body).slice(0, 120)})`);
}

// ── bookings: a week back and a week ahead, in two legal windows ──
const now = Math.floor(Date.now() / 60000);
const rs = [];
for (const [f, t] of [[now - 7 * 1440 + 1, now], [now, now + 7 * 1440]]) {
  const r = await api(`/api/owner/reservations?from=${f}&to=${t}`, { token: tok });
  if (r.status !== 200) line('FAIL', 'reservations read', `${r.status} ${JSON.stringify(r.body).slice(0, 100)}`);
  rs.push(...(r.body.reservations || []));
}
for (const x of rs.filter(x => T.test(x.name || x.contactName || ''))) {
  if (RSV_ENDED.has(x.status)) { line('ended', `booking ${x.id}`, `${x.status} ${x.name}`); continue; }
  if (!CLOSE) { open++; line('OPEN', `booking ${x.id}`, `${x.status} ${x.name}`); continue; }
  const r = await api(`/api/owner/reservations/${x.id}/action`, { method: 'POST', token: tok, body: { to: x.status === 'REQUESTED' ? 'DECLINED' : 'CANCELLED_BY_VENUE', reason: 'TEST walk cleanup' } });
  const ok = r.status === 200; if (!ok) open++;
  line(ok ? 'closed' : 'OPEN', `booking ${x.id}`, `${x.status} -> ${r.body?.status} (${r.status})`);
}

// ── the floor plan (EMPTY on this venue outside a walk) ──
const fp = await api('/api/owner/floorplan', { token: tok });
const zones = fp.body.zones || [];
if (fp.status !== 200) { open++; line('OPEN?', 'floor plan', `read answered ${fp.status}: unproved`); }
else if (!zones.length) line('ended', 'floor plan', 'empty');
else if (!CLOSE || !zones.every(z => T.test(z.name))) { open++; line('OPEN', 'floor plan', JSON.stringify(zones.map(z => [z.id, z.name, z.tables.map(t => t.n)]))); }
else {
  const r = await api('/api/owner/floorplan', { method: 'POST', token: tok, body: { zones: [] } });
  const z2 = (await api('/api/owner/floorplan', { token: tok })).body.zones || [];
  if (z2.length) open++;
  line(z2.length ? 'OPEN' : 'closed', 'floor plan', `${zones.map(z => z.name)} -> ${z2.length} zones (${r.status})`);
}

// ── room sittings still open (a TEST table) ──
const room = await api(`/api/staff/room?location_id=${LOC}`, { token: tok });
if (room.status !== 200) { open++; line('OPEN?', 'room sittings', `read answered ${room.status}: unproved`); }
for (const s of (room.body.sittings || []).filter(s => T.test(JSON.stringify(s)) || /^9[12]$/.test(String(s.table).split(':').pop()))) {
  open++; line('OPEN', `sitting ${s.sitting_id ?? s.id}`, `${s.table} rounds=${(s.rounds || []).map(r => r.id.slice(0, 8) + ':' + r.status).join(',')}`);
}

// ── staff and invites ──
let st = await api('/api/owner/staff', { token: tok });
if (CLOSE) for (const i of (st.body.invites || []).filter(i => T.test(i.name) && !i.expired)) {
  const who = Object.values(S).find(v => v && v.name === i.name && v.code);
  if (!who) { line('OPEN', `invite ${i.id}`, `${i.name}: no code in suite state`); continue; }
  const c = await api('/api/staff/claim', { method: 'POST', body: { email: who.email, code: who.code, password: who.password } });
  line(c.status === 200 ? 'info' : 'FAIL', `claimed invite ${i.id} to end it`, `${i.name} ${c.status}`);
}
st = await api('/api/owner/staff', { token: tok });
if (st.status !== 200) { open++; line('OPEN?', 'staff', `read answered ${st.status}: unproved`); }
for (const s of (st.body.staff || []).filter(s => T.test(s.name))) {
  if (!s.active) { line('ended', `staff ${s.id}`, `${s.name} suspended`); continue; }
  if (!CLOSE) { open++; line('OPEN', `staff ${s.id}`, `${s.name} ${s.role} active`); continue; }
  const r = await api(`/api/owner/staff/${s.id}`, { method: 'POST', token: tok, body: { active: false } });
  const a = ((await api('/api/owner/staff', { token: tok })).body.staff || []).find(x => x.id === s.id);
  if (a?.active) open++;
  line(a?.active ? 'OPEN' : 'closed', `staff ${s.id}`, `${s.name} -> active=${a?.active} (${r.status})`);
}
for (const i of ((await api('/api/owner/staff', { token: tok })).body.invites || []).filter(i => T.test(i.name) && !i.expired)) { open++; line('OPEN', `invite ${i.id}`, `${i.name} ${i.role} pending`); }
for (const k of ['waiter', 'kitchen']) if (S[k]?.email) {
  const l = await staffLogin(S[k].email, S[k].password);
  if (l.status === 200) { open++; line('OPEN', `TEST ${k} can still sign in`, S[k].email); }
  else if (l.status !== 401 && l.status !== 403) { open++; line('OPEN?', `TEST ${k} sign-in answered ${l.status}`, 'neither 200 nor a refusal: unproved'); }
}

// ── campaigns ──
const cp = await api('/api/owner/campaigns', { token: tok });
for (const c of (cp.body.campaigns || []).filter(c => T.test(c.name || ''))) line('info', `campaign ${c.id}`, `${c.name} sent=${c.report?.sent ?? c.sent ?? 0} status=${c.status || ''}`);

console.log(`\n${open} TEST artefact(s) still open`);
process.exit(open);
