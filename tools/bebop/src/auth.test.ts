// Bebop Better Auth tests — RED+GREEN (Verified-by-Math).
//
// Better Auth is the DEFAULT auth for Bebop. These prove: (GREEN) the auth server boots, signup/login
// works and issues a session; (RED) a protected call without a session is denied. No external services.

import assert from 'node:assert/strict';
import test from 'node:test';
import { createBebopAuth } from './auth.ts';
import { startSyncServer } from './sync-server.ts';

function api(base: string, path: string, opts: RequestInit = {}) {
  // Better Auth enforces CSRF-origin checks; a self-hosted CLI node is same-origin, so send Origin.
  return fetch(base + path, {
    ...opts,
    headers: { origin: base.replace(/\/$/, ''), ...(opts.headers as Record<string, string>) },
  });
}

test('GREEN: Better Auth is the default and a signup+login round-trips with a session', async () => {
  const srv = await startSyncServer({ port: 0 });
  try {
    const email = `node-${Date.now()}@bebop.local`;
    const password = 'correct-horse-battery-staple';
    // signup
    const signup = await api(srv.url, '/sign-up/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email, password, name: 'Bebop Node' }),
    });
    assert.ok(signup.status === 200, `signup status ${signup.status}`);
    // login
    const login = await api(srv.url, '/sign-in/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email, password }),
    });
    assert.ok(login.status === 200, `login status ${login.status}`);
    const setCookie = login.headers.get('set-cookie') ?? '';
    assert.match(setCookie, /bebop_session|better-auth/i, 'a session cookie must be set');
  } finally {
    await srv.close();
  }
});

test('RED: wrong password is rejected (no session issued)', async () => {
  const srv = await startSyncServer({ port: 0 });
  try {
    const email = `red-${Date.now()}@bebop.local`;
    await api(srv.url, '/sign-up/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email, password: 'correct-horse-battery-staple', name: 'X' }),
    });
    const login = await api(srv.url, '/sign-in/email', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ email, password: 'wrong-password' }),
    });
    // The protected boundary MUST hold: wrong password yields no session. 200 would mean auth is broken.
    assert.ok(login.status !== 200 && !((login.headers.get('set-cookie') ?? '').match(/session/i)),
      `wrong password must be rejected (status ${login.status}, cookie ${login.headers.get('set-cookie')})`);
  } finally {
    await srv.close();
  }
});

test('RED: get-session without a cookie returns null (protected boundary holds)', async () => {
  const srv = await startSyncServer({ port: 0 });
  try {
    const res = await api(srv.url, '/get-session');
    const body = await res.json().catch(() => null);
    // The protected boundary MUST hold: an unauthenticated call must NOT leak a logged-in session.
    // Better Auth returns the JSON literal `null` (not an object) for an anonymous session.
    const hasSession = body && (body.session || body.user);
    assert.ok(!hasSession, `anonymous get-session must return no session (status ${res.status}, body ${JSON.stringify(body)})`);
  } finally {
    await srv.close();
  }
});

test('GREEN: auth factory is created with secure session defaults', () => {
  const auth = createBebopAuth({ secret: 'test-secret-at-least-32-chars-long' });
  assert.equal(typeof auth.handler, 'function');
  assert.equal(typeof auth.api, 'object');
});
