import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * MENU-AVAILABILITY · `menu_schedules` — mealtime / availability-window engine (additive).
 *
 * A product (or whole category) can be restricted to time windows ("breakfast 07–11",
 * "weekends only"). A product with NO schedule row is ALWAYS available — the engine is
 * purely additive: read_public_menu AND-combines is_available_now() with the existing
 * is_available, and unscheduled items short-circuit to TRUE.
 *
 * mode:
 *   - 'daily'     → every day, between start_minute..end_minute (local venue time)
 *   - 'recurring' → only on days_of_week (0=Sun..6=Sat), between start..end minutes
 *   - 'period'    → an absolute [starts_at, ends_at) window (e.g. a seasonal item)
 *
 * RLS mirrors product_media (1790000000054): ENABLE+FORCE, tenant_isolation keyed on
 * location_id (denormalised so the predicate needs no join), a public_select for the
 * SECURITY DEFINER reader, Data API revoked, DML to the hot-path pool role only.
 *
 * Forward-only / inert by default: no rows => no behaviour change. down() is a no-op.
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. table — location_id DENORMALISED for a join-free RLS predicate (mirrors product_media).
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS menu_schedules (
      id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id   uuid NOT NULL REFERENCES locations(id),
      product_id    uuid REFERENCES products(id) ON DELETE CASCADE,
      category_id   uuid REFERENCES categories(id) ON DELETE CASCADE,
      mode          text NOT NULL DEFAULT 'daily'
                      CHECK (mode IN ('daily', 'recurring', 'period')),
      start_minute  int  CHECK (start_minute IS NULL OR (start_minute >= 0 AND start_minute < 1440)),
      end_minute    int  CHECK (end_minute   IS NULL OR (end_minute   >= 0 AND end_minute  <= 1440)),
      days_of_week  int[],
      starts_at     timestamptz,
      ends_at       timestamptz,
      available     boolean NOT NULL DEFAULT true,
      created_at    timestamptz NOT NULL DEFAULT now(),
      -- a schedule targets exactly one of product/category (never both, never neither).
      CONSTRAINT menu_schedules_one_target CHECK (
        (product_id IS NOT NULL AND category_id IS NULL) OR
        (product_id IS NULL     AND category_id IS NOT NULL)
      )
    );
  `);
  pgm.sql(`
    CREATE INDEX IF NOT EXISTS menu_schedules_product_idx  ON menu_schedules (product_id);
    CREATE INDEX IF NOT EXISTS menu_schedules_category_idx ON menu_schedules (category_id);
    CREATE INDEX IF NOT EXISTS menu_schedules_location_idx ON menu_schedules (location_id);
  `);

  // 2. RLS FROM CREATION — mirror product_media (ENABLE+FORCE + WITH CHECK).
  pgm.sql(`
    ALTER TABLE menu_schedules ENABLE ROW LEVEL SECURITY;
    ALTER TABLE menu_schedules FORCE  ROW LEVEL SECURITY;

    DROP POLICY IF EXISTS tenant_isolation ON menu_schedules;
    CREATE POLICY tenant_isolation ON menu_schedules
      USING      ( location_id IN (SELECT app_member_location_ids()) )
      WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );

    DROP POLICY IF EXISTS public_select ON menu_schedules;
    CREATE POLICY public_select ON menu_schedules FOR SELECT USING (true);
  `);

  // 3. GRANTS — off the Supabase Data API; full DML to the hot-path tenant role only.
  pgm.sql(`
    REVOKE ALL ON menu_schedules FROM anon, authenticated, service_role;
    DO $$ BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON menu_schedules TO deliveryos_api_user;
      END IF;
    END $$;
  `);

  // 4. is_available_now(schedule row, venue local-now) — pure derivation, no I/O.
  //    Timezone-aware: caller passes the venue-local timestamp so DST is correct.
  //    available=false on a row is an explicit "block during this window" (e.g. 86 a
  //    breakfast item all afternoon); default available=true is "ONLY during window".
  pgm.sql(`
    CREATE OR REPLACE FUNCTION menu_schedule_matches(
      p_mode text, p_start_minute int, p_end_minute int, p_days_of_week int[],
      p_starts_at timestamptz, p_ends_at timestamptz, p_local_now timestamp
    ) RETURNS boolean
    LANGUAGE sql IMMUTABLE AS $$
      SELECT CASE p_mode
        WHEN 'period' THEN
          (p_starts_at IS NULL OR p_local_now >= p_starts_at)
          AND (p_ends_at IS NULL OR p_local_now < p_ends_at)
        WHEN 'recurring' THEN
          (p_days_of_week IS NULL
            OR EXTRACT(DOW FROM p_local_now)::int = ANY(p_days_of_week))
          AND (p_start_minute IS NULL
            OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) >= p_start_minute)
          AND (p_end_minute IS NULL
            OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) < p_end_minute)
        ELSE -- 'daily'
          (p_start_minute IS NULL
            OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) >= p_start_minute)
          AND (p_end_minute IS NULL
            OR (EXTRACT(HOUR FROM p_local_now)::int * 60 + EXTRACT(MINUTE FROM p_local_now)::int) < p_end_minute)
      END;
    $$;
  `);

  // 5. product_available_now(product_id, location timezone) — AND-combines all schedules
  //    that apply to the product (its own + its category's). NO schedule => always TRUE.
  //    A blocking row (available=false) whose window matches forces FALSE; an allow row
  //    (available=true) requires AT LEAST ONE matching allow window when any allow row exists.
  pgm.sql(`
    CREATE OR REPLACE FUNCTION product_available_now(p_product_id uuid, p_category_id uuid, p_tz text)
    RETURNS boolean
    LANGUAGE plpgsql STABLE AS $$
    DECLARE
      v_local      timestamp := (now() AT TIME ZONE COALESCE(p_tz, 'UTC'));
      v_has_allow  boolean := false;
      v_allow_hit  boolean := false;
      r            record;
    BEGIN
      FOR r IN
        SELECT mode, start_minute, end_minute, days_of_week, starts_at, ends_at, available
        FROM menu_schedules
        WHERE product_id = p_product_id OR category_id = p_category_id
      LOOP
        IF menu_schedule_matches(r.mode, r.start_minute, r.end_minute, r.days_of_week, r.starts_at, r.ends_at, v_local) THEN
          IF r.available = false THEN
            RETURN false;            -- explicit block window currently active
          END IF;
          v_allow_hit := true;       -- an allow window is currently active
        END IF;
        IF r.available = true THEN
          v_has_allow := true;       -- this product is gated by allow windows
        END IF;
      END LOOP;
      -- No allow rows at all => unrestricted (purely additive). Allow rows exist =>
      -- available only while one of them is active.
      RETURN (NOT v_has_allow) OR v_allow_hit;
    END;
    $$;
  `);
}

export async function down(): Promise<void> {
  // Forward-only. Inert with no rows; nothing to reverse.
}
