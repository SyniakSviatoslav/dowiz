import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * Hotfix (follow-up to 0034) · read_public_menu_all_locales still assumed
 * modifier_groups.product_id (a direct FK) — but groups attach to products via
 * the product_modifier_groups junction (many-to-many), like read_public_menu.
 * 0034 fixed group_id/available; this fixes the join structure so the crawler
 * SSR (/s/:slug for bots) returns 200 with per-tenant <title>/OG/JSON-LD instead
 * of 500. Verified against live staging data before shipping.
 */

export async function up(pgm: MigrationBuilder): Promise<void> {
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
  WITH
  modifiers_json AS (
    SELECT m.group_id,
      jsonb_agg(jsonb_build_object(
        'id', m.id, 'name', m.name, 'available', m.available, 'sort_order', m.sort_order, 'price_delta', m.price_delta,
        'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, m.name)) FROM (SELECT v_def_locale as locale, m.name UNION ALL SELECT locale, name FROM modifier_translations WHERE modifier_id = m.id AND locale = ANY(v_supp_locales)) t)
      ) ORDER BY m.sort_order) as modifiers
    FROM modifiers m WHERE m.location_id = v_location_id AND m.available = true GROUP BY m.group_id
  ),
  modifier_groups_json AS (
    SELECT pmg.product_id,
      jsonb_agg(jsonb_build_object(
        'id', mg.id, 'name', mg.name,
        'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, mg.name)) FROM (SELECT v_def_locale as locale, mg.name UNION ALL SELECT locale, name FROM modifier_group_translations WHERE group_id = mg.id AND locale = ANY(v_supp_locales)) t),
        'required', mg.required, 'max_select', mg.max_select, 'min_select', mg.min_select, 'sort_order', pmg.sort_order,
        'modifiers', COALESCE(mj.modifiers, '[]'::jsonb)
      ) ORDER BY pmg.sort_order) as modifier_groups
    FROM modifier_groups mg
    JOIN product_modifier_groups pmg ON pmg.group_id = mg.id
    LEFT JOIN modifiers_json mj ON mj.group_id = mg.id
    WHERE mg.location_id = v_location_id
    GROUP BY pmg.product_id
  ),
  products_json AS (
    SELECT p.category_id, jsonb_agg(jsonb_build_object(
      'id', p.id, 'price', p.price, 'available', p.is_available, 'image_key', p.image_key, 'attributes', p.attributes,
      'available_names', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, p.name)) FROM (SELECT v_def_locale as locale, p.name UNION ALL SELECT locale, name FROM product_translations WHERE product_id = p.id AND locale = ANY(v_supp_locales)) t),
      'available_descriptions', (SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.description, p.description)) FROM (SELECT v_def_locale as locale, p.description UNION ALL SELECT locale, description FROM product_translations WHERE product_id = p.id AND locale = ANY(v_supp_locales)) t WHERE t.description IS NOT NULL),
      'modifier_groups', COALESCE(mgj.modifier_groups, '[]'::jsonb)
    ) ORDER BY p.sort_order) as products
    FROM products p
    LEFT JOIN modifier_groups_json mgj ON mgj.product_id = p.id
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
}

export async function down(): Promise<void> {
  // No-op: corrected definition.
}
