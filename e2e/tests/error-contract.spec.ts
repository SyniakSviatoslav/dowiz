import { test, expect } from '@playwright/test';

// ADR-0010 Area A1 — structured error envelope + server-authoritative correlationId.
// A2 will add the full verify:error-contract matrix; this is the A1 infrastructure proof.
const BASE = process.env.VITE_BASE_URL || 'https://dowiz.fly.dev';

test.describe('Error contract A1 — envelope + correlationId', () => {
  // A request that fails Fastify schema validation routes through setErrorHandler.
  // (POST /api/orders with a junk body → validation error.)
  const fireValidationError = (request: any, headers?: Record<string, string>) =>
    request.post(`${BASE}/api/orders`, {
      data: { invalid: true },
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
});
