import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';

export default (async function seoRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.get('/robots.txt', async (request, reply) => {
    const robots = `User-agent: *\nAllow: /s/\nDisallow: /api/\nDisallow: /admin/\nSitemap: https://dowiz.org/sitemap.xml`;
    reply.header('Content-Type', 'text/plain');
    reply.header('Cache-Control', 'public, max-age=86400');
    return reply.send(robots);
  });

  fastify.get('/sitemap.xml', async (request, reply) => {
    try {
      const client = await db.connect();
      try {
        const res = await client.query(
          `SELECT slug, supported_locales 
           FROM locations 
           WHERE deleted_at IS NULL AND status = 'active'`
        );
        
        let xml = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n`;
        
        for (const row of res.rows) {
          const baseUrl = `https://dowiz.org/s/${row.slug}`;
          xml += `  <url>\n    <loc>${baseUrl}</loc>\n`;
          for (const loc of row.supported_locales) {
            xml += `    <xhtml:link rel="alternate" hreflang="${loc}" href="${baseUrl}?locale=${loc}" />\n`;
          }
          xml += `    <xhtml:link rel="alternate" hreflang="x-default" href="${baseUrl}" />\n`;
          xml += `  </url>\n`;
        }
        
        xml += `</urlset>`;

        reply.header('Content-Type', 'application/xml');
        reply.header('Cache-Control', 'public, max-age=3600');
        return reply.send(xml);
      } finally {
        client.release();
      }
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send('Internal Server Error');
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
