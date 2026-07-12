import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import { buildErrorEnvelope } from '../src/lib/api-error.js';
import { registerReplySendError } from '../src/lib/reply-send-error.js';

// A2 (ADR-0010) — `reply.sendError` is the return-based drop-in for ad-hoc
// `reply.status(n).send({ error })` sites (first migrated file: owner/categories.ts). It must
// emit the SAME envelope as setErrorHandler via the shared `buildErrorEnvelope`. Proof:
//   (1) pure builder shape;
//   (2) the decorator wiring via inject() — code/status/correlationId/legacy error + header echo.

test('A2 sendError envelope', async (t) => {
  await t.test('buildErrorEnvelope emits the one envelope shape', () => {
    const env = buildErrorEnvelope(404, 'NOT_FOUND', 'Not found', 'cid-1');
    assert.equal(env.code, 'NOT_FOUND');
    assert.match(env.code, /^[A-Z][A-Z0-9_]*$/); // SCREAMING_SNAKE contract
    assert.equal(env.status, 404); // numeric status lives in `status`
    assert.equal(env.message, 'Not found');
    assert.equal(env.error, 'Not found'); // legacy string the un-migrated FE still reads
    assert.equal(env.correlationId, 'cid-1');
    assert.equal(env.fields, undefined);
    assert.equal(env.retryAfterMs, undefined);
    // opts flow through
    const env429 = buildErrorEnvelope(429, 'RATE_LIMIT', 'slow down', 'cid-2', { retryAfterMs: 5000 });
    assert.equal(env429.retryAfterMs, 5000);
  });

  await t.test('reply.sendError returns the envelope + echoes x-correlation-id (wiring)', async () => {
    // Pin the server-generated request id so correlationId is proven to TRACK request.id
    // (not the static Fastify-inject default '1'). genReqId mirrors production (server.ts:107).
    const KNOWN_REQ_ID = 'req-id-test-7f3a9c21';
    const fastify = Fastify({ genReqId: () => KNOWN_REQ_ID });
    // Register the REAL production decorator (server.ts:408) — no inlined re-implementation,
    // so any divergence in lib/reply-send-error.ts fails here instead of staying hidden.
    registerReplySendError(fastify);
    fastify.get('/cat', async (_req, reply: any) => reply.sendError(409, 'CATEGORY_NOT_EMPTY', 'Category contains products'));

    const res = await fastify.inject({ method: 'GET', url: '/cat' });
    assert.equal(res.statusCode, 409);
    const body = res.json();
    assert.equal(body.code, 'CATEGORY_NOT_EMPTY');
    assert.equal(body.status, 409);
    assert.equal(body.error, 'Category contains products'); // legacy preserved
    assert.equal(body.correlationId, KNOWN_REQ_ID); // tracks the server request id, not a static default
    assert.equal(res.headers['x-correlation-id'], KNOWN_REQ_ID); // header echoes the REAL id (not just body==header)

    await fastify.close();
  });
});
