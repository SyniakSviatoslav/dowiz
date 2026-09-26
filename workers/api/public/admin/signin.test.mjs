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
