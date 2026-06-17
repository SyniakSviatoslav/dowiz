import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { withTenant } from '@deliveryos/platform';

export default async function locationRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.patch(
    '/api/owner/locations/:locationId',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner']), server.requireLocationAccess],
      schema: {
        params: z.object({
          locationId: z.string().uuid()
        }),
        body: z.object({
          default_locale: z.string().min(2).optional(),
          supported_locales: z.array(z.string().min(2)).optional(),
          name: z.string().min(1).max(200).optional(),
          phone: z.string().min(3).max(30).optional(),
          currency_code: z.string().length(3).optional(),
          delivery_fee_flat: z.number().int().min(0).optional(),
          min_order_value: z.number().int().min(0).nullish(),
          free_delivery_threshold: z.number().int().min(0).nullish(),
          delivery_radius_km: z.number().min(0).nullish(),
          tax_rate: z.number().min(0).max(100).optional(),
          lat: z.number().min(-90).max(90).nullish(),
          lng: z.number().min(-180).max(180).nullish(),
          delivery_address: z.string().max(500).nullish(),
        }).strict()
      }
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const updates = request.body;
      const userId = (request.user as any).userId;

      if (Object.keys(updates).length === 0) return reply.status(400).send({ error: 'No updates provided' });

      // If we change default_locale, it must be in supported_locales
      if (updates.default_locale && updates.supported_locales) {
        if (!updates.supported_locales.includes(updates.default_locale)) {
          return reply.status(400).send({ error: 'default_locale must be in supported_locales' });
        }
      }

      const ALLOWED: Record<string, string> = {
        default_locale: 'default_locale', supported_locales: 'supported_locales',
        name: 'name', phone: 'phone', currency_code: 'currency_code',
        delivery_fee_flat: 'delivery_fee_flat', min_order_value: 'min_order_value',
        free_delivery_threshold: 'free_delivery_threshold', delivery_radius_km: 'delivery_radius_km',
        tax_rate: 'tax_rate', lat: 'lat', lng: 'lng', delivery_address: 'delivery_address',
      };

      const res = await withTenant(server.db, userId, async (client) => {
        const setClauses: string[] = [];
        const values: any[] = [locationId];
        let paramIdx = 2;

        for (const [k, v] of Object.entries(updates)) {
          const col = ALLOWED[k];
          if (!col) continue;
          setClauses.push(`${col} = $${paramIdx}`);
          values.push(v);
          paramIdx++;
        }

        /* eslint-disable local/no-raw-sql */
        return client.query(`UPDATE locations SET ${setClauses.join(', ')} WHERE id = $1 RETURNING *`, values);
        /* eslint-enable local/no-raw-sql */
      });
      if (res.rowCount === 0) return reply.status(404).send({ error: 'Not found' });
      return reply.send(res.rows[0]);
    }
  );
}
