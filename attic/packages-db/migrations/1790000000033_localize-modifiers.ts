import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * P0-i18n · Localize modifier groups + modifier options by requested locale, and
 * un-break the crawler SSR menu for published storefronts.
 *
 * Three coordinated fixes:
 *  1. read_public_menu (live SPA function, last set by 0032) emitted modifier-group
 *     and modifier names as the raw mg.name / m.name columns — never joining the
 *     *_translations tables. A storefront seeded with one-language modifiers then
 *     rendered those labels in that language regardless of the requested locale
 *     (the trilingual "Розмір / Мала / Велика" leak in the EN/SQ product modal).
 *     Fix: resolve names by p_locale exactly like products/categories already do.
 *  2. read_public_menu_all_locales (crawler/SSR function, last set by 0016) still
 *     gated on status = 'active'; the publish flow (0030) sets status = 'open' +
 *     published_at, so /s/:slug returned "Menu not found" to bots. Fix: serve when
 *     published OR live, mirroring 0032.
 *  3. Backfill sq/en/uk translations for the demo tenant's modifiers/groups so the
 *     fix is visible on an already-seeded DB without a reseed (idempotent).
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
  // ── 1. Locale-aware read_public_menu (SPA storefront) ──────────────────────
  pgm.sql(`
    CREATE OR REPLACE FUNCTION public.read_public_menu(p_location_id_or_slug text, p_locale text DEFAULT ''::text)
    RETURNS jsonb
    SECURITY DEFINER
    LANGUAGE plpgsql
    AS $$
    DECLARE
      v_result jsonb;
      v_location_id uuid;
      v_def_locale text;
      v_supp_locales text[];
      v_currency_code text;
      v_currency_minor_unit int;
      v_version bigint;
    BEGIN
      SELECT id, default_locale, supported_locales, currency_code, currency_minor_unit
      INTO v_location_id, v_def_locale, v_supp_locales, v_currency_code, v_currency_minor_unit
      FROM locations
      WHERE (id::text = p_location_id_or_slug OR slug = p_location_id_or_slug)
        AND (status IN ('active', 'open') OR published_at IS NOT NULL);

      IF NOT FOUND THEN
        RETURN NULL;
      END IF;

      IF NOT (p_locale = ANY(v_supp_locales)) THEN
        p_locale := v_def_locale;
      END IF;

      SELECT version INTO v_version FROM menu_versions WHERE location_id = v_location_id;
      IF v_version IS NULL THEN
        v_version := 1;
      END IF;

      WITH
      modifiers_json AS (
        SELECT
          m.group_id,
          jsonb_agg(
            jsonb_build_object(
              'id', m.id,
              'name', COALESCE(mt.name, mt_def.name, m.name),
              'price_delta', m.price_delta,
              'available', m.available,
              'sort_order', m.sort_order
            ) ORDER BY m.sort_order
          ) as modifiers
        FROM modifiers m
        LEFT JOIN modifier_translations mt ON mt.modifier_id = m.id AND mt.locale = p_locale
        LEFT JOIN modifier_translations mt_def ON mt_def.modifier_id = m.id AND mt_def.locale = v_def_locale
        WHERE m.location_id = v_location_id AND m.available = true
        GROUP BY m.group_id
      ),
      modifier_groups_json AS (
        SELECT
          pmg.product_id,
          jsonb_agg(
            jsonb_build_object(
              'id', mg.id,
              'name', COALESCE(mgt.name, mgt_def.name, mg.name),
              'min_select', mg.min_select,
              'max_select', mg.max_select,
              'required', mg.required,
              'sort_order', pmg.sort_order,
              'modifiers', COALESCE(mj.modifiers, '[]'::jsonb)
            ) ORDER BY pmg.sort_order
          ) as modifier_groups
        FROM modifier_groups mg
        JOIN product_modifier_groups pmg ON pmg.group_id = mg.id
        LEFT JOIN modifier_group_translations mgt ON mgt.group_id = mg.id AND mgt.locale = p_locale
        LEFT JOIN modifier_group_translations mgt_def ON mgt_def.group_id = mg.id AND mgt_def.locale = v_def_locale
        LEFT JOIN modifiers_json mj ON mj.group_id = mg.id
        WHERE mg.location_id = v_location_id
        GROUP BY pmg.product_id
      ),
      products_json AS (
        SELECT
          p.category_id,
          jsonb_agg(
            jsonb_build_object(
              'id', p.id,
              'name', COALESCE(pt.name, pt_def.name, p.name),
              'description', COALESCE(pt.description, pt_def.description, p.description),
              'price', p.price,
              'available', p.is_available,
              'image_key', p.image_key,
              'attributes', p.attributes,
              'modifier_groups', COALESCE(mgj.modifier_groups, '[]'::jsonb)
            ) ORDER BY p.sort_order
          ) as products
        FROM products p
        LEFT JOIN product_translations pt ON pt.product_id = p.id AND pt.locale = p_locale
        LEFT JOIN product_translations pt_def ON pt_def.product_id = p.id AND pt_def.locale = v_def_locale
        LEFT JOIN modifier_groups_json mgj ON mgj.product_id = p.id
        WHERE p.location_id = v_location_id AND p.is_available = true
        GROUP BY p.category_id
      ),
      categories_json AS (
        SELECT
          jsonb_agg(
            jsonb_build_object(
              'id', c.id,
              'name', COALESCE(ct.name, ct_def.name, c.name),
              'sort_order', c.sort_order,
              'products', COALESCE(pj.products, '[]'::jsonb)
            ) ORDER BY c.sort_order
          ) as categories
        FROM categories c
        LEFT JOIN category_translations ct ON ct.category_id = c.id AND ct.locale = p_locale
        LEFT JOIN category_translations ct_def ON ct_def.category_id = c.id AND ct_def.locale = v_def_locale
        JOIN products_json pj ON pj.category_id = c.id
        WHERE c.location_id = v_location_id
      )
      SELECT
        jsonb_build_object(
          'menu_version', v_version,
          'default_locale', v_def_locale,
          'supported_locales', v_supp_locales,
          'currency', jsonb_build_object('code', v_currency_code, 'minor_unit', v_currency_minor_unit),
          'categories', COALESCE((SELECT categories FROM categories_json), '[]'::jsonb)
        ) INTO v_result;

      RETURN v_result;
    END;
    $$;
  `);

  // ── 2. Crawler SSR: serve published storefronts (was status='active' only) ──
  pgm.sql(`
    CREATE OR REPLACE FUNCTION public.read_public_menu_all_locales(p_location_id_or_slug text)
    RETURNS jsonb LANGUAGE plpgsql AS $$
    DECLARE
      v_location_id uuid; v_def_locale text; v_supp_locales text[];
      v_currency_code text; v_currency_minor_unit int;
      v_name text; v_address text; v_phone text;
      v_hours jsonb; v_geo jsonb; v_menu_version bigint; v_result jsonb;
    BEGIN
      SELECT id, default_locale, supported_locales, currency_code, currency_minor_unit, name, address, public_phone, hours_json, geo, menu_version
      INTO v_location_id, v_def_locale, v_supp_locales, v_currency_code, v_currency_minor_unit, v_name, v_address, v_phone, v_hours, v_geo, v_menu_version
      FROM locations WHERE slug = p_location_id_or_slug
        AND (status IN ('active', 'open') OR published_at IS NOT NULL);
      IF v_location_id IS NULL THEN RETURN NULL; END IF;
      WITH products_json AS (
        SELECT p.category_id, jsonb_agg(jsonb_build_object(
          'id', p.id, 'price', p.price, 'available', p.is_available, 'image_key', p.image_key, 'attributes', p.attributes,
          'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, p.name)) FROM (SELECT v_def_locale as locale, p.name UNION ALL SELECT locale, name FROM product_translations WHERE product_id = p.id AND locale = ANY(v_supp_locales)) t),
          'available_descriptions', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.description, p.description)) FROM (SELECT v_def_locale as locale, p.description UNION ALL SELECT locale, description FROM product_translations WHERE product_id = p.id AND locale = ANY(v_supp_locales)) t WHERE t.description IS NOT NULL),
          'modifier_groups', COALESCE(mgj.modifier_groups, '[]'::jsonb)
        ) ORDER BY p.sort_order) as products
        FROM products p
        LEFT JOIN (SELECT product_id, jsonb_agg(jsonb_build_object('id', mg.id, 'name', mg.name, 'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, mg.name)) FROM (SELECT v_def_locale as locale, mg.name UNION ALL SELECT locale, name FROM modifier_group_translations WHERE group_id = mg.id AND locale = ANY(v_supp_locales)) t), 'required', mg.required, 'max_select', mg.max_select, 'min_select', mg.min_select, 'sort_order', mg.sort_order, 'modifiers', COALESCE(mj.modifiers, '[]'::jsonb)) ORDER BY mg.sort_order) as modifier_groups FROM modifier_groups mg LEFT JOIN (SELECT modifier_group_id, jsonb_agg(jsonb_build_object('id', m.id, 'name', m.name, 'available', m.is_available, 'sort_order', m.sort_order, 'price_delta', m.price_delta, 'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, m.name)) FROM (SELECT v_def_locale as locale, m.name UNION ALL SELECT locale, name FROM modifier_translations WHERE modifier_id = m.id AND locale = ANY(v_supp_locales)) t)) ORDER BY m.sort_order) as modifiers FROM modifiers m WHERE m.is_available = true GROUP BY m.modifier_group_id) mj ON mj.modifier_group_id = mg.id GROUP BY mg.product_id) mgj ON mgj.product_id = p.id
        WHERE p.location_id = v_location_id AND p.is_available = true GROUP BY p.category_id
      ), categories_json AS (
        SELECT jsonb_agg(jsonb_build_object('id', c.id, 'sort_order', c.sort_order,
          'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, c.name)) FROM (SELECT v_def_locale as locale, c.name UNION ALL SELECT locale, name FROM category_translations WHERE category_id = c.id AND locale = ANY(v_supp_locales)) t),
          'products', COALESCE(pj.products, '[]'::jsonb)
        ) ORDER BY c.sort_order) as categories
        FROM categories c JOIN products_json pj ON pj.category_id = c.id WHERE c.location_id = v_location_id
      )
      SELECT jsonb_build_object('menu_version', v_menu_version, 'default_locale', v_def_locale, 'supported_locales', v_supp_locales, 'currency', jsonb_build_object('code', v_currency_code, 'minor_unit', v_currency_minor_unit), 'location', jsonb_build_object('name', v_name, 'address', v_address, 'public_phone', v_phone, 'hours', v_hours, 'geo', v_geo), 'categories', COALESCE((SELECT categories FROM categories_json), '[]'::jsonb)) INTO v_result;
      RETURN v_result;
    END;
    $$;
  `);

  // ── 3. Backfill demo modifier/group translations (sq/en/uk), idempotent ─────
  // Owner bypasses RLS only when FORCE is off; drop FORCE for the inserts, restore after.
  pgm.sql(`ALTER TABLE modifier_translations NO FORCE ROW LEVEL SECURITY;`);
  pgm.sql(`ALTER TABLE modifier_group_translations NO FORCE ROW LEVEL SECURITY;`);

  pgm.sql(`
    INSERT INTO modifier_translations (modifier_id, locale, name)
    SELECT m.id, v.locale, v.name
    FROM modifiers m
    JOIN locations l ON l.id = m.location_id AND l.slug = 'demo'
    JOIN (VALUES
      ('Мала','sq','E vogël'), ('Мала','en','Small'), ('Мала','uk','Мала'),
      ('Велика','sq','E madhe'), ('Велика','en','Large'), ('Велика','uk','Велика'),
      ('Сирок','sq','Djathë'), ('Сирок','en','Cheese'), ('Сирок','uk','Сирок'),
      ('Гриби','sq','Kërpudha'), ('Гриби','en','Mushrooms'), ('Гриби','uk','Гриби')
    ) AS v(src, locale, name) ON v.src = m.name
    ON CONFLICT (modifier_id, locale) DO NOTHING;
  `);

  pgm.sql(`
    INSERT INTO modifier_group_translations (group_id, locale, name)
    SELECT mg.id, v.locale, v.name
    FROM modifier_groups mg
    JOIN locations l ON l.id = mg.location_id AND l.slug = 'demo'
    JOIN (VALUES
      ('Розмір','sq','Madhësia'), ('Розмір','en','Size'), ('Розмір','uk','Розмір'),
      ('Додатки','sq','Shtesa'), ('Додатки','en','Add-ons'), ('Додатки','uk','Додатки')
    ) AS v(src, locale, name) ON v.src = mg.name
    ON CONFLICT (group_id, locale) DO NOTHING;
  `);

  pgm.sql(`ALTER TABLE modifier_translations FORCE ROW LEVEL SECURITY;`);
  pgm.sql(`ALTER TABLE modifier_group_translations FORCE ROW LEVEL SECURITY;`);
}

export async function down(): Promise<void> {
  // No-op: superseded function definitions; reverting would reintroduce the i18n leak.
}
