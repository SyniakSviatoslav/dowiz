import type { FastifyInstance } from 'fastify';
import { z } from 'zod';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';

export default async function publicMenuRoutes(fastify: FastifyInstance) {
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get(
    '/public/locations/:locationIdOrSlug/menu',
    async (request, reply) => {
      const { locationIdOrSlug } = request.params as any;
      const locale = (request.query as any)?.locale || ''; // passed to PG

      const res = await server.db.query(
        `SELECT read_public_menu($1, $2) as menu`,
        [locationIdOrSlug, locale]
      );

      const menu = res.rows[0]?.menu;
      if (!menu) {
        return reply.status(404).send({ error: 'Location not found' });
      }

      reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=300');
      reply.header('X-Menu-Version', menu.menu_version.toString());
      
      return reply.send(menu);
    }
  );
}
