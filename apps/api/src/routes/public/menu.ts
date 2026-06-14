import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { getImageUrl } from '../../lib/image-url.js';

export default async function publicMenuRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get(
    '/public/locations/:locationIdOrSlug/menu',
    async (request, reply) => {
      const { locationIdOrSlug } = request.params as any;
      const locale = (request.query as any)?.locale || '';

      const [res, locRes] = await Promise.all([
        server.db.query(`SELECT read_public_menu($1, $2) as menu`, [locationIdOrSlug, locale]),
        server.db.query(`SELECT id, name FROM locations WHERE id::text = $1 OR slug = $1`, [locationIdOrSlug]),
      ]);

      const menu = res.rows[0]?.menu;
      if (!menu) {
        return reply.status(404).send({ error: 'Location not found' });
      }

      menu.location_id = menu.locationId = locRes.rows[0]?.id || null;
      menu.location_name = locRes.rows[0]?.name || '';

      if (menu.categories && Array.isArray(menu.categories)) {
        for (const cat of menu.categories) {
          if (cat.products && Array.isArray(cat.products)) {
            for (const prod of cat.products) {
              prod.imageUrl = getImageUrl(prod.image_key || prod.imageKey);
            }
          }
        }
      }

      reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=300');
      reply.header('X-Menu-Version', menu.menu_version.toString());

      return reply.send(menu);
    }
  );

  server.get(
    '/public/locations/:slug/info',
    async (request, reply) => {
      const { slug } = request.params as any;
      const res = await server.db.query(
        `SELECT id, name, slug, currency_code, currency_minor_unit, default_locale FROM locations WHERE slug = $1`,
        [slug]
      );
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );
}
