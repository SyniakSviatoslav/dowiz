// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function brandingPreviewRoutes(fastify, opts) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get('/branding-preview/:slug', async (request, reply) => {
    const csp = `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com; connect-src 'self' https://cdn.jsdelivr.net; frame-ancestors *`;

    reply.header('Content-Security-Policy', csp);
    reply.header('Cache-Control', 'no-cache, no-store, must-revalidate');
    try { reply.raw.removeHeader('X-Frame-Options'); } catch {}
    return reply.sendFile('index.html');
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
