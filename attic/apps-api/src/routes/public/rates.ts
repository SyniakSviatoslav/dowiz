import type { FastifyPluginAsync } from 'fastify';
import { z } from 'zod';

const RateResponse = z.object({
  base: z.literal('ALL'),
  target: z.literal('EUR'),
  rate: z.number(),
  fetchedAt: z.string(),
});

export default (async function ratesRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.get('/v1/rates', async (_request, reply) => {
    try {
      const res = await db.query(
        `SELECT rate, fetched_at FROM exchange_rates
         WHERE base_currency = 'ALL' AND target_currency = 'EUR'
         ORDER BY fetched_at DESC LIMIT 1`,
      );

      if (res.rowCount === 0) {
        // Fallback: the rates-refresh worker populates exchange_rates hourly, but a
        // fresh deploy (or a transient upstream FX outage) leaves the table empty.
        // Serve a sane static ALL→EUR rate so the storefront's secondary-currency
        // display keeps working instead of 503-ing the whole pricing UI. Short
        // cache so the live worker value is picked up as soon as it lands.
        reply.header('Cache-Control', 'public, max-age=300');
        return {
          base: 'ALL',
          target: 'EUR',
          rate: 0.0099,
          fetchedAt: new Date(0).toISOString(),
        };
      }

      const { rate, fetched_at } = res.rows[0];
      return {
        base: 'ALL',
        target: 'EUR',
        rate: parseFloat(rate),
        fetchedAt: fetched_at.toISOString(),
      };
    } catch (err) {
      console.error('[Rates] Error:', err);
      return reply.status(500).send({ error: 'internal_error' });
    }
  });
} as FastifyPluginAsync);
