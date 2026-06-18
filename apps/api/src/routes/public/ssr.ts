import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';

// Crawlers + social-link scrapers get the server-rendered HTML (JSON-LD + OG
// tags) for SEO/rich previews; real visitors get the React SPA storefront
// (the SPA router already maps /s/:slug → ClientRoutes → MenuPage).
export const BOT_UA = /bot|crawl|spider|slurp|mediapartners|facebookexternalhit|embedly|quora|pinterest|whatsapp|telegram|slackbot|twitter|linkedinbot|discord|google|bing|yandex|baidu|duckduck|applebot|petalbot|semrush|ahrefs/i;

export default (async function ssrRoutes(fastify: any, opts: any) {
  const { db } = opts as any;

  const paramsSchema = z.object({ slug: z.string() });

  fastify.get('/s/:slug', async (request: any, reply: any) => {
    const { slug } = request.params as any;
    const ua = String((request.headers as any)['user-agent'] || '');

    if (BOT_UA.test(ua)) {
      const html = await renderMenuPage(slug, db);
      return reply.type('text/html').send(html);
    }

    // Human → serve the SPA shell; React Router renders the storefront for :slug.
    return reply.sendFile('index.html');
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
