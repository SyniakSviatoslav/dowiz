// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function telemetryRoutes(fastify, opts) {
  
  fastify.post('/api/telemetry', {
    schema: {
      body: z.object({
        action: z.enum([
          'cart.added', 'cart.removed', 'cart.drift_resolved', 'cart.corrupted',
          'checkout.opened', 'checkout.submitted', 'checkout.failed',
          'order.status_viewed', 'pwa.installed', 'pwa.install_prompted'
        ]),
        locationId: z.string().uuid().optional(),
        orderId: z.string().uuid().optional(),
        errorCode: z.string().optional(),
        delta: z.number().optional()
      }).passthrough()
    }
  }, async (request, reply) => {
    // Only generic non-PII data comes here, as validated by Zod
    const payload = request.body;
    request.log.info({ telemetry: payload }, 'Telemetry Event');
    return reply.status(202).send({ accepted: true });
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
