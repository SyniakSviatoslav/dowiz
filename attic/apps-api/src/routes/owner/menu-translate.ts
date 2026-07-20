import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import { withTenant } from '@deliveryos/platform';
import { BUS_CHANNELS } from '../../lib/registry.js';

export default (async function menuTranslateRoutes(fastify: any, opts: any) {
  const { db, messageBus, translation } = opts as any;

  fastify.post('/locations/:id/menu/translate', {
    preHandler: [fastify.verifyAuth, fastify.requireRole(['owner']), fastify.requireLocationAccess],
    schema: {
      body: z.object({
        target_locales: z.array(z.string()).optional(),
        force: z.boolean().optional(),
        entity_filter: z.object({
          categories: z.array(z.string().uuid()).optional(),
          products: z.array(z.string().uuid()).optional(),
          modifiers: z.array(z.string().uuid()).optional()
        }).strict().optional()
      }).strict()
    },
    config: {
      rateLimit: {
        max: 1, // 1 call per minute per location
        timeWindow: '1 minute'
      }
    }
  }, async (request: any, reply: any) => {
    const { id: locationId } = request.params as any;
    const { target_locales, force, entity_filter } = request.body;
    const user = request.user as any;

    try {
      const result = await withTenant(db, user.userId, async (client) => {
        // Fetch location supported locales
        const locRes = await client.query(`SELECT default_locale, supported_locales FROM locations WHERE id = $1`, [locationId]);
        if (locRes.rowCount === 0) return { status: 404, error: 'Location not found' };
        
        const loc = locRes.rows[0];
        const defaultLocale = loc.default_locale;
        let targets = target_locales || loc.supported_locales.filter((l: string) => l !== defaultLocale);

        // Validate target_locales
        for (const t of targets) {
          if (!loc.supported_locales.includes(t)) {
            return { status: 400, error: `Unsupported locale: ${t}`, code: 'UNSUPPORTED_LOCALE' };
          }
        }

        if (targets.length === 0) {
          return { status: 400, error: 'No target locales specified or available' };
        }

        const counts = { categories: 0, products: 0, modifiers: 0 };
        const skipped = { categories: 0, products: 0, modifiers: 0 };
        let degraded = false;

        for (const targetLocale of targets) {
          // 1. Categories
          let catQuery = `
            SELECT c.id, c.name, t.is_auto 
            FROM categories c
            LEFT JOIN category_translations t ON t.category_id = c.id AND t.locale = $2
            WHERE c.location_id = $1
          `;
          let catParams: any[] = [locationId, targetLocale];
          if (entity_filter?.categories && entity_filter.categories.length > 0) {
            catQuery += ` AND c.id = ANY($3)`;
            catParams.push(entity_filter.categories);
          }

          const cats = await client.query(catQuery, catParams);
          const catsToTranslate = [];
          for (const row of cats.rows) {
            if (row.is_auto === false && !force) {
              skipped.categories++;
            } else {
              catsToTranslate.push(row);
            }
          }

          if (catsToTranslate.length > 0) {
            const req = {
              texts: catsToTranslate.map(c => c.name),
              from: defaultLocale,
              to: targetLocale,
              context: 'category' as const
            };
            const res = await translation.translate(req);
            if (res.model_id.includes('degraded') || res.model_id.includes('error')) degraded = true;
            
            for (let i = 0; i < catsToTranslate.length; i++) {
              await client.query(
                `INSERT INTO category_translations (category_id, locale, name, is_auto, last_edited_at, last_edited_by)
                 VALUES ($1, $2, $3, true, now(), $4)
                 ON CONFLICT (category_id, locale) DO UPDATE SET name = EXCLUDED.name, is_auto = true, last_edited_at = EXCLUDED.last_edited_at, last_edited_by = EXCLUDED.last_edited_by`,
                [catsToTranslate[i].id, targetLocale, res.translations[i], user.userId]
              );
              counts.categories++;
            }
          }

          // 2. Products
          let prodQuery = `
            SELECT p.id, p.name, p.description, t.is_auto 
            FROM products p
            LEFT JOIN product_translations t ON t.product_id = p.id AND t.locale = $2
            WHERE p.location_id = $1
          `;
          let prodParams: any[] = [locationId, targetLocale];
          if (entity_filter?.products && entity_filter.products.length > 0) {
            prodQuery += ` AND p.id = ANY($3)`;
            prodParams.push(entity_filter.products);
          }

          const prods = await client.query(prodQuery, prodParams);
          const prodsToTranslate = [];
          for (const row of prods.rows) {
            if (row.is_auto === false && !force) {
              skipped.products++;
            } else {
              prodsToTranslate.push(row);
            }
          }

          if (prodsToTranslate.length > 0) {
            // Need to translate name and description separately or batch together
            const textsToTranslate = [];
            for (const p of prodsToTranslate) {
              textsToTranslate.push(p.name);
              textsToTranslate.push(p.description || '');
            }

            const req = {
              texts: textsToTranslate,
              from: defaultLocale,
              to: targetLocale,
              context: 'menu_item' as const
            };
            const res = await translation.translate(req);
            if (res.model_id.includes('degraded') || res.model_id.includes('error')) degraded = true;

            for (let i = 0; i < prodsToTranslate.length; i++) {
              const transName = res.translations[i * 2];
              const transDesc = res.translations[i * 2 + 1];
              await client.query(
                `INSERT INTO product_translations (product_id, locale, name, description, is_auto, last_edited_at, last_edited_by)
                 VALUES ($1, $2, $3, $4, true, now(), $5)
                 ON CONFLICT (product_id, locale) DO UPDATE SET name = EXCLUDED.name, description = EXCLUDED.description, is_auto = true, last_edited_at = EXCLUDED.last_edited_at, last_edited_by = EXCLUDED.last_edited_by`,
                [prodsToTranslate[i].id, targetLocale, transName, transDesc || null, user.userId]
              );
              counts.products++;
            }
          }

          // 3. Modifiers
          let modQuery = `
            SELECT m.id, m.name, t.is_auto 
            FROM modifiers m
            LEFT JOIN modifier_translations t ON t.modifier_id = m.id AND t.locale = $2
            WHERE m.location_id = $1
          `;
          let modParams: any[] = [locationId, targetLocale];
          if (entity_filter?.modifiers && entity_filter.modifiers.length > 0) {
            modQuery += ` AND m.id = ANY($3)`;
            modParams.push(entity_filter.modifiers);
          }

          const mods = await client.query(modQuery, modParams);
          const modsToTranslate = [];
          for (const row of mods.rows) {
            if (row.is_auto === false && !force) {
              skipped.modifiers++;
            } else {
              modsToTranslate.push(row);
            }
          }

          if (modsToTranslate.length > 0) {
            const req = {
              texts: modsToTranslate.map(m => m.name),
              from: defaultLocale,
              to: targetLocale,
              context: 'modifier' as const
            };
            const res = await translation.translate(req);
            if (res.model_id.includes('degraded') || res.model_id.includes('error')) degraded = true;

            for (let i = 0; i < modsToTranslate.length; i++) {
              await client.query(
                `INSERT INTO modifier_translations (modifier_id, locale, name, is_auto, last_edited_at, last_edited_by)
                 VALUES ($1, $2, $3, true, now(), $4)
                 ON CONFLICT (modifier_id, locale) DO UPDATE SET name = EXCLUDED.name, is_auto = true, last_edited_at = EXCLUDED.last_edited_at, last_edited_by = EXCLUDED.last_edited_by`,
                [modsToTranslate[i].id, targetLocale, res.translations[i], user.userId]
              );
              counts.modifiers++;
            }
          }
        }

        // Publish event
        await messageBus.publish(BUS_CHANNELS.MENU_TRANSLATED, {
          locationId,
          counts,
          degraded
        });

        return { status: 200, response: { translated: counts, skipped_due_to_manual: skipped, degraded } };
      });

      if (result.error) return reply.status(result.status).send({ error: result.error, code: result.code });
      return reply.status(200).send(result.response);

    } catch (err: any) {
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal Server Error' });
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
