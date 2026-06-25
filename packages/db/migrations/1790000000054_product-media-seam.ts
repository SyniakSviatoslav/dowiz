import type { MigrationBuilder } from 'node-pg-migrate';

/**
 * ADR-0002 · `product_media` seam — "schema now, rich runtime later" (Phase 1).
 *
 * Fully INERT in Phase 1: `MEDIA_RICH_ENABLED=false`, `primary_media_id` always NULL,
 * no reader touches these objects. Ships only the irreducible schema so the
 * irreversible DDL/RLS/menu_version cost is paid once, in the cheapest window, before
 * the Phase-2 money/menu tables migrate. Runtime (upload, renderers, lazy endpoint,
 * read_public_menu column-read, client MediaRenderer registry) lands in Phase 2 behind
 * GO/NO-GO gates. See docs/design/cinematic-product-media/{proposal,resolution}.md.
 *
 * Hardened by Triadic Council (resolution.md):
 *  - Idempotent / re-runnable on a retried release_command (outage history) — every
 *    object guarded (DO/pg_type, IF NOT EXISTS, DROP POLICY IF EXISTS).
 *  - Data API CLOSED: REVOKE ALL FROM anon/authenticated/service_role; full DML granted
 *    only to the operational pool role (deliveryos_api_user), which is the runtime writer
 *    via withTenant + set_config('app.user_id'). tenant_isolation + WITH CHECK is the
 *    real write boundary (role is effectively NOBYPASSRLS).
 *  - product_media is DELIBERATELY NOT wired to bump_menu_version → secondary-media writes
 *    cause no version bump; a primary swap is a `products` UPDATE → existing trigger bumps.
 *  - location_id denormalised → RLS predicate needs no join; indexed for the budget SUM.
 *
 * Forward-only: down() is a no-op (inert while the flag is off and primary_media_id NULL).
 */
export async function up(pgm: MigrationBuilder): Promise<void> {
  // 1. media-kind enum (guarded — re-runnable)
  pgm.sql(`
    DO $$ BEGIN
      IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'product_media_kind') THEN
        CREATE TYPE product_media_kind AS ENUM ('image', 'video', 'spin', 'model');
      END IF;
    END $$;
  `);

  // 2. table (location_id DENORMALISED for RLS without a join)
  pgm.sql(`
    CREATE TABLE IF NOT EXISTS product_media (
      id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
      location_id   uuid NOT NULL REFERENCES locations(id),
      product_id    uuid NOT NULL REFERENCES products(id) ON DELETE CASCADE,
      kind          product_media_kind NOT NULL,
      storage_key   text NOT NULL,
      mime_type     text NOT NULL,
      bytes         bigint NOT NULL DEFAULT 0 CHECK (bytes >= 0),
      width         int,
      height        int,
      duration_ms   int,
      poster_key    text,
      alt           text,
      sort_order    int NOT NULL DEFAULT 0,
      available     boolean NOT NULL DEFAULT true,
      meta          jsonb NOT NULL DEFAULT '{}'::jsonb,
      created_at    timestamptz NOT NULL DEFAULT now()
    );
  `);
  pgm.sql(`
    CREATE INDEX IF NOT EXISTS product_media_product_idx  ON product_media (product_id, sort_order);
    CREATE INDEX IF NOT EXISTS product_media_location_idx ON product_media (location_id);
  `);

  // 3. RLS FROM CREATION — mirror products (ENABLE+FORCE), tightened with WITH CHECK.
  pgm.sql(`
    ALTER TABLE product_media ENABLE ROW LEVEL SECURITY;
    ALTER TABLE product_media FORCE  ROW LEVEL SECURITY;

    DROP POLICY IF EXISTS tenant_isolation ON product_media;
    CREATE POLICY tenant_isolation ON product_media
      USING      ( location_id IN (SELECT app_member_location_ids()) )
      WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );

    DROP POLICY IF EXISTS public_select ON product_media;
    CREATE POLICY public_select ON product_media FOR SELECT USING (true);
  `);

  // 4. GRANTS — off the Supabase Data API; full DML to the hot-path tenant role only.
  pgm.sql(`
    REVOKE ALL ON product_media FROM anon, authenticated, service_role;
    DO $$ BEGIN
      IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
        GRANT SELECT, INSERT, UPDATE, DELETE ON product_media TO deliveryos_api_user;
      END IF;
    END $$;
  `);

  // 5. nullable FK on products (additive; no row rewrite -> no mass menu_version bump).
  //    image_key remains the Tier-0 fallback forever; deleting media never breaks a product.
  pgm.sql(`
    ALTER TABLE products
      ADD COLUMN IF NOT EXISTS primary_media_id uuid REFERENCES product_media(id) ON DELETE SET NULL;
  `);

  // 6. tier gate column (CHECK, not an enum — the tier set evolves without a migration).
  pgm.sql(`
    ALTER TABLE locations
      ADD COLUMN IF NOT EXISTS plan text NOT NULL DEFAULT 'free' CHECK (plan IN ('free', 'business'));
  `);
}

export async function down(): Promise<void> {
  // Forward-only. The seam is inert while MEDIA_RICH_ENABLED is off and primary_media_id
  // is NULL; there is nothing to reverse and a down-migration would only risk dropping a
  // table once Phase 2 has written real media into it.
}
