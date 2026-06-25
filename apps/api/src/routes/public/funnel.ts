import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';
import { loadEnv } from '@deliveryos/config';
import { clientIp } from './access-requests.js';

// POST /api/funnel — SENSOR-BUS §1.3 anonymous storefront-funnel ingest (ADR-0009).
//
// Pure observation (brief §0.1 observe-don't-control): a lost/blocked funnel row NEVER affects a
// sale. Anonymous — no auth, no PII. `session_ref` is an opaque client-minted id (NOT a customer id,
// NOT a phone); it is never written onto an order and the FE rotates it at order submission, so the
// pre-order funnel session and the order are unlinkable (Breaker M2). RLS FORCE on funnel_events +
// REVOKE anon/authenticated/service_role keeps it off the Supabase Data API; the INSERT lands via the
// `app.current_tenant` arm of the table's WITH CHECK (the anonymous public writer has no member
// context). A forged location_id only pollutes that tenant's advisory funnel (FK-validated, never an
// actuator) — the accepted residual (proposal §7); the per-IP rate-limit + uniform 204 blunt floods
// and enumeration. Kill-switch: FUNNEL_INGEST_ENABLED='false' silences it without a deploy.

const env = loadEnv();

const funnelSchema = z
  .object({
    locationId: z.string().uuid(),
    sessionRef: z.string().min(1).max(128),
    eventType: z.enum(['menu_view', 'add_to_cart', 'checkout_start', 'checkout_abandon']),
    shownEtaLoMin: z.number().int().min(0).max(1440).optional(),
    shownEtaHiMin: z.number().int().min(0).max(1440).optional(),
  })
  .strict();

export default (async function funnelRoutes(fastify: any, opts: any) {
  const { db } = opts as any;
  const enabled = env.FUNNEL_INGEST_ENABLED !== 'false';

  fastify.post('/api/funnel', {
    config: {
      // Per-IP 60/min keyed by the REAL client IP (Fly-Client-IP only) — generous for one real
      // browsing session, lethal to a flood (Breaker H4). Overrides the global limiter.
      rateLimit: {
        max: 60,
        timeWindow: '1 minute',
        keyGenerator: (request: any) => clientIp(request),
      },
    },
  }, async (request: any, reply: any) => {
    // Uniform 204 regardless of validity / enabled-state / outcome — anti-enumeration + the funnel
    // never reveals whether a payload was accepted. Reply is sent on EVERY path.
    const parsed = funnelSchema.safeParse(request.body ?? {});
    if (!enabled || !parsed.success || !db) {
      return reply.code(204).send();
    }
    const d = parsed.data;

    // Best-effort INSERT — a failure (bad location FK / RLS / pool hiccup) is logged and dropped;
    // the funnel is observation and must never surface an error to the storefront. Tenant scoped via
    // app.current_tenant so the FORCE-RLS WITH CHECK passes for the anonymous writer.
    try {
      const client = await db.connect();
      try {
        await client.query('BEGIN');
        await client.query(`SELECT set_config('app.current_tenant', $1, true)`, [d.locationId]);
        await client.query(
          `INSERT INTO funnel_events (location_id, session_ref, event_type, shown_eta_lo_min, shown_eta_hi_min)
           VALUES ($1, $2, $3, $4, $5)`,
          [d.locationId, d.sessionRef, d.eventType, d.shownEtaLoMin ?? null, d.shownEtaHiMin ?? null],
        );
        await client.query('COMMIT');
      } catch (err) {
        try { await client.query('ROLLBACK'); } catch { /* no tx */ }
        request.log?.warn?.(`[funnel] ingest dropped: ${(err as any)?.message}`);
      } finally {
        client.release();
      }
    } catch (err) {
      request.log?.warn?.(`[funnel] connect failed (dropped): ${(err as any)?.message}`);
    }

    return reply.code(204).send();
  });
}) as FastifyPluginAsync<any>;
