import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';

export default (async function vapidRoutes(fastify, opts) {
  fastify.get('/api/push/vapid-public-key', async (_request, reply) => {
    const publicKey = process.env.VAPID_PUBLIC_KEY || '';
    if (!publicKey) return reply.status(404).send({ error: 'VAPID not configured' });
    return reply.send({ publicKey });
  });
});
