// Rendered-pin proof for the dashboard live map. The Playwright spec
// (dashboard-courier-pins.spec.ts) proves the data-level fix in any browser
// (real couriers populate the map → no empty state); this complements it by
// asserting the ACTUAL MapLibre markers render, using the full `chromium` build
// (the test-runner's headless-shell lacks WebGL). Emits a screenshot as evidence.
//
// Usage: BASE=https://dowiz-staging.fly.dev SECRET=stg-e2e-secret node scripts/verify-courier-pins.mjs
import { chromium } from '@playwright/test';

const BASE = process.env.BASE || 'https://dowiz-staging.fly.dev';
const SECRET = process.env.SECRET || 'stg-e2e-secret';
const H = { 'content-type': 'application/json', 'x-dev-auth-secret': SECRET };
const j = async (r) => (r.ok ? r.json() : null);
const post = (p, body, auth) => fetch(`${BASE}${p}`, { method: 'POST', headers: auth ? { ...H, authorization: `Bearer ${auth}` } : H, body: JSON.stringify(body) });
const get = (p, auth) => fetch(`${BASE}${p}`, { headers: auth ? { ...H, authorization: `Bearer ${auth}` } : H });

const main = async () => {
  // Seed an owner + 2 couriers with distinct GPS, assign + ping them.
  const owner = await j(await post('/api/dev/mock-auth', {}));
  const LOC = owner.activeLocationId;
  const settings = await j(await get('/api/owner/settings', owner.access_token));
  const baseLat = Number.isFinite(settings?.lat) ? settings.lat : 41.331;
  const baseLng = Number.isFinite(settings?.lng) ? settings.lng : 19.817;

  const couriers = [];
  for (let i = 0; i < 2; i++) {
    const c = await j(await post('/api/dev/mock-auth', { role: 'courier' }));
    couriers.push({ id: c.userId, token: c.access_token, lat: +(baseLat + 0.001 * (i + 1)).toFixed(6), lng: +(baseLng + 0.001 * (i + 1)).toFixed(6) });
  }
  const orders = await j(await get('/api/owner/orders', owner.access_token));
  const orderIds = (Array.isArray(orders) ? orders : []).slice(0, couriers.length).map((o) => o.id);
  if (orderIds.length < couriers.length) { console.log('FAIL: not enough seed orders'); process.exit(1); }
  for (let i = 0; i < couriers.length; i++) await post('/api/dev/create-assignment', { orderId: orderIds[i], courierId: couriers[i].id, locationId: LOC });
  for (const c of couriers) {
    const r = await post('/api/courier/shifts/ping', { lat: c.lat, lng: c.lng }, c.token);
    if (r.status !== 200) { console.log(`FAIL: ping ${c.id} -> ${r.status}`); process.exit(1); }
  }

  // Full chromium build (WebGL-capable) renders MapLibre; the headless shell does not.
  const browser = await chromium.launch();
  const page = await (await browser.newContext({ viewport: { width: 1280, height: 900 } })).newPage();
  await page.addInitScript((t) => { localStorage.setItem('dos_access_token', t); localStorage.setItem('dos_locale', 'en'); }, owner.access_token);
  await page.goto(`${BASE}/admin/orders`, { waitUntil: 'networkidle' });
  await page.getByText('Couriers Live').scrollIntoViewIfNeeded();
  await page.getByText('Loading map...').waitFor({ state: 'hidden', timeout: 45000 });

  let allPresent = true;
  for (const c of couriers) {
    const n = await page.locator(`[data-marker-id="${c.id}"]`).count();
    const present = n === 1;
    allPresent = allPresent && present;
    console.log(`${present ? 'PASS' : 'FAIL'}: pin rendered for courier ${c.id} (count=${n})`);
  }
  const total = await page.locator('[data-testid="map-marker"]').count();
  const enough = total >= couriers.length;
  console.log(`${enough ? 'PASS' : 'FAIL'}: total map markers ${total} >= ${couriers.length}`);

  await page.screenshot({ path: 'qa-shots/dashboard-courier-pins.png' });
  console.log('screenshot: qa-shots/dashboard-courier-pins.png');
  await browser.close();
  process.exit(allPresent && enough ? 0 : 1);
};

main().catch((e) => { console.error('ERROR', e?.message); process.exit(1); });
