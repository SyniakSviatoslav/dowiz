import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function publicThemeRoutes(fastify, opts) {
  const { db } = opts as any;

  fastify.get('/public/locations/:locationId/theme.css', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      querystring: z.object({ hash: z.string().optional() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    const { hash } = request.query;

    const client = await db.connect();
    try {
      let query = '';
      let params: any[] = [];
      
      if (hash) {
        query = `SELECT css_body FROM theme_versions WHERE location_id = $1 AND css_hash = $2`;
        params = [locationId, hash];
      } else {
        query = `SELECT css_body FROM theme_versions WHERE location_id = $1 ORDER BY version DESC LIMIT 1`;
        params = [locationId];
      }

      const res = await client.query(query, params);
      
      if (res.rows.length === 0) {
        return reply.status(404).send('Not Found');
      }

      reply.header('Content-Type', 'text/css; charset=utf-8');
      
      // If we matched by hash, we can cache forever. Otherwise, cache short.
      if (hash) {
        reply.header('Cache-Control', 'public, max-age=31536000, immutable');
      } else {
        reply.header('Cache-Control', 'public, max-age=60');
      }

      return reply.send(res.rows[0].css_body);
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
