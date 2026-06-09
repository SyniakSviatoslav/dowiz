// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function ssrRoutes(fastify, opts) {
  const { db } = opts as any;
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get('/s/:slug', async (request, reply) => {
    const { slug } = request.params as { slug: string };

    try {
      const client = await db.connect();
      try {
        const res = await client.query('SELECT read_public_menu_all_locales($1) as menu', [slug]);
        const menuData = res.rows[0]?.menu;

        if (!menuData) {
          return reply
            .status(404)
            .header('Content-Type', 'text/html; charset=utf-8')
            .send(`<!DOCTYPE html><html><head><meta name="robots" content="noindex"><link rel="canonical" href="/"><title>404 Not Found</title></head><body><h1>Faqja nuk u gjet / Page not found</h1></body></html>`);
        }

        const themeRes = await client.query(`
          SELECT lt.frame_ancestors
          FROM location_themes lt
          WHERE lt.location_id = (SELECT id FROM locations WHERE slug = $1)
          LIMIT 1
        `, [slug]);
        const frameAncestors = themeRes.rows[0]?.frame_ancestors?.join(' ') || "'self'";

        const csp = `default-src 'self'; img-src 'self' data: https://cdn.dowiz.org; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com; connect-src 'self' https://cdn.dowiz.org https://cdn.jsdelivr.net; frame-ancestors ${frameAncestors}`;

        reply.header('Content-Security-Policy', csp);
        reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=86400');

        return reply.sendFile('index.html');
      } finally {
        client.release();
      }
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send('Internal Server Error');
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
