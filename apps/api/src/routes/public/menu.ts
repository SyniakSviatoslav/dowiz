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

  // ── In-process menu cache (the storefront-blink fix) ──────────────────────────────
  // The public menu is the hottest read and changes only when the owner republishes
  // (menu_version bumps). Under load WITHOUT this cache every customer hit ran the full
  // read_public_menu query: a concurrent burst checked out more operational-pool
  // connections than the pool has (max), so excess requests waited connectionTimeoutMillis
  // then threw → HTTP 500 → the FE rendered an empty storefront. Caching collapses a burst
  // into ONE DB execution per (slug,locale,TTL), taking the DB off the hot path.
  //
  // Per-instance (each machine warms its own). TTL is short because availability windows
  // (menu_schedules) are time-based — 30s sits well under the finest mealtime boundary, and
  // the FE already polls menu_version for finer-grained invalidation. Stale-while-revalidate:
  // an expired-but-usable copy is served instantly while a single deduped refresh runs in the
  // background, so a customer NEVER blocks on the DB once the cache is warm.
  const MENU_CACHE_TTL_MS = 30_000;
  const MENU_CACHE_STALE_MS = 300_000;
  // Hard bound on cache size: the key is (slug,locale) and `locale` is caller-controlled, so
  // without a cap an attacker can fan out `?locale=<counter>` and grow the Map until the
  // instance OOMs (the cache built to stop the blink would become a remote crash primitive).
  // FIFO eviction on insert keeps memory at ≤ MAX_ENTRIES × ~40KB ≈ 20MB regardless of input;
  // real storefronts use a handful of locales so legit entries are never evicted in practice.
  const MENU_CACHE_MAX_ENTRIES = 500;
  // stale-on-error is a brief-hiccup guard, NOT an infinite fallback: never serve a copy older
  // than this on the error path (beyond it, surface the failure rather than ancient prices).
  const MENU_STALE_ON_ERROR_MAX_MS = 3_600_000; // 1h
  type MenuCacheEntry = { payload: any; freshUntil: number; staleUntil: number; bornAt: number };
  const menuCache = new Map<string, MenuCacheEntry>();
  const menuInflight = new Map<string, Promise<any>>();

  // /info shares the same starvation risk as /menu (a raw db.query on every storefront load):
  // under a connection burst it waited connectionTimeoutMillis then 500'd → checkout breaks
  // (locationId stays null). Cache the DB ROW (not the computed status) so the time-based
  // open/closed/busy is always recomputed fresh from `now`, while the DB read is collapsed +
  // protected by the same TTL / stale-on-error guard as the menu.
  type InfoCacheEntry = { row: any; freshUntil: number; bornAt: number };
  const infoCache = new Map<string, InfoCacheEntry>();
  const infoInflight = new Map<string, Promise<any>>();

  // One DB load + shaping pass. Returns the fully-shaped payload (image URLs resolved,
  // location_id/name present) or null for an unknown location. Updates the cache on success.
  async function refreshMenu(key: string, slug: string, locale: string): Promise<any | null> {
    const res = await server.db.query(`SELECT read_public_menu($1, $2) as menu`, [slug, locale]);
    const menu = res.rows[0]?.menu;
    if (!menu) return null;

    // F2: location_id/name come straight from read_public_menu (migration 1790000000064),
    // so the route no longer needs a second query. Fallback for deploy/rollback skew where
    // the live function predates 064 and omits them — keeps a down() safe.
    if (menu.location_id === undefined || menu.location_name === undefined) {
      const locRes = await server.db.query(
        `SELECT id, name FROM locations WHERE id::text = $1 OR slug = $1`,
        [slug],
      );
      menu.location_id = locRes.rows[0]?.id ?? null;
      menu.location_name = locRes.rows[0]?.name ?? '';
    }
    menu.locationId = menu.location_id; // preserve the camelCase alias prior consumers saw

    if (menu.categories && Array.isArray(menu.categories)) {
      for (const cat of menu.categories) {
        if (cat.products && Array.isArray(cat.products)) {
          for (const prod of cat.products) {
            prod.imageUrl = getImageUrl(prod.image_key || prod.imageKey);
          }
        }
      }
    }

    const now = Date.now();
    // FIFO-evict the oldest entry when inserting a NEW key past the cap (bounds memory).
    if (!menuCache.has(key) && menuCache.size >= MENU_CACHE_MAX_ENTRIES) {
      const oldest = menuCache.keys().next().value;
      if (oldest !== undefined) menuCache.delete(oldest);
    }
    menuCache.set(key, { payload: menu, freshUntil: now + MENU_CACHE_TTL_MS, staleUntil: now + MENU_CACHE_STALE_MS, bornAt: now });
    return menu;
  }

  async function getMenu(key: string, slug: string, locale: string): Promise<any | null> {
    const now = Date.now();
    const cached = menuCache.get(key);
    if (cached && now < cached.freshUntil) return cached.payload;

    // Stale-but-usable: serve it now, refresh in the background (deduped per key).
    if (cached && now < cached.staleUntil) {
      if (!menuInflight.has(key)) {
        menuInflight.set(
          key,
          refreshMenu(key, slug, locale).catch(() => null).finally(() => menuInflight.delete(key)),
        );
      }
      return cached.payload;
    }

    // Cold (or fully stale): block on a single shared refresh so a burst makes one DB call.
    let inflight = menuInflight.get(key);
    if (!inflight) {
      inflight = refreshMenu(key, slug, locale).finally(() => menuInflight.delete(key));
      menuInflight.set(key, inflight);
    }
    return inflight;
  }

  server.get(
    '/public/locations/:locationIdOrSlug/menu',
    async (request, reply) => {
      const { locationIdOrSlug } = request.params as any;
      // Normalize the caller-controlled locale to a bounded shape (BCP-47-ish: letters, digits,
      // '-' '_', ≤12 chars) before it ever becomes a cache key — shrinks the key space and stops
      // junk/unicode fan-out. read_public_menu still coerces unsupported locales to the default.
      const locale = String((request.query as any)?.locale || '')
        .toLowerCase()
        .replace(/[^a-z0-9_-]/g, '')
        .slice(0, 12);
      const key = `${locationIdOrSlug}::${locale}`;

      let menu: any;
      try {
        menu = await getMenu(key, locationIdOrSlug, locale);
      } catch (err) {
        // Last-ditch blink guard: if the DB/pool hiccups but we have a RECENT-ENOUGH cached
        // copy, serve it rather than 500 → a blank storefront. Bounded by MENU_STALE_ON_ERROR_MAX_MS
        // so a prolonged outage surfaces the failure instead of serving ancient prices forever.
        const stale = menuCache.get(key);
        if (stale && Date.now() - stale.bornAt < MENU_STALE_ON_ERROR_MAX_MS) {
          reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=300');
          reply.header('X-Menu-Version', String(stale.payload.menu_version));
          reply.header('X-Menu-Cache', 'stale-on-error');
          return reply.send(stale.payload);
        }
        throw err;
      }

      if (!menu) {
        return reply.status(404).send({ error: 'Location not found' });
      }

      reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=300');
      reply.header('X-Menu-Version', menu.menu_version.toString());

      return reply.send(menu);
    }
  );

  async function refreshInfoRow(slug: string): Promise<any | null> {
    const res = await server.db.query(
      `SELECT l.id, l.name, l.slug, l.currency_code, l.currency_minor_unit, l.default_locale,
              l.lat, l.lng, l.delivery_paused, l.hours_json, l.address, l.kitchen_busy_until,
              lt.google_rating, lt.google_review_count, lt.google_maps_url,
              lt.google_place_id, lt.social_instagram, lt.social_facebook
       FROM locations l
       LEFT JOIN location_themes lt ON lt.location_id = l.id
       WHERE l.slug = $1`,
      [slug]
    );
    const row = res.rows[0] ?? null;
    const now = Date.now();
    if (!infoCache.has(slug) && infoCache.size >= MENU_CACHE_MAX_ENTRIES) {
      const oldest = infoCache.keys().next().value;
      if (oldest !== undefined) infoCache.delete(oldest);
    }
    // Cache the row (incl. a null result, so unknown slugs don't re-hit the DB on every probe).
    infoCache.set(slug, { row, freshUntil: now + MENU_CACHE_TTL_MS, bornAt: now });
    return row;
  }

  async function getInfoRow(slug: string): Promise<any | null> {
    const cached = infoCache.get(slug);
    if (cached && Date.now() < cached.freshUntil) return cached.row;
    let inflight = infoInflight.get(slug);
    if (!inflight) {
      inflight = refreshInfoRow(slug).finally(() => infoInflight.delete(slug));
      infoInflight.set(slug, inflight);
    }
    return inflight;
  }

  server.get(
    '/public/locations/:slug/info',
    async (request, reply) => {
      const { slug } = request.params as any;
      let r: any;
      try {
        r = await getInfoRow(slug);
      } catch (err) {
        // Pool/DB hiccup: serve a recent-enough cached row instead of 500 (which leaves the
        // FE with no locationId → silent checkout failure). Bounded by the same stale window.
        const stale = infoCache.get(slug);
        if (stale && stale.row && Date.now() - stale.bornAt < MENU_STALE_ON_ERROR_MAX_MS) {
          reply.header('X-Info-Cache', 'stale-on-error');
          r = stale.row;
        } else {
          // No usable cache → a typed 503 (not a raw 500) so the FE can show "couldn't load".
          return reply.status(503).send({ error: 'Temporarily unavailable' });
        }
      }
      if (!r) return reply.status(404).send({ error: 'Not found' });

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

      // MENU-AVAILABILITY (additive) · surface the contract's `status` (open|closed|busy)
      // distinct from the legacy `isOpen` boolean. `busy` = venue is OPEN but the owner
      // has flagged the kitchen busy / raised ETA until kitchen_busy_until (a future ts).
      // NULL/past kitchen_busy_until => the open/closed result is untouched.
      const busyUntil = r.kitchen_busy_until ? new Date(r.kitchen_busy_until) : null;
      const isBusy = isOpen && busyUntil != null && busyUntil.getTime() > Date.now();
      const status: 'open' | 'closed' | 'busy' = !isOpen ? 'closed' : isBusy ? 'busy' : 'open';

      return reply.send({
        id: r.id, name: r.name, slug: r.slug,
        currency_code: r.currency_code, currency_minor_unit: r.currency_minor_unit,
        default_locale: r.default_locale,
        lat: r.lat != null ? Number(r.lat) : null,
        lng: r.lng != null ? Number(r.lng) : null,
        address: r.address ?? null,
        isOpen,
        status,
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
