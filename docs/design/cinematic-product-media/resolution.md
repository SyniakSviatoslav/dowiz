# Resolution — Phase-1 STOP-DESIGN-B (round 1)

Per-finding disposition for `breaker-findings.md` + `counsel-opinion.md`. Grounded in the
verified role topology:
- **Writer = `postgres`** (DATABASE_URL_SESSION = `postgres.<proj>`) — table **owner**, subject
  to FORCE RLS, needs **no** grants. Owner-write RLS works via `set_config('app.user_id',…)` →
  `app_member_location_ids()` → `tenant_isolation`.
- **Read pool = `deliveryos_api_user`** — **BYPASSRLS** (`1780691681296:8`), NOLOGIN custom role.
  Reads bypass RLS; needs an explicit table-level `SELECT` grant.
- `deliveryos_operational_user` (migration 015's target) is **aspirational / not the live role**
  — the access-requests landmine. Default-privileges to it are harmless but not load-bearing.

## CRITICAL

### C1 — grant-mirror leaks product_media onto the Supabase Data API → **FIXED (design changed)**
The "mirror every `products` grant" loop was wrong: `products` is never REVOKEd from
`anon/authenticated/service_role`, so the loop would re-expose `product_media` (storage_key,
meta.frameKeys, draft media) to PostgREST. **Drop the mirror loop entirely.** Replace with:
- `REVOKE ALL ON product_media FROM anon, authenticated, service_role` (off the Data API —
  product_media is read only by the app API / SECURITY DEFINER, never PostgREST).
- `GRANT SELECT ON product_media TO deliveryos_api_user` (role-existence-guarded) — the read pool.
- Owner (`postgres`) writes as table owner — no grant.
This is more legible than the loop and matches the repo's explicit-grant style for this role
(047/050). Closes the cross-tenant **read** the breaker found, not just the write.

### C2 — read_public_menu "byte-identical" risk + 125-line transcription drift → **FIXED (deferred to Phase 2)**
In Phase 1 `primary_media_id` is **always NULL** (no backfill, no upload path yet), so modifying
`read_public_menu` produces **zero** behavioral difference — the conditional merge always merges
`'{}'`. Copying ~125 hand-transcribed plpgsql lines on the 122nd migration for zero Phase-1
benefit is pure downside (the exact 034/035-class drift risk).
**Decision: defer the `read_public_menu` column-read to Phase 2**, where a non-NULL
`primary_media_id` actually exists and the "byte-identical when NULL / key present when set"
property is provable against real data. Phase 1 leaves the function **untouched** → byte-identity
is trivially guaranteed. (Note: the breaker's "merge interleaves keys" and counsel's "wrong
migration cited" both dissolve — the function isn't touched in Phase 1. For the record, I verified
`…033` *is* the latest `read_public_menu(text,text)` definition; counsel's claim it lives only in
`…022` is mistaken — but moot now.)

## HIGH

### H1 — `UPDATE locations SET plan` doesn't bump menu_version → gate flip misses CF cache → **ACCEPT-RISK (Phase 1) + Phase-2 X-blocker**
In Phase 1 `plan` is **inert** (no runtime reads it; `MEDIA_RICH_ENABLED=false` global). The
Phase-2 lazy media endpoint is **client-fetched on modal open**, not in the SSR-cached HTML, so it
needs no `menu_version`/CF-HTML invalidation — it has its own short cache. **X-blocker for Phase 2:**
the lazy endpoint response must reflect a `plan` change without a stale long cache (key on plan or
`max-age≤60`). Owner: apps/api.

### H2 — bare CREATE TYPE/TABLE/ADD COLUMN aborts a retried release_command → FATAL boot → **FIXED**
Given the outage history + sibling migrations' `IF NOT EXISTS` discipline, make 048 fully
re-runnable: `CREATE TYPE` in a `DO`/`pg_type` guard; `CREATE TABLE IF NOT EXISTS`;
`CREATE INDEX IF NOT EXISTS`; `ADD COLUMN IF NOT EXISTS`; `DROP POLICY IF EXISTS` before each
`CREATE POLICY`; ENABLE/FORCE RLS are already idempotent.

### H3 — no proof the writer role can actually insert; mirror loop may be empty → **FIXED**
Resolved by the role topology: writer = `postgres` owner (always can, subject to RLS). The GO gate
gains a **positive** assertion: a same-tenant insert (with `app.user_id` set to a member)
**succeeds**, alongside the negative (cross-tenant insert **rejected**). Both proven on the
throwaway local DB.

### H4 — public_select USING(true) exposes draft/unavailable media + storage internals → **RESOLVED via C1 + Phase-2 X-blocker**
The Data API surface is closed by C1's REVOKE. Keep `public_select USING(true)` for exact
`products` parity (the read pool is BYPASSRLS, so policy-tightening wouldn't constrain it anyway —
the real draft filter must live in the endpoint query). **X-blocker for Phase 2:** the lazy media
endpoint query filters `available = true`. Owner: apps/api.

## MEDIUM

### M1 — deleting the primary media transitively bumps menu_version → **ACCEPT (correct behavior)**
Deleting the row a product points at via `primary_media_id` fires `ON DELETE SET NULL` → a
`products` UPDATE → bump. This is **correct**: the primary visibly changed, so the cache *should*
invalidate. Documented, not a defect. (No media exists in Phase 1; first relevant in Phase 2.)

### M2 — client registry refactor touches MenuPage.tsx (99.6%ile churn, health 4.1) for zero Phase-1 behavior → **FIXED (deferred to Phase 2)**
Introducing the `MediaRenderer` registry in Phase 1 mutates a fragile hot file with no
user-visible change and no natural pixel-identity gate. **Decision: defer the client registry to
Phase 2 start**, where it lands alongside the SpinViewer/VideoClip that actually exercise it and a
pixel-identity gate applies. Phase 1 touches **no** client code.

### M3 — index lacks location_id lead for RLS predicate / budget SUM → **FIXED**
Add `CREATE INDEX product_media_location_idx ON product_media (location_id)` (supports the
`location_id IN (…)` RLS predicate and the Phase-2 per-location `SUM(bytes)` budget). Keep the
existing `(product_id, sort_order)` for the per-product fetch.

## LOW
- **L1** (`bytes=0` evades cap): Phase-2 concern — the upload-confirm step sets real `bytes`; the
  budget SUM is computed there. Documented.
- **L2/L3** (function replacement outside flag reach / undetectable drift): dissolved by C2 defer.

## Counsel non-blocking
- **#2** (grant-mirror cites wrong precedent — 041 mirrors `orders`): dissolved by C1 (mirror loop
  dropped; explicit grants instead; doc corrected).
- **#3** (`locations.plan` bare text invites typo-drift): **FIXED** — add
  `CHECK (plan IN ('free','business'))` (a CHECK, not an enum, so the tier set evolves without an
  enum migration).
- **Open question → carried to STOP-DESIGN-B human GO:** record *now*, while the outage memory is
  fresh, an explicit pre-committed condition under which the program STOPS at Phase 4 and ships
  **zero** cinematic reveal — so "schema now, runtime later" stays honest ("runtime *maybe never*"
  must be a real option, not a runway). Phase 1 is safe precisely because it carries none of the
  Phase-5 unbounded-lifecycle runtime.

## Net effect on Phase 1 scope
Phase 1 shrinks to its irreducible, fully-inert core:
1. **Migration 048** (hardened): enum + `product_media` table + dual RLS (FORCE, WITH CHECK) +
   `REVOKE`/explicit grants + `primary_media_id` FK + `locations.plan` (CHECK) + two indexes —
   **idempotent/re-runnable**, forward-only.
2. **Config**: `MEDIA_RICH_ENABLED=false`.
`read_public_menu` column-read and the client `MediaRenderer` registry **move to Phase 2 start**
(provable with real non-NULL data + a pixel gate). No hot-path SQL, no fragile client file touched
in Phase 1.

---

# Round 2 — regression disposition

The breaker found **1 new CRITICAL** (the writer-role premise was wrong); everything else in this
resolution was independently verified to hold (Data-API REVOKE load-bearing incl. `service_role`,
H2 idempotency incl. the crash-after-CREATE edge, `locations.plan` CHECK lands, deferral leaves no
hidden reader — grep for `product_media`/`primary_media_id` in non-migration source is empty).

### RC1 — writer is `deliveryos_api_user`, not `postgres` → SELECT-only grant blocks Phase-2 writes → **FIXED**
Verified against source: `server.db` = the **operational pool** (`createOperationalPool`,
`deliveryos_api_user`, port 6543); owner menu CRUD runs through
`withTenant(server.db, userId, …)` (`packages/platform/src/auth/tenant.ts`) which sets
`set_config('app.user_id', userId, true)` and issues the writes. So the **runtime writer of tenant
tables is `deliveryos_api_user`**, not the `postgres` owner (`postgres` is session-pool/DDL only;
the operational pool FATAL-rejects connecting as `postgres` — `packages/db/src/index.ts:32`).

My round-1 C1 grant (`SELECT` only) was therefore wrong — the Phase-2 upload-confirm write would
hit `permission denied`. **Fix the grant to full DML:**
`GRANT SELECT, INSERT, UPDATE, DELETE ON product_media TO deliveryos_api_user` (role-guarded).
`REVOKE … FROM anon, authenticated, service_role` stays (Data API closed). Owner (`postgres`)
remains owner — needs nothing.

**RLS enforcement note (evidence-based):** the live use of `withTenant` (`set_config app.user_id`)
across owner CRUD is strong evidence `deliveryos_api_user` is effectively **NOBYPASSRLS** in
production (migration `1780691681296` *attempts* `ALTER ROLE … BYPASSRLS` but swallows errors via
`EXCEPTION WHEN OTHERS` — on Supabase `postgres` typically lacks the privilege, so it is a silent
no-op). Under NOBYPASSRLS, `tenant_isolation` + `WITH CHECK` is the **real** write boundary. *If*
the live role were BYPASSRLS, that would be a **pre-existing** condition affecting every tenant
table (products included), not introduced here — flagged separately, out of scope for 048.

**GO-gate hardened (role-accurate proof):** the throwaway-DB proof creates a NOBYPASSRLS role
`deliveryos_api_user`, `SET ROLE`s to it with `app.user_id` set, and proves: same-tenant insert
**succeeds** (grant + WITH CHECK pass); cross-tenant insert **rejected** (WITH CHECK); SELECT works;
`anon/authenticated/service_role` have **no** privileges (REVOKE); `product_media` write does **not**
bump `menu_versions` while `UPDATE products SET primary_media_id` **does**. (Running as superuser
would be false-green — fixed.)

No new hole from the RC1 fix: granting DML to the role that already carries tenant-table DML at
runtime is consistent and additive; the REVOKE keeps the Data-API surface closed.

## ZERO unresolved CRITICAL/HIGH. ZERO ETHICAL-STOP. Carried Phase-2 X-blockers: H1, H4, L1, and
## (RC1) "product_media writes MUST run via withTenant on the operational pool, location_id scoped
## server-side from membership — never a raw BYPASSRLS write."
