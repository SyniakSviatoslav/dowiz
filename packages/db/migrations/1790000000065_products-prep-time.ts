import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * Prep-Time + ETA (v1) · CHANGE 1 — prep_time_minutes on products.
 *
 * Adds `products.prep_time_minutes int NOT NULL DEFAULT 15` and surfaces it in the
 * storefront menu function so the card can show it (as element #2 after price).
 *
 * Why DEFAULT 15 (and KEEP the default): ~7 INSERT paths (owner create, PDF import,
 * onboarding, dev-seed) do NOT pass this column. A NOT NULL column without a default
 * would 500 every one of them and break dev-seed (Visual Regression Net base). The
 * default protects those paths; the OWNER requirement ("can't save without prep-time")
 * is enforced at the API/validation layer, not the DB.
 *
 * The function body is a VERBATIM copy of 1790000000064 (read-public-menu-perf) with a
 * single added field `'prep_time_minutes', p.prep_time_minutes` in the product object —
 * additive to the JSON shape (consumers ignore unknown fields), cache contract unchanged.
 * down() restores the exact 064 body and drops the column.
 */
const FN_065_PREPTIME = `
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
              'attributes', p.attributes,
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
`;

// VERBATIM prior body (migration 1790000000064) — restored by down() for rollback.
const FN_064_PRIOR = `
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
              'attributes', p.attributes,
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
`;

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TABLE products ADD COLUMN IF NOT EXISTS prep_time_minutes int NOT NULL DEFAULT 15;`);
  pgm.sql(FN_065_PREPTIME);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(FN_064_PRIOR);
  pgm.sql(`ALTER TABLE products DROP COLUMN IF EXISTS prep_time_minutes;`);
}
