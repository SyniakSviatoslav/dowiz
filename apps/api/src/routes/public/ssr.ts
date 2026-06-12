// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';

export default (async function ssrRoutes(fastify, opts) {
  const { db } = opts as any;
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get('/s/:slug', async (request, reply) => {
    const { slug } = request.params as { slug: string };
    const queryURL = request.url;
    const isEmbed = queryURL.includes('embed=true');
    const isPreview = queryURL.includes('preview=true');

    try {
      const client = await db.connect();
      try {
        const res = await client.query('SELECT read_public_menu_all_locales($1) as menu, status, name, slug FROM locations WHERE slug = $1', [slug]);
        const row = res.rows[0];
        const menuData = row?.menu;

        if (!menuData) {
          return reply
            .status(404)
            .header('Content-Type', 'text/html; charset=utf-8')
            .send(`<!DOCTYPE html><html><head><meta name="robots" content="noindex"><link rel="canonical" href="/"><title>404 Not Found</title></head><body><h1>Faqja nuk u gjet / Page not found</h1></body></html>`);
        }

        const locationStatus = row?.status;
        if (locationStatus === 'deleted') {
          return reply
            .status(410)
            .header('Content-Type', 'text/html; charset=utf-8')
            .header('Cache-Control', 'public, max-age=86400')
            .send(`<!DOCTYPE html><html><head><meta name="robots" content="noindex"><title>410 Gone</title></head><body><h1>This restaurant is no longer available</h1></body></html>`);
        }

        let frameAncestors = "'self'";
        if (isEmbed) {
          frameAncestors = '*';
        } else {
          const themeRes = await client.query(`
            SELECT lt.frame_ancestors
            FROM location_themes lt
            WHERE lt.location_id = (SELECT id FROM locations WHERE slug = $1)
            LIMIT 1
          `, [slug]);
          frameAncestors = themeRes.rows[0]?.frame_ancestors?.join(' ') || "'self'";
        }

        const menuVersion = menuData?.menu_version || menuData?.version;
        if (menuVersion) {
          reply.header('X-Menu-Version', String(menuVersion));
        }

        const csp = `default-src 'self'; img-src 'self' data: https://tiles.openfreemap.org; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com; worker-src 'self' blob:; connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org; frame-ancestors ${frameAncestors}`;

        reply.header('Content-Security-Policy', csp);
        reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=86400');
        reply.header('Content-Type', 'text/html; charset=utf-8');

        const host = request.hostname;
        const protocol = request.protocol || 'https';
        const baseUrl = `${protocol}://${host}`;

        let logoUrl = null;
        try {
          const themeRes = await client.query(`
            SELECT lt.logo_url FROM location_themes lt WHERE lt.location_id = (SELECT id FROM locations WHERE slug = $1) LIMIT 1
          `, [slug]);
          logoUrl = themeRes.rows[0]?.logo_url || null;
        } catch { }

        const html = renderMenuPage(menuData, slug, logoUrl, baseUrl, isEmbed, isPreview, locationStatus);

        return reply.send(html);
      } finally {
        client.release();
      }
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send('Internal Server Error');
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
