# Phase 1 — concrete implementation under STOP-DESIGN-B review (HARDENED, round 1)

Hardened after Triadic Council round 1 (see `resolution.md`). Phase 1 shrank to its irreducible,
fully-inert core: **migration + config flag only**. `read_public_menu` column-read and the client
`MediaRenderer` registry moved to **Phase 2 start** (provable with real non-NULL data + pixel gate).

## A. Migration `packages/db/migrations/1790000000048_product-media-seam.ts` (forward-only, idempotent)

```sql
-- 1. enum (guarded — re-runnable on a retried release_command)
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'product_media_kind') THEN
    CREATE TYPE product_media_kind AS ENUM ('image', 'video', 'spin', 'model');
  END IF;
END $$;

-- 2. table (location_id DENORMALISED for RLS without a join)
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
CREATE INDEX IF NOT EXISTS product_media_product_idx  ON product_media (product_id, sort_order);
CREATE INDEX IF NOT EXISTS product_media_location_idx ON product_media (location_id);  -- M3: RLS + budget SUM

-- 3. RLS FROM CREATION (mirror products + tighten with WITH CHECK). Idempotent policies.
ALTER TABLE product_media ENABLE ROW LEVEL SECURITY;
ALTER TABLE product_media FORCE  ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation ON product_media;
CREATE POLICY tenant_isolation ON product_media
  USING ( location_id IN (SELECT app_member_location_ids()) )
  WITH CHECK ( location_id IN (SELECT app_member_location_ids()) );
DROP POLICY IF EXISTS public_select ON product_media;
CREATE POLICY public_select ON product_media FOR SELECT USING (true);  -- products parity; Data API closed below

-- 4. GRANTS (C1/H3/RC1): off the Supabase Data API; full DML to the hot-path tenant role.
--    The runtime writer is deliveryos_api_user (operational pool, via withTenant + set_config
--    app.user_id), NOT the postgres owner. tenant_isolation+WITH CHECK is the write boundary
--    (role is effectively NOBYPASSRLS). NO fragile mirror loop.
REVOKE ALL ON product_media FROM anon, authenticated, service_role;
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'deliveryos_api_user') THEN
    GRANT SELECT, INSERT, UPDATE, DELETE ON product_media TO deliveryos_api_user;
  END IF;
END $$;

-- 5. FK on products (additive; no row rewrite, no trigger fire → no mass bump)
ALTER TABLE products
  ADD COLUMN IF NOT EXISTS primary_media_id uuid REFERENCES product_media(id) ON DELETE SET NULL;

-- 6. tier gate column (CHECK, not bare text — counsel #3)
ALTER TABLE locations
  ADD COLUMN IF NOT EXISTS plan text NOT NULL DEFAULT 'free' CHECK (plan IN ('free','business'));
```

`down()` is a no-op (forward-only; inert while the flag is off and `primary_media_id` is NULL).

### Deliberately NOT in Phase 1 (deferred to Phase 2 start)
- **`read_public_menu` column-read** of `primary_media_id` — zero Phase-1 benefit (always NULL),
  avoids a 125-line plpgsql copy on the 122nd migration (C2). Lands in Phase 2 with real data.
- **Client `MediaRenderer` registry** — avoids mutating `MenuPage.tsx` (high churn / low health)
  for zero behavior (M2). Lands in Phase 2 with the renderers that exercise it + a pixel gate.

### Invariants held
- `product_media` is **NOT** wired to `bump_menu_version_trigger_fn` → secondary media writes cause
  no version bump. A primary swap (`UPDATE products SET primary_media_id`) fires the existing
  `trg_bump_menu_version_products` → bump. (Deleting the primary fires SET NULL → correct bump, M1.)
- `location_id` denormalised → RLS needs no join.

## B. Config — `packages/config/src/index.ts`
Add `MEDIA_RICH_ENABLED: z.enum(['true','false']).default('false')` (mirrors `OTP_ENABLED`).

## Phase-1 GO gate (proven on a throwaway local Postgres — never prod)
1. migration applies clean **and is re-runnable** (apply twice → no error) — H2;
2. cross-tenant `product_media` insert **rejected** by RLS WITH CHECK;
3. same-tenant insert (member context) **succeeds** — H3 positive proof;
4. a `product_media` insert/update does **NOT** bump `menu_versions`; an `UPDATE products SET
   primary_media_id` **does**;
5. `pnpm typecheck` + build green.

## Carried Phase-2 X-blockers (into the error-fix matrix)
- **H1/H4**: the lazy media endpoint reflects `plan` without a stale cache AND filters
  `available = true`.
- **L1**: upload-confirm sets real `bytes`; budget SUM enforced there.
- **R6 (Phase 5)**: pre-committed STOP-at-Phase-4 condition (counsel open question → human GO).
