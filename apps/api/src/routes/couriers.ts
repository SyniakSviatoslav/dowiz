import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import crypto from 'crypto';
import { withTenant } from '@deliveryos/platform';

export default async function courierRoutes(fastify: FastifyInstance) {

  fastify.post('/couriers/invites', {
    preHandler: [(fastify as any).verifyAuth, (fastify as any).requireRole(['owner'])],
    schema: {
      body: z.object({ locationId: z.string().uuid() }).strict()
    }
  }, async (request: any, reply: any) => {
    // Check if the owner has access to this location. We can use requireLocationAccess 
    // but that is for path parameters. Here it's in body, so we check it manually via withTenant.
    const { locationId } = request.body as any;
    const user = (request as any).user!;
    if (user.role !== 'owner') {
      return reply.sendError(403, 'FORBIDDEN', 'Forbidden');
    }

    try {
      return await withTenant((fastify as any).db, user.userId, async (client) => {
        // #7 (security-hardening-2026-07): authorize the body `locationId` with an EXPLICIT
        // membership-ownership predicate — do NOT rely on RLS visibility. The operational pool runs
        // as a BYPASSRLS role, so `SELECT 1 FROM locations WHERE id=$1` returns ANY tenant's location
        // → an owner could mint an invite bound to another tenant's location. Require a LIVE active
        // owner membership for THIS specific location (ADR-0004 / requireLocationAccess semantics).
        const locRes = await client.query(
          `SELECT 1 FROM memberships
           WHERE user_id = $1 AND location_id = $2 AND role = 'owner' AND status = 'active' LIMIT 1`,
          [user.userId, locationId]
        );
        if (locRes.rowCount === 0) {
          return reply.sendError(404, 'NOT_FOUND', 'Location not found');
        }

        const code = crypto.randomBytes(4).toString('hex').slice(0, 6).toUpperCase();
        const codeHash = crypto.createHash('sha256').update(code).digest('hex');

        await client.query(
          `INSERT INTO courier_invites (location_id, code_hash, created_by_owner_id, expires_at)
           VALUES ($1, $2, $3, now() + interval '7 days')`,
          [locationId, codeHash, user.userId]
        );

        return { code };
      });
    } catch (err) {
      request.log.error(err);
      return reply.sendError(500, 'INTERNAL', 'Internal server error');
    }
  });
}
