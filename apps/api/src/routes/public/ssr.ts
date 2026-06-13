// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ssrRoutes(fastify, opts) {
  fastify.get('/s/:slug', async (request, reply) => {
    return reply.sendFile('index.html');
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
