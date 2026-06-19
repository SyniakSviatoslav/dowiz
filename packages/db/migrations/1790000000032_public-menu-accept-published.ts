import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * P0-STATUS · Make read_public_menu() serve a *published* storefront.
 *
 * Bug: 1790000000018_fix-public-menu-slug-lookup filtered `status = 'active'`,
 * but the onboarding publish flow (1790000000030_onboarding-publish-state) sets
 * `status = 'open'` (the daily open/close switch) and `published_at IS NOT NULL`
 * — it never sets `status = 'active'`. Result: a freshly published tenant's
 * public menu returns NULL → API responds "Location not found".
 *
 * Fix: a location is serveable if it is published (`published_at IS NOT NULL`)
 * OR carries a live status (`status IN ('active','open')`). This keeps legacy
 * 'active' locations working while also serving the new 'open'/published ones.
 * Everything else in the function body is byte-identical to 0018.
 *
 * -----------------------------------------------------------------------------
 * PROD BACKFILL (write-only — DO NOT run from this migration). For the 21 prod
 * locations that were published but left at status='open' without published_at,
 * run the following ONCE against prod after deploy (review row count first):
 *
 *   -- dry run: how many rows would change?
 *   SELECT count(*) FROM locations
 *   WHERE published_at IS NULL
 *     AND ( status = 'open'
 *           OR EXISTS (SELECT 1 FROM products p WHERE p.location_id = locations.id) );
 *
 *   -- backfill: stamp published_at for live storefronts that lack it
 *   UPDATE locations l SET published_at = COALESCE(l.published_at, l.created_at, now())
 *   WHERE l.published_at IS NULL
 *     AND ( l.status = 'open'
 *           OR EXISTS (SELECT 1 FROM products p WHERE p.location_id = l.id) );
 *
 * (This new function does not REQUIRE the backfill — `status IN ('active','open')`
 * already serves them — but stamping published_at keeps the publish-state model
 * consistent for the rest of the app.)
 * -----------------------------------------------------------------------------
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
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
              'name', m.name,
              'price_delta', m.price_delta,
              'available', m.available,
              'sort_order', m.sort_order
            ) ORDER BY m.sort_order
          ) as modifiers
        FROM modifiers m
        WHERE m.location_id = v_location_id AND m.available = true
        GROUP BY m.group_id
      ),
      modifier_groups_json AS (
        SELECT
          pmg.product_id,
          jsonb_agg(
            jsonb_build_object(
              'id', mg.id,
              'name', mg.name,
              'min_select', mg.min_select,
              'max_select', mg.max_select,
              'required', mg.required,
              'sort_order', pmg.sort_order,
              'modifiers', COALESCE(mj.modifiers, '[]'::jsonb)
            ) ORDER BY pmg.sort_order
          ) as modifier_groups
        FROM modifier_groups mg
        JOIN product_modifier_groups pmg ON pmg.group_id = mg.id
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
}

export async function down(): Promise<void> {
  // No-op: superseded definition; reverting would reintroduce the bug.
}
