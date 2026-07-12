import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';
import { isBot, serveSpaShell } from '../../lib/spa-shell.js';

// /s/:slug is the menu. Humans get the React SPA storefront (one cart, shared
// across menu → cart → checkout → order via /s/:slug/* in client-flow.ts).
// Crawlers/social scrapers get the SSR menu so JSON-LD + OG tags survive (the
// only SEO-relevant surface; cart/checkout/order are noindex). The SPA's cart
// is dos_cart_<slug>, used end-to-end, so there is no SSR↔SPA cart crossing.
export default (async function ssrRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  const paramsSchema = z.object({ slug: z.string() });

  fastify.get('/s/:slug', async (request: any, reply: any) => {
    const { slug } = request.params as any;

    if (isBot(request.headers['user-agent'])) {
      const html = await renderMenuPage(slug, db);
      return reply.type('text/html').send(html);
    }

    return serveSpaShell(reply, db, slug);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
