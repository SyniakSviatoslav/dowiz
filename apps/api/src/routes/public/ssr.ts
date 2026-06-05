// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderMenuPage } from '../../lib/ssr-renderer.js';
import { LRUCache } from 'lru-cache';
import { detectPiiLeak } from '../../lib/pii-leak-detector.js';
import crypto from 'node:crypto';

// Development in-memory cache for SSR
const ssrCache = new LRUCache<string, { html: string, version: string }>({
  max: 1000,
  ttl: 60 * 1000 // 60 seconds
});

export default (async function ssrRoutes(fastify, opts) {
  const { db } = opts as any;
  const server = fastify.withTypeProvider<ZodTypeProvider>();

  server.get('/s/:slug', async (request, reply) => {
    const { slug } = request.params as { slug: string };
    const { embed, widget } = request.query as any;
    
    const isEmbed = !!embed || !!widget;

    try {
      const client = await db.connect();
      try {
        // Fetch public menu
        const res = await client.query('SELECT read_public_menu_all_locales($1) as menu', [slug]);
        const menuData = res.rows[0]?.menu;

        if (!menuData) {
          return reply
            .status(404)
            .header('Content-Type', 'text/html; charset=utf-8')
            .send(`<!DOCTYPE html><html><head><meta name="robots" content="noindex"><link rel="canonical" href="/"><title>404 Not Found</title></head><body><h1>Сторінку не знайдено / Page not found</h1></body></html>`);
        }

        const versionStr = menuData.menu_version.toString();
        const cacheKey = `${slug}:${versionStr}`;

        // Fetch theme
        const themeRes = await client.query(`
          SELECT t.css_hash, t.version, lt.frame_ancestors 
          FROM location_themes lt 
          LEFT JOIN theme_versions t ON lt.location_id = t.location_id 
          WHERE lt.location_id = (SELECT id FROM locations WHERE slug = $1) 
          ORDER BY t.version DESC NULLS LAST LIMIT 1
        `, [slug]);
        const theme = themeRes.rows[0];
        const cssHash = theme?.css_hash || '';
        const themeVersion = theme?.version || 0;
        const frameAncestors = theme?.frame_ancestors?.join(' ') || "'self'";

        const nonce = crypto.randomBytes(16).toString('base64');

        // Development cache hit
        let htmlOutput: string;
        const cached = ssrCache.get(cacheKey);
        
        if (cached && cached.version === versionStr) {
          htmlOutput = cached.html;
        } else {
          // Render HTML
          htmlOutput = renderMenuPage(menuData, slug);

          // PII check
          const leaks = detectPiiLeak(htmlOutput);
          if (leaks.length > 0) {
            request.log.error({ leaks }, 'PII leak detected in SSR render!');
            // Strip leaks explicitly just to be safe
            for (const leak of leaks) {
              htmlOutput = htmlOutput.replaceAll(leak, '[REDACTED_LEAK]');
            }
          }

          ssrCache.set(cacheKey, { html: htmlOutput, version: versionStr });
        }

        // Inject nonce, theme, and embed-helper if needed
        htmlOutput = htmlOutput.replace(/<style>/g, `<style nonce="${nonce}">`);
        
        if (cssHash) {
          htmlOutput = htmlOutput.replace('</head>', `<link rel="stylesheet" href="/public/locations/${menuData.location_id}/theme.css?hash=${cssHash}&v=${themeVersion}">\n</head>`);
        }

        if (isEmbed) {
          htmlOutput = htmlOutput.replace('</head>', `<meta name="robots" content="noindex" />\n<script src="/dist/embed-helper.js"></script>\n</head>`);
          // Note: UI components (like header) should handle ?embed=1 in client scripts, or we can add a class here
          htmlOutput = htmlOutput.replace('<body', '<body class="embed-mode"');
        }

        // Set Headers
        reply.header('Content-Type', 'text/html; charset=utf-8');
        reply.header('Cache-Control', 'public, max-age=60, stale-while-revalidate=86400');
        reply.header('X-Menu-Version', versionStr);
        
        let csp = `default-src 'self'; img-src 'self' data: https://cdn.dowiz.org; style-src 'self' 'nonce-${nonce}' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; script-src 'self' 'unsafe-eval' https://cdn.tailwindcss.com; connect-src 'self' https://cdn.dowiz.org; frame-ancestors ${frameAncestors}`;
        reply.header('Content-Security-Policy', csp);


        return reply.status(200).send(htmlOutput);
      } finally {
        client.release();
      }
    } catch (err) {
      request.log.error(err);
      return reply.status(500).send('Internal Server Error');
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
