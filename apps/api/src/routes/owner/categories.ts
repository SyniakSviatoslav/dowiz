import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { withTenant } from '@deliveryos/platform';

export default async function categoryRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  // Helper schema for common path params
  const LocationParams = z.object({
    locationId: z.string().uuid()
  });

  const CategoryParams = LocationParams.extend({
    id: z.string().uuid()
  });

  server.post(
    '/api/owner/locations/:locationId/categories',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: LocationParams,
        body: z.object({
          name: z.string().min(1),
          sort_order: z.number().int().optional(),
          image_key: z.string().nullable().optional() // if we had image_key for categories
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { name, sort_order } = request.body;
      const { locationId } = request.params;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `INSERT INTO categories (location_id, name, sort_order)
           VALUES ($1, $2, COALESCE($3, 0))
           RETURNING id, name, sort_order, created_at`,
          [locationId, name, sort_order]
        );
      });

      return reply.status(201).send(res.rows[0]);
    }
  );

  server.get(
    '/api/owner/locations/:locationId/categories',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: LocationParams,
        querystring: z.object({
          cursor: z.string().uuid().optional(),
          limit: z.coerce.number().min(1).max(100).default(50)
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const { cursor, limit } = request.query;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        let query = `SELECT * FROM categories WHERE location_id = $1`;
        const params: any[] = [locationId, limit];

        if (cursor) {
          query += ` AND id > $3`;
          params.push(cursor);
        }

        query += ` ORDER BY id ASC LIMIT $2`;

        /* eslint-disable local/no-raw-sql */
        return client.query(query, params);
        /* eslint-enable local/no-raw-sql */
      });
      
      return reply.send({ data: res.rows });
    }
  );

  server.get(
    '/api/owner/locations/:locationId/categories/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: CategoryParams
      }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `SELECT * FROM categories WHERE location_id = $1 AND id = $2`,
          [locationId, id]
        );
      });

      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );

  server.patch(
    '/api/owner/locations/:locationId/categories/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: CategoryParams,
        body: z.object({
          name: z.string().min(1).optional(),
          sort_order: z.number().int().optional()
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const updates = request.body;
      const userId = (request.user as any).userId;

      if (Object.keys(updates).length === 0) {
        return reply.status(400).send({ error: 'No updates provided' });
      }

      const res = await withTenant(server.db, userId, async (client) => {
        const setClauses: string[] = [];
        const values: any[] = [locationId, id];
        let paramIdx = 3;

        for (const [k, v] of Object.entries(updates)) {
          setClauses.push(`${k} = $${paramIdx}`);
          values.push(v);
          paramIdx++;
        }

        const query = `
          UPDATE categories
          SET ${setClauses.join(', ')}
          WHERE location_id = $1 AND id = $2
          RETURNING *
        `;

        /* eslint-disable local/no-raw-sql */
        return client.query(query, values);
        /* eslint-enable local/no-raw-sql */
      });

      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );

  server.delete(
    '/api/owner/locations/:locationId/categories/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: CategoryParams
      }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        // Check if products exist
        const prodRes = await client.query(
          `SELECT id FROM products WHERE category_id = $1 LIMIT 1`,
          [id]
        );
        if (prodRes.rowCount && prodRes.rowCount > 0) {
          return null; // Signals 409
        }

        return client.query(
          `DELETE FROM categories WHERE location_id = $1 AND id = $2 RETURNING id`,
          [locationId, id]
        );
      });

      if (res === null) {
        return reply.status(409).send({ error: 'Category contains products' });
      }

      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.status(204).send();
    }
  );
}
