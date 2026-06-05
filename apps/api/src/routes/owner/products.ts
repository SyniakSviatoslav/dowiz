import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { withTenant } from '@deliveryos/platform';

export default async function productRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  const LocationParams = z.object({ locationId: z.string().uuid() });
  const ProductParams = LocationParams.extend({ id: z.string().uuid() });
  const TranslationParams = ProductParams.extend({ locale: z.string().min(2) });

  server.post(
    '/api/owner/locations/:locationId/products',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: LocationParams,
        body: z.object({
          category_id: z.string().uuid().optional().nullable(),
          name: z.string().min(1),
          description: z.string().optional().nullable(),
          price: z.number().int().nonnegative(),
          available: z.boolean().default(true),
          image_key: z.string().optional().nullable(),
          attributes: z.record(z.any()).optional().nullable(),
          sort_order: z.number().int().optional().default(0)
        }).strict()
      }
    },
    async (request, reply) => {
      const { locationId } = request.params;
      const { category_id, name, description, price, available, image_key, attributes, sort_order } = request.body;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `INSERT INTO products (location_id, category_id, name, description, price, is_available, image_key, attributes, sort_order)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING *`,
          [locationId, category_id, name, description, price, available, image_key, attributes ?? {}, sort_order]
        );
      });
      return reply.status(201).send(res.rows[0]);
    }
  );

  server.get(
    '/api/owner/locations/:locationId/products',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: LocationParams,
        querystring: z.object({
          cursor: z.string().uuid().optional(),
          limit: z.coerce.number().min(1).max(100).default(50),
          category_id: z.string().uuid().optional(),
          available: z.enum(['true', 'false']).optional()
        }).strict()
      }
    },
    async (request, reply) => {
      const { locationId } = request.params;
      const { cursor, limit, category_id, available } = request.query;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        let query = `SELECT * FROM products WHERE location_id = $1`;
        const params: any[] = [locationId, limit];
        let paramIdx = 3;

        if (category_id) {
          query += ` AND category_id = $${paramIdx++}`;
          params.push(category_id);
        }
        if (available) {
          query += ` AND is_available = $${paramIdx++}`;
          params.push(available === 'true');
        }
        if (cursor) {
          query += ` AND id > $${paramIdx++}`;
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
    '/api/owner/locations/:locationId/products/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: ProductParams }
    },
    async (request, reply) => {
      const { locationId, id } = request.params;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`SELECT * FROM products WHERE location_id = $1 AND id = $2`, [locationId, id]);
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );

  server.patch(
    '/api/owner/locations/:locationId/products/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: ProductParams,
        body: z.object({
          category_id: z.string().uuid().optional().nullable(),
          name: z.string().min(1).optional(),
          description: z.string().optional().nullable(),
          price: z.number().int().nonnegative().optional(),
          available: z.boolean().optional(),
          image_key: z.string().optional().nullable(),
          attributes: z.record(z.any()).optional().nullable(),
          sort_order: z.number().int().optional()
        }).strict()
      }
    },
    async (request, reply) => {
      const { locationId, id } = request.params;
      const updates = request.body;
      const userId = request.user!.userId;

      if (Object.keys(updates).length === 0) return reply.status(400).send({ error: 'No updates provided' });

      const res = await withTenant(server.db, userId, async (client) => {
        const setClauses: string[] = [];
        const values: any[] = [locationId, id];
        let paramIdx = 3;

        for (const [k, v] of Object.entries(updates)) {
          if (k === 'available') {
            setClauses.push(`is_available = $${paramIdx}`);
          } else {
            setClauses.push(`${k} = $${paramIdx}`);
          }
          values.push(v);
          paramIdx++;
        }

        /* eslint-disable local/no-raw-sql */
        return client.query(`UPDATE products SET ${setClauses.join(', ')} WHERE location_id = $1 AND id = $2 RETURNING *`, values);
        /* eslint-enable local/no-raw-sql */
      });

      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );

  server.delete(
    '/api/owner/locations/:locationId/products/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: ProductParams }
    },
    async (request, reply) => {
      const { locationId, id } = request.params;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`DELETE FROM products WHERE location_id = $1 AND id = $2 RETURNING id`, [locationId, id]);
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.status(204).send();
    }
  );

  // Translations
  server.put(
    '/api/owner/locations/:locationId/products/:id/translations/:locale',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: TranslationParams,
        body: z.object({
          name: z.string().min(1),
          description: z.string().optional().nullable()
        }).strict()
      }
    },
    async (request, reply) => {
      const { locationId, id, locale } = request.params;
      const { name, description } = request.body;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        // Validate locale exists in supported_locales
        const locRes = await client.query(`SELECT supported_locales FROM locations WHERE id = $1`, [locationId]);
        if (!locRes.rows[0]?.supported_locales?.includes(locale)) {
          return null; // Signal 400
        }

        return client.query(
          `INSERT INTO product_translations (product_id, locale, name, description)
           VALUES ($1, $2, $3, $4)
           ON CONFLICT (product_id, locale) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description
           RETURNING *`,
          [id, locale, name, description]
        );
      });

      if (!res) {
        return reply.status(400).send({ error: 'unsupported locale' });
      }

      return reply.send(res.rows[0]);
    }
  );

  server.get(
    '/api/owner/locations/:locationId/products/:id/translations',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: ProductParams }
    },
    async (request, reply) => {
      const { id } = request.params;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`SELECT * FROM product_translations WHERE product_id = $1`, [id]);
      });
      return reply.send({ data: res.rows });
    }
  );

  server.delete(
    '/api/owner/locations/:locationId/products/:id/translations/:locale',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: TranslationParams }
    },
    async (request, reply) => {
      const { id, locale } = request.params;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(`DELETE FROM product_translations WHERE product_id = $1 AND locale = $2 RETURNING locale`, [id, locale]);
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.status(204).send();
    }
  );

  // Product Modifier Groups Sync
  server.put(
    '/api/owner/locations/:locationId/products/:id/modifier-groups',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: {
        params: ProductParams,
        body: z.array(z.object({
          group_id: z.string().uuid(),
          sort_order: z.number().int().default(0)
        }).strict())
      }
    },
    async (request, reply) => {
      const { locationId, id } = request.params;
      const payload = request.body;
      const userId = request.user!.userId;

      await withTenant(server.db, userId, async (client) => {
        await client.query(`DELETE FROM product_modifier_groups WHERE product_id = $1`, [id]);
        
        for (const item of payload) {
          await client.query(
            `INSERT INTO product_modifier_groups (product_id, group_id, sort_order, location_id)
             VALUES ($1, $2, $3, $4)`,
            [id, item.group_id, item.sort_order, locationId]
          );
        }
      });
      return reply.send({ success: true });
    }
  );

  server.get(
    '/api/owner/locations/:locationId/products/:id/modifier-groups',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner'])],
      schema: { params: ProductParams }
    },
    async (request, reply) => {
      const { id } = request.params;
      const userId = request.user!.userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `SELECT pmg.sort_order, mg.* 
           FROM product_modifier_groups pmg
           JOIN modifier_groups mg ON pmg.group_id = mg.id
           WHERE pmg.product_id = $1
           ORDER BY pmg.sort_order ASC`,
          [id]
        );
      });
      return reply.send({ data: res.rows });
    }
  );
}
