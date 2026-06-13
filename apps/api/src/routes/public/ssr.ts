// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ssrRoutes(fastify, opts) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get('/s/:slug', async (request, reply) => {
    const { slug } = request.params as { slug: string };
    const queryURL = request.url;
    const searchParams = queryURL.includes('?') ? queryURL.substring(queryURL.indexOf('?')) : '';

    return reply.redirect(301, `/branding-preview/${slug}${searchParams}`);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
