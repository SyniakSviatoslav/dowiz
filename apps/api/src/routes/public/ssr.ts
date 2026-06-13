// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ssrRoutes(fastify, opts) {
  fastify.get('/s/:slug', async (request, reply) => {
    const { slug } = request.params as { slug: string };
    const queryURL = request.url;
    const qs = queryURL.includes('?') ? queryURL.slice(queryURL.indexOf('?')) : '';
    reply.redirect(301, `/branding-preview/${slug}${qs}`);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
