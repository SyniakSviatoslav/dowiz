import { test, expect } from '@playwright/test';
import fs from 'node:fs';

/**
 * Capture the two mobile surfaces the main harness can't reach without a live order:
 * courier ACTIVE DELIVERY (/courier/delivery/:id) and order TRACKING (/s/:slug/order/:id).
 * Seeds an order via /dev/seed-visual-state, assigns it to a fresh mock courier, captures at 390px.
 *
 * Run: CAPTURE=1 VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
 *   CAPTURE_DIR=audit/mobile-polish-i3 pnpm exec playwright test e2e/tests/capture-delivery.spec.ts --project=desktop
 */
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
const SECRET = process.env.DEV_AUTH_SECRET || 'stg-e2e-secret';
const DIR = process.env.CAPTURE_DIR || '/root/dowiz/audit/mobile-polish-i3';
test.skip(!process.env.CAPTURE, 'set CAPTURE=1 to capture');
test.setTimeout(120_000);

test('capture courier delivery + tracking (390px)', async ({ page, request }) => {
  fs.mkdirSync(DIR, { recursive: true });
  const hdr = { 'x-dev-auth-secret': SECRET };
  await page.setViewportSize({ width: 390, height: 844 });

  // 1. Seed an order (+ its venue) and grab ids.
  const seedRes = await request.post(`${BASE}/api/dev/seed-visual-state`, { headers: hdr, data: {} });
  expect(seedRes.ok(), `seed failed ${seedRes.status()}`).toBeTruthy();
  const seed = await seedRes.json();
  const { orderId, slug, locationId } = seed;

  // 2. Fresh courier token (random courierId) bound to the seeded venue.
  const cRes = await request.post(`${BASE}/dev/mock-auth`, { headers: hdr, data: { role: 'courier', locationId } });
  const courier = await cRes.json();

  // 3. Assign the seeded order to THIS courier so the delivery view renders. The delivery route
  //    param is the ASSIGNMENT id (DeliveryPage fetches /courier/assignments/:id), not the order id.
  const asgnRes = await request.post(`${BASE}/dev/create-assignment`, { headers: hdr, data: { orderId, courierId: courier.userId, locationId } });
  const { assignmentId } = await asgnRes.json();

  const shot = async (name: string, token: string | undefined, path: string) => {
    await page.addInitScript((tk: any) => { if (tk) localStorage.setItem('dos_access_token', tk); else localStorage.removeItem('dos_access_token'); localStorage.setItem('dos_locale', 'sq'); }, token);
    await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' }).catch(() => {});
    await page.evaluate(() => (document as any).fonts?.ready).catch(() => {});
    await page.waitForTimeout(1600);
    await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: true }).catch(() => {});
  };

  // Courier active delivery (the safety-critical one) — route param is the assignment id.
  await shot('m-courier-delivery', courier.access_token, `/courier/delivery/${assignmentId}`);
  // Order tracking (best-effort: no customer token → likely the auth-expired state, itself worth grading).
  await shot('m-client-tracking', undefined, `/s/${slug || 'visual-open'}/order/${orderId}`);

  console.log('CAPTURED delivery+tracking; orderId', orderId, 'slug', slug);
  expect(fs.existsSync(`${DIR}/m-courier-delivery.png`)).toBeTruthy();
});
