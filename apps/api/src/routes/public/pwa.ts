// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';

export default (async function pwaRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.get('/s/:slug/manifest.webmanifest', async (request, reply) => {
    const slug = (request.params as any).slug || 'app';

    let locationName = slug;
    let themeColor = '#ea4f16';
    let bgColor = '#121212';

    try {
      const client = await db.connect();
      try {
        const res = await client.query(
          `SELECT l.name, lt.primary_color FROM locations l 
           LEFT JOIN location_themes lt ON lt.location_id = l.id 
           WHERE l.slug = $1 LIMIT 1`,
          [slug]
        );
        if (res.rows[0]) {
          locationName = res.rows[0].name || slug;
          const pc = res.rows[0].primary_color;
          if (pc) themeColor = pc;
        }
      } finally {
        client.release();
      }
    } catch { }

    reply.header('Content-Type', 'application/manifest+json');
    reply.header('Cache-Control', 'public, max-age=3600');

    return {
      name: locationName,
      short_name: locationName.length > 12 ? locationName.slice(0, 12) + '...' : locationName,
      description: `Order food delivery from ${locationName}`,
      start_url: `/s/${slug}?source=pwa`,
      display: 'standalone',
      background_color: bgColor,
      theme_color: themeColor,
      icons: [
        { src: '/icons/icon-192.png', sizes: '192x192', type: 'image/png', purpose: 'any maskable' },
        { src: '/icons/icon-512.png', sizes: '512x512', type: 'image/png', purpose: 'any maskable' },
      ],
    };
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
