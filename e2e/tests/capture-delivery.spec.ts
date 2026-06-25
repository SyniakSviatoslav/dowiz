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
  // The seeded order lives on the `open` venue; ids are nested there (not top-level).
  const orderId = seed.orderId;
  const locationId = seed.open.locationId;
  const slug = seed.open.slug;

  // 2. Token for the SYNTHETIC seeded courier — mock-auth re-derives that one fixture by its sentinel
  //    email-hash (it never accepts a caller-supplied courierId). The seed already created an encrypted
  //    courier + shift + assignment for the seeded order, so the live delivery view renders.
  const cRes = await request.post(`${BASE}/dev/mock-auth`, { headers: hdr, data: { role: 'courier', synthetic: true, locationId } });
  const courier = await cRes.json();
  const assignmentId = seed.syntheticAssignmentId;

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
