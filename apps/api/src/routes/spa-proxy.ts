// @ts-nocheck
import type { FastifyInstance } from 'fastify';
import { jwtVerify } from 'jose';
import { loadEnv } from '@deliveryos/config';
import crypto from 'crypto';

const env = loadEnv();

function getPublicKey(): crypto.KeyObject {
  const raw = env.JWT_PUBLIC_KEY;
  if (!raw) throw new Error('JWT_PUBLIC_KEY missing');
  return crypto.createPublicKey(raw.replace(/\\n/g, '\n'));
}

export default async function spaProxyRoutes(fastify: FastifyInstance, opts: { db: any }) {
  const { db } = opts;

  async function getLocationId(request: any): Promise<string | null> {
    const auth = request.headers.authorization;
    if (!auth?.startsWith('Bearer ')) return null;
    try {
      const token = auth.slice(7);
      const { payload } = await jwtVerify(token, getPublicKey(), { algorithms: ['RS256'] });
      const claims = payload as any;
      if (claims.role !== 'owner') return null;
      if (claims.userId || claims.sub) {
        const uid = claims.userId || claims.sub;
        const res = await db.query(
          `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
          [uid]
        );
        if (res.rows.length > 0) return res.rows[0].location_id;
      }
      return null;
    } catch {
      return null;
    }
  }

  // GET /api/owner/menu/categories
  fastify.get('/api/owner/menu/categories', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(`SELECT id, name, sort_order FROM categories WHERE location_id = $1 ORDER BY sort_order`, [locId]);
    return reply.send(res.rows);
  });

  // POST /api/owner/menu/categories
  fastify.post('/api/owner/menu/categories', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { name } = request.body as any;
    const id = crypto.randomUUID();
    await db.query(`INSERT INTO categories (id, location_id, name) VALUES ($1, $2, $3)`, [id, locId, name]);
    return reply.status(201).send({ id, name });
  });

  // GET /api/owner/menu/products
  fastify.get('/api/owner/menu/products', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const catId = (request.query as any)?.category_id;
    let q = `SELECT id, name, price, is_available, category_id, description, image_key FROM products WHERE location_id = $1`;
    const params: any[] = [locId];
    if (catId) { q += ` AND category_id = $2`; params.push(catId); }
    q += ` ORDER BY sort_order`;
    const res = await db.query(q, params);
    return reply.send(res.rows);
  });

  // POST /api/owner/menu/products
  fastify.post('/api/owner/menu/products', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { name, price, description, available, category_id, image_key } = request.body as any;
    const id = crypto.randomUUID();
    await db.query(
      `INSERT INTO products (id, location_id, category_id, name, price, description, is_available, image_key) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)`,
      [id, locId, category_id, name, price || 0, description || null, available !== false, image_key || null]
    );
    return reply.status(201).send({ id, name, price });
  });

  // PATCH /api/owner/menu/products/:productId
  fastify.patch('/api/owner/menu/products/:productId', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const pid = (request.params as any).productId;
    const { name, price, description, available, category_id, image_key } = request.body as any;
    const sets: string[] = [];
    const vals: any[] = [];
    let idx = 1;
    if (name !== undefined) { sets.push(`name = $${idx++}`); vals.push(name); }
    if (price !== undefined) { sets.push(`price = $${idx++}`); vals.push(price); }
    if (description !== undefined) { sets.push(`description = $${idx++}`); vals.push(description); }
    if (available !== undefined) { sets.push(`is_available = $${idx++}`); vals.push(available); }
    if (category_id !== undefined) { sets.push(`category_id = $${idx++}`); vals.push(category_id); }
    if (image_key !== undefined) { sets.push(`image_key = $${idx++}`); vals.push(image_key); }
    if (sets.length === 0) return reply.status(400).send({ error: 'No fields to update' });
    vals.push(pid, locId);
    await db.query(`UPDATE products SET ${sets.join(', ')} WHERE id = $${idx++} AND location_id = $${idx++}`, vals);
    return reply.send({ id: pid });
  });

  // DELETE /api/owner/menu/products/:productId
  fastify.delete('/api/owner/menu/products/:productId', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const pid = (request.params as any).productId;
    await db.query(`DELETE FROM products WHERE id = $1 AND location_id = $2`, [pid, locId]);
    return reply.status(204).send();
  });

  // GET /api/owner/orders → dashboard snapshot
  fastify.get('/api/owner/orders', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(
      `SELECT o.id, o.status, o.created_at, o.total, o.delivery_address,
              c.name as customer_name, c.phone as customer_phone
       FROM orders o
       LEFT JOIN customers c ON c.id = o.customer_id
       WHERE o.location_id = $1 AND o.status NOT IN ('DELIVERED','CANCELLED','REJECTED')
       ORDER BY o.created_at DESC LIMIT 20`,
      [locId]
    );
    const orders = res.rows.map((r: any) => ({
      id: r.id,
      status: r.status,
      createdAt: r.created_at,
      total: r.total,
      customerName: r.customer_name || 'Unknown',
      shortId: '#' + r.id.toString().substring(0, 4),
      itemsSummary: '',
      items: [],
      itemCount: 0,
      customerPhone: r.customer_phone || '',
      etaMinutes: null,
      elapsedSeconds: 0,
      courierName: null,
    }));
    return reply.send(orders);
  });

  // GET /api/owner/couriers
  fastify.get('/api/owner/couriers', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(
      `SELECT u.id, u.display_name, u.phone, m.status FROM users u JOIN memberships m ON m.user_id = u.id WHERE m.location_id = $1 AND m.role = 'courier'`,
      [locId]
    );
    return reply.send(res.rows);
  });

  // GET /api/owner/brand → theme
  fastify.get('/api/owner/brand', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(`SELECT primary_color, bg_color, text_color, logo_url FROM location_themes WHERE location_id = $1`, [locId]);
    return reply.send(res.rows[0] || {});
  });

  // PUT /api/owner/brand → update theme
  fastify.put('/api/owner/brand', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { primaryColor, bgColor, logoUrl } = request.body as any;
    await db.query(
      `INSERT INTO location_themes (location_id, primary_color, bg_color, logo_url) VALUES ($1,$2,$3,$4)
       ON CONFLICT (location_id) DO UPDATE SET primary_color = COALESCE($2, location_themes.primary_color), bg_color = COALESCE($3, location_themes.bg_color), logo_url = COALESCE($4, location_themes.logo_url)`,
      [locId, primaryColor || null, bgColor || null, logoUrl || null]
    );
    return reply.send({ ok: true });
  });

  // GET /api/owner/settings
  fastify.get('/api/owner/settings', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(`SELECT name, phone, delivery_fee_flat, min_order_value, free_delivery_threshold, delivery_radius_km FROM locations WHERE id = $1`, [locId]);
    if (!res.rows[0]) return reply.status(404).send({ error: 'Not found' });
    return reply.send(res.rows[0]);
  });

  // PUT /api/owner/settings
  fastify.put('/api/owner/settings', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { locationName, phone, address, deliveryFee, minOrder, radiusKm } = request.body as any;
    await db.query(
      `UPDATE locations SET name = COALESCE($1, name), phone = COALESCE($2, phone), delivery_fee_flat = COALESCE($3, delivery_fee_flat), min_order_value = COALESCE($4, min_order_value), delivery_radius_km = COALESCE($5, delivery_radius_km) WHERE id = $6`,
      [locationName || null, phone || null, deliveryFee ?? null, minOrder ?? null, radiusKm ?? null, locId]
    );
    return reply.send({ ok: true });
  });

  // POST /api/owner/courier-invites — onboarding courier invite
  fastify.post('/api/owner/courier-invites', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { phone } = request.body as any;
    const code = crypto.randomUUID().substring(0, 8);
    const link = `https://${locId}.dowiz.org/courier/join?code=${code}`;
    return reply.send({ link, code });
  });

  // POST /api/owner/onboarding — complete onboarding
  fastify.post('/api/owner/onboarding', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { name, phone, slug, lat, lng, delivery_radius_km, address, menu_items,
            courier_option, courier_phone, primary_color, logo_url, test_order_completed } = request.body as any;

    // Update location
    await db.query(
      `UPDATE locations SET name = COALESCE($1, name), phone = COALESCE($2, phone),
       lat = COALESCE($3, lat), lng = COALESCE($4, lng),
       delivery_radius_km = COALESCE($5, delivery_radius_km) WHERE id = $6`,
      [name || null, phone || null, lat ?? null, lng ?? null, delivery_radius_km ?? null, locId]
    );

    // Add menu items if provided
    if (Array.isArray(menu_items)) {
      for (const item of menu_items) {
        if (item.name && item.price) {
          const pid = crypto.randomUUID();
          await db.query(
            `INSERT INTO products (id, location_id, name, price) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING`,
            [pid, locId, item.name, item.price]
          );
        }
      }
    }

    // Branding
    if (primary_color || logo_url) {
      await db.query(
        `INSERT INTO location_themes (location_id, primary_color, logo_url) VALUES ($1, $2, $3)
         ON CONFLICT (location_id) DO UPDATE SET primary_color = COALESCE($2, location_themes.primary_color), logo_url = COALESCE($3, location_themes.logo_url)`,
        [locId, primary_color || null, logo_url || null]
      );
    }

    return reply.send({ success: true, slug, url: `https://${slug}.dowiz.org` });
  });
}
