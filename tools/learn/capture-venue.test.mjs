// node --test tools/learn/capture-venue.test.mjs -- sign-in, snapshot, diff and the closing
// sweep against a fake venue (an injected fetch), so every branch runs without the network.
import test from 'node:test';
import assert from 'node:assert/strict';
import { call, seed, signIn, snapshot, diff, closeNew, REASON, ROLES_SIGNED } from './capture-venue.mjs';

/// A fake venue: answers by "METHOD path", records every call; a list answer is consumed in order.
function venue(routes) {
  const calls = [];
  const f = async (url, init) => {
    const u = new URL(url), k = `${init.method} ${u.pathname}`;
    calls.push({ k, init, search: u.search });
    let a = routes[k];
    if (Array.isArray(a) && Array.isArray(a[0])) a = a.length > 1 ? a.shift() : a[0];
    if (!a) a = [404, 'not found'];
    const [status, body] = a;
    return { status, text: async () => typeof body === 'string' ? body : JSON.stringify(body) };
  };
  return { f, calls };
}
const H = 'https://v';
const C = { OWNER_EMAIL: 'o@v', OWNER_PASSWORD: 'pw' };

test('call: JSON parsed, text kept, bearer sent, UA set', async () => {
  const v = venue({ 'GET /a': [200, { x: 1 }], 'POST /b': [200, 'plain'] });
  assert.deepEqual(await call(H, '/a', { token: 't' }, v.f), { status: 200, body: { x: 1 } });
  assert.deepEqual(await call(H, '/b', { method: 'POST', body: { y: 2 } }, v.f), { status: 200, body: 'plain' });
  assert.equal(v.calls[0].init.headers.authorization, 'Bearer t');
  assert.match(v.calls[0].init.headers['user-agent'], /Mozilla/);
  assert.equal(v.calls[1].init.body, '{"y":2}');
});

test('call: a 503 on a read or a sign-in is retried (bounded); a write is never retried', async () => {
  const r = venue({ 'GET /a': [[503, 'error code: 1102'], [200, { ok: 1 }]] });
  assert.equal((await call(H, '/a', {}, r.f, 0)).status, 200);
  assert.equal(r.calls.length, 2);
  const forever = venue({ 'POST /api/auth/login': [[503, 'x']] });
  assert.equal((await call(H, '/api/auth/login', { method: 'POST', body: {} }, forever.f, 0)).status, 503);
  assert.equal(forever.calls.length, 8);
  const w = venue({ 'POST /api/x': [[503, 'x'], [200, {}]] });
  assert.equal((await call(H, '/api/x', { method: 'POST', body: {} }, w.f, 0)).status, 503);
  assert.equal(w.calls.length, 1);
});

test('signIn: both tokens or none; unknown roles are refused', async () => {
  const ok = venue({ 'POST /api/auth/login': [200, { access_token: 'A', refresh_token: 'R' }], 'POST /api/staff/login': [200, { jwt: 'J', staff: { id: 's' } }] });
  const w = await signIn(H, 'waiter', C, ok.f);
  assert.deepEqual([w.token, w.staff, w.status], ['A', 'J', '200/200']);
  assert.deepEqual(JSON.parse(ok.calls[0].init.body), { email: 'o@v', password: 'pw' });
  const half = venue({ 'POST /api/auth/login': [200, { access_token: 'A' }], 'POST /api/staff/login': [401, { error: 'no' }] });
  assert.equal((await signIn(H, 'owner', C, half.f)).token, null);
  assert.equal((await signIn(H, 'kitchen', C, ok.f)).token, null);
  assert.equal((await signIn(H, 'guest', C, ok.f)).token, 'A');
  assert.deepEqual(ROLES_SIGNED, ['owner', 'waiter', 'courier', 'guest']);
  const cr = venue({ 'POST /api/auth/login': [200, { access_token: 'A' }], 'POST /api/staff/login': [200, { jwt: 'J' }], 'POST /api/courier/auth/login': [200, { jwt: 'K' }] });
  const k = await signIn(H, 'courier', { ...C, COURIER_PHONE: '+1', COURIER_PASSWORD: 'q' }, cr.f);
  assert.deepEqual([k.token, k.courier, k.status], ['A', 'K', '200/200/200']);
  assert.deepEqual(JSON.parse(cr.calls[2].init.body), { phone: '+1', password: 'q' });
  const nocr = venue({ 'POST /api/auth/login': [200, { access_token: 'A' }], 'POST /api/staff/login': [200, { jwt: 'J' }], 'POST /api/courier/auth/login': [403, {}] });
  const n = await signIn(H, 'courier', C, nocr.f);
  assert.deepEqual([n.token, n.status], [null, '200/200/403']);
});

test('seed: the console gets its token pair, the room its session; others nothing', () => {
  const who = { owner: { access_token: 'A', refresh_token: 'R', user: { locationId: 'L' } }, staffBody: { jwt: 'J', staff: { id: 's' } } };
  assert.deepEqual(seed('owner', who), { local: { dw_rt: 'R', dw_loc: 'L' }, session: { dw_at: 'A' } });
  assert.deepEqual(JSON.parse(seed('waiter', who).local.dw_room_session), who.staffBody);
  assert.deepEqual(seed('courier', { courier: 'K' }), { local: { dw_c_jwt: 'K' }, session: {} });
  assert.deepEqual(seed('guest', who), { local: {}, session: {} });
  assert.equal(seed('kitchen', who), null);
});

test('snapshot + diff: what appeared, and an unreadable venue is an error, not an empty list', async () => {
  const v = venue({ 'GET /api/owner/orders': [[200, { orders: [{ id: 'a', status: 'PENDING' }] }], [200, [{ id: 'a', status: 'PENDING' }, { id: 'b', status: 'CONFIRMED' }]]],
    'GET /api/staff/room': [[200, { sittings: [{ sitting_id: 1 }] }], [200, { sittings: [{ sitting_id: 1 }, { id: 2 }] }]] });
  const who = { token: 'A', staff: 'J', f: v.f };
  const a = await snapshot(H, 'loc', who), b = await snapshot(H, 'loc', who);
  assert.equal(v.calls[0].search, '?location_id=loc');
  assert.deepEqual(diff(a, b), { orders: [{ id: 'b', status: 'CONFIRMED' }], sittings: ['2'] });
  const bad = venue({ 'GET /api/owner/orders': [401, {}], 'GET /api/staff/room': [200, {}] });
  await assert.rejects(snapshot(H, 'loc', { f: bad.f }), /unreadable \(orders 401, room 200\)/);
});

test('closeNew: PENDING rejected, further on refunded, ended left alone, sittings reported open', async () => {
  const v = venue({ 'POST /api/owner/orders/p/action': [200, {}], 'POST /api/staff/orders/c/refund': [200, {}], 'POST /api/staff/orders/x/refund': [409, {}] });
  const log = [];
  const open = await closeNew(H, 'loc', { token: 'A', staff: 'J', f: v.f },
    { orders: [{ id: 'p', status: 'PENDING' }, { id: 'c', status: 'CONFIRMED' }, { id: 'd', status: 'DELIVERED' }, { id: 'x', status: 'READY' }], sittings: ['9'] },
    l => log.push(l));
  assert.equal(open, 2);                                  // x refused, sitting 9
  const rej = v.calls.find(c => c.k === 'POST /api/owner/orders/p/action');
  assert.deepEqual(JSON.parse(rej.init.body), { action: 'reject', location_id: 'loc', reason: REASON });
  assert.equal(v.calls.filter(c => c.k === 'POST /api/staff/orders/c/refund').length, 2);
  assert.ok(!v.calls.some(c => c.k.includes('/d/')));
  assert.match(log.join('\n'), /closed order p[\s\S]*ended  order d[\s\S]*OPEN   order x[\s\S]*OPEN   sitting 9/);
  assert.equal(await closeNew(H, 'loc', { f: v.f }, { orders: [], sittings: [] }), 0);
});

test('signIn + snapshot + closeNew: a null body is no token; bodies without lists are empty; the default log is silent', async () => {
  const nul = venue({ 'POST /api/auth/login': [200, null], 'POST /api/staff/login': [200, { jwt: 'J' }] });
  assert.equal((await signIn(H, 'owner', C, nul.f)).token, null);
  const bare = venue({ 'GET /api/owner/orders': [200, {}], 'GET /api/staff/room': [200, {}] });
  assert.deepEqual(await snapshot(H, 'loc', { token: 'A', staff: 'J', f: bare.f }), { orders: {}, sittings: [] });
  assert.equal(await closeNew(H, 'loc', { f: bare.f }, { orders: [{ id: 'd', status: 'SERVED' }], sittings: [] }), 0);
});
