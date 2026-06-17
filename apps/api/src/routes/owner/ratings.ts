import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';
import { withTenant } from '@deliveryos/platform';

const MIN_RATINGS_FOR_AGGREGATE = 3;

export default (async function ownerRatingsRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));
  fastify.addHook('onRequest', fastify.requireLocationAccess);

  // GET /api/owner/locations/:locationId/ratings/summary
  fastify.get('/:locationId/ratings/summary', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({
        days: z.coerce.number().int().min(1).max(365).default(30),
      }),
    },
  }, async (request: any, reply: any) => {
    const { locationId } = request.params;
    const { days } = request.query;
    const user = request.user as any;

    const result = await withTenant(db, user.userId, async (client) => {
      const since = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();

      const summaryRes = await client.query(
        `SELECT
           COUNT(*)::int          AS total_count,
           AVG(stars)::numeric(3,2) AS avg_stars
         FROM order_ratings
         WHERE location_id = $1 AND created_at >= $2`,
        [locationId, since],
      );

      const { total_count, avg_stars } = summaryRes.rows[0];

      const courierRes = await client.query(
        `SELECT
           or2.courier_id,
           c.name AS courier_name,
           COUNT(*)::int            AS count,
           AVG(or2.stars)::numeric(3,2) AS avg_stars
         FROM order_ratings or2
         LEFT JOIN couriers c ON c.id = or2.courier_id
         WHERE or2.location_id = $1
           AND or2.courier_id IS NOT NULL
           AND or2.created_at >= $2
         GROUP BY or2.courier_id, c.name
         HAVING COUNT(*) >= $3
         ORDER BY avg_stars ASC`,
        [locationId, since, MIN_RATINGS_FOR_AGGREGATE],
      );

      return {
        totalCount: total_count,
        avgStars: total_count >= MIN_RATINGS_FOR_AGGREGATE ? parseFloat(avg_stars) : null,
        minReached: total_count >= MIN_RATINGS_FOR_AGGREGATE,
        periodDays: days,
        bycourier: courierRes.rows.map(r => ({
          courierId: r.courier_id,
          courierName: r.courier_name,
          count: r.count,
          avgStars: parseFloat(r.avg_stars),
        })),
      };
    });

    return reply.status(200).send(result);
  });
}) satisfies FastifyPluginAsync;
