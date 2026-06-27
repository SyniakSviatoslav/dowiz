import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import fastifyRateLimit from '@fastify/rate-limit';
import { ApiError, rateLimitError } from '../src/lib/api-error.js';

// ADR-0010 Area A3 — the global @fastify/rate-limit 429. The plugin THROWS the
// `errorResponseBuilder` return value (index.js:333), so it must be a throwable ApiError,
// NOT a plain body — a plain `{status:429,…}` made `setErrorHandler` read `.statusCode` as
// undefined → 500 with "Internal server error" (caught on staging). The fix routes the 429
// through the ONE envelope source by throwing an ApiError that carries `statusCode`.
//
// Proof is deterministic + IP-pollution-free: a throwaway Fastify with max:1 + the REAL
// builder + a faithful setErrorHandler, fired via inject() (no real network). It reproduces
// the regression (429 not 500) AND the full envelope. The live envelope is also covered by
// e2e/tests/error-contract.spec against staging.

// A faithful mirror of the server.ts setErrorHandler ApiError branch (the bits A3 depends on).
function registerEnvelopeHandler(fastify: ReturnType<typeof Fastify>) {
  fastify.setErrorHandler((error: any, request, reply) => {
    const correlationId = String(request.id);
    const apiErr = error instanceof ApiError ? error : null;
    const status = apiErr?.status ?? error.statusCode ?? 500;
    const code = apiErr?.code ?? (status >= 500 ? 'INTERNAL' : 'ERROR');
    const message = status >= 500 ? 'Internal server error' : apiErr?.message || error.message;
    reply.header('x-correlation-id', correlationId); // mirror server.ts:421 — divergence guard
    reply.status(status).send({
      code,
      message,
      correlationId,
      retryAfterMs: apiErr?.retryAfterMs,
      status,
      error: message,
    });
  });
}

test('A3 rate-limit error', async (t) => {
  await t.test('pure factory returns a throwable ApiError carrying 429 + RATE_LIMIT', () => {
    const err = rateLimitError(429, 1500);
    assert.ok(err instanceof ApiError);
    assert.equal(err.status, 429);
    assert.equal(err.statusCode, 429); // getter — so a thrown ApiError carries the HTTP status
    assert.equal(err.code, 'RATE_LIMIT'); // SCREAMING_SNAKE contract
    assert.match(err.code, /^[A-Z][A-Z0-9_]*$/);
    assert.equal(err.retryAfterMs, 1500);
    assert.match(err.message, /Try again in 2s/); // 1500ms → ceil(1.5)=2s
  });

  await t.test('plugin throw → setErrorHandler → 429 envelope (NOT 500)', async () => {
    const fastify = Fastify();
    await fastify.register(fastifyRateLimit, {
      max: 1,
      timeWindow: '1 minute',
      errorResponseBuilder: (_request, context) => rateLimitError(context.statusCode, context.ttl),
    });
    registerEnvelopeHandler(fastify);
    fastify.get('/ping', async () => ({ ok: true }));

    const first = await fastify.inject({ method: 'GET', url: '/ping' });
    assert.equal(first.statusCode, 200); // under the limit

    const limited = await fastify.inject({ method: 'GET', url: '/ping' });
    assert.equal(limited.statusCode, 429); // regression guard: was 500 before the fix
    const body = limited.json();
    assert.equal(body.code, 'RATE_LIMIT');
    assert.equal(body.status, 429);
    assert.match(body.message, /Too many requests/); // NOT "Internal server error"
    assert.equal(typeof body.retryAfterMs, 'number');
    // context.ttl is the remaining window — a '1 minute' window must yield ~60_000ms here,
    // not TTL=1/1000 (a unit/scale bug that `> 0` would silently pass).
    assert.ok(body.retryAfterMs >= 50_000 && body.retryAfterMs <= 60_000);
    assert.equal(typeof body.correlationId, 'string');
    assert.ok(body.correlationId.length > 0);
    // Envelope MUST echo the correlationId in the x-correlation-id header (server.ts:421).
    assert.equal(limited.headers['x-correlation-id'], body.correlationId);
    // The plugin sets Retry-After itself, before the throw.
    assert.ok(limited.headers['retry-after'] !== undefined);

    await fastify.close();
  });
});
