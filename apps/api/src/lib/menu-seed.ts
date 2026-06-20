/**
 * Seed a FRESH location's menu from a parsed import draft.
 *
 * Used by the menu-first onboarding claim (POST /owner/onboarding/start with an
 * anonymous_import_id): the location was just created, so there is nothing to
 * conflict with — this is the simple add-only subset of the authenticated
 * /menu/import/commit path, deliberately kept separate so the working commit
 * route is untouched. Categories must exist before products reference them, and
 * modifier groups before modifiers/links — hence the ordering below.
 *
 * Runs on the caller's client/transaction. Inserts use ON CONFLICT DO NOTHING so
 * a retry (or a draft with duplicate keys) can't 23505 the whole claim.
 */

export interface SeedCounts {
  categories: number;
  products: number;
  modifierGroups: number;
  modifiers: number;
  links: number;
}

export async function seedMenuFromDraft(
  client: any,
  locationId: string,
  draft: any
): Promise<SeedCounts> {
  const counts: SeedCounts = { categories: 0, products: 0, modifierGroups: 0, modifiers: 0, links: 0 };
  if (!draft || typeof draft !== 'object') return counts;

  for (const cat of draft.categories || []) {
    await client.query(
      `INSERT INTO categories (location_id, external_key, name) VALUES ($1, $2, $3)
       ON CONFLICT (location_id, external_key) WHERE external_key IS NOT NULL DO NOTHING`,
      [locationId, cat.externalKey, cat.name]
    );
    counts.categories++;
  }

  for (const prod of draft.products || []) {
    const catRes = await client.query(
      `SELECT id FROM categories WHERE location_id = $1 AND external_key = $2`,
      [locationId, prod.categoryKey]
    );
    if (catRes.rowCount === 0) continue; // skip products whose category didn't seed
    await client.query(
      `INSERT INTO products (location_id, category_id, external_key, name, description, price, is_available, attributes, image_key)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
       ON CONFLICT (location_id, external_key) WHERE external_key IS NOT NULL DO NOTHING`,
      [locationId, catRes.rows[0].id, prod.externalKey, prod.name, prod.description || null,
       prod.price, prod.available, prod.attributesJson ?? {}, prod.imageKey || null]
    );
    counts.products++;
  }

  for (const grp of draft.modifierGroups || []) {
    await client.query(
      `INSERT INTO modifier_groups (location_id, external_key, name, min_select, max_select, required)
       VALUES ($1, $2, $3, $4, $5, $6)
       ON CONFLICT (location_id, external_key) WHERE external_key IS NOT NULL DO NOTHING`,
      [locationId, grp.externalKey, grp.name, grp.minSelect, grp.maxSelect, grp.required]
    );
    counts.modifierGroups++;
  }

  for (const mod of draft.modifiers || []) {
    const grpRes = await client.query(
      `SELECT id FROM modifier_groups WHERE location_id = $1 AND external_key = $2`,
      [locationId, mod.groupKey]
    );
    if (grpRes.rowCount === 0) continue;
    await client.query(
      `INSERT INTO modifiers (location_id, group_id, external_key, name, price_delta, available, sort_order)
       VALUES ($1, $2, $3, $4, $5, $6, $7)
       ON CONFLICT (group_id, external_key) WHERE external_key IS NOT NULL DO NOTHING`,
      [locationId, grpRes.rows[0].id, mod.externalKey, mod.name, mod.priceDelta, mod.available, mod.sortOrder || 0]
    );
    counts.modifiers++;
  }

  for (const link of draft.links || []) {
    const prodRes = await client.query(`SELECT id FROM products WHERE location_id = $1 AND external_key = $2`, [locationId, link.productKey]);
    const grpRes = await client.query(`SELECT id FROM modifier_groups WHERE location_id = $1 AND external_key = $2`, [locationId, link.groupKey]);
    if ((prodRes.rowCount ?? 0) > 0 && (grpRes.rowCount ?? 0) > 0) {
      await client.query(
        `INSERT INTO product_modifier_groups (product_id, group_id, sort_order, location_id)
         VALUES ($1, $2, $3, $4) ON CONFLICT (product_id, group_id) DO NOTHING`,
        [prodRes.rows[0].id, grpRes.rows[0].id, link.sortOrder, locationId]
      );
      counts.links++;
    }
  }

  return counts;
}
