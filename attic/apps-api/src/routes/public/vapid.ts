import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';

export default (async function vapidRoutes(fastify: any, opts: any) {
  fastify.get('/api/push/vapid-public-key', async (_request: any, reply: any) => {
    const publicKey = process.env.VAPID_PUBLIC_KEY || '';
    if (!publicKey) return reply.status(404).send({ error: 'VAPID not configured' });
    return reply.send({ publicKey });
  });
});
