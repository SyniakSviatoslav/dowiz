import type { FastifyInstance } from 'fastify';
import { jwtVerify } from 'jose';
import { loadEnv } from '@deliveryos/config';
import crypto from 'crypto';
import { withTenant } from '@deliveryos/platform';
import { getImageUrl } from '../lib/image-url.js';
import { maskStr } from '../lib/pii-mask.js';
import { decryptPII } from '../lib/pii-cipher.js';
import { z } from 'zod';

const env = loadEnv();

const HEX_COLOR = /^#[0-9a-fA-F]{6}$/;

function getPublicKey(): crypto.KeyObject {
  const raw = env.JWT_PUBLIC_KEY;
  if (!raw) throw new Error('JWT_PUBLIC_KEY missing');
  return crypto.createPublicKey(raw.replace(/\\n/g, '\n'));
}

function validateImageKey(val: unknown): string | null | undefined {
  if (val === undefined || val === null) return val;
  const s = String(val);
  if (s.startsWith('data:') || s.startsWith('blob:')) {
    throw new Error('Image must be uploaded via the image upload endpoint, not sent as a data URL');
  }
  return s;
}

const brandSchema = z.object({
  primaryColor: z.string().regex(HEX_COLOR).optional().nullable(),
  bgColor: z.string().regex(HEX_COLOR).optional().nullable(),
  textColor: z.string().regex(HEX_COLOR).optional().nullable(),
  logoUrl: z.string().max(1000).optional().nullable(),
  googleRating: z.number().min(0).max(5).optional().nullable(),
  googleReviewCount: z.number().int().nonnegative().optional().nullable(),
  googleMapsUrl: z.string().max(500).optional().nullable(),
}).strict();

// pg returns NUMERIC columns as strings; coerce all numeric fields so the schema
// accepts both the raw DB values echoed back by the frontend and proper JS numbers
const coerceNum = (inner: z.ZodType) =>
  z.preprocess((v) => (v != null && v !== '' ? Number(v) : v), inner);

const settingsSchema = z.object({
  locationName: z.string().min(1).max(200).optional().nullable(),
  phone: z.string().max(50).optional().nullable(),
  address: z.string().max(500).optional().nullable(),
  deliveryFee: coerceNum(z.number().int().nonnegative()).optional().nullable(),
  minOrder: coerceNum(z.number().int().nonnegative()).optional().nullable(),
  radiusKm: coerceNum(z.number().nonnegative()).optional().nullable(),
  freeDeliveryThreshold: coerceNum(z.number().int().nonnegative()).optional().nullable(),
  taxRate: coerceNum(z.number().min(0).max(100)).optional().nullable(),
  currencyCode: z.enum(['ALL', 'EUR']).optional().nullable(),
  lat: coerceNum(z.number().min(-90).max(90)).optional().nullable(),
  lng: coerceNum(z.number().min(-180).max(180)).optional().nullable(),
  hoursJson: z.any().optional().nullable(),
  deliveryPaused: z.boolean().optional(),
}).strip();

export default async function spaProxyRoutes(fastify: FastifyInstance, opts: { db: any; storage?: any }) {
  const { db, storage } = opts;

  async function getLocationId(request: any): Promise<string | null> {
    const auth = request.headers.authorization;
    if (!auth?.startsWith('Bearer ')) return null;
    try {
      const token = auth.slice(7);
      const { payload } = await jwtVerify(token, getPublicKey(), { algorithms: ['RS256'] });
      const claims = payload as any;
      if (claims.role !== 'owner') return null;
      // Prefer activeLocationId from JWT (new auth system)
      if (claims.activeLocationId) return claims.activeLocationId;
      // Fall back to memberships lookup (legacy auth)
      const uid = claims.userId || claims.sub;
      if (uid) {
        const res = await db.query(
          `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
          [uid]
        );
        if (res.rows.length > 0) return res.rows[0].location_id;
      }
      return null;
    } catch (err: any) {
      console.warn('[spa-proxy] JWT verification failed:', err?.message);
      return null;
    }
  }

  // True when the request carries a valid owner JWT, regardless of whether that
  // owner has created a location yet. Used to tell a brand-new owner (no location)
  // apart from an unauthenticated/expired caller — see GET /api/owner/settings (O1).
  async function isValidOwnerToken(request: any): Promise<boolean> {
    const auth = request.headers.authorization;
    if (!auth?.startsWith('Bearer ')) return false;
    try {
      const { payload } = await jwtVerify(auth.slice(7), getPublicKey(), { algorithms: ['RS256'] });
      return (payload as any).role === 'owner';
    } catch {
      return false;
    }
  }

  async function getOwnerContext(request: any): Promise<{ locId: string; userId: string } | null> {
    const auth = request.headers.authorization;
    if (!auth?.startsWith('Bearer ')) return null;
    try {
      const token = auth.slice(7);
      const { payload } = await jwtVerify(token, getPublicKey(), { algorithms: ['RS256'] });
      const claims = payload as any;
      if (claims.role !== 'owner') return null;
      const uid = claims.userId || claims.sub;
      if (!uid) return null;
      // Prefer activeLocationId from JWT (new auth system)
      if (claims.activeLocationId) return { locId: claims.activeLocationId, userId: uid };
      // Fall back to memberships lookup (legacy auth)
      const res = await db.query(
        `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
        [uid]
      );
      if (res.rows.length > 0) return { locId: res.rows[0].location_id, userId: uid };
      return null;
    } catch (err: any) {
      console.warn('[spa-proxy] JWT verification failed:', err?.message);
      return null;
    }
  }

  const APP_BASE = process.env.APP_BASE_URL || 'https://dowiz.fly.dev';

   // Serve product images from local storage (CDN placeholder until Cloudflare R2 is set up)
   fastify.get('/images/*', async (request, reply) => {
     const raw = (request.params as any)['*'] as string;
     const key = raw.startsWith('/') ? raw.slice(1) : raw;
    try {
      const buf = await storage.get(key);
      if (!buf) return reply.status(404).send({ error: 'Image not found' });
      reply.header('Cache-Control', 'public, max-age=31536000, immutable');
      reply.header('Content-Type', 'image/webp');
      return reply.send(buf);
    } catch (err: any) {
      console.warn('[spa-proxy] image fetch failed:', err?.message);
      return reply.status(404).send({ error: 'Image not found' });
    }
  });

  // POST /api/owner/menu/products/:productId/image — upload product image
  fastify.post('/api/owner/menu/products/:productId/image', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const pid = (request.params as any).productId;
    const data = await request.file();
    if (!data) return reply.status(400).send({ error: 'No file uploaded' });
    const buffer = await data.toBuffer();
    let processed: Buffer;
    try {
      const sharp = (await import('sharp')).default;
      processed = await sharp(buffer)
        .resize({ width: 800, height: 800, fit: 'inside' })
        .webp({ quality: 82 })
        .toBuffer();
    } catch (sharpErr: any) {
      return reply.status(400).send({ error: 'Invalid image file', detail: sharpErr.message });
    }
    // Content-hash the key so each distinct image gets a unique URL. /images/*
    // serves with a 1-year immutable cache, so a fixed key would pin the FIRST
    // upload forever — re-uploads (image "change") would never show. A hashed
    // key means a changed image is a new URL the browser/CDN fetches fresh.
    const crypto = await import('node:crypto');
    const hash = crypto.createHash('sha256').update(processed).digest('hex').slice(0, 12);
    const key = `${locId}/${pid}-${hash}.webp`;
    const imageUrl = getImageUrl(key, APP_BASE);

    // Capture the prior key so we can drop the orphaned object after the swap.
    let oldKey: string | null = null;
    try {
      const cur = await db.query(`SELECT image_key FROM products WHERE id = $1 AND location_id = $2`, [pid, locId]);
      oldKey = cur.rows[0]?.image_key ?? null;
    } catch { /* non-fatal — cleanup is best-effort */ }

    try {
      await storage.put(key, processed);
    } catch (putErr: any) {
      return reply.status(500).send({ error: 'Failed to store image', detail: putErr.message });
    }
    try {
      await db.query(`UPDATE products SET image_key = $1, image_url = $2 WHERE id = $3 AND location_id = $4`, [key, imageUrl, pid, locId]);
    } catch (dbErr: any) {
      return reply.status(500).send({ error: 'Failed to update product record', detail: dbErr.message });
    }
    // Best-effort: remove the replaced object (skip external-URL legacy keys).
    if (oldKey && oldKey !== key && !/^https?:\/\//.test(oldKey)) {
      try { await storage.delete(oldKey); } catch (e: any) { console.warn('[spa-proxy] old image cleanup failed:', e?.message); }
    }
    return reply.send({ imageUrl, imageKey: key });
  });

  // GET /api/owner/analytics
  fastify.get('/api/owner/analytics', async (request, reply) => {
    const ctx = await getOwnerContext(request);
    if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
    const locId = ctx.locId;
    // withTenant: RLS tenant scoping (set app.user_id) as defense-in-depth on top of
    // the explicit WHERE location_id binding below.
    return withTenant(db, ctx.userId, async (client) => {
    const { rows: todayOrders } = await client.query(
      `SELECT total, created_at, confirmed_at, delivered_at, delivery_lat, delivery_lng
       FROM orders WHERE location_id = $1 AND created_at >= NOW() - INTERVAL '7 days'`,
      [locId]
    );
    let totalRev = 0, orderCount = 0, totalTime = 0, deliveredCount = 0;
    const geoLocations = [];
    for (const o of todayOrders) {
      totalRev += o.total || 0; orderCount++;
      if (o.delivered_at && o.created_at) {
        totalTime += (new Date(o.delivered_at).getTime() - new Date(o.created_at).getTime()) / 60000;
        deliveredCount++;
      }
      if (o.delivery_lat && o.delivery_lng) geoLocations.push({ lat: o.delivery_lat, lng: o.delivery_lng });
    }
    const avgOrderValue = orderCount > 0 ? Math.round(totalRev / orderCount) : 0;
    const avgTime = deliveredCount > 0 ? Math.round(totalTime / deliveredCount) : 0;
    const { rows: topProducts } = await client.query(
      `SELECT oi.name_snapshot AS name, COUNT(DISTINCT oi.order_id)::int AS orders,
              SUM(oi.price_snapshot * oi.quantity)::int AS revenue, p.image_key AS "imageUrl"
       FROM order_items oi LEFT JOIN products p ON p.id = oi.product_id
       JOIN orders o ON o.id = oi.order_id WHERE o.location_id = $1
       GROUP BY oi.name_snapshot, p.image_key ORDER BY revenue DESC LIMIT 20`,
      [locId]
    );
    const { rows: chartRows } = await client.query(
      `SELECT to_char(date_trunc('day', created_at), 'Dy') AS day, SUM(total)::int AS revenue
       FROM orders WHERE location_id = $1 AND created_at >= NOW() - INTERVAL '7 days'
       GROUP BY date_trunc('day', created_at) ORDER BY MIN(created_at)`,
      [locId]
    );
    const weekdays = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
    const chart = weekdays.map(day => {
      const match = chartRows.find((r: any) => r.day === day);
      return { day, revenue: match ? match.revenue : 0 };
    });
    const { rows: heatmapRows } = await client.query(
      `SELECT EXTRACT(DOW FROM o.created_at)::int AS dow,
              CASE WHEN EXTRACT(HOUR FROM o.created_at) BETWEEN 0 AND 3 THEN 0
                   WHEN EXTRACT(HOUR FROM o.created_at) BETWEEN 4 AND 7 THEN 1
                   WHEN EXTRACT(HOUR FROM o.created_at) BETWEEN 8 AND 11 THEN 2
                   WHEN EXTRACT(HOUR FROM o.created_at) BETWEEN 12 AND 15 THEN 3
                   WHEN EXTRACT(HOUR FROM o.created_at) BETWEEN 16 AND 19 THEN 4 ELSE 5 END AS slot,
              COUNT(*)::int AS cnt,
              jsonb_agg(DISTINCT oi.name_snapshot) FILTER (WHERE oi.name_snapshot IS NOT NULL) AS products
       FROM orders o LEFT JOIN order_items oi ON oi.order_id = o.id
       WHERE o.location_id = $1 AND o.created_at >= NOW() - INTERVAL '30 days'
       GROUP BY dow, slot ORDER BY dow, slot`,
      [locId]
    );
    const heatmap = [];
    for (let di = 0; di < weekdays.length; di++) {
      const pgDow = di === 6 ? 0 : di + 1;
      const hours = []; const products = [];
      for (let s = 0; s < 6; s++) {
        const match = heatmapRows.find((r: any) => r.dow === pgDow && r.slot === s);
        hours.push(match ? match.cnt : 0);
        products.push(match ? (match.products || []) : []);
      }
      heatmap.push({ day: weekdays[di], hours, products });
    }
    return reply.send({
      revenue: { today: totalRev, trend: '+15%' },
      orders: { today: orderCount, trend: '+5' },
      avgOrderValue: { value: avgOrderValue, trend: '+2%' },
      deliveryTime: { avg: avgTime, trend: '-2%' },
      chart, topProducts, geoLocations, heatmap,
    });
    });
  });

  // GET /api/owner/analytics/product-orders
  fastify.get('/api/owner/analytics/product-orders', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const name = (request.query as any).name;
    if (!name) return reply.status(400).send({ error: 'Missing product name' });
    const { rows } = await db.query(
      `SELECT o.id, o.total, o.currency_code, o.created_at, o.status,
              c.name AS customer_name, oi.quantity, oi.price_snapshot AS price
       FROM order_items oi JOIN orders o ON o.id = oi.order_id
       JOIN customers c ON c.id = o.customer_id
       WHERE o.location_id = $1 AND oi.name_snapshot = $2
       ORDER BY o.created_at DESC LIMIT 50`,
      [locId, name]
    );
    return reply.send(rows);
  });

  // GET /api/owner/orders
  fastify.get('/api/owner/orders', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const statusFilter = (request.query as any)?.status;
    let statusClause = '';
    const params: any[] = [locId];
    if (statusFilter && statusFilter !== 'all') { statusClause = 'AND o.status = $2'; params.push(statusFilter); }
    const res = await db.query(
      `SELECT o.id, o.status, o.created_at, o.confirmed_at, o.ready_at, o.delivered_at, o.total, o.subtotal,
              o.delivery_fee, o.delivery_address, o.payment_method, o.metadata,
              c.name as customer_name, c.phone as customer_phone,
              (SELECT jsonb_agg(jsonb_build_object('name', oi.name_snapshot, 'qty', oi.quantity, 'price', oi.price_snapshot))
               FROM order_items oi WHERE oi.order_id = o.id) as items,
              EXTRACT(EPOCH FROM (COALESCE(o.confirmed_at, now()) - o.created_at))::int as confirm_seconds,
              cr.full_name_encrypted as courier_name_enc
       FROM orders o LEFT JOIN customers c ON c.id = o.customer_id
       LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status IN ('accepted','picked_up','delivered')
       LEFT JOIN couriers cr ON cr.id = ca.courier_id
       WHERE o.location_id = $1 ${statusClause}
       ORDER BY o.created_at DESC LIMIT 50`,
      params
    );
    const { decryptPII } = await import('../lib/pii-cipher.js').catch(() => ({ decryptPII: (_s: string) => '' }));
    // Per-order rating/feedback (separate query + try/catch so a missing
    // order_ratings table — before its migration — can't break the list).
    const ratingByOrder: Record<string, any> = {};
    try {
      const ids = res.rows.map((r: any) => r.id);
      if (ids.length) {
        const rr = await db.query(`SELECT order_id, rating, feedback FROM order_ratings WHERE order_id = ANY($1)`, [ids]);
        for (const x of rr.rows) ratingByOrder[x.order_id] = x;
      }
    } catch { /* table not yet migrated */ }
    return reply.send(res.rows.map((r: any) => {
      const meta = r.metadata || {};
      const courierRaw = r.courier_name_enc ? decryptPII(r.courier_name_enc) : null;
      const courierName = courierRaw ? (courierRaw.charAt(0) + '***') : null;
      return {
        id: r.id, status: r.status, createdAt: r.created_at,
        confirmedAt: r.confirmed_at, readyAt: r.ready_at, deliveredAt: r.delivered_at,
        total: r.total, subtotal: r.subtotal, deliveryFee: r.delivery_fee,
        deliveryAddress: r.delivery_address, paymentMethod: r.payment_method,
        customerName: r.customer_name || 'Unknown', customerPhone: r.customer_phone || '',
        shortId: '#' + r.id.toString().substring(0, 4).toUpperCase(), items: r.items || [],
        itemCount: r.items ? r.items.length : 0,
        itemsSummary: r.items ? r.items.map((i: any) => `${i.name} x${i.qty}`).join(', ') : '',
        confirmSeconds: r.confirm_seconds, courierName,
        rating: ratingByOrder[r.id]?.rating ?? null,
        feedback: ratingByOrder[r.id]?.feedback ?? null,
        elapsedSeconds: Math.floor((Date.now() - new Date(r.created_at).getTime()) / 1000),
        signals: {
          otpVerified: meta.otp_verified === true,
          reputationScore: typeof meta.reputation_score === 'number' ? meta.reputation_score : 75,
        },
      };
    }));
  });

  // GET /api/owner/couriers
  fastify.get('/api/owner/couriers', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const client = await db.connect();
    try {
      await client.query('BEGIN');
      await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [locId]);
      const res = await client.query(
        `SELECT c.id, c.full_name_encrypted, c.phone_encrypted, c.status,
                COALESCE(cs.shift_status, 'offline') as courier_status,
                (SELECT COUNT(*) FROM courier_assignments ca2 WHERE ca2.courier_id = c.id AND ca2.status = 'delivered') as deliveries_completed
         FROM couriers c
         JOIN courier_locations cl ON c.id = cl.courier_id
         LEFT JOIN LATERAL (
           SELECT status as shift_status FROM courier_shifts
           WHERE courier_id = c.id AND location_id = $1 AND status IN ('available', 'on_delivery')
             AND (ended_at IS NULL OR ended_at > NOW())
           ORDER BY started_at DESC NULLS LAST LIMIT 1
         ) cs ON true
         WHERE cl.location_id = $1`,
        [locId]
      );
      // Average rating per courier from order_ratings. SAVEPOINT-guarded so a
      // missing table (before its migration) can't abort the transaction.
      const ratingByCourier: Record<string, number> = {};
      try {
        await client.query('SAVEPOINT r');
        const rr = await client.query(
          `SELECT courier_id, ROUND(AVG(rating)::numeric, 1) AS avg
             FROM order_ratings WHERE courier_id = ANY($1) GROUP BY courier_id`,
          [res.rows.map((r: any) => r.id)]
        );
        for (const x of rr.rows) ratingByCourier[x.courier_id] = Number(x.avg);
      } catch { await client.query('ROLLBACK TO SAVEPOINT r').catch(() => {}); }
      const rows = res.rows.map((r: any) => {
        let name = 'Unknown';
        try { name = r.full_name_encrypted ? (decryptPII(r.full_name_encrypted) || 'Unknown') : 'Unknown'; } catch {}
        return {
          id: r.id, name, phone: '',
          status: r.courier_status === 'available' ? 'online' : r.courier_status === 'on_delivery' ? 'busy' : 'offline',
          deliveriesCompleted: parseInt(r.deliveries_completed) || 0, rating: ratingByCourier[r.id] ?? 0,
        };
      });
      await client.query('COMMIT');
      return reply.send(rows);
    } catch (e) {
      await client.query('ROLLBACK').catch(() => {});
      throw e;
    } finally {
      client.release();
    }
  });

  // GET /api/public/theme/:slug
  fastify.get('/api/public/theme/:slug', async (request, reply) => {
    const slug = (request.params as any).slug;
    const locRes = await db.query(`SELECT id, name, supported_locales FROM locations WHERE slug = $1`, [slug]);
    if (locRes.rows.length === 0) return reply.status(404).send({ error: 'Not found' });
    const locId = locRes.rows[0].id;
    const locName = locRes.rows[0].name;
    const themeRes = await db.query(
      `SELECT primary_color, bg_color, text_color, logo_url FROM location_themes WHERE location_id = $1`,
      [locId]
    );
    const t = themeRes.rows[0] || {};
    return reply.send({
      primaryColor: t.primary_color || null, bgColor: t.bg_color || null,
      textColor: t.text_color || null, logoUrl: t.logo_url || null, locationName: locName,
      supportedLocales: Array.isArray(locRes.rows[0].supported_locales) ? locRes.rows[0].supported_locales : null,
    });
  });

  // GET /api/owner/brand
  fastify.get('/api/owner/brand', async (request, reply) => {
    const ctx = await getOwnerContext(request);
    if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await withTenant(db, ctx.userId, async (client) =>
      client.query(
        `SELECT primary_color, bg_color, text_color, logo_url, frame_ancestors,
                google_rating, google_review_count, google_maps_url
         FROM location_themes WHERE location_id = $1`,
        [ctx.locId]
      )
    );
    const t = res.rows[0] || {};
    return reply.send({
      id: ctx.locId,
      locationId: ctx.locId,
      primaryColor: t.primary_color || null,
      bgColor: t.bg_color || null,
      textColor: t.text_color || null,
      logoUrl: t.logo_url || null,
      frameAncestors: t.frame_ancestors || null,
      googleRating: t.google_rating != null ? Number(t.google_rating) : null,
      googleReviewCount: t.google_review_count != null ? Number(t.google_review_count) : null,
      googleMapsUrl: t.google_maps_url ?? null,
    });
  });

  // PUT /api/owner/brand — update theme
  fastify.put('/api/owner/brand', async (request, reply) => {
    const ctx = await getOwnerContext(request);
    if (!ctx) return reply.status(401).send({ error: 'Unauthorized' });
    const parsed = brandSchema.parse(request.body);
    const logoUrl = parsed.logoUrl ? validateImageKey(parsed.logoUrl) : null;
    const res = await withTenant(db, ctx.userId, async (client) => {
      await client.query(
        `INSERT INTO location_themes (location_id, primary_color, bg_color, text_color, logo_url, google_rating, google_review_count, google_maps_url)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
         ON CONFLICT (location_id) DO UPDATE SET
           primary_color = COALESCE($2, location_themes.primary_color),
           bg_color = COALESCE($3, location_themes.bg_color),
           text_color = COALESCE($4, location_themes.text_color),
           logo_url = COALESCE($5, location_themes.logo_url),
           google_rating = COALESCE($6, location_themes.google_rating),
           google_review_count = COALESCE($7, location_themes.google_review_count),
           google_maps_url = COALESCE($8, location_themes.google_maps_url)`,
        [ctx.locId, parsed.primaryColor || null, parsed.bgColor || null, parsed.textColor || null, logoUrl,
         parsed.googleRating ?? null, parsed.googleReviewCount ?? null, parsed.googleMapsUrl ?? null]
      );
      return client.query(
        `SELECT primary_color, bg_color, text_color, logo_url, google_rating, google_review_count, google_maps_url
         FROM location_themes WHERE location_id = $1`,
        [ctx.locId]
      );
    });
    const t = res.rows[0] || {};
    return reply.send({
      primaryColor: t.primary_color, bgColor: t.bg_color, textColor: t.text_color, logoUrl: t.logo_url,
      googleRating: t.google_rating != null ? Number(t.google_rating) : null,
      googleReviewCount: t.google_review_count != null ? Number(t.google_review_count) : null,
      googleMapsUrl: t.google_maps_url ?? null,
    });
  });

  // GET /api/owner/settings
  fastify.get('/api/owner/settings', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) {
      // A valid owner who hasn't created a location yet is a fresh signup that
      // belongs in onboarding — NOT an expired session. Returning 401 here makes
      // apiClient treat first-run as "session expired" and bounce to /login (O1).
      // Hand back a benign empty profile so AdminHome routes to /admin/onboarding.
      if (await isValidOwnerToken(request)) return reply.send({ id: null });
      return reply.status(401).send({ error: 'Unauthorized' });
    }
    const res = await db.query(
      `SELECT id, name, slug, phone, delivery_fee_flat, min_order_value, free_delivery_threshold, delivery_radius_km, currency_code, tax_rate, lat, lng, address, hours_json, delivery_paused
       FROM locations WHERE id = $1`,
      [locId]
    );
    if (!res.rows[0]) return reply.status(404).send({ error: 'Not found' });
    const r = res.rows[0];
    return reply.send({
      id: r.id, slug: r.slug, locationName: r.name, phone: r.phone || '',
      deliveryFee: Number(r.delivery_fee_flat) || 0,
      minOrder: Number(r.min_order_value) || 0,
      radiusKm: Number(r.delivery_radius_km) || 0,
      freeDeliveryThreshold: Number(r.free_delivery_threshold) || 0,
      currencyCode: r.currency_code || 'ALL',
      taxRate: Number(r.tax_rate) || 0,
      lat: r.lat !== null ? Number(r.lat) : null,
      lng: r.lng !== null ? Number(r.lng) : null,
      address: r.address || '',
      hoursJson: r.hours_json || {},
      deliveryPaused: r.delivery_paused ?? false,
    });
  });

  // PUT /api/owner/settings
  fastify.put('/api/owner/settings', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const parsed = settingsSchema.parse(request.body);
    const res = await db.query(
      `UPDATE locations SET
         name = COALESCE($1, name), phone = COALESCE($2, phone),
         delivery_fee_flat = COALESCE($3, delivery_fee_flat),
         min_order_value = COALESCE($4, min_order_value),
         delivery_radius_km = COALESCE($5, delivery_radius_km),
         free_delivery_threshold = COALESCE($6, free_delivery_threshold),
         tax_rate = COALESCE($7, tax_rate), lat = COALESCE($8, lat), lng = COALESCE($9, lng),
         address = COALESCE($10, address), hours_json = COALESCE($11, hours_json),
         delivery_paused = COALESCE($13, delivery_paused)
       WHERE id = $12
       RETURNING id, slug, name, phone, delivery_fee_flat, min_order_value, free_delivery_threshold, delivery_radius_km, tax_rate, lat, lng, address, hours_json, delivery_paused`,
      [parsed.locationName || null, parsed.phone || null,
       parsed.deliveryFee ?? null, parsed.minOrder ?? null, parsed.radiusKm ?? null,
       parsed.freeDeliveryThreshold ?? null, parsed.taxRate ?? null,
       parsed.lat ?? null, parsed.lng ?? null,
       parsed.address || null, parsed.hoursJson ? JSON.stringify(parsed.hoursJson) : null, locId,
       parsed.deliveryPaused ?? null]
    );
    const r = res.rows[0];
    if (!r) return reply.status(404).send({ error: 'Location not found' });
    return reply.send({
      id: r.id, slug: r.slug, locationName: r.name, phone: r.phone || '',
      deliveryFee: Number(r.delivery_fee_flat) || 0,
      minOrder: Number(r.min_order_value) || 0,
      radiusKm: Number(r.delivery_radius_km) || 0,
      freeDeliveryThreshold: Number(r.free_delivery_threshold) || 0,
      taxRate: Number(r.tax_rate) || 0,
      lat: r.lat !== null ? Number(r.lat) : null,
      lng: r.lng !== null ? Number(r.lng) : null,
      address: r.address || '',
      hoursJson: r.hours_json || {},
      deliveryPaused: r.delivery_paused ?? false,
    });
  });

  // POST /api/owner/courier-invites
  fastify.post('/api/owner/courier-invites', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { phone } = request.body as any;
    const code = crypto.randomUUID().substring(0, 8);
    const link = `https://${locId}.dowiz.org/courier/join?code=${code}`;
    return reply.send({ link, code, phone: phone || null });
  });

  // POST /api/owner/onboarding
  fastify.post('/api/owner/onboarding', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { name, phone, slug, lat, lng, delivery_radius_km, address, menu_items,
            courier_option, courier_phone, primary_color, logo_url, test_order_completed } = request.body as any;

    await db.query(
      `UPDATE locations SET name = COALESCE($1, name), phone = COALESCE($2, phone),
       lat = COALESCE($3, lat), lng = COALESCE($4, lng),
       delivery_radius_km = COALESCE($5, delivery_radius_km) WHERE id = $6`,
      [name || null, phone || null, lat ?? null, lng ?? null, delivery_radius_km ?? null, locId]
    );

    if (Array.isArray(menu_items)) {
      for (const item of menu_items) {
        if (item.name && item.price) {
          const pid = crypto.randomUUID();
          await db.query(
            `INSERT INTO products (id, location_id, name, price, description, is_available)
             VALUES ($1, $2, $3, $4, $5, true) ON CONFLICT DO NOTHING`,
            [pid, locId, item.name, item.price, item.description || null]
          );
        }
      }
    }

    const cleanLogo = logo_url ? validateImageKey(logo_url) : null;
    if (primary_color || cleanLogo) {
      await db.query(
        `INSERT INTO location_themes (location_id, primary_color, logo_url) VALUES ($1, $2, $3)
         ON CONFLICT (location_id) DO UPDATE SET primary_color = COALESCE($2, location_themes.primary_color), logo_url = COALESCE($3, location_themes.logo_url)`,
        [locId, primary_color || null, cleanLogo]
      );
    }

    return reply.send({ success: true, slug, url: `https://${slug}.dowiz.org` });
  });

  function formatRelativeTime(dateStr: string): string {
    const diff = Date.now() - new Date(dateStr).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return `${Math.floor(days / 7)}w ago`;
  }

  // GET /api/owner/customers
  fastify.get('/api/owner/customers', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(
      `SELECT c.id, c.name, c.phone, COUNT(o.id)::int as orders,
              COALESCE(SUM(o.total), 0)::int as ltv, MAX(o.created_at) as last_order_at
       FROM customers c LEFT JOIN orders o ON o.customer_id = c.id AND o.location_id = $1
       WHERE c.location_id = $1 GROUP BY c.id ORDER BY orders DESC LIMIT 50`,
      [locId]
    );
    return reply.send(res.rows.map((r: any) => ({
      id: r.id, name: r.name || 'Unknown', phone: maskStr(r.phone),
      orders: r.orders || 0, ltv: r.ltv || 0,
      lastOrder: r.last_order_at ? formatRelativeTime(r.last_order_at) : 'never',
    })));
  });

  // GET /api/owner/customers/:id/analytics
  fastify.get('/api/owner/customers/:id/analytics', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const cid = (request.params as any).id;
    const ordersRes = await db.query(
      `SELECT o.id, o.status, o.total, o.created_at, o.delivery_address,
              (SELECT jsonb_agg(jsonb_build_object('name', oi.name_snapshot, 'qty', oi.quantity, 'price', oi.price_snapshot))
               FROM order_items oi WHERE oi.order_id = o.id) as items
       FROM orders o WHERE o.customer_id = $1 AND o.location_id = $2
       ORDER BY o.created_at DESC LIMIT 20`,
      [cid, locId]
    );
    const prefsRes = await db.query(
      `SELECT oi.name_snapshot as name, SUM(oi.quantity)::int as total_qty,
              SUM(oi.price_snapshot * oi.quantity)::int as total_spent
       FROM order_items oi JOIN orders o ON o.id = oi.order_id
       WHERE o.customer_id = $1 AND o.location_id = $2
       GROUP BY oi.name_snapshot ORDER BY total_qty DESC LIMIT 10`,
      [cid, locId]
    );
    const heatmapRes = await db.query(
      `SELECT EXTRACT(DOW FROM o.created_at)::int as dow, EXTRACT(HOUR FROM o.created_at)::int as hour, COUNT(*)::int as cnt
       FROM orders o WHERE o.customer_id = $1 AND o.location_id = $2
       GROUP BY dow, hour ORDER BY cnt DESC`,
      [cid, locId]
    );
    return reply.send({ orders: ordersRes.rows, preferences: prefsRes.rows, heatmap: heatmapRes.rows });
  });
}

