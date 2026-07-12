import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';

export default (async function publicThemeRoutes(fastify: any, opts: any) {
  const { db } = opts;

  const DEFAULT_CSS = `:root{--brand-primary:#ea4f16;--brand-primary-hover:#ffa12e;--brand-bg:#121212;--brand-surface:#1e1e1e;--brand-text:#ffffff;--brand-text-muted:#a8a8a8;--brand-border:#2c2c2c;--brand-radius:12px;--color-success:#059669;--color-warning:#D97706;--color-danger:#DC2626;--color-info:#2563EB}@media(prefers-color-scheme:dark){:root{--brand-bg:#0F172A;--brand-surface:#1E293B;--brand-text:#F1F5F9;--brand-text-muted:#94A3B8;--brand-border:#334155}}`;

  fastify.get('/public/locations/:locationId/theme.css', async (request: any, reply: any) => {
    const locationId = (request.params as { locationId: string }).locationId || '';
    const hash = (request.query as { hash?: string })?.hash || '';

    reply.header('Content-Type', 'text/css; charset=utf-8');

    try {
      const client = await db.connect();
      try {
        let locUuid = locationId;
        const isUuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(locationId);
        if (!isUuid) {
          const locRes = await client.query('SELECT id FROM locations WHERE slug = $1', [locationId]);
          if (locRes.rows.length > 0) locUuid = locRes.rows[0].id;
        }

        let query = '';
        let params: any[] = [];
        if (hash) {
          query = 'SELECT css_body FROM theme_versions WHERE location_id = $1 AND css_hash = $2';
          params = [locUuid, hash];
        } else {
          query = 'SELECT css_body FROM theme_versions WHERE location_id = $1 ORDER BY version DESC LIMIT 1';
          params = [locUuid];
        }

        const res = await client.query(query, params);
        if (res.rows.length > 0) {
          reply.header('Cache-Control', hash ? 'public, max-age=31536000, immutable' : 'public, max-age=60');
          return reply.send(res.rows[0].css_body);
        }
      } finally {
        client.release();
      }
    } catch (err) {
      request.log.warn({ err }, 'Theme DB query failed, returning default CSS');
    }

    reply.header('Cache-Control', 'public, max-age=60');
    return reply.send(DEFAULT_CSS);
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
