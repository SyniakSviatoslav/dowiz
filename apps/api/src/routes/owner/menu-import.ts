// @ts-nocheck
import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import crypto from 'crypto';
import { ParseModeEnum } from '@deliveryos/shared-types';
import { withTenant } from '@deliveryos/platform';

export default (async function menuImportRoutes(fastify, opts) {
  const { db, messageBus, parsers, storage } = opts as any;

  async function getLocationId(user: any): Promise<string | null> {
    if (!user?.userId) return null;
    const res = await db.query(
      `SELECT location_id FROM memberships WHERE user_id = $1 AND role = 'owner' LIMIT 1`,
      [user.userId]
    );
    return res.rows.length > 0 ? res.rows[0].location_id : null;
  }

  // ─── POST /preview ───────────────────────────────────────────────────
  fastify.post('/menu/import/preview', {
    preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])],
    config: {
      rateLimit: {
        max: 5,
        timeWindow: '1 minute'
      }
    }
  }, async (request, reply) => {
    const user = request.user!;
    const locationId = await getLocationId(user);
    if (!locationId) return reply.status(401).send({ error: 'Unauthorized' });
    
    // Read multipart
    const data = await request.file({ limits: { fileSize: 5 * 1024 * 1024 } });
    if (!data) {
      return reply.status(400).send({ error: 'Missing file' });
    }

    const buffer = await data.toBuffer();
    
    let mode = 'merge';
    if (data.fields.mode && 'value' in data.fields.mode) {
      mode = String(data.fields.mode.value);
    }
    const validatedMode = ParseModeEnum.safeParse(mode);
    if (!validatedMode.success) {
      return reply.status(400).send({ error: 'Invalid mode', details: validatedMode.error });
    }

    let config = {};
    if (data.fields.config && 'value' in data.fields.config) {
      try {
        config = JSON.parse(String(data.fields.config.value));
      } catch (e) {
        return reply.status(400).send({ error: 'Invalid config JSON' });
      }
    }

    // Tenant check and fetch currency
    let locationCurrency = 'ALL';
    let locationMinorUnit = 0;

    const locOk = await withTenant(db, user.userId, async (client) => {
      const res = await client.query('SELECT currency_code, currency_minor_unit FROM locations WHERE id = $1', [locationId]);
      if (res.rowCount === 0) return false;
      locationCurrency = res.rows[0].currency_code;
      locationMinorUnit = res.rows[0].currency_minor_unit;
      return true;
    });

    if (!locOk) {
      return reply.status(404).send({ error: 'Location not found' });
    }

    let source = 'csv';
    let kind = 'csv';
    const mime = data.mimetype;
    if (mime === 'application/pdf') {
      return reply.status(400).send({ error: 'PDF import is not supported yet. Please upload a CSV or Excel file.' });
    }
    if (mime.startsWith('image/')) {
      source = 'ai-ocr';
      kind = 'image';
    } else if (data.fields.source && 'value' in data.fields.source) {
      source = String(data.fields.source.value);
    }

    const parser = parsers[source];
    if (!parser) {
      return reply.status(400).send({ error: `Unsupported source: ${source}` });
    }

    // Parse data
    const parseResult = await parser.parse({
      kind: kind as any,
      mime: mime as any,
      bytes: buffer,
      config: { ...config, expectedCurrency: locationCurrency, currencyMinorUnit: locationMinorUnit }
    });

    parseResult.summary.mode = validatedMode.data;

    // Save draft to temporary storage
    const storageKey = `import_${crypto.randomUUID()}.csv`;
    await storage.put(storageKey, buffer); // Storing the raw file

    const idempotencyKey = (request.headers['idempotency-key'] as string) || null;

    if (source === 'ai-ocr') {
      (parseResult.draft as any)._provenance = {
        source: 'ai-ocr',
        raw_text_hash: crypto.createHash('sha256').update(buffer).digest('hex')
      };
    }

    // Save to import_sessions
    let importSessionId: string;
    await withTenant(db, user.userId, async (client) => {
      const res = await client.query(
        `INSERT INTO import_sessions 
          (location_id, owner_id, status, mode, draft_json, issues_json, summary_json, idempotency_key, expires_at)
         VALUES ($1, $2, 'previewed', $3, $4, $5, $6, $7, now() + interval '30 minutes')
         RETURNING id`,
        [locationId, user.userId, validatedMode.data, JSON.stringify(parseResult.draft), JSON.stringify(parseResult.issues), JSON.stringify(parseResult.summary), idempotencyKey]
      );
      importSessionId = res.rows[0].id;
    });

    // We don't fetch diff for now to keep it simple, but we report what is to be created
    const draftPreview = {
      categories_to_create: parseResult.draft.categories.map((c: any) => c.name),
      products_to_create: parseResult.draft.products.map((p: any) => p.name),
      modifier_groups_to_create: parseResult.draft.modifierGroups.map((g: any) => g.name),
      modifiers_to_create: parseResult.draft.modifiers.map((m: any) => m.name),
      links_to_create: parseResult.draft.links
    };

    await messageBus.publish('menu.import.previewed', {
      locationId,
      importSessionId: importSessionId!,
      counts: { valid: parseResult.summary.valid }
    });

    return reply.status(200).send({
      import_session_id: importSessionId!,
      expires_at: new Date(Date.now() + 30 * 60000).toISOString(),
      summary: parseResult.summary,
      issues: parseResult.issues,
      draft_preview: draftPreview
    });
  });

  // ─── POST /commit ────────────────────────────────────────────────────
  fastify.post('/menu/import/commit', {
    preHandler: [fastify.verifyAuth, fastify.requireRole(['owner'])],
    schema: {
      body: z.object({
        import_session_id: z.string().uuid(),
        commit_token: z.string().uuid().optional(),
        force: z.boolean().optional()
      }).strict()
    }
  }, async (request, reply) => {
    const user = request.user!;
    const locationId = await getLocationId(user);
    if (!locationId) return reply.status(401).send({ error: 'Unauthorized' });
    
    const finalCommitToken = commit_token || crypto.randomUUID();

    try {
      const commitRes = await withTenant(db, user.userId, async (client) => {
        await client.query('BEGIN');

        const sessionRes = await client.query(
          `SELECT id, status, mode, draft_json, summary_json, commit_token, expires_at 
           FROM import_sessions 
           WHERE id = $1 AND location_id = $2 
           FOR UPDATE`,
          [import_session_id, locationId]
        );

        if (sessionRes.rowCount === 0) {
          await client.query('ROLLBACK');
          return { status: 404, error: 'Import session not found' };
        }

        const session = sessionRes.rows[0];

        if (session.status === 'committed') {
          if (session.commit_token === finalCommitToken) {
            await client.query('ROLLBACK');
            return { status: 200, response: { message: 'Already committed', import_session_id: session.id, commit_token: session.commit_token } };
          } else {
            await client.query('ROLLBACK');
            return { status: 409, error: 'Commit token mismatch', code: 'COMMIT_TOKEN_MISMATCH' };
          }
        }

        if (session.status === 'expired' || new Date(session.expires_at) < new Date()) {
          await client.query('ROLLBACK');
          return { status: 410, error: 'Import session expired', code: 'IMPORT_SESSION_EXPIRED' };
        }

        if (session.status === 'failed') {
          await client.query('ROLLBACK');
          return { status: 409, error: 'Import session failed', code: 'IMPORT_SESSION_FAILED' };
        }

        const draft = session.draft_json;
        const summary = session.summary_json;
        const mode = session.mode;

        // Block if low_confidence_count > 0 and !force
        if (summary.low_confidence_count && summary.low_confidence_count > 0 && !force) {
          await client.query('ROLLBACK');
          return { status: 422, error: 'Commit requires force due to low confidence', code: 'LOW_CONFIDENCE_REQUIRES_FORCE' };
        }

        if (force && draft._provenance) {
          draft._provenance.forced_by_owner = true;
        }

        // Upsert operations based on mode
        let counts = { categories: 0, products: 0, modifier_groups: 0, modifiers: 0, links: 0, translations: 0 };

        // 1. Categories
        for (const cat of draft.categories) {
          if (mode === 'add_only') {
            await client.query(
              `INSERT INTO categories (location_id, external_key, name) VALUES ($1, $2, $3)`,
              [locationId, cat.externalKey, cat.name]
            );
          } else {
            await client.query(
              `INSERT INTO categories (location_id, external_key, name) VALUES ($1, $2, $3)
               ON CONFLICT (location_id, external_key) WHERE external_key IS NOT NULL 
               DO UPDATE SET name = EXCLUDED.name`,
              [locationId, cat.externalKey, cat.name]
            );
          }
          counts.categories++;
        }

        // 2. Products
        for (const prod of draft.products) {
          const catRes = await client.query(`SELECT id FROM categories WHERE location_id = $1 AND external_key = $2`, [locationId, prod.categoryKey]);
          if (catRes.rowCount === 0) {
            await client.query('ROLLBACK');
            return { status: 400, error: `Category external_key ${prod.categoryKey} not found for product ${prod.externalKey}` };
          }
          const catId = catRes.rows[0].id;

          if (mode === 'add_only') {
            await client.query(
              `INSERT INTO products (location_id, category_id, external_key, name, description, price, available, attributes_json, image_key)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)`,
              [locationId, catId, prod.externalKey, prod.name, prod.description || null, prod.price, prod.available, prod.attributesJson || null, prod.imageKey || null]
            );
          } else {
            await client.query(
              `INSERT INTO products (location_id, category_id, external_key, name, description, price, available, attributes_json, image_key)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (location_id, external_key) WHERE external_key IS NOT NULL 
               DO UPDATE SET 
                  category_id = EXCLUDED.category_id,
                  name = EXCLUDED.name,
                  description = EXCLUDED.description,
                  price = EXCLUDED.price,
                  available = EXCLUDED.available,
                  attributes_json = EXCLUDED.attributes_json,
                  image_key = EXCLUDED.image_key`,
              [locationId, catId, prod.externalKey, prod.name, prod.description || null, prod.price, prod.available, prod.attributesJson || null, prod.imageKey || null]
            );
          }
          counts.products++;
        }

        // 3. Modifier Groups
        for (const grp of draft.modifierGroups) {
          if (mode === 'add_only') {
            await client.query(
              `INSERT INTO modifier_groups (location_id, external_key, name, min_select, max_select, required)
               VALUES ($1, $2, $3, $4, $5, $6)`,
              [locationId, grp.externalKey, grp.name, grp.minSelect, grp.maxSelect, grp.required]
            );
          } else {
            await client.query(
              `INSERT INTO modifier_groups (location_id, external_key, name, min_select, max_select, required)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT (location_id, external_key) WHERE external_key IS NOT NULL 
               DO UPDATE SET 
                  name = EXCLUDED.name,
                  min_select = EXCLUDED.min_select,
                  max_select = EXCLUDED.max_select,
                  required = EXCLUDED.required`,
              [locationId, grp.externalKey, grp.name, grp.minSelect, grp.maxSelect, grp.required]
            );
          }
          counts.modifier_groups++;
        }

        // 4. Modifiers
        for (const mod of draft.modifiers) {
          const grpRes = await client.query(`SELECT id FROM modifier_groups WHERE location_id = $1 AND external_key = $2`, [locationId, mod.groupKey]);
          if (grpRes.rowCount === 0) {
            await client.query('ROLLBACK');
            return { status: 400, error: `Modifier group external_key ${mod.groupKey} not found for modifier ${mod.externalKey}` };
          }
          const grpId = grpRes.rows[0].id;

          if (mode === 'add_only') {
            await client.query(
              `INSERT INTO modifiers (location_id, group_id, external_key, name, price_delta, available, sort_order)
               VALUES ($1, $2, $3, $4, $5, $6, $7)`,
              [locationId, grpId, mod.externalKey, mod.name, mod.priceDelta, mod.available, mod.sortOrder || 0]
            );
          } else {
            await client.query(
              `INSERT INTO modifiers (location_id, group_id, external_key, name, price_delta, available, sort_order)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               ON CONFLICT (group_id, external_key) WHERE external_key IS NOT NULL 
               DO UPDATE SET 
                  name = EXCLUDED.name,
                  price_delta = EXCLUDED.price_delta,
                  available = EXCLUDED.available,
                  sort_order = EXCLUDED.sort_order`,
              [locationId, grpId, mod.externalKey, mod.name, mod.priceDelta, mod.available, mod.sortOrder || 0]
            );
          }
          counts.modifiers++;
        }

        // 5. Links
        for (const link of draft.links) {
          const prodRes = await client.query(`SELECT id FROM products WHERE location_id = $1 AND external_key = $2`, [locationId, link.productKey]);
          const grpRes = await client.query(`SELECT id FROM modifier_groups WHERE location_id = $1 AND external_key = $2`, [locationId, link.groupKey]);
          
          if (prodRes.rowCount > 0 && grpRes.rowCount > 0) {
            await client.query(
              `INSERT INTO product_modifier_groups (product_id, group_id, sort_order)
               VALUES ($1, $2, $3)
               ON CONFLICT (product_id, group_id) DO UPDATE SET sort_order = EXCLUDED.sort_order`,
              [prodRes.rows[0].id, grpRes.rows[0].id, link.sortOrder]
            );
            counts.links++;
          }
        }

        // Mode: replace (delete omitted)
        if (mode === 'replace') {
          const keepCategoryKeys = draft.categories.map((c: any) => c.externalKey);
          if (keepCategoryKeys.length > 0) {
            // Check historical orders before deleting products
            const prodsToDelete = await client.query(
              `SELECT p.id FROM products p
               JOIN categories c ON p.category_id = c.id
               WHERE p.location_id = $1 AND c.external_key != ALL($2)`,
              [locationId, keepCategoryKeys]
            );
            if (prodsToDelete.rowCount && prodsToDelete.rowCount > 0) {
              const orderCheck = await client.query(
                `SELECT 1 FROM order_items WHERE product_id = ANY($1) LIMIT 1`,
                [prodsToDelete.rows.map(r => r.id)]
              );
              if (orderCheck.rowCount && orderCheck.rowCount > 0) {
                await client.query('ROLLBACK');
                return { status: 409, error: 'Cannot replace: historical orders exist for deleted products', code: 'REPLACE_BLOCKED_BY_HISTORICAL_ORDERS' };
              }
            }
            await client.query(
              `DELETE FROM categories WHERE location_id = $1 AND (external_key IS NULL OR external_key != ALL($2))`,
              [locationId, keepCategoryKeys]
            );
          }

          const keepProductKeys = draft.products.map((p: any) => p.externalKey);
          if (keepProductKeys.length > 0) {
            await client.query(
              `DELETE FROM products WHERE location_id = $1 AND (external_key IS NULL OR external_key != ALL($2))`,
              [locationId, keepProductKeys]
            );
          }

          const keepGroupKeys = draft.modifierGroups.map((g: any) => g.externalKey);
          if (keepGroupKeys.length > 0) {
            await client.query(
              `DELETE FROM modifier_groups WHERE location_id = $1 AND (external_key IS NULL OR external_key != ALL($2))`,
              [locationId, keepGroupKeys]
            );
          }
        }

        // Update import_sessions
        await client.query(
          `UPDATE import_sessions SET status = 'committed', commit_token = $1, committed_at = now() WHERE id = $2`,
          [finalCommitToken, import_session_id]
        );

        // Explicitly touch menu_version via the existing bump_menu_version() function or let triggers handle it
        // The trigger on categories/products handles it natively!
        // But to return the new version, we read it
        const mvRes = await client.query(`SELECT menu_version FROM locations WHERE id = $1`, [locationId]);
        const menuVersion = mvRes.rows[0].menu_version;

        await client.query('COMMIT');

        return { 
          status: 200, 
          response: { 
            import_session_id, 
            commit_token: finalCommitToken, 
            menu_version: menuVersion, 
            counts 
          } 
        };
      });

      if (commitRes.error) {
        return reply.status(commitRes.status).send({ error: commitRes.error, code: commitRes.code });
      }

      // Claim-check event
      await messageBus.publish('menu.imported', {
        locationId,
        importSessionId: import_session_id,
        counts: commitRes.response?.counts
      });

      return reply.status(200).send(commitRes.response);

    } catch (err: any) {
      if (err.code === '23505') {
        return reply.status(409).send({ error: 'Duplicate external_key found', code: 'DUPLICATE_KEY', details: err.detail });
      }
      request.log.error(err);
      return reply.status(500).send({ error: 'Internal Server Error' });
    }
  });
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;
