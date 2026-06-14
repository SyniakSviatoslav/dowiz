import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';

export default (async function ssrRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  const paramsSchema = z.object({ slug: z.string() });

  fastify.get('/s/:slug', async (request: any, reply: any) => {
    const { slug } = request.params as any;
    const locale = (request.query as any)?.locale || '';

    const html = await renderMenuPage(slug, db);
    reply.type('text/html').send(html);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
