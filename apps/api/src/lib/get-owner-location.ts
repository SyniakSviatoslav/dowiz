import type { Pool } from 'pg';
import type { FastifyRequest } from 'fastify';

export async function getOwnerLocationId(request: FastifyRequest, db: Pool): Promise<string | null> {
  const user = (request as any).user;
  if (!user || user.role !== 'owner') return null;
  // Use activeLocationId from JWT if present
  if (user.activeLocationId) return user.activeLocationId;
  // Fallback: query memberships table
  const res = await db.query(
    `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
    [user.userId]
  );
  return res.rows[0]?.location_id ?? null;
}
