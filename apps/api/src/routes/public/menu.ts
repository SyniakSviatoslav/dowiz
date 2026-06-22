import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { getImageUrl } from '../../lib/image-url.js';
import { loadEnv } from '@deliveryos/config';
import { mediaServingAllowed } from '../../lib/product-media-validation.js';

// TODO import from shared-types (@deliveryos/shared-types) once the Data agent ships it.
type ProductMediaKind = 'image' | 'video' | 'spin' | 'model';
interface ProductMedia {
  id: string;
  kind: ProductMediaKind;
  url: string;
  posterUrl?: string | null;
  mimeType: string;
  width?: number | null;
  height?: number | null;
  durationMs?: number | null;
  alt?: string | null;
  sortOrder: number;
  meta?: { frameCount?: number; frameUrls?: string[] } | null;
}

// Resolve a stored key to a public URL: an http(s) value (seed/external) passes
// through verbatim; anything else is a content-addressed key served by /media/*.
function resolveMediaUrl(key: string | null | undefined): string | null {
  if (!key) return null;
  return /^https?:\/\//i.test(key) ? key : `/media/${key}`;
}

export default async function publicMenuRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();
  // Read once at plugin registration — the flag is a deploy-time kill-switch, not
  // a per-request toggle.
  const mediaRichEnabled = loadEnv().MEDIA_RICH_ENABLED === 'true';

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
                lt.google_rating, lt.google_review_count, lt.google_maps_url,
                lt.google_place_id, lt.social_instagram, lt.social_facebook
         FROM locations l
         LEFT JOIN location_themes lt ON lt.location_id = l.id
         WHERE l.slug = $1`,
        [slug]
      );
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      const r = res.rows[0];

      // Compute isOpen from hours_json + delivery_paused
      let isOpen = !(r.delivery_paused ?? false);
      if (isOpen && r.hours_json) {
        try {
          const days = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];
          const now = new Date();
          const dayName = days[now.getDay()] as string;
          const dayData = (r.hours_json as Record<string, any>)[dayName];
          if (dayData && typeof dayData === 'object') {
            if (dayData.isOpen === false) {
              isOpen = false;
            } else if (dayData.open && dayData.close) {
              const [oh, om] = dayData.open.split(':').map(Number);
              const [ch, cm] = dayData.close.split(':').map(Number);
              const nowMins = now.getHours() * 60 + now.getMinutes();
              const openMins = oh * 60 + om;
              const closeMins = ch * 60 + cm;
              isOpen = nowMins >= openMins && nowMins < closeMins;
            }
          }
        } catch { /* ignore parse errors */ }
      }

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
        googlePlaceId: r.google_place_id ?? null,
        socialInstagram: r.social_instagram ?? null,
        socialFacebook: r.social_facebook ?? null,
      });
    }
  );

  // GET /public/locations/:slug/products/:productId/media — lazy media payload.
  // Loaded by the client only when a product modal opens. Defence-in-depth gate:
  // the feature is dark unless MEDIA_RICH_ENABLED='true' AND the location is on
  // the 'business' plan; otherwise an empty list (200) — the client also won't
  // import the renderer chunks. (Phase-2 contract §Lazy media endpoint.)
  server.get(
    '/public/locations/:slug/products/:productId/media',
    async (request, reply) => {
      const { slug, productId } = request.params as { slug: string; productId: string };
      reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=300');

      if (!mediaRichEnabled) {
        return reply.send({ media: [] as ProductMedia[] });
      }

      const locRes = await server.db.query(
        `SELECT id, plan FROM locations WHERE id::text = $1 OR slug = $1`,
        [slug],
      );
      const loc = locRes.rows[0];
      if (!loc || !mediaServingAllowed(mediaRichEnabled, loc.plan)) {
        return reply.send({ media: [] as ProductMedia[] });
      }

      const res = await server.db.query(
        `SELECT id, kind, storage_key, mime_type, width, height, duration_ms,
                poster_key, alt, sort_order, meta
           FROM product_media
          WHERE location_id = $1 AND product_id = $2 AND available = true
          ORDER BY sort_order ASC, created_at ASC`,
        [loc.id, productId],
      );

      const media: ProductMedia[] = res.rows.map((r: any) => {
        const meta = (r.meta && typeof r.meta === 'object') ? { ...r.meta } : null;
        if (r.kind === 'spin' && meta && Array.isArray(meta.frameKeys)) {
          meta.frameUrls = meta.frameKeys
            .map((k: string) => resolveMediaUrl(k))
            .filter((u: string | null): u is string => !!u);
          delete meta.frameKeys;
        }
        return {
          id: r.id,
          kind: r.kind,
          url: resolveMediaUrl(r.storage_key) || '',
          posterUrl: resolveMediaUrl(r.poster_key),
          mimeType: r.mime_type,
          width: r.width ?? null,
          height: r.height ?? null,
          durationMs: r.duration_ms ?? null,
          alt: r.alt ?? null,
          sortOrder: r.sort_order,
          meta,
        };
      });

      return reply.send({ media });
    }
  );
}
