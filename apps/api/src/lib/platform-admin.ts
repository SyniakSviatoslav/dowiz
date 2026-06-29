import type { FastifyRequest, FastifyReply, FastifyInstance } from 'fastify';
import type { Pool } from 'pg';

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
