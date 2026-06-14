import pg from 'pg';

const client = new pg.Client({
  connectionString: 'postgresql://postgres.elxukhxvuycnftqwaghg:7V%23KxApMx8Z5B5.@aws-1-eu-central-1.pooler.supabase.com:5432/postgres'
});

async function run() {
  await client.connect();
  console.log("Connected");

  const sql = `
    CREATE OR REPLACE FUNCTION read_public_menu_all_locales(p_location_id_or_slug text)
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
      v_name text;
      v_phone text;
    BEGIN
      -- 1. Get Location Info from actual DB columns
      SELECT 
        id, name, phone,
        COALESCE(default_locale, 'sq'),
        COALESCE(supported_locales, ARRAY['sq','en']),
        COALESCE(currency_code, 'ALL'),
        COALESCE(currency_minor_unit, 0)
      INTO 
        v_location_id, v_name, v_phone,
        v_def_locale,
        v_supp_locales,
        v_currency_code,
        v_currency_minor_unit
      FROM locations
      WHERE (id::text = p_location_id_or_slug OR slug = p_location_id_or_slug);

      IF NOT FOUND THEN
        RETURN NULL;
      END IF;

      -- 2. Get Menu Version
      SELECT version INTO v_version FROM menu_versions WHERE location_id = v_location_id;
      IF v_version IS NULL THEN
        v_version := 1;
      END IF;

      -- 3. Build JSON with all locales from actual translation tables
      WITH 
      modifiers_json AS (
        SELECT 
          m.group_id,
          jsonb_agg(
            jsonb_build_object(
              'id', m.id,
              'price_delta', m.price_delta,
              'available', m.available,
              'sort_order', m.sort_order,
              'available_names', (
                SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, m.name))
                FROM (
                  SELECT v_def_locale as locale, m.name
                  UNION ALL
                  SELECT locale, name FROM modifier_translations WHERE modifier_id = m.id AND locale = ANY(v_supp_locales)
                ) t
              )
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
              'min_select', mg.min_select,
              'max_select', mg.max_select,
              'required', mg.required,
              'sort_order', pmg.sort_order,
              'available_names', (
                SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, mg.name))
                FROM (
                  SELECT v_def_locale as locale, mg.name
                  UNION ALL
                  SELECT locale, name FROM modifier_group_translations WHERE group_id = mg.id AND locale = ANY(v_supp_locales)
                ) t
              ),
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
              'price', p.price,
              'available', p.is_available,
              'image_key', p.image_key,
              'attributes', p.attributes,
              'available_names', (
                SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, p.name))
                FROM (
                  SELECT v_def_locale as locale, p.name
                  UNION ALL
                  SELECT locale, name FROM product_translations WHERE product_id = p.id AND locale = ANY(v_supp_locales)
                ) t
              ),
              'available_descriptions', (
                SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.description, p.description))
                FROM (
                  SELECT v_def_locale as locale, p.description
                  UNION ALL
                  SELECT locale, description FROM product_translations WHERE product_id = p.id AND locale = ANY(v_supp_locales)
                ) t
                WHERE t.description IS NOT NULL
              ),
              'modifier_groups', COALESCE(mgj.modifier_groups, '[]'::jsonb)
            ) ORDER BY p.sort_order
          ) as products
        FROM products p
        LEFT JOIN modifier_groups_json mgj ON mgj.product_id = p.id
        WHERE p.location_id = v_location_id AND p.is_available = true
        GROUP BY p.category_id
      ),
      categories_json AS (
        SELECT 
          jsonb_agg(
            jsonb_build_object(
              'id', c.id,
              'sort_order', c.sort_order,
              'available_names', (
                SELECT jsonb_object_agg(COALESCE(t.locale, v_def_locale), COALESCE(t.name, c.name))
                FROM (
                  SELECT v_def_locale as locale, c.name
                  UNION ALL
                  SELECT locale, name FROM category_translations WHERE category_id = c.id AND locale = ANY(v_supp_locales)
                ) t
              ),
              'products', COALESCE(pj.products, '[]'::jsonb)
            ) ORDER BY c.sort_order
          ) as categories
        FROM categories c
        JOIN products_json pj ON pj.category_id = c.id
        WHERE c.location_id = v_location_id
      )
      SELECT 
        jsonb_build_object(
          'menu_version', v_version,
          'default_locale', v_def_locale,
          'supported_locales', v_supp_locales,
          'currency', jsonb_build_object('code', v_currency_code, 'minor_unit', v_currency_minor_unit),
          'location', jsonb_build_object(
            'id', v_location_id,
            'name', v_name,
            'public_phone', v_phone,
            'fallback_phone', v_phone
          ),
          'categories', COALESCE((SELECT categories FROM categories_json), '[]'::jsonb)
        ) INTO v_result;

      RETURN v_result;
    END;
    $$;
  `;

  try {
    await client.query(sql);
    console.log("Function replaced successfully with actual DB column values.");
  } catch(e) {
    console.error(e);
  } finally {
    await client.end();
  }
}
run();
