import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { QUEUE_NAMES } from '../../lib/registry.js';

// POST /api/access-requests — public "register interest" capture (ADR-soft-access-gate).
// This route is ONLY registered when ACCESS_GATE_PUBLIC_ENABLED=true (server.ts); while
// the flag is off the path 404s via setNotFoundHandler — the STOP-1 reachable-surface gate.
//
// Anti-enumeration invariant (R3-3b): indistinguishable along the email-existence axis —
// new / duplicate / honeypot / no-consent / malformed-email all return byte-identical
// 200 {ok:true}; new vs duplicate both run the SAME INSERT (enqueue is off the reply path),
// so the response never reveals whether an email is already stored.

const env = loadEnv();

const FLY_IP_HEADER = 'fly-client-ip';
let lastFlyMissingWarnAt = 0;

/**
 * Real client IP — reads `Fly-Client-IP` ONLY (R2-2). The Fly edge sets & overwrites it
 * on every request, so it is not client-injectable (unlike X-Forwarded-For, whose trust
 * fallthrough is deliberately REMOVED). Non-prod degrades to request.ip (deterministic,
 * never the client-controlled XFF). Prod with no header → fail closed to a single shared
 * bucket (degrade, never trust a spoofable header) + a throttled re-warn ≤1/min (R3-2).
 */
export function clientIp(request: any): string {
  const fly = request?.headers?.[FLY_IP_HEADER];
  if (typeof fly === 'string' && fly.length > 0) return fly;
  if (process.env.NODE_ENV !== 'production') {
    return request?.ip ?? 'unknown';
  }
  const now = Date.now();
  if (now - lastFlyMissingWarnAt > 60_000) {
    lastFlyMissingWarnAt = now;
    request?.log?.warn?.('[access-requests] Fly-Client-IP missing in production — rate-limit degraded to a shared bucket');
  }
  return 'shared:no-fly-ip';
}

function hashIp(ip: string): string {
  return crypto.createHash('sha256').update(ip).digest('hex').slice(0, 16);
}

// Structural lawful-basis + honeypot gate (R2-8). `consent` MUST be the literal boolean
// true — "true" / 1 / "false" / missing all FAIL structurally (a hand `if (body.consent)`
// would let the truthy string through). On fail the route takes the silent uniform-200
// honeypot path (R2-3): no INSERT, no consent_at, no enqueue.
const ControlFields = z.object({
  consent: z.literal(true),
  website: z.string().max(0).optional(), // honeypot: any non-empty value fails the parse
  locale: z.string().max(8).optional(),
});
// Email is parsed SEPARATELY and leniently (never gated) so email-existence stays un-enumerated.
const emailSchema = z.string().email().max(320);

// A cheap DB round-trip to keep the no-op (honeypot / no-consent / bad-email) path's
// latency in the same ballpark as a real INSERT — blunts a fast-reject timing oracle.
async function cheapNoOp(db: any): Promise<void> {
  if (!db) return;
  try {
    const c = await db.connect();
    try { await c.query('SELECT 1'); } finally { c.release(); }
  } catch { /* timing parity is best-effort; never surface */ }
}

export default (async function accessRequestRoutes(fastify: any, opts: any) {
  const { db, queue } = opts;

  fastify.post('/api/access-requests', {
    config: {
      // Per-IP 5/min keyed by the REAL client IP (Fly-Client-IP only). Overrides the
      // global 100/min limiter for this PII-capture endpoint.
      rateLimit: {
        max: 5,
        timeWindow: '1 minute',
        keyGenerator: (request: any) => clientIp(request),
      },
    },
  }, async (request: any, reply: any) => {
    const body = (request.body ?? {}) as Record<string, unknown>;

    // 1. Structural consent + honeypot gate → silent uniform 200 on fail.
    const control = ControlFields.safeParse(body);
    if (!control.success) {
      await cheapNoOp(db);
      return reply.code(200).send({ ok: true });
    }

    // 2. Lenient, separate email parse → silent uniform 200 on a malformed email.
    const rawEmail = typeof body.email === 'string' ? body.email.trim().toLowerCase() : '';
    const emailParse = emailSchema.safeParse(rawEmail);
    if (!emailParse.success) {
      await cheapNoOp(db);
      return reply.code(200).send({ ok: true });
    }
    const email = emailParse.data;

    // 3. INSERT (the only blocking dependency on the user path). ON CONFLICT(email)
    //    DO NOTHING → duplicate returns zero rows (no enqueue); same statement, same
    //    latency on the new and duplicate paths.
    let insertedId: string | null = null;
    if (db) {
      const client = await db.connect();
      try {
        const res = await client.query(
          `INSERT INTO access_requests (email, source, locale, ip_hash, consent_at, privacy_version)
           VALUES ($1, $2, $3, $4, now(), $5)
           ON CONFLICT (email) DO NOTHING
           RETURNING id`,
          [
            email,
            typeof body.source === 'string' ? body.source : null,
            control.data.locale ?? null,
            hashIp(clientIp(request)),
            env.PRIVACY_NOTICE_VERSION,
          ],
        );
        insertedId = res.rows[0]?.id ?? null;
      } finally {
        client.release();
      }
    }

    // 4. Reply FIRST (B5) — response latency is INSERT-only on every path.
    reply.code(200).send({ ok: true });

    // 5. Fire-and-forget enqueue AFTER the reply, only for a genuinely new row. A lost
    //    enqueue (crash between commit and send) is recovered by access-request.reconcile.
    if (insertedId && queue?.boss) {
      queue.boss
        .send(QUEUE_NAMES.ACCESS_REQUEST_NOTIFY, { requestId: insertedId })
        .catch((err: any) =>
          request.log?.warn?.(`[access-requests] enqueue failed (sweep will recover): ${err?.message}`),
        );
    }
    return reply;
  });
}) as FastifyPluginAsync<any>;
