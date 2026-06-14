import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { withTenant } from '@deliveryos/platform';

export default async function modifierGroupRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  const LocationParams = z.object({ locationId: z.string().uuid() });
  const GroupParams = LocationParams.extend({ id: z.string().uuid() });
  const ModifierParams = LocationParams.extend({ id: z.string().uuid() });
  const GroupModifierParams = LocationParams.extend({ groupId: z.string().uuid() });

  server.post(
    '/api/owner/locations/:locationId/modifier-groups',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: LocationParams,
        body: z.object({
          name: z.string().min(1),
          min_select: z.number().int().nonnegative().default(0),
          max_select: z.number().int().nonnegative().default(1),
          required: z.boolean().default(false)
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const { name, min_select, max_select, required } = request.body;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `INSERT INTO modifier_groups (location_id, name, min_select, max_select, required)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *`,
          [locationId, name, min_select, max_select, required]
        );
      });
      return reply.status(201).send(res.rows[0]);
    }
  );

  server.get(
    '/api/owner/locations/:locationId/modifier-groups',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: LocationParams }
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`SELECT * FROM modifier_groups WHERE location_id = $1 ORDER BY created_at ASC`, [locationId]);
      });
      return reply.send({ data: res.rows });
    }
  );

  server.patch(
    '/api/owner/locations/:locationId/modifier-groups/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: GroupParams,
        body: z.object({
          name: z.string().min(1).optional(),
          min_select: z.number().int().nonnegative().optional(),
          max_select: z.number().int().nonnegative().optional(),
          required: z.boolean().optional()
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const updates = request.body;
      const userId = (request.user as any).userId;
      
      if (Object.keys(updates).length === 0) return reply.status(400).send({ error: 'No updates provided' });

      const res = await withTenant(server.db, userId, async (client) => {
        const setClauses: string[] = [];
        const values: any[] = [locationId, id];
        let paramIdx = 3;

        for (const [k, v] of Object.entries(updates)) {
          setClauses.push(`${k} = $${paramIdx}`);
          values.push(v);
          paramIdx++;
        }

        /* eslint-disable local/no-raw-sql */
        return client.query(`UPDATE modifier_groups SET ${setClauses.join(', ')} WHERE location_id = $1 AND id = $2 RETURNING *`, values);
        /* eslint-enable local/no-raw-sql */
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );

  server.delete(
    '/api/owner/locations/:locationId/modifier-groups/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: GroupParams }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`DELETE FROM modifier_groups WHERE location_id = $1 AND id = $2 RETURNING id`, [locationId, id]);
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.status(204).send();
    }
  );

  // Modifiers
  server.post(
    '/api/owner/locations/:locationId/modifier-groups/:groupId/modifiers',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: GroupModifierParams,
        body: z.object({
          name: z.string().min(1),
          price_delta: z.number().int().default(0),
          available: z.boolean().default(true),
          sort_order: z.number().int().default(0)
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId, groupId } = request.params;
      const { name, price_delta, available, sort_order } = request.body;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `INSERT INTO modifiers (location_id, group_id, name, price_delta, available, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING *`,
          [locationId, groupId, name, price_delta, available, sort_order]
        );
      });
      return reply.status(201).send(res.rows[0]);
    }
  );

  server.patch(
    '/api/owner/locations/:locationId/modifiers/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: ModifierParams,
        body: z.object({
          name: z.string().min(1).optional(),
          price_delta: z.number().int().optional(),
          available: z.boolean().optional(),
          sort_order: z.number().int().optional()
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const updates = request.body;
      const userId = (request.user as any).userId;
      
      if (Object.keys(updates).length === 0) return reply.status(400).send({ error: 'No updates provided' });

      const res = await withTenant(server.db, userId, async (client) => {
        const setClauses: string[] = [];
        const values: any[] = [locationId, id];
        let paramIdx = 3;

        for (const [k, v] of Object.entries(updates)) {
          setClauses.push(`${k} = $${paramIdx}`);
          values.push(v);
          paramIdx++;
        }

        /* eslint-disable local/no-raw-sql */
        return client.query(`UPDATE modifiers SET ${setClauses.join(', ')} WHERE location_id = $1 AND id = $2 RETURNING *`, values);
        /* eslint-enable local/no-raw-sql */
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );

  server.delete(
    '/api/owner/locations/:locationId/modifiers/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: ModifierParams }
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`DELETE FROM modifiers WHERE location_id = $1 AND id = $2 RETURNING id`, [locationId, id]);
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.status(204).send();
    }
  );
}
