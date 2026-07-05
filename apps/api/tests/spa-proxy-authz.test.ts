import './_env-stub.js';
import test from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import { signAuthToken } from '@deliveryos/platform';
import spaProxyRoutes from '../src/routes/spa-proxy.js';
import { registerReplySendError } from '../src/lib/reply-send-error.js';

// #6 (security-hardening-2026-07 / ADR-0004): the spa-proxy owner resolvers must not trust the
// baked JWT `activeLocationId` — they must live-recheck an ACTIVE owner membership on every
// request (the canonical get-owner-location.ts pattern). A removed/downgraded owner holding a
// still-valid ≤24h token must lose access IMMEDIATELY, not at token TTL.
//
// RED (pre-fix): getLocationId returned `claims.activeLocationId` directly → the revoked-owner
// case below returned 200. GREEN (post-fix): the live membership SELECT returns 0 rows → 401.

const OWNER = '11111111-1111-1111-1111-111111111111';
const LOC = '22222222-2222-2222-2222-222222222222';

function makeDb({ membershipActive }: { membershipActive: boolean }) {
  return {
    async query(sql: string, params: any[]) {
      // The live active-membership re-check (the load-bearing #6 query).
      if (/FROM\s+memberships/i.test(sql)) {
        return membershipActive
          ? { rows: [{ location_id: params[1] ?? LOC }], rowCount: 1 }
          : { rows: [], rowCount: 0 };
      }
      // The settings write itself (only reached for an authorized owner).
      if (/UPDATE\s+locations/i.test(sql)) {
        return {
          rows: [{
            id: LOC, slug: 'demo', name: 'Demo', phone: '',
            delivery_fee_flat: 0, min_order_value: 0, free_delivery_threshold: 0,
            delivery_radius_km: 0, tax_rate: 0, lat: null, lng: null,
            address: '', hours_json: {}, delivery_paused: false,
          }],
          rowCount: 1,
        };
      }
      return { rows: [], rowCount: 0 };
    },
  } as any;
}

function makeApp(db: any) {
  const app = Fastify();
  registerReplySendError(app);
  app.register(spaProxyRoutes, { db, storage: {} });
  return app;
}

async function putSettings(app: any, token: string) {
  return app.inject({
    method: 'PUT',
    url: '/api/owner/settings',
    headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json' },
    payload: { locationName: 'Demo' },
  });
}

test('#6 spa-proxy: revoked/downgraded owner is DENIED despite a valid baked-location token', async () => {
  // Token is cryptographically valid and carries a baked activeLocationId…
  const token = await signAuthToken({ role: 'owner', userId: OWNER, activeLocationId: LOC } as any, '24h');
  // …but the live membership is no longer active → must be denied at the resolver.
  const app = makeApp(makeDb({ membershipActive: false }));
  const res = await putSettings(app, token);
  assert.equal(res.statusCode, 401, 'a revoked owner must be denied immediately, not at token TTL');
});

test('#6 spa-proxy: active owner with the same baked-location token still succeeds', async () => {
  const token = await signAuthToken({ role: 'owner', userId: OWNER, activeLocationId: LOC } as any, '24h');
  const app = makeApp(makeDb({ membershipActive: true }));
  const res = await putSettings(app, token);
  assert.equal(res.statusCode, 200, 'a legitimate active owner must be unaffected by the recheck');
});
