import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * TG-NOTIF-2 · locations.timezone for quiet-hours gating (BR-14).
 *
 * NULLABLE, no default — BR-14 explicitly warned a NOT NULL DEFAULT silently imposes
 * an unaudited intent on every existing row. NULL => dispatcher falls back to the
 * default TZ and writes audit 'quiet_tz_fallback' (see 1790000000048).
 *
 * Validated against pg_timezone_names via a BEFORE trigger (a CHECK constraint cannot
 * reference a catalog view). Guards against `now() AT TIME ZONE '<bad>'` throwing and
 * silently breaking quiet-gating for a whole location. Locations are low-write, so the
 * per-write pg_timezone_names scan is negligible.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`ALTER TABLE locations ADD COLUMN IF NOT EXISTS timezone text;`);

  pgm.sql(`
    CREATE OR REPLACE FUNCTION validate_location_timezone() RETURNS trigger
    LANGUAGE plpgsql AS $$
    BEGIN
      IF NEW.timezone IS NOT NULL
         AND NOT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = NEW.timezone) THEN
        RAISE EXCEPTION 'invalid timezone: %', NEW.timezone
          USING ERRCODE = 'invalid_parameter_value';
      END IF;
      RETURN NEW;
    END
    $$;
  `);

  pgm.sql(`
    DROP TRIGGER IF EXISTS trg_validate_location_timezone ON locations;
    CREATE TRIGGER trg_validate_location_timezone
      BEFORE INSERT OR UPDATE OF timezone ON locations
      FOR EACH ROW EXECUTE FUNCTION validate_location_timezone();
  `);
}

export async function down(pgm: MigrationBuilder): Promise<void> {
  pgm.sql(`DROP TRIGGER IF EXISTS trg_validate_location_timezone ON locations;`);
  pgm.sql(`DROP FUNCTION IF EXISTS validate_location_timezone();`);
  pgm.sql(`ALTER TABLE locations DROP COLUMN IF EXISTS timezone;`);
}
