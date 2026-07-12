import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * MENU-AVAILABILITY · read_public_menu surfaces modifier-group `display_type`
 * and AND-combines the schedule engine with `is_available` (additive).
 *
 * VERBATIM copy of the latest definition (migration 1790000000063's predecessor
 * 1790000000055 — locale-aware modifiers + published-status serving + primary_media_id +
 * the `p_locale text DEFAULT ''::text` signature). Only additions versus 055:
 *   1. select `l.timezone` into v_tz (for schedule windows, venue-local).
 *   2. `'display_type', mg.display_type` in the modifier-groups object.
 *   3. the products WHERE adds `AND product_available_now(p.id, p.category_id, v_tz)`
 *      — a product with NO schedule rows is always-available, so unscheduled menus
 *      are byte-identical. (See 1790000000062 for the derivation semantics.)
 *
 * CREATE OR REPLACE preserves the parameter default (Postgres forbids removing it);
 * the signature matches 055. read_public_menu_all_locales (SSR) is deliberately NOT
 * touched (SSR/JSON-LD never carried modifiers or per-time availability).
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
      v_tz text;
    BEGIN
      SELECT id, default_locale, supported_locales, currency_code, currency_minor_unit, timezone
      INTO v_location_id, v_def_locale, v_supp_locales, v_currency_code, v_currency_minor_unit, v_tz
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
              'display_type', mg.display_type,
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
              'primary_media_id', p.primary_media_id,
              'attributes', p.attributes,
              'modifier_groups', COALESCE(mgj.modifier_groups, '[]'::jsonb)
            ) ORDER BY p.sort_order
          ) as products
        FROM products p
        LEFT JOIN product_translations pt ON pt.product_id = p.id AND pt.locale = p_locale
        LEFT JOIN product_translations pt_def ON pt_def.product_id = p.id AND pt_def.locale = v_def_locale
        LEFT JOIN modifier_groups_json mgj ON mgj.product_id = p.id
        WHERE p.location_id = v_location_id AND p.is_available = true
          AND product_available_now(p.id, p.category_id, v_tz)
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
  // Forward-only. The added fields are additive (display_type null when unset; a product
  // with no schedule is always-available); no need to revert.
}
