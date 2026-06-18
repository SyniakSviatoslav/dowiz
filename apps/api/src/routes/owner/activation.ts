import { z } from 'zod';

// Menu-first onboarding — publish gate + draft→publish transition (O1).
//
// Gate trinity (§4): menu confirmed (human-review, Z2) · notifications connected
// (Telegram ops) · fulfillment path (pickup OR ≥1 courier) + a base phone.
// Publish (§5) stamps published_at + flips status to open + bumps menu_version so the
// SSR /s/:slug cache (Stage 13) refreshes. Draft (published_at NULL) is preview-only
// and is rejected at order creation (Z7, enforced in orders.ts).

const GATE_SQL = `
  SELECT
    l.slug,
    l.menu_version,
    (l.published_at IS NOT NULL)                                   AS published,
    l.published_at,
    (l.menu_confirmed_at IS NOT NULL)                             AS menu_confirmed,
    EXISTS (SELECT 1 FROM owner_notification_targets o
            WHERE o.location_id = l.id AND o.status = 'active')   AS notifications_connected,
    (l.pickup_enabled
       OR EXISTS (SELECT 1 FROM courier_locations cl WHERE cl.location_id = l.id)) AS has_fulfillment,
    (l.phone IS NOT NULL AND length(btrim(l.phone)) > 0)          AS has_phone
  FROM locations l WHERE l.id = $1
`;

function buildGate(row: any) {
  const menuConfirmed = !!row.menu_confirmed;
  const notificationsConnected = !!row.notifications_connected;
  const fulfillmentReady = !!row.has_fulfillment && !!row.has_phone;
  const missing: Array<{ key: string; message: string }> = [];
  if (!menuConfirmed) missing.push({ key: 'menu', message: 'Confirm your menu (review prices & allergens)' });
  if (!notificationsConnected) missing.push({ key: 'notifications', message: 'Connect notifications so you see new orders (Telegram)' });
  if (!fulfillmentReady) {
    missing.push({ key: 'fulfillment', message: !row.has_phone
      ? 'Add a contact phone, and enable pickup or add a courier'
      : 'Enable pickup or add at least one courier' });
  }
  return {
    published: !!row.published,
    publishedAt: row.published_at ?? null,
    slug: row.slug,
    menuVersion: row.menu_version,
    gate: { menuConfirmed, notificationsConnected, fulfillmentReady },
    canPublish: menuConfirmed && notificationsConnected && fulfillmentReady,
    missing,
  };
}

export default (async function activationRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));

  // ─── Gate readiness (drives the checklist + Publish button) ──────────────
  fastify.get('/activation/:locationId/status', {
    schema: { params: z.object({ locationId: z.string().uuid() }) },
    preHandler: [fastify.requireLocationAccess],
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const res = await db.query(GATE_SQL, [locationId]);
    if (res.rowCount === 0) return reply.status(404).send({ error: 'Location not found' });
    return reply.send(buildGate(res.rows[0]));
  });

  // ─── Publish (draft → live), gated server-side ───────────────────────────
  fastify.post('/activation/:locationId/publish', {
    schema: { params: z.object({ locationId: z.string().uuid() }) },
    preHandler: [fastify.requireLocationAccess],
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const client = await db.connect();
    try {
      await client.query('BEGIN');
      const res = await client.query(GATE_SQL, [locationId]); // publish is idempotent (COALESCE)
      if (res.rowCount === 0) { await client.query('ROLLBACK'); return reply.status(404).send({ error: 'Location not found' }); }
      const gate = buildGate(res.rows[0]);
      if (!gate.canPublish) {
        await client.query('ROLLBACK');
        return reply.status(422).send({ error: 'NOT_READY_TO_PUBLISH', code: 'NOT_READY_TO_PUBLISH', missing: gate.missing });
      }

      // First publish stamps published_at; re-publish is idempotent. Flip the daily
      // switch to open so the storefront accepts orders immediately.
      await client.query(
        `UPDATE locations SET published_at = COALESCE(published_at, now()), status = 'open' WHERE id = $1`,
        [locationId],
      );
      // Bump the SSR menu cache key (Stage 13) so the live page reflects publish.
      await client.query(`SELECT upsert_menu_version($1)`, [locationId]);
      await client.query('COMMIT');

      return reply.send({ published: true, slug: gate.slug, url: `/s/${gate.slug}` });
    } catch (err) {
      await client.query('ROLLBACK').catch(() => {});
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal server error' });
    } finally {
      client.release();
    }
  });
} as any);
