import type { MigrationBuilder } from 'node-pg-migrate';

export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`
    CREATE OR REPLACE FUNCTION bump_menu_version_trigger_fn()
    RETURNS TRIGGER
    SECURITY DEFINER
    LANGUAGE plpgsql
    AS $$
    DECLARE
      v_location_id uuid;
    BEGIN
      IF TG_OP = 'DELETE' THEN
        IF TG_TABLE_NAME = 'product_translations' THEN
          -- Need to get location_id from products
          SELECT location_id INTO v_location_id FROM products WHERE id = OLD.product_id;
        ELSIF TG_TABLE_NAME = 'category_translations' THEN
          -- Need to get location_id from categories
          SELECT location_id INTO v_location_id FROM categories WHERE id = OLD.category_id;
        ELSIF TG_TABLE_NAME = 'locations' THEN
          v_location_id := OLD.id;
        ELSE
          v_location_id := OLD.location_id;
        END IF;
      ELSE
        IF TG_TABLE_NAME = 'product_translations' THEN
          SELECT location_id INTO v_location_id FROM products WHERE id = NEW.product_id;
        ELSIF TG_TABLE_NAME = 'category_translations' THEN
          SELECT location_id INTO v_location_id FROM categories WHERE id = NEW.category_id;
        ELSIF TG_TABLE_NAME = 'locations' THEN
          v_location_id := NEW.id;
        ELSE
          v_location_id := NEW.location_id;
        END IF;
      END IF;

      -- If we somehow don't have a location, just return (shouldn't happen for these tables)
      IF v_location_id IS NOT NULL THEN
        PERFORM upsert_menu_version(v_location_id);
      END IF;

      IF TG_OP = 'DELETE' THEN
        RETURN OLD;
      ELSE
        RETURN NEW;
      END IF;
    END;
    $$;

    -- Attach to all menu tables
    CREATE TRIGGER trg_bump_menu_version_categories
    AFTER INSERT OR UPDATE OR DELETE ON categories
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    CREATE TRIGGER trg_bump_menu_version_products
    AFTER INSERT OR UPDATE OR DELETE ON products
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    CREATE TRIGGER trg_bump_menu_version_modifier_groups
    AFTER INSERT OR UPDATE OR DELETE ON modifier_groups
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    CREATE TRIGGER trg_bump_menu_version_modifiers
    AFTER INSERT OR UPDATE OR DELETE ON modifiers
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    CREATE TRIGGER trg_bump_menu_version_product_modifier_groups
    AFTER INSERT OR UPDATE OR DELETE ON product_modifier_groups
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    CREATE TRIGGER trg_bump_menu_version_product_translations
    AFTER INSERT OR UPDATE OR DELETE ON product_translations
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    CREATE TRIGGER trg_bump_menu_version_category_translations
    AFTER INSERT OR UPDATE OR DELETE ON category_translations
    FOR EACH ROW EXECUTE FUNCTION bump_menu_version_trigger_fn();

    -- Locations trigger only on specific columns
    CREATE TRIGGER trg_bump_menu_version_locations
    AFTER UPDATE OF default_locale, supported_locales ON locations
    FOR EACH ROW
    WHEN (OLD.default_locale IS DISTINCT FROM NEW.default_locale OR OLD.supported_locales IS DISTINCT FROM NEW.supported_locales)
    EXECUTE FUNCTION bump_menu_version_trigger_fn();
  `);
}

export async function down(): Promise<void> {}
