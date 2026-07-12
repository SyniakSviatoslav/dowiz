import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

/**
 * Capture the two mobile surfaces the main harness can't reach without a live order:
 * courier ACTIVE DELIVERY (/courier/delivery/:id) and order TRACKING (/s/:slug/order/:id).
 * Seeds an order via /dev/seed-visual-state, assigns it to a fresh mock courier, captures at 390px.
 *
 * Run: CAPTURE=1 VITE_BASE_URL=https://dowiz-staging.fly.dev DEV_AUTH_SECRET=stg-e2e-secret \
 *   CAPTURE_DIR=audit/mobile-polish-i3 pnpm exec playwright test e2e/tests/capture-delivery.spec.ts --project=desktop
 */
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
// No fallback literal: a missing secret must FAIL the run, not silently mint a dev token.
const SECRET = process.env.DEV_AUTH_SECRET ?? (() => { throw new Error('DEV_AUTH_SECRET must be set'); })();
const DIR = process.env.CAPTURE_DIR || '/root/dowiz/audit/mobile-polish-i3';
test.skip(!process.env.CAPTURE, 'set CAPTURE=1 to capture');
test.setTimeout(120_000);
// Mutating spec (seeds an order + courier + shift + assignment) — never hit prod.
test.beforeAll(() => requireStaging(BASE));

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
  expectUuid(orderId, 'seed.orderId');
  expectUuid(locationId, 'seed.open.locationId');

  // 2. Token for the SYNTHETIC seeded courier — mock-auth re-derives that one fixture by its sentinel
  //    email-hash (it never accepts a caller-supplied courierId). The seed already created an encrypted
  //    courier + shift + assignment for the seeded order, so the live delivery view renders.
  const cRes = await request.post(`${BASE}/dev/mock-auth`, { headers: hdr, data: { role: 'courier', synthetic: true, locationId } });
  expect(cRes.status(), `mock-auth failed ${cRes.status()}`).toBe(200);
  const courier = await cRes.json();
  expectJwt(courier.access_token, 'courier.access_token');
  const assignmentId = seed.syntheticAssignmentId;
  expectUuid(assignmentId, 'seed.syntheticAssignmentId');

  // Isolation + auth controls on the data the delivery view actually reads
  // (GET /api/courier/assignments/:id — WHERE ca.id=$1 AND ca.courier_id=token).
  // NEGATIVE: no token must be rejected (requireRole → 401), so the gate isn't open.
  const noAuthRes = await request.get(`${BASE}/api/courier/assignments/${assignmentId}`);
  expect(noAuthRes.status(), 'assignment endpoint must 401 without a token').toBe(401);
  // POSITIVE: the synthetic courier token reads back ITS OWN assignment (200, value-verified).
  const okRes = await request.get(`${BASE}/api/courier/assignments/${assignmentId}`, {
    headers: { Authorization: `Bearer ${courier.access_token}` },
  });
  expect(okRes.status(), 'synthetic courier must read its own assignment').toBe(200);
  const task = await okRes.json();
  expect(task.id, 'returned assignment id must match the seeded one').toBe(assignmentId);
  expect(task.orderId, 'returned order id must match the seeded one').toBe(orderId);
  // TODO(cross-tenant): a true IDOR negative needs a REAL second tenant's assignment id
  // (the synthetic token must get 404 — handler returns 404 when ca.courier_id != token).
  // A nil/all-zero UUID would 404 by absence and prove nothing, so it is deliberately NOT used.
  // Requires a live staging run that seeds a second tenant + assignment. See needs_staging.

  // `ready` (when given) is a real readiness signal: wait for a delivery-specific element to be
  // VISIBLE before shooting — replaces the magic-number sleep and asserts the view actually rendered.
  const shot = async (name: string, token: string | undefined, path: string, ready?: string) => {
    await page.addInitScript((tk: any) => { if (tk) localStorage.setItem('dos_access_token', tk); else localStorage.removeItem('dos_access_token'); localStorage.setItem('dos_locale', 'sq'); }, token);
<<<<<<< Updated upstream
    await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' }).catch(() => {});
    await page.evaluate(() => (document as any).fonts?.ready).catch(() => {});
    await page.waitForTimeout(1600);
    await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: true }).catch(() => {});
=======
    await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' }).catch((e) => { void e; /* tolerated: capture even when networkidle times out on a slow live page */ });
    await page.evaluate(() => (document as any).fonts?.ready).catch((e) => { void e; /* tolerated: fonts API may be absent in the runtime */ });
    if (ready) {
      // Hard gate: the delivery view must render real content (not a spinner / login / 500) — a
      // failure here goes RED, not a blank screenshot. No swallowing.
      await expect(page.locator(ready).first()).toBeVisible({ timeout: 20_000 });
    }
    await page.screenshot({ path: `${DIR}/${name}.png`, fullPage: true }).catch((e) => { void e; /* tolerated: best-effort capture; the toBeVisible gate above is the real proof */ });
>>>>>>> Stashed changes
  };

  // Courier active delivery (the safety-critical one) — route param is the assignment id. The
  // `tel:` call link is rendered only once the assignment task loads (locale-independent), so it
  // proves the delivery view rendered actual content before we screenshot.
  await shot('m-courier-delivery', courier.access_token, `/courier/delivery/${assignmentId}`, 'a[href^="tel:"]');
  // Order tracking (best-effort: no customer token → the auth-expired state, itself worth grading).
  await shot('m-client-tracking', undefined, `/s/${slug || 'visual-open'}/order/${orderId}`);

  console.log('CAPTURED delivery+tracking; orderId', orderId, 'slug', slug);
  expect(fs.existsSync(`${DIR}/m-courier-delivery.png`)).toBeTruthy();
});
