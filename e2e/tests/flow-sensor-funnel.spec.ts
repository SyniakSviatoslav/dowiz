import { test, expect, type APIRequestContext } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

// Mutating spec — the valid path writes a real funnel_events row — so guard the target (Test
// Integrity §6: never write from a test against prod). Default to staging when VITE_BASE_URL is unset.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
test.beforeAll(() => requireStaging(BASE));

/**
 * SENSOR-BUS §1.3 runtime proof (ADR-0009) — the anonymous storefront-funnel ingest.
 * Run: VITE_BASE_URL=https://dowiz-staging.fly.dev pnpm exec playwright test flow-sensor-funnel --reporter=list
 *
 * The ingest is public (no auth) and uniform: it returns 204 on EVERY path (valid, invalid, kill-
 * switched) so it never reveals whether a payload was accepted (anti-enumeration) and never surfaces
 * an error to the storefront (observe-don't-control). These assertions prove the contract; the
 * definitive "the row landed in funnel_events under FORCE-RLS via app.current_tenant" is proven
 * out-of-band via psql against the staging DB and recorded in the commit.
 */

async function demoLocationId(request: APIRequestContext): Promise<string> {
  const menu = await request.get('/public/locations/demo/menu', { headers: { accept: 'application/json' } });
  expect(menu.ok(), 'demo menu loads').toBeTruthy();
  const m = await menu.json();
  const id = m.locationId ?? m.location_id;
  expectUuid(id, 'demo locationId');
  return id;
}

test('valid funnel event is accepted with a uniform 204', async ({ request }) => {
  const locationId = await demoLocationId(request);
  const res = await request.post('/api/funnel', {
    data: {
      locationId,
      sessionRef: `e2e-funnel-${Date.now()}`,
      eventType: 'checkout_abandon',
      shownEtaLoMin: 25,
      shownEtaHiMin: 40,
    },
  });
  expect(res.status(), `funnel ingest: ${await res.text()}`).toBe(204);
  // TODO(needs-staging): the 204 above proves the contract surface only — the DB-write path
  // (funnel.ts:57-76) is unobservable via any API (RLS FORCE + REVOKE keep funnel_events off the
  // Data API), so "the row landed in funnel_events under app.current_tenant" must be proven by psql
  // against the staging DB and recorded in the commit. A broken INSERT still returns 204 here.
});

test('malformed funnel payload still returns a uniform 204 (anti-enumeration, never an error)', async ({ request }) => {
  // Missing required fields + a bogus event_type — the ingest must NOT 400/500; it drops silently.
  const res = await request.post('/api/funnel', { data: { eventType: 'not_a_real_event', foo: 'bar' } });
  expect(res.status(), 'malformed funnel payload is a uniform 204').toBe(204);
});

test('an unknown locationId is dropped silently as 204 (FK-validated server-side, never an error)', async ({ request }) => {
  const res = await request.post('/api/funnel', {
    data: {
      locationId: '00000000-0000-0000-0000-000000000000',
      sessionRef: `e2e-funnel-badloc-${Date.now()}`,
      eventType: 'menu_view',
    },
  });
  expect(res.status(), 'unknown-location funnel ingest is a uniform 204').toBe(204);
});

// TODO(needs-staging, 2nd-tenant): the nil-UUID test above only exercises the FK-reject arm. A real
// cross-tenant isolation proof needs a SECOND real tenant (Test Integrity §5 bans the all-zero id as
// IDOR proof): post a valid locationId owned by tenant A, then assert tenant B cannot read tenant A's
// funnel_events via ANY surface (403/empty under FORCE-RLS + app.current_tenant). Requires a seeded
// second tenant + an owner token — staging-only; do not fake with a synthetic id.

test('a flood beyond the per-IP rate limit still returns a uniform 204, never a 429 (anti-enumeration)', async ({ request }) => {
  // The contract (funnel.ts:8) is "204 on EVERY path". The per-IP limiter (funnel.ts:38-41, 60/min)
  // is generous for one session but lethal to a flood — yet a 429 on overflow would leak that the
  // limiter tripped and break anti-enumeration. Fire >60 malformed (no DB write) and assert each
  // stays 204. TODO(needs-staging): only meaningful against a deployed instance whose Fly-Client-IP
  // keyed limiter is active; a 429 here is a real finding to escalate, not a test to weaken.
  for (let i = 0; i < 65; i++) {
    const res = await request.post('/api/funnel', { data: { eventType: 'not_a_real_event' } });
    expect(res.status(), `flood request ${i + 1} must stay a uniform 204 (not 429)`).toBe(204);
  }
});
