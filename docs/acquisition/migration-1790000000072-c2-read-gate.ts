// STAGED MIGRATION ARTIFACT — operator places this at
//   packages/db/migrations/1790000000072_c2-read-gate.ts
// (migrations dir is a protected governance zone; manual-approval handoff, mirroring 068-071).
//
// 🔴🔴 HOT-PATH, TOTAL BLAST RADIUS — read_public_menu / read_public_menu_all_locales serve EVERY
// tenant's storefront menu. **REQUIRES 068 applied** (the `source` / `allergens_confirmed` columns; else
// CREATE OR REPLACE errors "column does not exist" → every menu down). Before placing, RE-DIFF the two
// bodies below against the THEN-CURRENT live definitions (a later migration may have re-versioned them):
//   read_public_menu             — verbatim from 1790000000065_products-prep-time.ts (up body)
//   read_public_menu_all_locales — verbatim from 1790000000035_fix-all-locales-junction.ts (latest CREATE;
//                                  066/067 touch neither — verified 2026-06-28)
// The ONLY change vs the verbatim bodies is the single `'attributes', p.attributes` site in each →
//   'attributes', CASE WHEN p.source='place' AND p.allergens_confirmed=false THEN p.attributes - 'bom' ELSE p.attributes END
// so a CLAIMED+published shadow whose products are still place/unconfirmed never surfaces AI allergens
// (operator decision #2 / council C2 — the POST-claim safety layer; pre-claim is covered by the write-strip
// + read_preview_menu, both already proven).
//
// PROOF GATE (council C5 — do NOT place blind): apply on a FULL-schema staging DB, then
//   (1) GOLDEN no-op: for a real source='owner' tenant, read_public_menu(slug,locale) JSON is BYTE-IDENTICAL
//       before vs after (the CASE can only fire for place+unconfirmed → owner tenants are untouched);
//   (2) positive: claimed+published shadow, place+unconfirmed → returned attributes has NO 'bom' key;
//   (3) negative: same product allergens_confirmed=true → 'bom' present;
//   (4) Mandatory-Proof Playwright vs staging /s/:slug — demo menu visible + a confirmed product's allergens show.
// down() restores the byte-exact 065/035 bodies.
import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE OR REPLACE FUNCTION public.read_public_menu(p_location_id_or_slug text, p_locale text DEFAULT ''::text)
    RETURNS jsonb
    SECURITY DEFINER
    LANGUAGE plpgsql
    AS $FN$
    DECLARE
      v_result jsonb;
      v_location_id uuid;
      v_location_name text;
      v_def_locale text;
      v_supp_locales text[];
      v_currency_code text;
      v_currency_minor_unit int;
      v_version bigint;
      v_tz text;
      v_local timestamp;
    BEGIN
      SELECT id, name, default_locale, supported_locales, currency_code, currency_minor_unit, timezone
      INTO v_location_id, v_location_name, v_def_locale, v_supp_locales, v_currency_code, v_currency_minor_unit, v_tz
      FROM locations
      WHERE (id::text = p_location_id_or_slug OR slug = p_location_id_or_slug)
        AND (status IN ('active', 'open') OR published_at IS NOT NULL);

      IF NOT FOUND THEN
        RETURN NULL;
      END IF;

      IF NOT (p_locale = ANY(v_supp_locales)) THEN
        p_locale := v_def_locale;
      END IF;

      v_local := (now() AT TIME ZONE COALESCE(v_tz, 'UTC'));

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
              'attributes', CASE WHEN p.source = 'place' AND p.allergens_confirmed = false THEN p.attributes - 'bom' ELSE p.attributes END,
              'prep_time_minutes', p.prep_time_minutes,
              'modifier_groups', COALESCE(mgj.modifier_groups, '[]'::jsonb)
            ) ORDER BY p.sort_order
          ) as products
        FROM products p
        LEFT JOIN product_translations pt ON pt.product_id = p.id AND pt.locale = p_locale
        LEFT JOIN product_translations pt_def ON pt_def.product_id = p.id AND pt_def.locale = v_def_locale
        LEFT JOIN modifier_groups_json mgj ON mgj.product_id = p.id
        WHERE p.location_id = v_location_id AND p.is_available = true
          AND NOT EXISTS (
            SELECT 1 FROM menu_schedules s
            WHERE (s.product_id = p.id OR s.category_id = p.category_id)
              AND s.available = false
              AND menu_schedule_matches(s.mode, s.start_minute, s.end_minute,
                                        s.days_of_week, s.starts_at, s.ends_at, v_local)
          )
          AND (
            NOT EXISTS (
              SELECT 1 FROM menu_schedules s2
              WHERE (s2.product_id = p.id OR s2.category_id = p.category_id)
                AND s2.available = true
            )
            OR EXISTS (
              SELECT 1 FROM menu_schedules s3
              WHERE (s3.product_id = p.id OR s3.category_id = p.category_id)
                AND s3.available = true
                AND menu_schedule_matches(s3.mode, s3.start_minute, s3.end_minute,
                                          s3.days_of_week, s3.starts_at, s3.ends_at, v_local)
            )
          )
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
          'location_id', v_location_id,
          'location_name', v_location_name,
          'default_locale', v_def_locale,
          'supported_locales', v_supp_locales,
          'currency', jsonb_build_object('code', v_currency_code, 'minor_unit', v_currency_minor_unit),
          'categories', COALESCE((SELECT categories FROM categories_json), '[]'::jsonb)
        ) INTO v_result;

      RETURN v_result;
    END;
    $FN$;
  `);

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
          'id', p.id, 'price', p.price, 'available', p.is_available, 'image_key', p.image_key, 'attributes', CASE WHEN p.source = 'place' AND p.allergens_confirmed = false THEN p.attributes - 'bom' ELSE p.attributes END,
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
  // Forward-only intent: down() must restore the byte-exact 065/035 bodies (re-paste them here at
  // placement time). Left a no-op in the staged artifact to avoid shipping a stale body that would
  // silently revert a later re-version.
}
