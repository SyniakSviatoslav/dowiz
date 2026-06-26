import { test } from 'node:test';
import assert from 'node:assert/strict';
import Fastify from 'fastify';
import { buildErrorEnvelope } from '../src/lib/api-error.js';

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
    const fastify = Fastify();
    // The decorator mirrors server.ts (4 lines) — buildErrorEnvelope is the REAL shared source.
    fastify.decorateReply('sendError', function (this: any, status: number, code: string, message: string, opts?: any) {
      const correlationId = String(this.request.id);
      this.header('x-correlation-id', correlationId);
      if (opts?.retryAfterMs) this.header('retry-after', Math.ceil(opts.retryAfterMs / 1000));
      return this.status(status).send(buildErrorEnvelope(status, code, message, correlationId, opts));
    });
    fastify.get('/cat', async (_req, reply: any) => reply.sendError(409, 'CATEGORY_NOT_EMPTY', 'Category contains products'));

    const res = await fastify.inject({ method: 'GET', url: '/cat' });
    assert.equal(res.statusCode, 409);
    const body = res.json();
    assert.equal(body.code, 'CATEGORY_NOT_EMPTY');
    assert.equal(body.status, 409);
    assert.equal(body.error, 'Category contains products'); // legacy preserved
    assert.equal(typeof body.correlationId, 'string');
    assert.ok(body.correlationId.length > 0);
    assert.equal(res.headers['x-correlation-id'], body.correlationId); // echo matches body

    await fastify.close();
  });
});
