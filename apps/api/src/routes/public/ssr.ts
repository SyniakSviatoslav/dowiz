import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ssrRoutes(fastify: any, opts: any) {
  fastify.get('/s/:slug', async (request: any, reply: any) => {
    return reply.sendFile('index.html');
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
