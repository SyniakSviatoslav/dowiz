// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { renderTheme, ALLOWED_FONTS } from '../../lib/theme-renderer.js';
import sharp from 'sharp';

export default (async function ownerThemeRoutes(fastify, opts) {
  const { db, storage } = opts as any;

  // Auth: verify owner JWT + require owner role for all theme routes
  fastify.addHook('onRequest', fastify.verifyAuth);
  fastify.addHook('onRequest', fastify.requireRole(['owner']));

  // GET theme
  fastify.get('/api/owner/locations/:locationId/theme', {
    schema: {
      params: z.object({ locationId: z.string().uuid() })
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    const client = await db.connect();
    try {
      const res = await client.query(`SELECT * FROM location_themes WHERE location_id = $1`, [locationId]);
      const theme = res.rows[0];
      if (!theme) return reply.status(404).send({ error: 'Not found' });

      const versionRes = await client.query(
        `SELECT css_hash, version FROM theme_versions WHERE location_id = $1 ORDER BY version DESC LIMIT 1`,
        [locationId]
      );
      const version = versionRes.rows[0] || { css_hash: null, version: 0 };

      return reply.send({
        theme,
        cssHash: version.css_hash,
        version: version.version
      });
    } finally {
      client.release();
    }
  });

  // PUT theme
  fastify.put('/api/owner/locations/:locationId/theme', {
    schema: {
      params: z.object({ locationId: z.string().uuid() }),
      body: z.object({
        primary_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
        secondary_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
        font_family: z.enum(ALLOWED_FONTS).nullable().optional(),
        bg_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
        text_color: z.string().regex(/^#[0-9a-fA-F]{6}$/).nullable().optional(),
        frame_ancestors: z.array(z.string()).optional()
      }).strict()
    }
  }, async (request, reply) => {
    const { locationId } = request.params;
    const updates = request.body;
    
    const client = await db.connect();
    try {
      await client.query('BEGIN');

      // Update location_themes
      const fields = Object.keys(updates);
      if (fields.length > 0) {
        const setClauses = fields.map((f, i) => `${f} = $${i + 2}`).join(', ');
        const values = fields.map(f => (updates as any)[f]);
        await client.query(
          `UPDATE location_themes SET ${setClauses}, updated_at = now() WHERE location_id = $1`,
          [locationId, ...values]
        );
      }

      // Get full theme
      const res = await client.query(`SELECT * FROM location_themes WHERE location_id = $1`, [locationId]);
      const theme = res.rows[0];

      // Get current version
      const vRes = await client.query(`SELECT COALESCE(MAX(version), 0) as v FROM theme_versions WHERE location_id = $1`, [locationId]);
      const currentVersion = parseInt(vRes.rows[0].v, 10);

      // Render new CSS
      const rendered = renderTheme({
        primary_color: theme.primary_color,
        secondary_color: theme.secondary_color,
        font_family: theme.font_family,
        bg_color: theme.bg_color,
        text_color: theme.text_color,
        logo_url: theme.logo_url
      }, currentVersion);

      // Save new version
      await client.query(
        `INSERT INTO theme_versions (location_id, css_hash, css_body, version)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (location_id, css_hash) DO NOTHING`,
        [locationId, rendered.cssHash, rendered.css, rendered.version]
      );

      await client.query('COMMIT');
      
      return reply.send({
        cssHash: rendered.cssHash,
        version: rendered.version,
        warnings: rendered.warnings
      });
    } catch (e) {
      await client.query('ROLLBACK');
      throw e;
    } finally {
      client.release();
    }
  });

  // POST logo
  fastify.post('/api/owner/locations/:locationId/theme/logo', async (request, reply) => {
    const { locationId } = request.params as any;
    const data = await request.file();
    if (!data) return reply.status(400).send({ error: 'No file uploaded' });

    const buffer = await data.toBuffer();
    
    // Process with sharp: strip EXIF, resize
    const processed = await sharp(buffer)
      .resize({ width: 512, height: 512, fit: 'inside' })
      .withMetadata(false) // Strip EXIF/PII
      .webp({ quality: 80 })
      .toBuffer();

    const key = `locations/${locationId}/logo.webp`;
    
    // Assuming storage provider has upload method
    if (storage) {
       await storage.upload(key, processed, 'image/webp');
    }

    const client = await db.connect();
    try {
      const logoUrl = `https://cdn.dowiz.org/${key}`; // mocked CDN base
      await client.query(`UPDATE location_themes SET logo_url = $2 WHERE location_id = $1`, [locationId, logoUrl]);
      // Re-render theme would go here in a full impl
      return reply.send({ logo_url: logoUrl });
    } finally {
      client.release();
    }
  });

}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
