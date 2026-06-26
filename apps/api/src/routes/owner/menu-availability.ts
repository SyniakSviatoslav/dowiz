import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { withTenant } from '@deliveryos/platform';

/**
 * MENU-AVAILABILITY (additive) · owner controls for the availability vertical:
 *   - PATCH .../kitchen-busy   → set/clear the venue `busy` window (kitchen_busy_until).
 *   - CRUD  .../menu-schedules → mealtime / availability windows per product or category.
 *
 * All writes go through withTenant (set_config app.user_id) so menu_schedules' FORCE-RLS
 * tenant_isolation policy is the real boundary. Nothing here mutates money, the order
 * machine, or existing contract fields.
 */
export default async function menuAvailabilityRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  const LocationParams = z.object({ locationId: z.string().uuid() });
  const ScheduleParams = LocationParams.extend({ id: z.string().uuid() });

  // --- Venue busy toggle ------------------------------------------------------
  server.patch(
    '/api/owner/locations/:locationId/kitchen-busy',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner']), server.requireLocationAccess],
      schema: {
        params: LocationParams,
        // busy_until: an ISO ts in the future => busy; null => clear (back to open).
        body: z.object({ busy_until: z.string().datetime().nullable() }).strict(),
      },
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const { busy_until } = request.body;
      const userId = (request.user as any).userId;

      const res = await withTenant(server.db, userId, async (client) => {
        return client.query(
          `UPDATE locations SET kitchen_busy_until = $2 WHERE id = $1 RETURNING id, kitchen_busy_until`,
          [locationId, busy_until],
        );
      });
      if (res.rowCount === 0) return reply.sendError(404, 'NOT_FOUND', 'Not found');
      return reply.send({ id: res.rows[0].id, kitchenBusyUntil: res.rows[0].kitchen_busy_until });
    },
  );

  // --- Schedules CRUD ---------------------------------------------------------
  const ScheduleBody = z.object({
    product_id: z.string().uuid().nullable().optional(),
    category_id: z.string().uuid().nullable().optional(),
    mode: z.enum(['daily', 'recurring', 'period']).default('daily'),
    start_minute: z.number().int().min(0).max(1439).nullable().optional(),
    end_minute: z.number().int().min(0).max(1440).nullable().optional(),
    days_of_week: z.array(z.number().int().min(0).max(6)).nullable().optional(),
    starts_at: z.string().datetime().nullable().optional(),
    ends_at: z.string().datetime().nullable().optional(),
    available: z.boolean().default(true),
  }).strict();

  function rowToShape(r: any) {
    return {
      id: r.id,
      productId: r.product_id,
      categoryId: r.category_id,
      mode: r.mode,
      startMinute: r.start_minute,
      endMinute: r.end_minute,
      daysOfWeek: r.days_of_week,
      startsAt: r.starts_at,
      endsAt: r.ends_at,
      available: r.available,
    };
  }

  server.get(
    '/api/owner/locations/:locationId/menu-schedules',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner']), server.requireLocationAccess],
      schema: { params: LocationParams },
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const userId = (request.user as any).userId;
      const res = await withTenant(server.db, userId, async (client) =>
        client.query(
          `SELECT * FROM menu_schedules WHERE location_id = $1 ORDER BY created_at ASC`,
          [locationId],
        ),
      );
      return reply.send({ data: res.rows.map(rowToShape) });
    },
  );

  server.post(
    '/api/owner/locations/:locationId/menu-schedules',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner']), server.requireLocationAccess],
      schema: { params: LocationParams, body: ScheduleBody },
    },
    async (request: any, reply: any) => {
      const { locationId } = request.params;
      const b = request.body;
      const userId = (request.user as any).userId;

      // exactly one target — mirror the DB CHECK so we 400 instead of 500.
      const hasProduct = !!b.product_id;
      const hasCategory = !!b.category_id;
      if (hasProduct === hasCategory) {
        return reply.sendError(400, 'VALIDATION_FAILED', 'Provide exactly one of product_id or category_id');
      }

      const res = await withTenant(server.db, userId, async (client) =>
        client.query(
          `INSERT INTO menu_schedules
             (location_id, product_id, category_id, mode, start_minute, end_minute, days_of_week, starts_at, ends_at, available)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
           RETURNING *`,
          [locationId, b.product_id ?? null, b.category_id ?? null, b.mode,
            b.start_minute ?? null, b.end_minute ?? null, b.days_of_week ?? null,
            b.starts_at ?? null, b.ends_at ?? null, b.available],
        ),
      );
      return reply.status(201).send(rowToShape(res.rows[0]));
    },
  );

  server.delete(
    '/api/owner/locations/:locationId/menu-schedules/:id',
    {
      preValidation: [server.verifyAuth, server.requireRole(['owner']), server.requireLocationAccess],
      schema: { params: ScheduleParams },
    },
    async (request: any, reply: any) => {
      const { locationId, id } = request.params;
      const userId = (request.user as any).userId;
      const res = await withTenant(server.db, userId, async (client) =>
        client.query(`DELETE FROM menu_schedules WHERE location_id = $1 AND id = $2 RETURNING id`, [locationId, id]),
      );
      if (res.rowCount === 0) return reply.sendError(404, 'NOT_FOUND', 'Not found');
      return reply.status(204).send();
    },
  );
}
