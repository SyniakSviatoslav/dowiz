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
        `SELECT l.id, l.name, l.slug, l.currency_code, l.currency_minor_unit, l.default_locale,
                l.lat, l.lng, l.delivery_paused, l.hours_json, l.address,
                lt.google_rating, lt.google_review_count, lt.google_maps_url
         FROM locations l
         LEFT JOIN location_themes lt ON lt.location_id = l.id
         WHERE l.slug = $1`,
        [slug]
      );
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      const r = res.rows[0];

      // Compute isOpen from hours_json + delivery_paused (Albania timezone)
      let isOpen = !(r.delivery_paused ?? false);
      if (isOpen && r.hours_json) {
        try {
          const TZ = 'Europe/Tirane';
          const parts = new Intl.DateTimeFormat('en-US', {
            timeZone: TZ, weekday: 'long', hour: '2-digit', minute: '2-digit', hour12: false,
          }).formatToParts(new Date());
          const weekday = parts.find(p => p.type === 'weekday')?.value ?? '';
          const hour = parseInt(parts.find(p => p.type === 'hour')?.value ?? '0', 10) % 24;
          const minute = parseInt(parts.find(p => p.type === 'minute')?.value ?? '0', 10);
          const dayName = weekday.toLowerCase();
          const nowMins = hour * 60 + minute;
          const dayData = (r.hours_json as Record<string, any>)[dayName];
          if (dayData && typeof dayData === 'object') {
            if (dayData.isOpen === false) {
              isOpen = false;
            } else if (dayData.open && dayData.close) {
              const [oh, om] = dayData.open.split(':').map(Number);
              const [ch, cm] = dayData.close.split(':').map(Number);
              isOpen = nowMins >= oh * 60 + om && nowMins < ch * 60 + cm;
            }
          }
        } catch { /* ignore parse errors */ }
      }

      reply.header('Cache-Control', 'no-store');
      return reply.send({
        id: r.id, name: r.name, slug: r.slug,
        currency_code: r.currency_code, currency_minor_unit: r.currency_minor_unit,
        default_locale: r.default_locale,
        lat: r.lat != null ? Number(r.lat) : null,
        lng: r.lng != null ? Number(r.lng) : null,
        address: r.address ?? null,
        isOpen,
        googleRating: r.google_rating != null ? Number(r.google_rating) : null,
        googleReviewCount: r.google_review_count != null ? Number(r.google_review_count) : null,
        googleMapsUrl: r.google_maps_url ?? null,
      });
    }
  );
}
