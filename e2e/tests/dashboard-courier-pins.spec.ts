import { test, expect, type APIRequestContext } from '@playwright/test';

// Proof that the admin dashboard's live map renders REAL courier pins (one marker
// per on-shift courier with a known position), not the old hardcoded cu1/cu2
// fixtures. Seeds 2 couriers with distinct GPS positions via the dev endpoints,
// then loads the owner dashboard and asserts their markers render on the map.
//
// Requires a deployment carrying the live-map fix + DEV_AUTH_SECRET (injected by
// playwright.config.ts). Run: pnpm exec playwright test dashboard-courier-pins --reporter=list

async function authedJson(request: APIRequestContext, path: string, token: string) {
  const r = await request.get(path, { headers: { authorization: `Bearer ${token}` } });
  return r.ok() ? r.json() : null;
}

test('admin dashboard renders a real pin per on-shift courier', async ({ page, request }) => {
  // 1. Owner + location.
  const ownerRes = await request.post('/api/dev/mock-auth', { data: {} });
  test.skip(!ownerRes.ok(), 'mock-auth unavailable (no DEV_AUTH_SECRET on target)');
  const owner = await ownerRes.json();
  const LOC = owner.activeLocationId;
  expect(LOC, 'owner must resolve a location').toBeTruthy();

  const settings = await authedJson(request, '/api/owner/settings', owner.access_token);
  const baseLat = Number.isFinite(settings?.lat) ? settings.lat : 41.331;
  const baseLng = Number.isFinite(settings?.lng) ? settings.lng : 19.817;

  // 2. Two couriers with distinct positions.
  const couriers: Array<{ id: string; token: string; lat: number; lng: number }> = [];
  for (let i = 0; i < 2; i++) {
    const c = await (await request.post('/api/dev/mock-auth', { data: { role: 'courier' } })).json();
    couriers.push({ id: c.userId, token: c.access_token, lat: +(baseLat + 0.001 * (i + 1)).toFixed(6), lng: +(baseLng + 0.001 * (i + 1)).toFixed(6) });
  }

  // 3. Assign each to an existing order (seeds courier row + available shift), then ping.
  const orders = await authedJson(request, '/api/owner/orders', owner.access_token);
  const orderIds = (Array.isArray(orders) ? orders : []).slice(0, couriers.length).map((o: any) => o.id);
  test.skip(orderIds.length < couriers.length, 'not enough seed orders to assign couriers');

  for (let i = 0; i < couriers.length; i++) {
    await request.post('/api/dev/create-assignment', { data: { orderId: orderIds[i], courierId: couriers[i].id, locationId: LOC } });
  }
  for (const c of couriers) {
    const ping = await request.post('/api/courier/shifts/ping', { headers: { authorization: `Bearer ${c.token}` }, data: { lat: c.lat, lng: c.lng } });
    expect(ping.status(), 'ping should be accepted').toBe(200);
  }

  // 4. Load the dashboard as the owner. /admin/orders renders DashboardPage directly
  //    (no activation redirect), and the page seeds positions from /couriers/live.
  await page.addInitScript((t) => {
    localStorage.setItem('dos_access_token', t);
    localStorage.setItem('dos_locale', 'en');
  }, owner.access_token);
  await page.goto('/admin/orders');
  await page.waitForLoadState('networkidle');

  // 5. The live map must render a marker for each seeded courier (real UUID ids),
  //    not the removed cu1/cu2 fixtures. Markers are added after the map style
  //    loads, so allow generous time.
  for (const c of couriers) {
    await expect(page.locator(`[data-marker-id="${c.id}"]`)).toHaveCount(1, { timeout: 25000 });
  }
  // And at least our two real couriers are on the map.
  expect(await page.getByTestId('map-marker').count()).toBeGreaterThanOrEqual(couriers.length);
});
