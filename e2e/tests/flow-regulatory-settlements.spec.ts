import { test, expect } from '@playwright/test';
import { expectJwt, expectUuid } from '../helpers/assert-shape';
import { requireStaging } from '../helpers/staging-guard';

const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';
let authToken: string;
let activeLocationId: string;
let signalId: string;
let alertId: string;
let settlementId: string;
let gdprRequestId: string;

test.describe.configure({ mode: 'serial' });

test.describe('Flow: Regulatory & Financial — GDPR, Settlements, Signals, Alerts', () => {

  // ════════════════════════════════════════════════════════════════
  // SETUP
  // ════════════════════════════════════════════════════════════════
  test.beforeAll(async ({ request }) => {
    requireStaging(BASE); // mutating spec (GDPR create, settlement approve/dispute) — never hit prod
    const authRes = await request.post(`${BASE}/api/dev/mock-auth`, { data: {} });
    expect(authRes.status()).toBe(200);
    const authBody = await authRes.json();
    authToken = authBody.access_token;
    activeLocationId = authBody.activeLocationId;
    expectJwt(authToken, 'access_token');
    expectUuid(activeLocationId, 'activeLocationId');

    // Fetch IDs for existing signals/alerts/settlements for action tests.
    // beforeAll MUST assert setup status 200 — a swallowed 401/404/500 here would silently
    // strand signalId/alertId/settlementId as undefined and skip every action flow green.
    const sigRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/signals`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(sigRes.status(), `signals setup fetch (${sigRes.status()})`).toBe(200);
    const sigs = await sigRes.json();
    const sigArr = sigs.signals || sigs.data || [];
    signalId = sigArr.length > 0 ? sigArr[0].id : undefined;

    const altRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/alerts`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(altRes.status(), `alerts setup fetch (${altRes.status()})`).toBe(200);
    const alerts = await altRes.json();
    const alertArr = alerts.alerts || alerts.data || [];
    alertId = alertArr.length > 0 ? alertArr[0].id : undefined;

    const settRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(settRes.status(), `settlements setup fetch (${settRes.status()})`).toBe(200);
    const s = await settRes.json();
    const payouts = s.payouts || s.data || s;
    settlementId = Array.isArray(payouts) && payouts.length > 0 ? payouts[0].id : undefined;
    // TODO(needs_staging): seed ≥1 signal/alert/pending-settlement so Flows 8/9/10/12 run a real
    // action assertion instead of test.skip on an empty tenant (conditional-skip vacuity).
  });

  // ════════════════════════════════════════════════════════════════
  // NEGATIVE AUTH CONTROL — every sensitive route rejects no-token (401)
  // ════════════════════════════════════════════════════════════════

  test('Flow 0: unauthenticated requests are rejected (401)', async ({ request }) => {
    const paths = [
      `/api/owner/locations/${activeLocationId}/gdpr-requests`,
      `/api/owner/locations/${activeLocationId}/settlements`,
      `/api/owner/locations/${activeLocationId}/signals`,
      `/api/owner/locations/${activeLocationId}/alerts`,
      `/api/owner/locations/${activeLocationId}/couriers/live`,
    ];
    for (const p of paths) {
      const res = await request.get(`${BASE}${p}`); // no Authorization header
      expect(res.status(), `unauthenticated ${p} must be 401 (got ${res.status()})`).toBe(401);
    }
    // POST create without a token must also be rejected before any DB write.
    const postRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/gdpr-requests`,
      { data: { phone: '+355600000003', reason: 'no-auth' } }
    );
    expect(postRes.status(), `unauthenticated GDPR create must be 401 (got ${postRes.status()})`).toBe(401);
  });

  // TODO(needs_staging): cross-tenant IDOR — seed a SECOND owner + location, then assert this
  // owner's token GETting the second tenant's gdpr-requests/settlements/signals returns 404
  // (requireLocationAccess denies). A random/nil UUID 404s by absence and proves nothing, so this
  // requires a real second tenant on staging — not added here to avoid a false-green.

  // ════════════════════════════════════════════════════════════════
  // GDPR — create request, list requests
  // ════════════════════════════════════════════════════════════════

  test('Flow 1: GDPR — create erasure request', async ({ request }) => {
    // Positive control: a unique phone that matches NO existing customer skips the 409/429
    // state-machine branches in the handler, so a healthy route MUST return 201. A 401/403/500
    // (or a swallowed error) now fails the test instead of passing via `.not.toBe(500)`.
    const uniquePhone = `+35569${Date.now().toString().slice(-7)}`;
    const gdprRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/gdpr-requests`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { phone: uniquePhone, reason: 'E2E test GDPR request' } }
    );
    expect(gdprRes.status(), `GDPR create returned ${gdprRes.status()}, expected 201`).toBe(201);
    const body = await gdprRes.json();
    gdprRequestId = body.requestId || body.id;
    expectUuid(gdprRequestId, 'gdprRequestId');
    expect(body.status).toBe('pending');
  });

  test('Flow 2: GDPR — list erasure requests', async ({ request }) => {
    const listRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/gdpr-requests`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(listRes.status()).toBe(200);
    const body = await listRes.json();
    const requests = body.requests || body.data || body;
    expect(Array.isArray(requests)).toBe(true);
  });

  test('Flow 3: GDPR — get request detail', async ({ request }) => {
    test.skip(!gdprRequestId, 'No GDPR request created');
    const detailRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/gdpr-requests/${gdprRequestId}`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(detailRes.status()).toBe(200);
    const body = await detailRes.json();
    expect(body.id || body.requestId).toBe(gdprRequestId);
    expect(body.status).toBeTruthy();
  });

  // ════════════════════════════════════════════════════════════════
  // SETTLEMENTS — list, get by ID
  // ════════════════════════════════════════════════════════════════

  test('Flow 4: Owner — list settlements', async ({ request }) => {
    const settRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    const settStatus = settRes.status();
    expect(settStatus, `List settlements returned ${settStatus}, expected 200`).toBe(200);
    const body = await settRes.json();
    const payouts = body.payouts || body.data || body;
    expect(Array.isArray(payouts)).toBe(true);
    if (payouts.length > 0) {
      const p = payouts[0];
      expect(p.id || p.settlementId).toBeTruthy();
      expect('status' in p).toBe(true);
    }
  });

  test('Flow 5: Owner — get settlement detail', async ({ request }) => {
    test.skip(!settlementId, 'No settlements found');
    const detailRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements/${settlementId}`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(detailRes.status()).toBe(200);
    const body = await detailRes.json();
    const payout = body.payout || body;
    expect(payout.id || payout.settlementId || payout.payoutId).toBeTruthy();
  });

  test('Flow 6: Owner — settlement list filtered by status', async ({ request }) => {
    const settRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements?status=pending`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(settRes.status(), `Filtered settlements returned ${settRes.status()}, expected 200`).toBe(200);
  });

  // ════════════════════════════════════════════════════════════════
  // SIGNALS — compute, acknowledge, dismiss
  // ════════════════════════════════════════════════════════════════

  test('Flow 7: Owner — compute signals', async ({ request }) => {
    const compRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/signals/compute`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(compRes.status()).toBe(200);
    const body = await compRes.json();
    expect(body.computedAt || body.signals || body.data).toBeTruthy();
  });

  test('Flow 8: Owner — acknowledge signal', async ({ request }) => {
    test.skip(!signalId, 'No signals to acknowledge');
    const ackRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/signals/${signalId}/acknowledge`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(ackRes.status()).toBe(200);
    const body = await ackRes.json();
    expect(body.id || body.signalId).toBeTruthy();
    expect(body.acknowledgedAt).toBeTruthy();
  });

  test('Flow 9: Owner — dismiss signal', async ({ request }) => {
    test.skip(!signalId, 'No signals to dismiss');
    const dismissRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/signals/${signalId}/dismiss`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { reason: 'E2E test dismissal' } }
    );
    expect(dismissRes.status()).toBe(200);
  });

  // ════════════════════════════════════════════════════════════════
  // ALERTS — acknowledge, acknowledge-all
  // ════════════════════════════════════════════════════════════════

  test('Flow 10: Owner — acknowledge alert', async ({ request }) => {
    test.skip(!alertId, 'No alerts to acknowledge');
    const ackRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/alerts/${alertId}/acknowledge`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(ackRes.status()).toBe(200);
    const body = await ackRes.json();
    expect(body.id || body.alertId).toBeTruthy();
    expect(body.status || body.acknowledgedAt).toBeTruthy();
  });

  test('Flow 11: Owner — acknowledge all alerts', async ({ request }) => {
    const ackAllRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/alerts/acknowledge-all`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(ackAllRes.status()).toBe(200);
    const body = await ackAllRes.json();
    expect(typeof body.acknowledged).toBe('number');
  });

  // ════════════════════════════════════════════════════════════════
  // SETTLEMENT ACTIONS — approve, pay, reopen (if any pending)
  // ════════════════════════════════════════════════════════════════

  test('Flow 12: Owner — settlement approve/pay/dispute', async ({ request }) => {
    test.skip(!settlementId, 'No settlements for action tests');

    const approveRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements/${settlementId}/approve`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(approveRes.status()).toBe(200);

    const disputeRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements/${settlementId}/dispute`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { reason: 'E2E test dispute' } }
    );
    expect(disputeRes.status()).toBe(200);

    const reopenRes = await request.post(
      `${BASE}/api/owner/locations/${activeLocationId}/settlements/${settlementId}/reopen`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { reason: 'E2E test reopen' } }
    );
    expect(reopenRes.status()).toBe(200);
  });

  // ════════════════════════════════════════════════════════════════
  // COURIER PAYOUTS (owner-side settlement view)
  // ════════════════════════════════════════════════════════════════

  test('Flow 13: Owner — courier details and live map', async ({ request }) => {
    // Get courier details (uses owner/couriers.ts)
    const courierRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/couriers/live`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(courierRes.status()).toBe(200);
    const body = await courierRes.json();
    const couriers = body.couriers || body.data || body;
    expect(Array.isArray(couriers)).toBe(true);
    if (couriers.length > 0) {
      const c = couriers[0];
      const detailRes = await request.get(
        `${BASE}/api/owner/locations/${activeLocationId}/couriers/${c.courierId || c.id}/details`,
        { headers: { Authorization: `Bearer ${authToken}` } }
      );
      expect(detailRes.status()).toBe(200);
    }
  });

  test('Flow 14: Owner — update courier status', async ({ request }) => {
    const courierRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/couriers`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    const body = await courierRes.json();
    const couriers = body.couriers || body.data || [];
    if (couriers.length === 0) {
      test.skip(true, 'No couriers to update');
    }
    const courierId = couriers[0].id;
    const patchRes = await request.patch(
      `${BASE}/api/owner/locations/${activeLocationId}/couriers/${courierId}`,
      { headers: { Authorization: `Bearer ${authToken}` }, data: { status: 'active' } }
    );
    expect(patchRes.status()).toBe(200);
    expect((await patchRes.json()).success).toBe(true);
  });

  // ════════════════════════════════════════════════════════════════
  // SIGNALS with filtering
  // ════════════════════════════════════════════════════════════════

  test('Flow 15: Owner — signals list with status filter', async ({ request }) => {
    const sigRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/signals?status=active&limit=5`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(sigRes.status()).toBe(200);
    const body = await sigRes.json();
    const sigArr = body.signals || body.data || [];
    expect(Array.isArray(sigArr)).toBe(true);
  });

  test('Flow 16: Owner — alerts list with status filter', async ({ request }) => {
    const altRes = await request.get(
      `${BASE}/api/owner/locations/${activeLocationId}/alerts?status=active&limit=5`,
      { headers: { Authorization: `Bearer ${authToken}` } }
    );
    expect(altRes.status()).toBe(200);
    const body = await altRes.json();
    const alertArr = body.alerts || body.data || [];
    expect(Array.isArray(alertArr)).toBe(true);
  });
});
