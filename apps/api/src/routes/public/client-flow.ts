// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderClientShell } from '../../lib/ssr-client-renderer.js';
import crypto from 'node:crypto';

export default (async function clientFlowRoutes(fastify, opts) {
  const { db } = opts as any;

  const paramsSchema = z.object({ slug: z.string() });

  async function getThemeAndCSP(slug: string, reply: any) {
    const nonce = crypto.randomBytes(16).toString('base64');
    let themeData: any = null;
    let frameAncestors = "'self'";

    const client = await db.connect();
    try {
      const res = await client.query(`
        SELECT lt.location_id, lt.frame_ancestors, t.css_hash, t.version 
        FROM locations l
        JOIN location_themes lt ON lt.location_id = l.id
        LEFT JOIN theme_versions t ON t.location_id = l.id
        WHERE l.slug = $1
        ORDER BY t.version DESC NULLS LAST LIMIT 1
      `, [slug]);
      
      if (res.rows.length > 0) {
        themeData = res.rows[0];
        if (themeData.frame_ancestors) {
          frameAncestors = themeData.frame_ancestors.join(' ');
        }
      }
    } finally {
      client.release();
    }

    const csp = `default-src 'self'; img-src 'self' data: https://cdn.dowiz.org https://tiles.openfreemap.org; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://cdn.jsdelivr.net; font-src 'self' https://fonts.gstatic.com https://cdn.jsdelivr.net; script-src 'self' 'unsafe-inline' 'unsafe-eval' https://cdn.tailwindcss.com; connect-src 'self' https://cdn.dowiz.org https://cdn.jsdelivr.net https://tiles.openfreemap.org; frame-ancestors ${frameAncestors}`;
    reply.header('Content-Security-Policy', csp);

    return { nonce, themeData };
  }

  fastify.get('/s/:slug/cart', async (request, reply) => {
    const { slug } = request.params as any;
    const { nonce, themeData } = await getThemeAndCSP(slug, reply);
    const html = renderClientShell({ 
      title: 'Cart', slug, scriptUrl: '/dist/cart/app.js', nonce,
      cssHash: themeData?.css_hash, themeVersion: themeData?.version, locationId: themeData?.location_id
    });
    return reply.header('Content-Type', 'text/html; charset=utf-8').send(html);
  });

  fastify.get('/s/:slug/checkout', async (request, reply) => {
    const { slug } = request.params as any;
    const { nonce, themeData } = await getThemeAndCSP(slug, reply);
    const html = renderClientShell({ 
      title: 'Checkout', slug, scriptUrl: '/dist/checkout/app.js', nonce,
      cssHash: themeData?.css_hash, themeVersion: themeData?.version, locationId: themeData?.location_id
    });
    return reply.header('Content-Type', 'text/html; charset=utf-8').send(html);
  });

  fastify.get('/s/:slug/orders/:orderId', async (request, reply) => {
    const { slug } = request.params as any;
    const { nonce, themeData } = await getThemeAndCSP(slug, reply);
    const html = renderClientShell({ 
      title: 'Order Status', slug, scriptUrl: '/dist/status/app.js', nonce,
      cssHash: themeData?.css_hash, themeVersion: themeData?.version, locationId: themeData?.location_id
    });
    return reply.header('Content-Type', 'text/html; charset=utf-8').send(html);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
