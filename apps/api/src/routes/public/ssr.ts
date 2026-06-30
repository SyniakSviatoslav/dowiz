import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';
import { isBot, serveSpaShell } from '../../lib/spa-shell.js';
import { renderShadowPreview } from '../../lib/preview-render.js';

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

    // P6-3 (D): a SHADOW tenant is never-orderable + noindex + generic-OG. read_preview_menu returns
    // non-null ONLY for shadows (owner_id NULL + closed + published_at NULL). Split by client:
    //   • BOTS / no-JS unfurlers → the bare server-rendered labeled preview (renderShadowPreview): it
    //     carries the honest banner + menu in static HTML with GENERIC OG (H3 — no real name in meta),
    //     so a pasted link never doxes the unconsented venue and crawlers get readable content.
    //   • HUMANS → the real React storefront via serveSpaShell, which ALREADY serves shadows with
    //     noindex + generic OG (owner_id NULL branch). The SPA fetches the menu (/public/.../menu now
    //     falls back to the preview, flagged is_preview) and renders it in NON-ORDERABLE preview mode —
    //     same design as a live store, no cart/checkout, claim CTA. The richer P6-3 honest preview.
    // If migration 070 isn't applied the fn is absent (42883) → fall through to the real-tenant path
    // (where serveSpaShell's B2 shadow guard still suppresses the real name as a safety net).
    try {
      const previewRes = await db.query('SELECT read_preview_menu($1) AS m', [slug]);
      const previewMenu = previewRes.rows[0]?.m;
      if (previewMenu) {
        reply.header('X-Robots-Tag', 'noindex, nofollow');
        if (isBot(request.headers['user-agent'])) {
          return reply.type('text/html').send(renderShadowPreview(previewMenu));
        }
        return serveSpaShell(reply, db, slug);
      }
    } catch (err: any) {
      if (err?.code !== '42883') console.debug('[ssr] preview lookup failed:', err?.message); // 42883 = fn not yet migrated
    }

    if (isBot(request.headers['user-agent'])) {
      const html = await renderMenuPage(slug, db);
      return reply.type('text/html').send(html);
    }

    return serveSpaShell(reply, db, slug);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
