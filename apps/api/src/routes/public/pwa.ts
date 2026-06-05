// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function pwaRoutes(fastify, opts) {

  fastify.get('/s/:slug/manifest.webmanifest', async (request, reply) => {
    const slug = (request.params as any).slug || 'app';

    reply.header('Content-Type', 'application/manifest+json');
    reply.header('Cache-Control', 'public, max-age=3600');

    return {
      name: slug,
      short_name: slug,
      start_url: `/s/${slug}?source=pwa`,
      display: "standalone",
      background_color: "#ffffff",
      theme_color: "#ea4f16",
      icons: [
        { src: "https://cdn.dowiz.org/locations/default/logo-192.png", sizes: "192x192", type: "image/png" },
        { src: "https://cdn.dowiz.org/locations/default/logo-512.png", sizes: "512x512", type: "image/png" }
      ]
    };
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
