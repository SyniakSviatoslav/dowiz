import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import { registerAdminPlaneGate, requirePlatformAdmin } from '../src/lib/platform-admin.js';

// Integration proof (ADR-admin-platform-authz DoD #4c/#4d/#4e): the REAL root-instance gate +
// requirePlatformAdmin, against live Fastify, gating a CHILD (via the parent plane), a SIBLING
// registered OUTSIDE the parent, and a lookalike — with a mock platform_admins allowlist.

const ADMINS = new Set(['admin-1']);

function buildApp() {
  const app = Fastify();
  // mock pool: the platform_admins point-read
  const pool: any = {
    async query(_sql: string, params?: any[]) {
      const uid = params?.[0];
      return { rowCount: ADMINS.has(uid) ? 1 : 0 };
    },
  };
  app.decorate('db', pool);
  // stub verifyAuth: 'x-test-user' header → request.user = {userId, role}; absent → 401 (mirrors real).
  app.decorate('verifyAuth', async (req: any, reply: any) => {
    const u = req.headers['x-test-user'];
    if (!u) { reply.status(401).send({ error: 'Unauthorized' }); return; }
    req.user = { userId: String(u), role: 'owner' };
  });
  app.decorateRequest('user', null);

  registerAdminPlaneGate(app); // the structural authority, BEFORE route registration

  // parent plane (mirrors routes/admin/index.ts shape) with a child route
  app.register(async (plane) => {
    plane.addHook('onRequest', (plane as any).verifyAuth);
    plane.addHook('onRequest', requirePlatformAdmin);
    plane.get('/backups', async () => ({ ok: 'child' }));
  }, { prefix: '/api/admin' });

  // SIBLING registered OUTSIDE the parent (the F1/RA2-5 hole) — NO own hooks
  app.register(async (evil) => {
    evil.get('/metrics', async () => ({ secret: 'cross-tenant' }));
  }, { prefix: '/api/admin' });

  // non-admin + lookalike (must NOT be gated)
  app.get('/api/orders', async () => ({ ok: 'orders' }));
  app.get('/api/administrators', async () => ({ ok: 'lookalike' }));
  return app;
}

async function hit(app: any, url: string, user?: string) {
  const headers = user ? { 'x-test-user': user } : {};
  const r = await app.inject({ method: 'GET', url, headers });
  return r.statusCode;
}

test('child route: owner-not-admin → 403, platform-admin → 200, no-token → 401', async () => {
  const app = buildApp();
  assert.equal(await hit(app, '/api/admin/backups', 'owner-x'), 403);
  assert.equal(await hit(app, '/api/admin/backups', 'admin-1'), 200);
  assert.equal(await hit(app, '/api/admin/backups'), 401);
  await app.close();
});

test('SIBLING route outside the parent is STILL gated by the root hook (the F1/RA2-5 closure)', async () => {
  const app = buildApp();
  assert.equal(await hit(app, '/api/admin/metrics', 'owner-x'), 403, 'ungated sibling → 403 by construction');
  assert.equal(await hit(app, '/api/admin/metrics', 'admin-1'), 200);
  assert.equal(await hit(app, '/api/admin/metrics'), 401);
  await app.close();
});

test('non-admin + lookalike are NOT gated (zero false-positive)', async () => {
  const app = buildApp();
  assert.equal(await hit(app, '/api/orders', 'owner-x'), 200);
  assert.equal(await hit(app, '/api/administrators', 'owner-x'), 200, '/api/administrators must not be gated');
  await app.close();
});

// NOTE — the double-prefix regression (adminPlane must strip the inherited `prefix` before registering
// children, else they mount at /api/admin/api/admin/* and the real paths 404) is covered by the staging
// E2E (e2e/tests/admin-platform-authz.spec.ts), which caught it live; a unit test can't import the real
// adminPlane (its children pull in env-requiring modules, same gate as websocket-churn).

test('revocation takes effect at request-entry (remove from allowlist → next request 403)', async () => {
  const app = buildApp();
  ADMINS.add('temp-admin');
  assert.equal(await hit(app, '/api/admin/backups', 'temp-admin'), 200);
  ADMINS.delete('temp-admin'); // == setting revoked_at
  assert.equal(await hit(app, '/api/admin/backups', 'temp-admin'), 403, 'revoked → denied next request');
  await app.close();
});
