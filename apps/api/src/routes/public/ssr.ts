import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';

// NOTE: a previous iteration served the React SPA here for human visitors. That
// broke the order flow — the SPA menu writes the cart to dos_cart_<id> while the
// SSR checkout (routes/public/client-flow.ts) reads dos_cart, and the SPA's own
// checkout doesn't carry the cart either. Until the SPA storefront's cart +
// checkout work end to end, /s/:slug stays on the SSR menu so menu → checkout is
// one consistent flow. "SPA as the only storefront" is a dedicated follow-up
// (fix the SPA cart/checkout first, then route /s/:slug/* to it).
export default (async function ssrRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  const paramsSchema = z.object({ slug: z.string() });

  fastify.get('/s/:slug', async (request: any, reply: any) => {
    const { slug } = request.params as any;
    const html = await renderMenuPage(slug, db);
    reply.type('text/html').send(html);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
