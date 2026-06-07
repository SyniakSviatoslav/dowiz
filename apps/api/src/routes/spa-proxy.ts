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

  // GET /api/owner/orders → full order history with items
  fastify.get('/api/owner/orders', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    
    const statusFilter = (request.query as any)?.status;
    let statusClause = '';
    const params: any[] = [locId];
    if (statusFilter && statusFilter !== 'all') {
      statusClause = 'AND o.status = $2';
      params.push(statusFilter);
    }
    
    const res = await db.query(
      `SELECT o.id, o.status, o.created_at, o.confirmed_at, o.total, o.subtotal,
              o.delivery_fee, o.delivery_address, o.payment_method,
              c.name as customer_name, c.phone as customer_phone,
              (SELECT jsonb_agg(jsonb_build_object('name', oi.name_snapshot, 'qty', oi.quantity, 'price', oi.price_snapshot))
               FROM order_items oi WHERE oi.order_id = o.id) as items,
              EXTRACT(EPOCH FROM (COALESCE(o.confirmed_at, now()) - o.created_at))::int as confirm_seconds,
              COALESCE(cr.display_name, u.display_name) as courier_name
       FROM orders o
       LEFT JOIN customers c ON c.id = o.customer_id
       LEFT JOIN courier_assignments ca ON ca.order_id = o.id AND ca.status IN ('accepted','picked_up','delivered')
       LEFT JOIN users cr ON cr.id = ca.courier_id
       LEFT JOIN memberships m2 ON m2.user_id = cr.id
       LEFT JOIN users u ON u.id = m2.user_id
       WHERE o.location_id = $1 ${statusClause}
       ORDER BY o.created_at DESC LIMIT 50`,
      params
    );
    
    const orders = res.rows.map((r: any) => ({
      id: r.id,
      status: r.status,
      createdAt: r.created_at,
      confirmedAt: r.confirmed_at,
      total: r.total,
      subtotal: r.subtotal,
      deliveryFee: r.delivery_fee,
      deliveryAddress: r.delivery_address,
      paymentMethod: r.payment_method,
      customerName: r.customer_name || 'Unknown',
      customerPhone: r.customer_phone || '',
      shortId: '#' + r.id.toString().substring(0, 4),
      items: r.items || [],
      itemCount: r.items ? r.items.length : 0,
      itemsSummary: r.items ? r.items.map((i: any) => `${i.name} x${i.qty}`).join(', ') : '',
      confirmSeconds: r.confirm_seconds,
      courierName: r.courier_name || null,
      elapsedSeconds: Math.floor((Date.now() - new Date(r.created_at).getTime()) / 1000),
    }));
    
    return reply.send(orders);
  });

  // GET /api/owner/couriers
  fastify.get('/api/owner/couriers', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(
      `SELECT u.id, COALESCE(u.display_name, u.email) as display_name, u.phone,
              COALESCE(cs.status, 'offline') as courier_status,
              (SELECT COUNT(*) FROM courier_assignments ca2 WHERE ca2.courier_id = u.id AND ca2.status = 'delivered') as deliveries_completed
       FROM users u
       JOIN memberships m ON m.user_id = u.id
       LEFT JOIN courier_shifts cs ON cs.courier_id = u.id AND cs.status != 'offline'
       WHERE m.location_id = $1 AND m.role = 'courier'
       GROUP BY u.id, cs.status`,
      [locId]
    );
    const couriers = res.rows.map((r: any) => ({
      id: r.id,
      name: r.display_name || 'Unknown',
      phone: r.phone || '',
      status: r.courier_status === 'available' ? 'online' : r.courier_status === 'on_delivery' ? 'busy' : 'offline',
      deliveriesCompleted: parseInt(r.deliveries_completed) || 0,
      rating: 0,
    }));
    return reply.send(couriers);
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
    const res = await db.query(`SELECT name, phone, delivery_fee_flat, min_order_value, free_delivery_threshold, delivery_radius_km, currency_code, tax_rate FROM locations WHERE id = $1`, [locId]);
    if (!res.rows[0]) return reply.status(404).send({ error: 'Not found' });
    const r = res.rows[0];
    return reply.send({
      locationName: r.name,
      phone: r.phone || '',
      address: r.public_address || '',
      deliveryFee: r.delivery_fee_flat || 0,
      minOrder: r.min_order_value || 0,
      radiusKm: r.delivery_radius_km || 0,
      freeDeliveryThreshold: r.free_delivery_threshold || 0,
      currencyCode: r.currency_code || 'ALL',
      taxRate: r.tax_rate || 0,
    });
  });

  // PUT /api/owner/settings
  fastify.put('/api/owner/settings', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const { locationName, phone, address, deliveryFee, minOrder, radiusKm, freeDeliveryThreshold, taxRate } = request.body as any;
    await db.query(
      `UPDATE locations SET name = COALESCE($1, name), phone = COALESCE($2, phone),
       delivery_fee_flat = COALESCE($3, delivery_fee_flat), min_order_value = COALESCE($4, min_order_value),
       delivery_radius_km = COALESCE($5, delivery_radius_km), free_delivery_threshold = COALESCE($6, free_delivery_threshold),
       tax_rate = COALESCE($7, tax_rate) WHERE id = $8`,
      [locationName || null, phone || null, deliveryFee ?? null, minOrder ?? null, radiusKm ?? null, freeDeliveryThreshold ?? null, taxRate ?? null, locId]
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

  function formatRelativeTime(dateStr: string): string {
    const diff = Date.now() - new Date(dateStr).getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    const weeks = Math.floor(days / 7);
    if (weeks < 4) return `${weeks}w ago`;
    const months = Math.floor(days / 30);
    return `${months}mo ago`;
  }

  // GET /api/owner/customers
  fastify.get('/api/owner/customers', async (request, reply) => {
    const locId = await getLocationId(request);
    if (!locId) return reply.status(401).send({ error: 'Unauthorized' });
    const res = await db.query(
      `SELECT c.id, c.name, c.phone, COUNT(o.id)::int as orders,
              COALESCE(SUM(o.total), 0)::int as ltv,
              MAX(o.created_at) as last_order_at
       FROM customers c
       LEFT JOIN orders o ON o.customer_id = c.id AND o.location_id = $1
       WHERE c.location_id = $1
       GROUP BY c.id
       ORDER BY orders DESC LIMIT 50`,
      [locId]
    );
    const customers = res.rows.map((r: any) => ({
      id: r.id,
      name: r.name || 'Unknown',
      phone: r.phone ? r.phone.substring(0, 8) + '***' : '',
      orders: r.orders || 0,
      ltv: r.ltv || 0,
      lastOrder: r.last_order_at ? formatRelativeTime(r.last_order_at) : 'never',
    }));
    return reply.send(customers);
  });

  // GET /api/owner/customers/:id/analytics — order history + preferences
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
       FROM order_items oi
       JOIN orders o ON o.id = oi.order_id
       WHERE o.customer_id = $1 AND o.location_id = $2
       GROUP BY oi.name_snapshot
       ORDER BY total_qty DESC LIMIT 10`,
      [cid, locId]
    );

    const heatmapRes = await db.query(
      `SELECT EXTRACT(DOW FROM o.created_at)::int as dow,
              EXTRACT(HOUR FROM o.created_at)::int as hour,
              COUNT(*)::int as cnt
       FROM orders o WHERE o.customer_id = $1 AND o.location_id = $2
       GROUP BY dow, hour ORDER BY cnt DESC`,
      [cid, locId]
    );

    return reply.send({
      orders: ordersRes.rows,
      preferences: prefsRes.rows,
      heatmap: heatmapRes.rows,
    });
  });
}
