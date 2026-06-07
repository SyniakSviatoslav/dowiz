// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function telemetryRoutes(fastify, opts) {
  
  fastify.post('/api/telemetry', {
    config: { rateLimit: false }
  }, async (request, reply) => {
    try {
      const payload = request.body;
      request.log.info({ telemetry: payload }, 'Telemetry Event');
      return reply.status(202).send({ accepted: true });
    } catch (err) {
      request.log.warn({ err }, 'Telemetry rejected');
      return reply.status(202).send({ accepted: false });
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
