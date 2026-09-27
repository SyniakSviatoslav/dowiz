// node --test workers/api/public/admin/signin.test.mjs
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { signIn } from './signin.js';

/// A fetch that answers each URL from a table, and records what was asked.
function hub(table){
  const asked = [];
  const f = async (url, init) => {
    asked.push({ url, body: JSON.parse(init.body) });
    const [status, body, type] = table[url];
    return new Response(typeof body === 'string' ? body : JSON.stringify(body), { status, headers: { 'content-type': type || (typeof body === 'string' ? 'text/plain' : 'application/json') } });
  };
  return { f, asked };
}

test('a member of staff signs in although the owner door refuses in plain text (the live defect)', async () => {
  const h = hub({
    '/api/auth/login': [403, 'no active owner membership'],
    '/api/staff/login': [200, { jwt: 'staff.jwt', staff: { locationId: 'qa-durres', role: 'kitchen' } }],
  });
  const s = await signIn(h.f, 'k@x', 'pw');
  assert.deepEqual(s, { token: 'staff.jwt', refresh: null, loc: 'qa-durres', staff: true });
  assert.deepEqual(h.asked.map(a => a.url), ['/api/auth/login', '/api/staff/login']);
  assert.deepEqual(h.asked[1].body, { email: 'k@x', password: 'pw' });
});

test('an owner signs in at the first door and staff is never asked', async () => {
  const h = hub({ '/api/auth/login': [200, { access_token: 'o.jwt', refresh_token: 'r1', user: { locationId: 'qa-durres' } }] });
  assert.deepEqual(await signIn(h.f, 'o@x', 'pw'), { token: 'o.jwt', refresh: 'r1', loc: 'qa-durres', staff: false });
  assert.equal(h.asked.length, 1);
});

test('a mistyped password shows the hub\'s words, not a JSON parse error', async () => {
  const h = hub({ '/api/auth/login': [401, 'invalid credentials'], '/api/staff/login': [401, 'invalid credentials'] });
  await assert.rejects(signIn(h.f, 'o@x', 'bad'), e => e.message === 'invalid credentials' && !/JSON/.test(e.message));
});

test('staff\'s reason wins over the owner door\'s "not an owner here"', async () => {
  const h = hub({ '/api/auth/login': [403, 'no active owner membership'], '/api/staff/login': [403, 'not staff at this venue'] });
  await assert.rejects(signIn(h.f, 'k@x', 'pw'), /not staff at this venue/);
});

test('an owner door that answers JSON errors is read as JSON', async () => {
  const h = hub({ '/api/auth/login': [403, { error: 'that location is not this venue' }], '/api/staff/login': [401, 'invalid credentials'] });
  await assert.rejects(signIn(h.f, 'o@x', 'pw'), /that location is not this venue/);
});

test('no words at all still says which status', async () => {
  const h = hub({ '/api/auth/login': [500, ''], '/api/staff/login': [503, ''] });
  await assert.rejects(signIn(h.f, 'o@x', 'pw'), /HTTP 503/);
});

// ── "I have a staff code" (POST /api/staff/claim) ──
import { claim } from './signin.js';

test('a staff code turns into a signed-in console at the venue it was issued for', async () => {
  const h = hub({ '/api/staff/claim': [200, { jwt: 'k.jwt', staff: { locationId: 'dubin-sushi', role: 'kitchen', caps: 'advance,catalog,stock' } }] });
  assert.deepEqual(await claim(h.f, ' cook@x.al ', ' 4821 ', 'long-password'), { token: 'k.jwt', refresh: null, loc: 'dubin-sushi', staff: true });
  assert.deepEqual(h.asked, [{ url: '/api/staff/claim', body: { email: 'cook@x.al', code: '4821', password: 'long-password' } }]);
});

test('a code the hub refuses shows the hub\'s words', async () => {
  const h = hub({ '/api/staff/claim': [400, 'that code does not match'] });
  await assert.rejects(claim(h.f, 'cook@x.al', '0000', 'long-password'), e => e.message === 'that code does not match');
});

test('an existing account must bring its own password (the hub\'s 401 is shown)', async () => {
  const h = hub({ '/api/staff/claim': [401, 'this address already has an account: use its password'] });
  await assert.rejects(claim(h.f, 'owner@x.al', '4821', 'guess-guess'), /use its password/);
});

test('an empty field is not sent at all', async () => {
  const h = hub({});
  for (const [e, c, p] of [['', '1', 'pw'], ['a@x', '', 'pw'], ['a@x', '1', '']]) await assert.rejects(claim(h.f, e, c, p), /missing/);
  assert.equal(h.asked.length, 0);
});

test('a claim with no words still says which status', async () => {
  const h = hub({ '/api/staff/claim': [502, ''] });
  await assert.rejects(claim(h.f, 'a@x', '1', 'pw'), /HTTP 502/);
});

// ── a member of staff changes their own password (POST /api/staff/password) ──
import { changePassword } from './signin.js';

test('a password change sends the old and the new, then signs in again with the new one', async () => {
  const h = hub({
    '/api/staff/password': [200, { changed: true, sessionsEnded: 2 }],
    '/api/auth/login': [403, 'no active owner membership'],
    '/api/staff/login': [200, { jwt: 'fresh.jwt', staff: { locationId: 'dubin-sushi', role: 'kitchen' } }],
  });
  assert.deepEqual(await changePassword(h.f, ' cook@x.al ', 'old-password', 'new-password'), { token: 'fresh.jwt', refresh: null, loc: 'dubin-sushi', staff: true });
  assert.deepEqual(h.asked[0], { url: '/api/staff/password', body: { email: 'cook@x.al', old_password: 'old-password', new_password: 'new-password' } });
  assert.deepEqual(h.asked.slice(1).map(a => a.body), [{ email: 'cook@x.al', password: 'new-password' }, { email: 'cook@x.al', password: 'new-password' }]);
});

test('a wrong old password shows the hub\'s 401 words', async () => {
  const h = hub({ '/api/staff/password': [401, 'invalid credentials'] });
  await assert.rejects(changePassword(h.f, 'cook@x.al', 'wrong', 'new-password'), e => e.message === 'invalid credentials');
});

test('a short new password or an empty field is refused before anything is sent', async () => {
  const h = hub({});
  await assert.rejects(changePassword(h.f, 'cook@x.al', 'old-password', '1234567'), /short/);
  for (const [e, o, n] of [['', 'o', 'new-password'], ['a@x', '', 'new-password'], ['a@x', 'o', '']]) await assert.rejects(changePassword(h.f, e, o, n), /missing/);
  assert.equal(h.asked.length, 0);
});
