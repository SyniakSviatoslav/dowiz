import { test, expect } from '@playwright/test';
import { expectUuid } from '../helpers/assert-shape';

// ADR-0010 Area A1 — structured error envelope + server-authoritative correlationId.
// A2 will add the full verify:error-contract matrix; this is the A1 infrastructure proof.
// Read-only suite (validation 400s + GET probes) — never mutates, so no requireStaging guard;
// default BASE to staging (never the prod host) so a CI run can't probe prod by accident.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz-staging.fly.dev';

test.describe('Error contract A1 — envelope + correlationId', () => {
  // This is an API contract proof — viewport/browser are irrelevant. Pin it to ONE project:
  // the suite runs across 5 projects (mobile/tablet/desktop/webkit×2) and the validation
  // endpoint (track/exchange) is per-route rate-limited at 10/min/IP, so 6 requests × 5
  // projects on a shared egress IP trips the limit and the validation path returns 429
  // instead of 400 (flaky-by-design). One project keeps it deterministic and under the limit.
  // Playwright requires the first arg to be an object-destructuring pattern; eslint's
  // no-empty-pattern forbids `{}` — disable it for this one line (we only need testInfo).
  // eslint-disable-next-line no-empty-pattern
  test.beforeEach(({}, testInfo) => {
    test.skip(
      testInfo.project.name !== 'desktop',
      'API contract proof runs on a single project (shared-IP per-route rate limit)',
    );
  });

  // A request that fails Fastify(Zod) SCHEMA validation routes through setErrorHandler.
  // track/exchange has `schema: { body: exchangeSchema }` (a real Fastify schema, not an
  // ad-hoc in-handler Zod parse), is public, and is not order-rate-limited — so it's the
  // deterministic A1 error path. (POST /api/orders validates ad-hoc + is velocity-gated → A2.)
  const fireValidationError = (request: any, headers?: Record<string, string>) =>
    request.post(`${BASE}/api/customer/track/exchange`, {
      data: {}, // missing required `code` → schema validation error
      headers: { 'Content-Type': 'application/json', ...(headers || {}) },
      failOnStatusCode: false,
    });

  test('error response carries the structured envelope', async ({ request }) => {
    const res = await fireValidationError(request);
    expect(res.status()).toBe(400); // status preserved (code-preserving rollout)
    const body = await res.json();
    // New A1 keys
    expect(typeof body.code).toBe('string'); // SCREAMING_SNAKE machine code, not a number
    expect(body.code).toMatch(/^[A-Z][A-Z0-9_]*$/);
    expect(body.code).toBe('VALIDATION_FAILED');
    expectUuid(body.correlationId, 'correlationId'); // crypto.randomUUID, not just truthy
    expect(body.status).toBe(400); // numeric status now lives in `status`
    // 422-style field paths: must be NON-empty AND name the actually-missing field. An empty
    // `[]` would pass `Array.isArray` while proving the validator surfaced nothing (Zod
    // exchangeSchema rejects the missing `code` → fields entry { path:'code', code:<keyword> }).
    expect(Array.isArray(body.fields)).toBe(true);
    expect(body.fields.length).toBeGreaterThan(0);
    const codeField = body.fields.find((f: { path: string }) => f.path === 'code');
    expect(codeField, 'fields must carry the rejected `code` path').toBeTruthy();
    expect(typeof codeField.code).toBe('string'); // keyword/path only, never the submitted value (B4)
    // Legacy key retained so un-migrated FE keeps working (code-preserving rollout)
    expect(typeof body.error).toBe('string');
  });

  test('x-correlation-id header echoes the envelope correlationId', async ({ request }) => {
    const res = await fireValidationError(request);
    const header = res.headers()['x-correlation-id'];
    const body = await res.json();
    expect(header).toBeTruthy();
    expect(header).toBe(body.correlationId);
  });

  test('correlationId is SERVER-generated and ignores a forged inbound header (B6)', async ({ request }) => {
    const forged = 'forged-victim-support-code-0001';
    const res = await fireValidationError(request, { 'x-correlation-id': forged });
    const body = await res.json();
    // The server must NOT echo the attacker-supplied id — it generates its own (crypto.randomUUID).
    expect(body.correlationId).not.toBe(forged);
    expect(res.headers()['x-correlation-id']).not.toBe(forged);
    // crypto.randomUUID shape (8-4-4-4-12 hex)
    expect(body.correlationId).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i);
  });

  test('two requests get distinct correlationIds', async ({ request }) => {
    const [a, b] = await Promise.all([fireValidationError(request), fireValidationError(request)]);
    const [ba, bb] = await Promise.all([a.json(), b.json()]);
    expect(ba.correlationId).not.toBe(bb.correlationId);
  });

  test('error envelope leaks no internal detail (no err.detail/stack/details)', async ({ request }) => {
    // Leak guard on the deterministic path we CAN reliably trigger (validation): the envelope
    // never carries a `details`/`stack`/`detail` field.
    // TODO(needs_staging): this does NOT exercise a real 5xx (DB-down / unhandled throw). A
    // genuine 500 that leaks a stack/PG detail would stay green here. A dedicated 5xx proof
    // needs a route that deterministically throws on staging (no such public trigger exists
    // without mutating state) — escalate to add a staging-only 5xx fixture.
    const res = await fireValidationError(request);
    const body = await res.json();
    expect(body.details).toBeUndefined();
    expect(body.detail).toBeUndefined();
    expect(body.stack).toBeUndefined();
  });

  test('protected route without a token returns EXACTLY 401 (auth negative control)', async ({ request }) => {
    // GET /api/owner/onboarding/:locationId/state is guarded by verifyAuth + requireRole(['owner'])
    // (apps/api/src/routes/owner/onboarding.ts:29-30). No Bearer header → verifyAuth short-circuits 401.
    const res = await request.get(
      `${BASE}/api/owner/onboarding/00000000-0000-4000-8000-000000000000/state`,
      { headers: { Accept: 'application/json' }, failOnStatusCode: false },
    );
    expect(res.status()).toBe(401); // exact — not [401,403]; missing token, not wrong role
    const body = await res.json();
    expect(typeof body.error).toBe('string'); // legacy auth body conveys the failure
    expect(body.error.length).toBeGreaterThan(0);
    // TODO(needs_staging): the auth-guard 401/403 path (plugins/auth.ts) still emits the legacy
    // `{error}` body — the ADR-0010 structured envelope (code/correlationId) is NOT applied there.
    // Asserting `body.code`/`correlationId` here would (correctly) go red — a real envelope-coverage
    // gap to escalate, not to fake-green. Verify against live staging before tightening.
  });

  test('unmatched API route returns the NOT_FOUND envelope (A2 notFound handler)', async ({ request }) => {
    const res = await request.get(`${BASE}/api/this-route-does-not-exist-xyz`, {
      headers: { Accept: 'application/json' },
      failOnStatusCode: false,
    });
    expect(res.status()).toBe(404);
    const body = await res.json();
    expect(body.code).toBe('NOT_FOUND'); // was an ad-hoc {error,path} before A2
    expect(body.status).toBe(404);
    expectUuid(body.correlationId, 'correlationId');
    expect(body.path).toBeUndefined(); // path no longer leaked in the body
  });
});
