import type { Pool } from 'pg';
import type { FastifyRequest } from 'fastify';

export async function getOwnerLocationId(request: FastifyRequest, db: Pool): Promise<string | null> {
  const user = (request as any).user;
  if (!user || user.role !== 'owner') return null;
  // P-d (ADR-0004): NEVER trust the baked JWT activeLocationId on its own — a removed or
  // downgraded owner can still hold a valid ≤24h token. Verify it against a LIVE active owner
  // membership on every call (one indexed read via memberships_user_id_active_idx). This is the
  // load-bearing per-request enforcement that bounds the insider-removal write window to zero.
  if (user.activeLocationId) {
    const ok = await db.query(
      `SELECT 1 FROM memberships
       WHERE user_id = $1 AND location_id = $2 AND role = 'owner' AND status = 'active' LIMIT 1`,
      [user.userId, user.activeLocationId]
    );
    return (ok.rowCount ?? 0) > 0 ? user.activeLocationId : null;
  }
  // No baked location → resolve a current active owner membership.
  const res = await db.query(
    `SELECT location_id FROM memberships
     WHERE user_id = $1 AND role = 'owner' AND status = 'active' LIMIT 1`,
    [user.userId]
  );
  return res.rows[0]?.location_id ?? null;
}
