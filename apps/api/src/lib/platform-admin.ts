import type { FastifyRequest, FastifyReply, FastifyInstance } from 'fastify';
import type { Pool } from 'pg';
import crypto from 'node:crypto';

/**
 * ADR-admin-platform-authz (B4) — the platform-admin authority.
 *
 * Authority is a SERVER-SIDE fact in the `platform_admins` allowlist (a non-tenant GLOBAL table,
 * NO RLS — protected by table GRANTs), re-read on EVERY admin request. There is no platform-admin
 * claim in any token (no forgeable claim, no mint site, no discriminatedUnion change). Setting
 * `revoked_at` takes effect on the next request → immediate insider-removal (mirrors ADR-0004).
 *
 * The point-read returns identical results under BYPASSRLS (today) and NOBYPASSRLS (post-B3) — the
 * table has no RLS, so a NOBYPASSRLS role with `GRANT SELECT` reads every row. Genuinely B3-independent
 * (no GUC, no DEFINER fn). Canonical principal field = `request.user.userId` (owner mint sets
 * `sub == userId`; the rest of the auth layer keys on `userId`).
 */

/** The plain indexed point-read. Throws on DB error → the hook maps that to 503 (fail CLOSED). */
export async function isPlatformAdmin(pool: Pool, userId: string): Promise<boolean> {
  const r = await pool.query(
    `SELECT 1 FROM platform_admins WHERE user_id = $1 AND revoked_at IS NULL`,
    [userId],
  );
  return (r.rowCount ?? 0) > 0;
}

/**
 * onRequest gate. MUST run AFTER `verifyAuth` (so `request.user` is populated). A non-allowlisted
 * principal → 403; a DB blip on the re-check → 503 (fail CLOSED — never fail-open at the top tier);
 * a request with no resolvable `userId` (courier/customer token, or no token) → 401.
 */
export async function requirePlatformAdmin(request: FastifyRequest, reply: FastifyReply): Promise<void> {
  const userId = (request.user as { userId?: string } | null)?.userId;
  if (!userId) {
    // No authenticated user-principal (courier/customer tokens carry no userId, or unauthenticated).
    reply.status(401).send({ error: 'Unauthorized' });
    return;
  }
  const pool = request.server.db;
  let ok: boolean;
  try {
    ok = await isPlatformAdmin(pool, userId);
  } catch (err) {
    request.log.error({ err }, '[platform-admin] re-check failed — failing CLOSED (503)');
    reply.status(503).send({ error: 'admin_unavailable' }); // fail CLOSED — no fail-open at the top tier
    return;
  }
  if (!ok) {
    reply.status(403).send({ error: 'Forbidden' });
    return;
  }
  // allowlisted + not revoked → fall through (gate passes)
}

/**
 * The plane predicate for the ROOT-instance gate: is this request routed to an `/api/admin` handler?
 * Keys on the MATCHED ROUTE PATTERN (`request.routeOptions.url`), NOT the raw URL — find-my-way has
 * already decoded/normalized/rewritten the path, so case / `%2e` / `%2f` / trailing-slash tricks
 * either route to the `/api/admin/…` pattern (→ gated) or to no admin handler (→ 404). The boundary
 * (`/` or end) excludes the `/api/administrators` lookalike. `undefined` = 404 (no route matched).
 */
export function isAdminRoutedPath(request: FastifyRequest): boolean {
  const u = request.routeOptions?.url;
  if (u === undefined) return false;
  return u === '/api/admin' || u.startsWith('/api/admin/');
}

/**
 * Register the ROOT-instance structural authority (ADR §3.5, RESOLVE round 3). A root `onRequest`
 * hook flows into EVERY route context by construction, so it gates every `/api/admin*` matched route
 * — child, sibling, or future — with zero detection (the route-tree boot-guard was unrealizable:
 * Fastify introspection can't see context-inherited hooks). For non-admin requests it is one
 * string-predicate check then an immediate return (no DB, no allocation) — negligible on the hot path.
 */
export function registerAdminPlaneGate(fastify: FastifyInstance): void {
  fastify.addHook('onRequest', async (request, reply) => {
    if (!isAdminRoutedPath(request)) return;
    await fastify.verifyAuth(request, reply); // populates request.user; 401 short-circuits (reply.sent)
    if (reply.sent) return;
    await requirePlatformAdmin(request, reply);
  });
}

// ── platform_admin_audit_log (ADR §4.4/§6) — actor trail, hashed ip/ua only (no raw PII) ──

const sha256 = (s: string | undefined): string | null =>
  s ? crypto.createHash('sha256').update(s).digest('hex') : null;

export interface AuditCtx { actorId: string; action: string; target: string | null; ipHash: string | null; uaHash: string | null; }

/**
 * Build an audit context from the gated request (the platform-admin is `request.user.userId`).
 * Structural param (not `FastifyRequest`) so it accepts any route's request generic (e.g. the
 * ZodTypeProvider-typed handlers) without a type-argument mismatch.
 */
export function auditCtx(
  request: { user: unknown; ip: string; headers: Record<string, string | string[] | undefined> },
  action: string,
  target?: string | null,
): AuditCtx {
  const ua = request.headers['user-agent'];
  return {
    actorId: (request.user as { userId?: string } | null)?.userId ?? 'unknown',
    action,
    target: target ?? null,
    ipHash: sha256(request.ip),
    uaHash: sha256(Array.isArray(ua) ? ua[0] : ua),
  };
}

/**
 * WRITE-AHEAD intent row (F5/RA2-4): a `started` row committed in its OWN statement BEFORE a
 * destructive drill, so no side-effect can occur without a pre-committed trail. Returns the row id.
 */
export async function auditStart(pool: Pool, ctx: AuditCtx): Promise<string> {
  const r = await pool.query(
    `INSERT INTO platform_admin_audit_log (actor_id, action, target, status, ip_hash, user_agent_hash)
     VALUES ($1, $2, $3, 'started', $4, $5) RETURNING id`,
    [ctx.actorId, ctx.action, ctx.target, ctx.ipHash, ctx.uaHash],
  );
  return String(r.rows[0].id);
}

/** Close out a write-ahead row. */
export async function auditFinish(pool: Pool, id: string, status: 'completed' | 'failed'): Promise<void> {
  await pool.query(`UPDATE platform_admin_audit_log SET status = $2 WHERE id = $1`, [id, status]);
}

/** Single `completed` row for a read-only endpoint. Best-effort (a read must not fail on an audit blip). */
export async function auditCompleted(pool: Pool, ctx: AuditCtx, log?: { error: (...a: any[]) => void }): Promise<void> {
  try {
    await pool.query(
      `INSERT INTO platform_admin_audit_log (actor_id, action, target, status, ip_hash, user_agent_hash)
       VALUES ($1, $2, $3, 'completed', $4, $5)`,
      [ctx.actorId, ctx.action, ctx.target, ctx.ipHash, ctx.uaHash],
    );
  } catch (err) {
    log?.error({ err }, '[platform-admin] read audit write failed (non-blocking)');
  }
}
