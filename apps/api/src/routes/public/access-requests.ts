import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';
import crypto from 'node:crypto';
import { loadEnv } from '@deliveryos/config';
import { QUEUE_NAMES } from '../../lib/registry.js';
import { rejectReservedTld } from '../../lib/synthetic-courier.js';
import { clientIp } from '../../lib/client-ip.js';

// POST /api/access-requests — public "register interest" capture (ADR-soft-access-gate).
// This route is ONLY registered when ACCESS_GATE_PUBLIC_ENABLED=true (server.ts); while
// the flag is off the path 404s via setNotFoundHandler — the STOP-1 reachable-surface gate.
//
// Anti-enumeration invariant (R3-3b): indistinguishable along the email-existence axis —
// new / duplicate / honeypot / no-consent / malformed-email all return byte-identical
// 200 {ok:true}; new vs duplicate both run the SAME INSERT (enqueue is off the reply path),
// so the response never reveals whether an email is already stored.

const env = loadEnv();

// #9: real client-IP resolution moved to the shared lib/client-ip.ts (Fly-Client-IP only,
// never X-Forwarded-For; IPv6-normalized; prod fail-closed). One source of truth; re-exported
// for backwards-compatible importers/tests.
export { clientIp };

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
// The reserved-TLD reject (.test/.example/.invalid/.localhost — registration-namespace hygiene,
// constraint #4) chains here: a reserved-TLD email simply fails the parse and takes the SAME
// silent uniform-200 honeypot no-op path below (no INSERT, byte-identical reply), so the
// anti-enumeration invariant (R3-3b) is preserved — NOT a 400.
const emailSchema = z.string().email().max(320).refine(rejectReservedTld[0], rejectReservedTld[1]);

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
