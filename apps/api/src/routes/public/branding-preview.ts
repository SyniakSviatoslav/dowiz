import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function brandingPreviewRoutes(fastify: any, opts: any) {
  fastify.get('/branding-preview/:slug', async (request: any, reply: any) => {
    const r2PublicUrl = process.env.R2_PUBLIC_URL;
    let r2ImgSrc = '';
    if (r2PublicUrl) {
      try { r2ImgSrc = ' ' + new URL(r2PublicUrl).origin; } catch (_) {}
    }
    const csp = `default-src 'self'; img-src 'self' data: https:${r2ImgSrc}; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com https://plausible.io; worker-src 'self' blob:; connect-src 'self' https://cdn.jsdelivr.net https://tiles.openfreemap.org; frame-ancestors *`;

    reply.header('Content-Security-Policy', csp);
    reply.header('Cache-Control', 'no-cache, no-store, must-revalidate');
    try { reply.raw.removeHeader('X-Frame-Options'); } catch (err: any) {
      console.debug('[branding-preview] failed to remove X-Frame-Options header:', err?.message);
    }
    return reply.sendFile('index.html');
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
