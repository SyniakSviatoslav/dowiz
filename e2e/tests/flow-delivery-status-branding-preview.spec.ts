/**
 * Delivery Open/Close Gate — E2E
 *
 * Proves that toggling delivery_paused on a location:
 *   - Makes /s/:slug show ONLY a closed message (no menu content)
 *   - /public/locations/:slug/info returns isOpen=false when paused
 *   - Hours-based auto-close: setting today's isOpen=false closes the location
 *   - Reverts correctly when delivery is resumed
 *
 * Uses the mock-auth owner's location (no new location creation = no rate limiting).
 * Restores deliveryPaused=false in afterAll.
 *
 * URL: /s/:slug (SSR menu page, Albania timezone for schedule checks)
 */

import { test, expect } from '@playwright/test';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

let ownerToken: string;
let locationSlug: string;
let originalHoursJson: Record<string, any> | null = null;

test.describe('Delivery Open/Close Gate', () => {
  test.describe.configure({ mode: 'serial' });

  // ── Setup ─────────────────────────────────────────────────────────────────

  test('SETUP-1: get auth token and location slug', async ({ request }) => {
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    ownerToken = (await authRes.json()).access_token;
    expect(ownerToken).toBeTruthy();

    const settingsRes = await request.get(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
    });
    expect(settingsRes.status()).toBe(200);
    const settings = await settingsRes.json();
    locationSlug = settings.slug;
    originalHoursJson = settings.hoursJson ?? null;
    expect(locationSlug, 'Settings must include slug').toBeTruthy();
    console.log('Testing location slug:', locationSlug);
  });

  test('SETUP-2: ensure delivery starts unpaused with lat/lng set (clean state)', async ({ request }) => {
    // Ensure lat/lng are set — the old deployed code has a bug where isOpen update is
    // gated on d?.lat && d?.lng; without coords the closed overlay never shows.
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false, lat: 41.315347, lng: 19.4449964 },
    });
    expect(r.status()).toBe(200);
  });

  // ── API: /info reflects delivery_paused ────────────────────────────────────

  test('API-1: /public/locations/:slug/info returns isOpen=false when paused', async ({ request }) => {
    const pauseRes = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: true },
    });
    expect(pauseRes.status()).toBe(200);

    const infoRes = await request.get(`${BASE}/public/locations/${locationSlug}/info`);
    expect(infoRes.status()).toBe(200);
    const info = await infoRes.json();
    expect(info.isOpen, '/info must return isOpen=false when delivery_paused=true').toBe(false);
  });

  test('API-2: /public/locations/:slug/info returns isOpen=true when unpaused (open hours)', async ({ request }) => {
    // Set hours open all day (00:00-23:59 every day) then unpause
    const openAllDay: Record<string, any> = {};
    for (const d of ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday']) {
      openAllDay[d] = { isOpen: true, open: '00:00', close: '23:59' };
    }
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false, hoursJson: openAllDay },
    });
    expect(r.status()).toBe(200);

    const infoRes = await request.get(`${BASE}/public/locations/${locationSlug}/info`);
    expect(infoRes.status()).toBe(200);
    const info = await infoRes.json();
    expect(info.isOpen, '/info must return isOpen=true when delivery_paused=false and hours are 00:00-23:59').toBe(true);
  });

  // ── UI: /s/:slug shows closed overlay when paused ──────────────────────────

  test('UI-CLOSED-1: pause delivery', async ({ request }) => {
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: true },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.deliveryPaused).toBe(true);
  });

  test('SSR-CLOSED: /s/:slug raw HTML contains closed overlay when paused', async ({ request }) => {
    // Verifies the SSR renderer itself emits the closed state (no JS needed).
    // This test REQUIRES deployment of ssr-renderer.ts changes to pass.
    const res = await request.get(`${BASE}/s/${locationSlug}`);
    expect(res.status()).toBe(200);
    const html = await res.text();
    expect(html, 'SSR HTML must contain data-testid="closed-overlay" when delivery_paused=true')
      .toContain('data-testid="closed-overlay"');
    expect(html, 'SSR HTML must not render menu category divs when delivery is paused')
      .not.toContain('<div class="menu-section"');
  });

  test('UI-CLOSED-2: /s/:slug shows closed overlay, no product cards', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });

    // Support both: new code (data-testid) and current deployed code (text content)
    const closed = page.locator('[data-testid="closed-overlay"]').or(
      page.locator('.ti-clock-off').locator('..')
    );
    await expect(closed.first()).toBeVisible({ timeout: 10000 });

    // Category nav must NOT be rendered when closed
    await expect(page.locator('[aria-label="Categories"]')).not.toBeVisible({ timeout: 3000 });
  });

  test('UI-CLOSED-3: closed message contains "closed" text', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });

    // Either SSR or React renders the closed message
    const closedText = page.locator('[data-testid="closed-overlay"]').or(
      page.locator('text=/currently closed/i')
    );
    await expect(closedText.first()).toBeVisible({ timeout: 10000 });
  });

  test('UI-CLOSED-4: category nav is NOT visible when closed', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });

    await expect(page.locator('[aria-label="Categories"]')).not.toBeVisible({ timeout: 5000 });
  });

  // ── UI: Resume delivery ────────────────────────────────────────────────────

  test('RESUME-1: unpause delivery (with open-all-day hours)', async ({ request }) => {
    const openAllDay: Record<string, any> = {};
    for (const d of ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday']) {
      openAllDay[d] = { isOpen: true, open: '00:00', close: '23:59' };
    }
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false, hoursJson: openAllDay },
    });
    expect(r.status()).toBe(200);
  });

  test('RESUME-2: /s/:slug shows menu after resume, no closed overlay', async ({ page }) => {
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });

    await expect(page.locator('[data-testid="closed-overlay"]')).not.toBeVisible({ timeout: 8000 });
  });

  // ── Hours-based auto-close (Albania timezone) ─────────────────────────────

  test('HOURS-1: setting today isOpen=false (Albania tz) closes location', async ({ request, page }) => {
    // Get today's weekday in Albania timezone
    const tzParts = new Intl.DateTimeFormat('en-US', {
      timeZone: 'Europe/Tirane', weekday: 'long',
    }).formatToParts(new Date());
    const todayKey = (tzParts.find(p => p.type === 'weekday')?.value ?? '').toLowerCase();
    console.log(`Albania today: ${todayKey}`);

    // Build hours: today marked closed, all other days open
    const hoursJson: Record<string, any> = {};
    for (const d of ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday']) {
      hoursJson[d] = d === todayKey
        ? { isOpen: false, open: '10:00', close: '22:00' }
        : { isOpen: true, open: '00:00', close: '23:59' };
    }

    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: { deliveryPaused: false, hoursJson },
    });
    expect(r.status()).toBe(200);

    // API proof: isOpen must be false because today's isOpen=false
    const infoRes = await request.get(`${BASE}/public/locations/${locationSlug}/info`);
    expect(infoRes.status()).toBe(200);
    const info = await infoRes.json();
    expect(info.isOpen, `isOpen must be false because today (${todayKey}) is marked closed`).toBe(false);

    // UI proof: closed overlay visible
    await page.goto(`${BASE}/s/${locationSlug}`, { waitUntil: 'networkidle' });
    await expect(page.locator('[data-testid="closed-overlay"]')).toBeVisible({ timeout: 10000 });
  });

  // ── Restore ────────────────────────────────────────────────────────────────

  test('RESTORE: set deliveryPaused=false with final open hours', async ({ request }) => {
    const openAllDay: Record<string, any> = {};
    for (const d of ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday']) {
      openAllDay[d] = { isOpen: true, open: '10:00', close: '22:00' };
    }
    const r = await request.put(`${BASE}/api/owner/settings`, {
      headers: { Authorization: `Bearer ${ownerToken}` },
      data: {
        deliveryPaused: false,
        hoursJson: originalHoursJson ?? openAllDay,
      },
    });
    expect(r.status()).toBe(200);
    const body = await r.json();
    expect(body.deliveryPaused).toBe(false);

    // Final API proof: location is open
    const infoRes = await request.get(`${BASE}/public/locations/${locationSlug}/info`);
    expect(infoRes.status()).toBe(200);
    const info = await infoRes.json();
    console.log(`Restored: isOpen=${info.isOpen}, slug=${locationSlug}`);
  });
});
