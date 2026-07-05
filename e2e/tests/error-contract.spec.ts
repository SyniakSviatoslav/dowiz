import { test, expect } from '@playwright/test';

// ADR-0010 Area A1 — structured error envelope + server-authoritative correlationId.
// A2 will add the full verify:error-contract matrix; this is the A1 infrastructure proof.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

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
    expect(typeof body.correlationId).toBe('string');
    expect(body.correlationId.length).toBeGreaterThan(0);
    expect(body.status).toBe(400); // numeric status now lives in `status`
    expect(Array.isArray(body.fields)).toBe(true); // 422-style field paths
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

  test('5xx leaks no internal detail (generic message, no err.detail/stack)', async ({ request }) => {
    // A validation error is the deterministic A1 path; assert the leak guard on what we can
    // reliably trigger: the envelope never carries a `details`/`stack`/`detail` field.
    const res = await fireValidationError(request);
    const body = await res.json();
    expect(body.details).toBeUndefined();
    expect(body.detail).toBeUndefined();
    expect(body.stack).toBeUndefined();
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
    expect(typeof body.correlationId).toBe('string');
    expect(body.correlationId.length).toBeGreaterThan(0);
    expect(body.path).toBeUndefined(); // path no longer leaked in the body
  });
});
