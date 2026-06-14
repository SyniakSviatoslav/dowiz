import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';

const MAX_URLS_PER_SHARD = 50000;

export default (async function seoRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  async function getActiveLocations(client: any) {
    const res = await client.query(`
      SELECT l.slug, l.supported_locales, l.menu_version, l.updated_at,
        COALESCE(mv.version, 1) as mv_version,
        EXISTS(
          SELECT 1 FROM products p
          WHERE p.location_id = l.id AND p.is_available = true
          LIMIT 1
        ) as has_products
      FROM locations l
      LEFT JOIN menu_versions mv ON mv.location_id = l.id
      WHERE l.status IS DISTINCT FROM 'deleted'
        AND l.status IS DISTINCT FROM 'disabled'
      ORDER BY l.slug
    `);
    return res.rows;
  }

  function buildUrlTag(loc: string, lastmod: string, supportedLocales: string[] | undefined) {
    let xml = `  <url>\n    <loc>${loc}</loc>\n    <lastmod>${lastmod}</lastmod>\n`;
    for (const locale of supportedLocales || ['sq', 'en']) {
      xml += `    <xhtml:link rel="alternate" hreflang="${locale}" href="${loc}?locale=${locale}" />\n`;
    }
    xml += `    <xhtml:link rel="alternate" hreflang="x-default" href="${loc}" />\n`;
    xml += `  </url>\n`;
    return xml;
  }

  fastify.get('/robots.txt', async (request: any, reply: any) => {
    const host = request.hostname;
    const protocol = request.protocol || 'https';
    const sitemapUrl = `${protocol}://${host}/sitemap.xml`;

    const robots = `User-agent: *
Allow: /$
Allow: /s/
Allow: /public/
Disallow: /s/*/cart
Disallow: /s/*/checkout
Disallow: /s/*/order/
Disallow: /*?embed=true
Disallow: /*?preview=true
Disallow: /admin/
Disallow: /courier/
Disallow: /onboarding
Disallow: /api/
Disallow: /dist/
Disallow: /icons/

# AI answer engines — allowed for citation
User-agent: GPTBot
Allow: /s/
User-agent: OAI-SearchBot
Allow: /s/
User-agent: ChatGPT-User
Allow: /s/
User-agent: ClaudeBot
Allow: /s/
User-agent: Claude-Web
Allow: /s/
User-agent: PerplexityBot
Allow: /s/
User-agent: Google-Extended
Allow: /s/

Sitemap: ${sitemapUrl}`;

    reply.header('Content-Type', 'text/plain');
    reply.header('Cache-Control', 'public, max-age=86400');
    return reply.send(robots);
  });

  // Sitemap index → sharded children
  fastify.get('/sitemap.xml', async (request: any, reply: any) => {
    try {
      const client = await db.connect();
      try {
        const rows = await getActiveLocations(client);
        const total = rows.filter((r: any) => r.has_products).length;
        const shardCount = Math.max(1, Math.ceil(total / MAX_URLS_PER_SHARD));

        const host = request.hostname;
        const protocol = request.protocol || 'https';
        const baseUrl = `${protocol}://${host}`;

        let xml = `<?xml version="1.0" encoding="UTF-8"?>\n<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n`;
        for (let i = 1; i <= shardCount; i++) {
          xml += `  <sitemap>\n    <loc>${baseUrl}/sitemap-locations-${i}.xml</loc>\n  </sitemap>\n`;
        }
        xml += `</sitemapindex>`;

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

  // Sharded location sitemaps
  fastify.get('/sitemap-locations-:shard.xml', async (request: any, reply: any) => {
    const shard = parseInt((request.params as any).shard, 10) || 1;

    try {
      const client = await db.connect();
      try {
        const rows = await getActiveLocations(client);
        const filtered = rows.filter((r: any) => r.has_products);

        const start = (shard - 1) * MAX_URLS_PER_SHARD;
        const slice = filtered.slice(start, start + MAX_URLS_PER_SHARD);

        if (slice.length === 0) {
          return reply.status(404).send('Not found');
        }

        const host = request.hostname;
        const protocol = request.protocol || 'https';
        const baseUrl = `${protocol}://${host}`;

        let xml = `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">\n`;

        for (const row of slice) {
          const loc = `${baseUrl}/s/${row.slug}`;
          const version = row.mv_version || row.menu_version || 1;
          const updated = row.updated_at || new Date(version * 1000).toISOString();
          const lastmod = updated ? new Date(updated).toISOString().split('T')[0] : new Date().toISOString().split('T')[0];
          xml += buildUrlTag(loc, lastmod as string, row.supported_locales);
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
