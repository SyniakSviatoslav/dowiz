import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { auditCtx, auditCompleted } from '../../lib/platform-admin.js';

export default (async function fallbackAdminRoutes(fastify: any, opts: any) {
  const { db } = opts;

  // ADR-admin-platform-authz (B4): auth is the platform-admin gate on the parent plane
  // (routes/admin/index.ts) + the root-instance gate in server.ts — NOT a per-file owner check.

  // ─── GET Fallback Health Overview ────────────────────────────────
  fastify.get('/fallback/health', async (request: any, reply: any) => {
    const locationsRes = await db.query(
      `SELECT l.id, l.name, l.slug, l.phone AS public_phone, l.fallback_config,
              COUNT(ont.id) FILTER (WHERE ont.channel = 'telegram' AND ont.status = 'active') AS telegram_active,
              COUNT(ont.id) FILTER (WHERE ont.channel = 'push' AND ont.status = 'active') AS push_active,
              COUNT(ont.id) FILTER (WHERE ont.last_error IS NOT NULL) AS dead_channels
       FROM locations l
       LEFT JOIN owner_notification_targets ont ON ont.location_id = l.id
       GROUP BY l.id
       ORDER BY l.name
       LIMIT 100`,
    );

    const locations = locationsRes.rows.map((row: any) => {
      const config = row.fallback_config || {};
      return {
        id: row.id,
        name: row.name,
        slug: row.slug,
        publicPhone: row.public_phone,
        fallbackPhone: config.phone || null,
        showPhoneOnError: config.show_phone_on_error !== false,
        showPhoneOnOffline: config.show_phone_on_offline !== false,
        telegramActive: parseInt(row.telegram_active, 10),
        pushActive: parseInt(row.push_active, 10),
        deadChannels: parseInt(row.dead_channels, 10),
      };
    });

    await auditCompleted(db, auditCtx(request, 'fallback.health'), request.log);
    return reply.send({ locations });
  });

  // ─── Trigger R2 fallback coverage check ──────────────────────────
  fastify.post('/fallback/r2-check', async (request: any, reply: any) => {
    const res = await db.query(
      `SELECT COUNT(*)::int AS total_locations,
              COUNT(*) FILTER (WHERE fallback_config->>'phone' IS NOT NULL AND fallback_config->>'phone' != '') AS with_fallback_phone
       FROM locations`,
    );
    const { total_locations, with_fallback_phone } = res.rows[0];
    await auditCompleted(db, auditCtx(request, 'fallback.r2_check'), request.log);
    return reply.send({
      totalLocations: total_locations,
      withFallbackPhone: with_fallback_phone,
      coveragePct: total_locations > 0 ? Math.round((with_fallback_phone / total_locations) * 100) : 0,
    });
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
