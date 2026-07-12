/**
 * Full Settings lifecycle E2E — serial, UI-first
 *
 *  1. API: Read and record baseline settings
 *  2. UI: /admin/settings loads, form renders without JS errors
 *  3. UI: Form fields are editable — fill name and address, verify input values
 *  4. API: PUT new name, address, phone, lat/lng, hours_json
 *         → proves the save endpoint accepts all fields correctly
 *  5. API: GET settings → confirm all fields persisted
 *  6. UI:  Reload /admin/settings → name and address visible in form inputs
 *  7. UI:  Attempt form submit with filled-in values — verify no JS crash
 *  8. UI:  Delivery pause toggle — toggle ON then OFF, verify API reflects state
 *  9. API: Set permanent final values (Dubin & Sushi, Durrës, 10:00-22:00 hours)
 *
 * Map coordinates source: https://maps.app.goo.gl/9Fc9YR4UiK7d8hRw5
 *   → 41.315347, 19.4449964
 */
import { test, expect } from '@playwright/test';
<<<<<<< Updated upstream
=======
import { expectJwt } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';
>>>>>>> Stashed changes

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

const FINAL_NAME = 'Dubin & Sushi';
const FINAL_ADDRESS = 'Rruga Sulejman Kadiu, Durrës';
const FINAL_PHONE = '+355683085694';
const FINAL_LAT = 41.315347;
const FINAL_LNG = 19.4449964;

const FINAL_HOURS: Record<string, { isOpen: boolean; open: string; close: string }> = {
  monday:    { isOpen: true, open: '10:00', close: '22:00' },
  tuesday:   { isOpen: true, open: '10:00', close: '22:00' },
  wednesday: { isOpen: true, open: '10:00', close: '22:00' },
  thursday:  { isOpen: true, open: '10:00', close: '22:00' },
  friday:    { isOpen: true, open: '10:00', close: '22:00' },
  saturday:  { isOpen: true, open: '10:00', close: '22:00' },
  sunday:    { isOpen: true, open: '10:00', close: '22:00' },
};

let ownerToken: string;
let baselineSettings: Record<string, any>;

test.describe.configure({ mode: 'serial' });

test.describe('UI: Admin Settings — name, address, phone, hours, delivery toggle', () => {

  test.beforeAll(async ({ request }) => {
    // This spec MUTATES settings and uses the /dev/mock-auth backdoor — refuse to run
    // against prod (where the backdoor 404s anyway, and a write would corrupt prod data).
    requireStaging(BASE);
    const r = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(r.status()).toBe(200);
    const body = await r.json();
    ownerToken = body.access_token;
    expect(ownerToken).toBeTruthy();
  });

  // ─── STEP 1: Record baseline ──────────────────────────────────────────────────
  test('Step 1: API — GET /owner/settings returns 200 and valid shape', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(r.status(), 'Settings endpoint must return 200').toBe(200);
    baselineSettings = await r.json();
    expect(
      baselineSettings.locationName ?? baselineSettings.name,
      'Settings must include a location name'
    ).toBeTruthy();
    console.log('Baseline:', {
      name: baselineSettings.locationName,
      address: baselineSettings.address,
      lat: baselineSettings.lat,
      lng: baselineSettings.lng,
      phone: baselineSettings.phone,
      deliveryPaused: baselineSettings.deliveryPaused,
    });
  });

  // ─── STEP 2: Page loads without JS errors ────────────────────────────────────
  test('Step 2: UI — /admin/settings loads, form is visible, no JS errors', async ({ page }) => {
    const jsErrors: string[] = [];
    page.on('pageerror', err => jsErrors.push(err.message));

    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    const nameInput = page.locator('#settings-locationName');
    await nameInput.waitFor({ state: 'visible', timeout: 10000 });

    const currentName = await nameInput.inputValue();
    expect(currentName.length, 'Name input must have a value').toBeGreaterThan(0);

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('ResizeObserver') && !e.includes('Non-Error')
    );
    expect(critical, `JS errors on settings page: ${critical.join('; ')}`).toEqual([]);
    console.log('Settings page loaded. Current name:', currentName);
  });

  // ─── STEP 3: Form fields are editable ────────────────────────────────────────
  test('Step 3: UI — name and address inputs accept new values', async ({ page }) => {
    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    const nameInput = page.locator('#settings-locationName');
    await nameInput.waitFor({ state: 'visible', timeout: 10000 });
    await nameInput.fill(FINAL_NAME);
    expect(await nameInput.inputValue()).toBe(FINAL_NAME);

    const addressInput = page.locator('#settings-address');
    await addressInput.fill(FINAL_ADDRESS);
    expect(await addressInput.inputValue()).toBe(FINAL_ADDRESS);

    const phoneInput = page.locator('#settings-phone');
    if (await phoneInput.isVisible({ timeout: 3000 }).catch(() => false)) {
      await phoneInput.fill(FINAL_PHONE);
      expect(await phoneInput.inputValue()).toBe(FINAL_PHONE);
    }

    const hasMap = await page.locator('canvas').first().isVisible({ timeout: 5000 }).catch(() => false);
    console.log('Fields editable — name:', FINAL_NAME, '| phone:', FINAL_PHONE, '| map canvas:', hasMap);
  });

  // ─── STEP 4: API save — name, address, phone, lat/lng, hours ─────────────────
  test('Step 4: API — PUT settings with new name, address, phone, lat/lng, hoursJson', async ({ request }) => {
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: {
        locationName: FINAL_NAME,
        address: FINAL_ADDRESS,
        phone: FINAL_PHONE,
        lat: FINAL_LAT,
        lng: FINAL_LNG,
        hoursJson: FINAL_HOURS,
      },
    });
    expect(r.status(), `PUT /owner/settings must return 200 — got: ${r.status()}`).toBe(200);
    const body = await r.json();
    expect(body.locationName).toBe(FINAL_NAME);
    expect(body.address).toBe(FINAL_ADDRESS);
    console.log('Saved via API — name:', body.locationName, '| lat:', body.lat, '| lng:', body.lng);
  });

  // ─── STEP 5: API verify all fields persisted ─────────────────────────────────
  test('Step 5: API — GET /owner/settings confirms name, address, phone, and map pin saved', async ({ request }) => {
    const r = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();

    expect(body.locationName, 'Location name must be "Dubin & Sushi"').toBe(FINAL_NAME);
    expect(body.address, 'Address must be the Durrës street').toBe(FINAL_ADDRESS);
    expect(body.phone, 'Phone must be updated').toBe(FINAL_PHONE);
    expect(Number(body.lat)).toBeCloseTo(FINAL_LAT, 4);
    expect(Number(body.lng)).toBeCloseTo(FINAL_LNG, 4);
    const hasHours = body.hoursJson && typeof body.hoursJson === 'object';
    expect(hasHours, 'hoursJson must be an object').toBe(true);

    console.log('Confirmed — name:', body.locationName, '| phone:', body.phone, '| lat:', body.lat, '| hoursJson set:', hasHours);
  });

  // ─── STEP 5b: Auth — PUT /owner/settings is gated (negative + positive controls) ─
  test('Step 5b: API — PUT /owner/settings rejects unauthenticated and wrong-role callers', async ({ request }) => {
    // Negative control: no Authorization header → 401 (global auth hook, server.ts:397-401).
    const noAuth = await request.put(`${BASE}/api/owner/settings`, { data: { locationName: 'HACK' } });
    expect(noAuth.status(), 'Unauthenticated PUT must be 401').toBe(401);

    // Wrong-role: a courier token passes the auth hook but getLocationId() returns null for
    // role!=='owner', so the route returns 401 (no separate 403 path — spa-proxy.ts:660-662).
    const courierRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courierRes.status(), 'mock-auth must mint a courier token on staging').toBe(200);
    const courierToken = (await courierRes.json()).access_token;
    expectJwt(courierToken, 'courier token');
    const wrongRole = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${courierToken}` },
      data: { locationName: 'HACK' },
    });
    expect(wrongRole.status(), 'Courier-role PUT must be rejected (401)').toBe(401);

    // Positive control: the owner token still saves (200) — proves the gate is not rejecting everyone.
    const owner = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { locationName: baselineSettings.locationName ?? FINAL_NAME },
    });
    expect(owner.status(), 'Owner PUT must succeed (200)').toBe(200);
  });

  // ─── STEP 6: UI — reload page and verify form displays saved values ───────────
  test('Step 6: UI — reload /admin/settings shows updated name and address', async ({ page }) => {
    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(3000);

    const nameInput = page.locator('#settings-locationName');
    await nameInput.waitFor({ state: 'visible', timeout: 10000 });

    const displayedName = await nameInput.inputValue();
    const displayedAddress = await page.locator('#settings-address').inputValue();

    expect(displayedName, 'Form must display "Dubin & Sushi" after reload').toBe(FINAL_NAME);
    expect(displayedAddress, 'Form must display Durrës address after reload').toBe(FINAL_ADDRESS);

    console.log('UI displays — name:', displayedName, '| address:', displayedAddress);
  });

  // ─── STEP 7: UI — form submit cycle ──────────────────────────────────────────
  test('Step 7: UI — form submit with valid phone does not crash the page', async ({ page }) => {
    const jsErrors: string[] = [];
    page.on('pageerror', err => jsErrors.push(err.message));

    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    const phoneInput = page.locator('#settings-phone');
    const currentPhone = await phoneInput.inputValue();
    if (!/^\+\d{7,15}$/.test(currentPhone)) {
      await phoneInput.fill(FINAL_PHONE);
    }

    const saveBtn = page.locator('button[type="submit"]').first();
    await saveBtn.waitFor({ state: 'visible', timeout: 5000 });

    // Finding #1: intercept the save PUT so a silent 4xx (toast never renders) FAILS the test
    // instead of passing invisibly. The save handler calls PUT /owner/settings.
    const [putResp] = await Promise.all([
      page.waitForResponse(
        r => r.url().includes('/api/owner/settings') && r.request().method() === 'PUT',
        { timeout: 15000 },
      ),
      saveBtn.click(),
    ]);
    expect(putResp.status(), 'Save PUT /owner/settings must return 200').toBe(200);

    // Assert the success confirmation actually renders (previously only console.log'd).
    await expect(
      page.getByTestId('settings-saved'),
      'Success confirmation must be visible after a successful save',
    ).toBeVisible({ timeout: 5000 });

    const critical = jsErrors.filter(e =>
      !e.includes('favicon') && !e.includes('ResizeObserver') && !e.includes('Non-Error')
    );
    expect(critical, `JS errors after form submit: ${critical.join('; ')}`).toEqual([]);
  });

  // ─── STEP 8: UI — delivery pause toggle ──────────────────────────────────────
  test('Step 8: UI — delivery pause toggle can be turned on and off', async ({ page, request }) => {
    await page.goto(`${BASE}/admin`, { waitUntil: 'load', timeout: 30000 });
    await page.evaluate((token) => { localStorage.setItem('dos_access_token', token); }, ownerToken);
    await page.goto(`${BASE}/admin/settings`, { waitUntil: 'networkidle', timeout: 30000 });
    await page.waitForTimeout(2000);

    // Finding #2: the delivery status toggle MUST be present — a missing toggle used to
    // silently `return` and pass the test. The Toggle component renders role="switch" with
    // an accessible name of "Open"/"Closed" (packages/ui/src/components/admin/Toggle.tsx).
    const toggle = page.getByRole('switch', { name: /open|closed/i }).first();
    await expect(
      toggle,
      'Delivery status toggle must be present on the settings page',
    ).toBeVisible({ timeout: 8000 });

    // Read initial pause state from API
    const beforeGet = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    const before = await beforeGet.json();
    console.log('Before toggle — deliveryPaused:', before.deliveryPaused);

    // Toggle to paused via API (UI toggle calls PUT immediately)
    const pauseRes = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: true },
    });
    expect(pauseRes.status()).toBe(200);
    const paused = await pauseRes.json();
    expect(paused.deliveryPaused, 'deliveryPaused must be true after pausing').toBe(true);
    console.log('Paused delivery:', paused.deliveryPaused);

    // Verify public info reflects paused state
    const infoRes = await request.get(`${BASE}/public/locations/sushi-durres/info`);
    if (infoRes.status() === 200) {
      const info = await infoRes.json();
      expect(info.isOpen, 'isOpen must be false when delivery is paused').toBe(false);
      console.log('Public info isOpen after pause:', info.isOpen);
    }

    // Resume delivery
    const resumeRes = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false },
    });
    expect(resumeRes.status()).toBe(200);
    const resumed = await resumeRes.json();
    expect(resumed.deliveryPaused, 'deliveryPaused must be false after resuming').toBe(false);
    console.log('Resumed delivery:', resumed.deliveryPaused);

    console.log('Step 8 PASS — delivery pause toggle works correctly');
  });

  // ─── STEP 9: Set permanent final values ──────────────────────────────────────
  test('Step 9: API — permanently set Dubin & Sushi as final location data', async ({ request }) => {
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: {
        locationName: FINAL_NAME,
        address: FINAL_ADDRESS,
        phone: FINAL_PHONE,
        lat: FINAL_LAT,
        lng: FINAL_LNG,
        hoursJson: FINAL_HOURS,
        deliveryPaused: false,
      },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();

    expect(body.locationName).toBe(FINAL_NAME);
    expect(body.address).toBe(FINAL_ADDRESS);
    expect(body.phone).toBe(FINAL_PHONE);
    expect(body.deliveryPaused).toBe(false);

    // Verify public info shows correct location center
    const infoRes = await request.get(`${BASE}/public/locations/sushi-durres/info`);
    expect(infoRes.status(), 'Public info endpoint must return 200').toBe(200);
    const info = await infoRes.json();
    expect(Number(info.lat)).toBeCloseTo(FINAL_LAT, 3);
    expect(Number(info.lng)).toBeCloseTo(FINAL_LNG, 3);

    console.log('Step 9 PASS — permanent data set:', {
      name: body.locationName,
      phone: body.phone,
      address: body.address,
      lat: info.lat,
      lng: info.lng,
    });
  });
});
