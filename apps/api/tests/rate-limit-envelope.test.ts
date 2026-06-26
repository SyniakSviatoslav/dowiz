import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import fastifyRateLimit from '@fastify/rate-limit';
import { rateLimitEnvelope } from '../src/lib/api-error.js';

// ADR-0010 Area A3 — the global @fastify/rate-limit 429 builds its OWN body (it never enters
// setErrorHandler), so the structured envelope must be reconstructed in `errorResponseBuilder`.
// Proof is deterministic + IP-pollution-free: a throwaway Fastify with max:1 + the REAL builder,
// fired via inject() (no real network). This proves the WIRING — the plugin actually invokes the
// builder with `context.ttl` — not just the pure function.

test('A3 rate-limit envelope', async (t) => {
  await t.test('pure builder emits the contract envelope', () => {
    const env = rateLimitEnvelope('cid-123', 1500);
    assert.equal(env.code, 'RATE_LIMIT'); // SCREAMING_SNAKE contract
    assert.match(env.code, /^[A-Z][A-Z0-9_]*$/);
    assert.equal(env.status, 429); // numeric status lives in `status`, not `code`
    assert.equal(env.retryAfterMs, 1500);
    assert.equal(env.correlationId, 'cid-123');
    assert.match(env.message, /Try again in 2s/); // 1500ms → ceil(1.5)=2s
    assert.equal(env.error, 'Too many requests'); // legacy key retained (code-preserving)
  });

  await t.test('plugin 429 returns the envelope (wiring, via inject)', async () => {
    const fastify = Fastify();
    await fastify.register(fastifyRateLimit, {
      max: 1,
      timeWindow: '1 minute',
      errorResponseBuilder: (request, context) => rateLimitEnvelope(String(request.id), context.ttl),
    });
    fastify.get('/ping', async () => ({ ok: true }));

    const first = await fastify.inject({ method: 'GET', url: '/ping' });
    assert.equal(first.statusCode, 200); // under the limit

    const limited = await fastify.inject({ method: 'GET', url: '/ping' });
    assert.equal(limited.statusCode, 429); // over the limit → plugin builds its own body
    const body = limited.json();
    assert.equal(body.code, 'RATE_LIMIT');
    assert.equal(body.status, 429);
    assert.equal(typeof body.retryAfterMs, 'number');
    assert.ok(body.retryAfterMs > 0); // context.ttl actually flowed through
    assert.equal(typeof body.correlationId, 'string');
    assert.ok(body.correlationId.length > 0);
    // The plugin sets Retry-After itself (no double header, no global hook).
    assert.ok(limited.headers['retry-after'] !== undefined);

    await fastify.close();
  });
});
