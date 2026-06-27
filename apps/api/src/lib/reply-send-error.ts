import type { FastifyInstance } from 'fastify';
import { buildErrorEnvelope, type ErrorEnvelopeOpts } from './api-error.js';

/**
 * Registers `reply.sendError(status, code, message, opts?)` (ADR-0010 A2) on a Fastify instance —
 * the return-based drop-in for ad-hoc `reply.status(n).send({ error })`. Emits the SAME envelope as
 * setErrorHandler (shared `buildErrorEnvelope`) with the server correlationId + x-correlation-id echo.
 *
 * Extracted from server.ts so route-unit tests that build a bare Fastify (e.g. orders-guards.test.ts)
 * register the SAME decorator — without it, a migrated route calling `reply.sendError` throws → 500
 * (the exact A2-sweep regression this prevents from recurring). Called directly (not registered) so
 * the decorator lands on the target instance's scope with no plugin-encapsulation surprises.
 */
export function registerReplySendError(fastify: FastifyInstance): void {
  fastify.decorateReply(
    'sendError',
    function (this: any, status: number, code: string, message: string, opts?: ErrorEnvelopeOpts) {
      const correlationId = String(this.request.id);
      this.header('x-correlation-id', correlationId);
      if (opts?.retryAfterMs) this.header('retry-after', Math.ceil(opts.retryAfterMs / 1000));
      return this.status(status).send(buildErrorEnvelope(status, code, message, correlationId, opts));
    },
  );
}
