import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { accentFor, cityOf, pickHeroUrl, fetchImageDataUri, renderOgCardPng } from '../../lib/og-card.js';

// GET /og/:slug.png — the per-venue Open Graph card (see lib/og-card.ts). Referenced from
// <meta property="og:image"> and embeddable as an <img> in outreach email. Generated on demand and
// cached in-process (6h) + at the edge (Cache-Control), because a crawler re-fetches the same key and
// the underlying menu changes only on republish. `?play=1` adds the ▶ badge (video-teaser variant).
export default (async function ogCardRoutes(fastify: any, opts: any) {
  const { db } = opts as any;
  const TTL_MS = 6 * 60 * 60 * 1000;
  const cache = new Map<string, { png: Buffer; exp: number }>();

  fastify.get('/og/:slug.png', async (request: any, reply: any) => {
    const slug = String(request.params?.slug || '');
    const play = request.query?.play != null;
    const key = play ? `${slug}::play` : slug;
    const now = Date.now();

    const cached = cache.get(key);
    if (!cached || now >= cached.exp) {
      // Name + address + Google rating (same source as /public/locations/:slug/info).
      const locRes = await db.query(
        `SELECT l.name, l.address, lt.google_rating
           FROM locations l LEFT JOIN location_themes lt ON lt.location_id = l.id
          WHERE l.slug = $1 LIMIT 1`,
        [slug],
      );
      const row = locRes.rows[0];
      if (!row) return reply.code(404).header('Cache-Control', 'public, max-age=300').type('text/plain').send('not found');

      // Hero photo: published menu first, then the shadow preview (never-live demos).
      let menu: any = null;
      try { menu = (await db.query(`SELECT read_public_menu($1, '') AS m`, [slug])).rows[0]?.m; } catch { /* ignore */ }
      if (!menu) {
        try { menu = (await db.query(`SELECT read_preview_menu($1) AS m`, [slug])).rows[0]?.m; } catch { /* 42883 pre-migration */ }
      }
      const heroUrl = pickHeroUrl(menu);
      const heroDataUri = heroUrl ? await fetchImageDataUri(heroUrl) : null;

      const png = await renderOgCardPng({
        name: row.name || slug,
        city: cityOf(row.address),
        rating: row.google_rating != null ? Number(row.google_rating) : null,
        accent: accentFor(slug, row.name),
        heroDataUri,
        play,
      });

      if (cache.size > 200) cache.clear(); // simple bound; keys are venue slugs
      cache.set(key, { png, exp: now + TTL_MS });
    }

    return reply
      .header('Content-Type', 'image/png')
      .header('Cache-Control', 'public, max-age=21600, stale-while-revalidate=86400')
      .header('X-Robots-Tag', 'noindex')
      .send(cache.get(key)!.png);
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
