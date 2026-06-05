import type { FastifyInstance } from 'fastify';
import { signAuthToken } from '@deliveryos/platform';

export default async function mockAuthRoutes(fastify: FastifyInstance) {
  console.log('[API] Registering mockAuthRoutes: /dev/mock-auth');
  fastify.post('/dev/mock-auth', async (request, reply) => {

    const email = 'dev@deliveryos.com';
    const googleSub = 'mock-google-12345';
    const name = 'Dev Owner';

    let userId: string;
    try {
      const res = await fastify.db.query(
        `INSERT INTO users (email, google_sub, display_name) 
         VALUES ($1, $2, $3)
         ON CONFLICT (google_sub) DO UPDATE SET email = EXCLUDED.email, display_name = EXCLUDED.display_name
         RETURNING id`,
        [email, googleSub, name]
      );
      userId = res.rows[0].id;
    } catch (e) {
      const updateRes = await fastify.db.query(
        `UPDATE users SET google_sub = $2, display_name = COALESCE(users.display_name, $3) WHERE email = $1 RETURNING id`,
        [email, googleSub, name]
      );
      if (updateRes.rowCount === 0) {
        throw new Error('Failed to upsert dev user');
      }
      userId = updateRes.rows[0].id;
    }

    // Check if they have an active location
    const memberRes = await fastify.db.query(
      `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
      [userId]
    );
    const activeLocationId = memberRes.rowCount > 0 ? memberRes.rows[0].location_id : undefined;

    const accessToken = await signAuthToken({ role: 'owner', userId, activeLocationId } as any, '1d');
    
    return reply.send({ access_token: accessToken, userId, activeLocationId });
  });
}
