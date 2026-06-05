import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import crypto from 'crypto';
import { withTenant } from '@deliveryos/platform';

export default async function courierRoutes(fastify: FastifyInstance) {

  fastify.post('/couriers/invites', {
    preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])],
    schema: {
      body: z.object({ locationId: z.string().uuid() }).strict()
    }
  }, async (request, reply) => {
    // Check if the owner has access to this location. We can use requireLocationAccess 
    // but that is for path parameters. Here it's in body, so we check it manually via withTenant.
    const { locationId } = request.body as any;
    const user = request.user!;
    if (user.role !== 'owner') {
      return reply.status(403).send({ error: 'Forbidden' });
    }

    try {
      return await withTenant(fastify.db, user.userId, async (client) => {
        // Double check location exists and is visible to the owner (due to RLS)
        const locRes = await client.query(`SELECT 1 FROM locations WHERE id = $1`, [locationId]);
        if (locRes.rowCount === 0) {
          return reply.status(404).send({ error: 'Location not found' });
        }

        const code = crypto.randomBytes(4).toString('hex').slice(0, 6).toUpperCase();
        const codeHash = crypto.createHash('sha256').update(code).digest('hex');

        await client.query(
          `INSERT INTO courier_invites (location_id, code_hash, created_by, expires_at)
           VALUES ($1, $2, $3, now() + interval '7 days')`,
          [locationId, codeHash, user.userId]
        );

        return { code };
      });
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal server error' });
    }
  });
}
