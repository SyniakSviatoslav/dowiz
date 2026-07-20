import { test, expect } from '@playwright/test';

/**
 * Seed bootstrap — Playwright Agents' starting position (Tooling Plan v2, Step 4).
 *
 * A deliberately "boring" fixture: it drives the app into a known logged-in,
 * seeded state (owner + courier via the dev mock-auth endpoint) and asserts the
 * owner lands on a live /admin surface. The Planner agent explores from here;
 * the Generator/Healer build real flows on top.
 *
 * No secrets live in this file — auth comes from the dev mock-auth endpoint and
 * the token is injected via localStorage (the app's `dos_access_token` key).
 * Runs against VITE_BASE_URL (defaults to prod, per the Mandatory Proof Rule).
 */

const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

type MockAuth = { access_token: string; userId: string; activeLocationId: string };

test.describe('seed: bootstrap to a logged-in, seeded state', () => {
  test('owner & courier obtain dev sessions with an active location', async ({ request }) => {
    const owner = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(owner.status(), 'owner mock-auth').toBe(200);
    const ownerBody = (await owner.json()) as MockAuth;
    expect(ownerBody.access_token, 'owner token').toBeTruthy();
    expect(ownerBody.activeLocationId, 'owner active location').toBeTruthy();

    const courier = await request.post(`${BASE}/api/dev/mock-auth`, { data: { role: 'courier' } });
    expect(courier.status(), 'courier mock-auth').toBe(200);
    const courierBody = (await courier.json()) as MockAuth;
    expect(courierBody.access_token, 'courier token').toBeTruthy();
  });

  test('owner lands on a live /admin surface', async ({ page, request }) => {
    const res = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(res.status()).toBe(200);
    const { access_token } = (await res.json()) as MockAuth;

    await page.addInitScript((t: string) => localStorage.setItem('dos_access_token', t), access_token);
    await page.goto(`${BASE}/admin`, { waitUntil: 'load' });

    await expect(page.locator('body')).toBeVisible();
    await expect(page.locator('body')).toContainText(
      /dashboard|orders|delivery|pending|confirmed|active/i,
      { timeout: 15000 },
    );
  });
});
