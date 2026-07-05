import { test, expect, type APIRequestContext } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

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
