import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function pwaRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.get('/s/:slug/manifest.webmanifest', {
    schema: {
      params: z.object({ slug: z.string() })
    }
  }, async (request, reply) => {
    const { slug } = request.params;

    try {
      const client = await db.connect();
      try {
        const res = await client.query(`SELECT name FROM locations WHERE slug = $1 AND deleted_at IS NULL`, [slug]);
        const loc = res.rows[0];

        if (!loc) {
          return reply.status(404).send({ error: 'Location not found' });
        }

        const manifest = {
          name: loc.name,
          short_name: loc.name,
          start_url: `/s/${slug}?source=pwa`,
          display: "standalone",
          background_color: "#ffffff",
          theme_color: "#e63946", // Default, P15 will customize
          icons: [
            { src: "https://cdn.dowiz.org/locations/default/logo-192.png", sizes: "192x192", type: "image/png" },
            { src: "https://cdn.dowiz.org/locations/default/logo-512.png", sizes: "512x512", type: "image/png" }
          ]
        };

        reply.header('Content-Type', 'application/manifest+json');
        reply.header('Cache-Control', 'public, max-age=3600');
        return reply.send(manifest);
      } finally {
        client.release();
      }
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send('Internal Server Error');
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
