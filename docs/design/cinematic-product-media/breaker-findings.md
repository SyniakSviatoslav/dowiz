# Breaker Findings — Phase-1 `product_media` Seam (migration 1790000000048 + config flag + client registry)

**Breaker:** System Breaker DeliveryOS · **Target:** `phase1-implementation.md` (exact code under STOP-DESIGN-B)
**Axis:** competitive truth — where does Phase-1 break. Zero fixes; findings only.
**Grounding read:** migrations 033 (current `read_public_menu`), 021 (bump trigger), 015 (operational role + default privileges), 041 (grant-mirror pattern), `1780310072731_menu.ts` + `1780338741329_public-menu-rls.ts` (products RLS/grants), `1780315000000_customer-rls.ts`, `1780421100065_lockdown-nontenant-api-surface.ts`, `apps/web/.../MenuPage.tsx:441-452`.

---

## CRITICAL

### C1 — B-SEC · grant-mirror copies `anon`/`authenticated`/`service_role` DML onto `product_media` → cross-tenant write surface + Supabase Data API exposure
**Finding.** The DO-block (phase1 §A step 4) iterates `role_table_grants WHERE table_name='products'` and re-GRANTs every privilege to every grantee `<> 'PUBLIC'`. It does **not** filter `anon`, `authenticated`, `service_role`.
**Demonstration.** `products` is created by a bare `CREATE TABLE` (`1780310072731_menu.ts:18`) and is **never REVOKEd** from the Supabase roles — confirmed: `grep "REVOKE.*products"` across all migrations returns **empty**. On a Supabase Postgres, fresh `public` tables carry default grants to `anon`/`authenticated`/`service_role` (Supabase's `ALTER DEFAULT PRIVILEGES` baseline). So `role_table_grants` for `products` returns those roles holding SELECT/INSERT/UPDATE/DELETE → the DO-block faithfully mirrors all of them onto `product_media`. Result: `product_media` becomes writable/readable through the Supabase Data API (PostgREST) by `anon`. RLS `tenant_isolation` is `FORCE`d, but `public_select USING(true)` means `anon` can SELECT **every tenant's** media rows via PostgREST, and the `WITH CHECK` only blocks inserts whose `location_id` is not in `app_member_location_ids()` — under the `anon` role `app_member_location_ids()` is empty, so writes are blocked, but the **read** exposure (storage_key, meta.frameKeys[], poster_key, bytes per location) is a cross-tenant data-map leak through a perimeter the repo explicitly closed for every non-tenant table.
**Why the cited precedent does NOT hold.** The proposal claims it "mirrors the access-requests 041 pattern." It does the opposite: 041 mirrors from **`orders`** (different grant set) and then **explicitly executes `REVOKE ALL PRIVILEGES ... FROM anon, authenticated, service_role`** (`1790000000041_access-requests.ts:53`) *before* the mirror. Phase-1's DO-block has **no REVOKE** at all. The proven-pattern claim is false; the safety step was dropped.
**Broken invariant.** "Remove from the Supabase Data API perimeter" (the 1780421100065 lockdown doctrine) + tenant read isolation of storage internals. R7 ("cross-tenant write — Closed") is overstated; the open hole is cross-tenant *read* via the auto-granted Data-API role, not write.

### C2 — B-CONSIST · `read_public_menu` is NOT byte-identical for NULL `primary_media_id` — `jsonb_build_object || '{}'` mutates physical key order, busting Cloudflare HTML/menu-JSON cache + JSON-LD on the entire fleet
**Finding.** phase1 §A step 7 asserts the menu JSON is "byte-identical when NULL (merging '{}' is a no-op; jsonb is key-canonical)." Postgres `jsonb` is **not byte-canonical** — it stores keys ordered by `(length, bytewise)`. The current product object (033:103-112) is built by a single `jsonb_build_object(...)` with keys in source order: `id, name, description, price, available, image_key, attributes, modifier_groups`. Concatenating `|| '{}'::jsonb` (or, once a primary exists, `|| jsonb_build_object('primary_media_id', ...)`) forces `jsonb` to **re-serialize the whole object through its canonical key order**. The text output of `jsonb_agg` over a concatenated object differs from the text output of a flat `jsonb_build_object` even when the key *set* is identical, because the byte layout (and therefore `->>` text emission ordering and any downstream `::text`) is normalized.
**Demonstration / sharper truth.** Even granting that *both* sides canonicalize identically when key-set is equal (they do, since both are `jsonb`), the claim of "byte-identical" is still violated the moment a product has a non-NULL `primary_media_id`: the object gains a key (`primary_media_id`, length 16) that lands by canonical order **before** `modifier_groups` (length 15? — `modifier_groups` is 15 chars, `primary_media_id` is 16, `description`=11) — i.e. the new key is **interleaved mid-object**, not appended. Any consumer/test that pins the SSR/menu-JSON shape by string snapshot, hash, or ordered diff breaks. More importantly, the GO-gate "read_public_menu returns byte-identical JSON for a product with NULL primary_media_id" can pass on a *NULL* row yet still ship a function whose **whole body was rewritten via CREATE OR REPLACE** (033 is ~125 lines of plpgsql) — any transcription drift in that verbatim copy (a single `COALESCE` arg, the `m.group_id` vs `m.modifier_group_id` divergence that already bit 034/035) silently corrupts every locale's live storefront. The "byte-identical" gate tests one NULL row; the risk is in the 124 other lines copied by hand.
**Broken invariant.** Hot-path inviolability + SSR↔JSON-LD↔client consistency derived from `menu_version`. The CF HTML cache is keyed by version, but the *menu-JSON* `max-age=60` body changing shape (key interleave) on the first primary-set is a content-shape change the design assumed was append-only.

---

## HIGH

### H1 — B-DATA/B-OPS · `ALTER TABLE locations ADD COLUMN plan text NOT NULL DEFAULT 'free'` is forward-only but the gate it backs is dead in Phase 1, and the column is mis-placed for the actual read path
**Finding.** The gate column lands on `locations`. PG11+ makes `ADD COLUMN NOT NULL DEFAULT <const>` a metadata-only op (no rewrite) — that part is fine. But: (a) `locations` carries the **column-scoped** menu_version trigger `trg_bump_menu_version_locations AFTER UPDATE OF default_locale, supported_locales` (021:80-84) — DDL `ADD COLUMN` does not fire it, so no mass bump; **however** any future `UPDATE locations SET plan=...` to *upgrade a tenant to Business* will **not** bump menu_version, meaning the storefront's cached SSR/menu-JSON will not refresh to reveal newly-enabled rich media until an unrelated edit bumps the version. The gate flip is invisible to the cache layer.
**Broken invariant.** Operability: "scaling-gate/flag really closes." The plan flip has no cache-invalidation edge, so "flip to Business → media appears" is not guaranteed within `max-age=60`; it's guaranteed never, absent a separate bump.

### H2 — B-OPS/B-DATA · migration is **not idempotent on retry**; `release_command` re-run aborts on `CREATE TYPE` / `CREATE TABLE` / `ADD COLUMN` without `IF NOT EXISTS`
**Finding.** The DDL (phase1 §A) uses bare `CREATE TYPE product_media_kind`, `CREATE TABLE product_media`, `CREATE INDEX`, `ALTER TABLE products ADD COLUMN primary_media_id`, `ALTER TABLE locations ADD COLUMN plan`. node-pg-migrate wraps a migration in a transaction and records it only on success — but the repo's own outage history (MEMORY: "prod-outage-schema-drift", "release_command may retry") shows partial/retried releases. If the migration transaction is interrupted *after* `CREATE TYPE` commits in a non-transactional edge (or if a prior failed attempt left the enum), the retry hits `ERROR: type "product_media_kind" already exists` and the boot-guard FATAL-exits — repeating the exact "unguarded boot path takes prod down" failure class the proposal claims to be avoiding. The sibling patterns the design cites (015:18 `IF NOT EXISTS`, access-requests notify-queue, anti-fake-signals `CREATE TABLE IF NOT EXISTS`) all use `IF NOT EXISTS`; Phase-1's DDL drops it.
**Broken invariant.** Forward-only + idempotent-on-retry release. Failure-first doctrine ("bound every boot path") violated.

### H3 — B-CONSIST · grant-mirror produces an **empty loop** if products' grants live with a role-name the query doesn't surface → owner cannot write `product_media` in Phase 2 (silent, discovered only at upload time)
**Finding.** The DO-block's correctness depends on `role_table_grants` for `products` returning the *operational/session* role the app actually writes with. The session pool uses `deliveryos_api_user` (BYPASSRLS, per `1780421100050`/pgboss migrations) — but there is **no `GRANT ... ON products`** to that role anywhere (`grep GRANT.*products` = empty); the write path works today purely because `deliveryos_api_user` is the table **owner**/superuser-adjacent and/or relies on Supabase default privileges. `role_table_grants` does **not** list ownership-implied privileges. So the mirror may copy only the Supabase Data-API roles (the C1 leak) and **not** the role that actually needs write — or, if the owner-role isn't a grantee, the loop body that matters is empty. Phase 1 is inert so this is latent, but it is a **Phase-2 time-bomb**: the first owner upload `INSERT INTO product_media` fails with `permission denied` under FORCE RLS + missing grant, with no Phase-1 test catching it (Phase-1 GO only proves a *rejected* cross-tenant insert, never a *successful* same-tenant insert).
**Broken invariant.** "DML grants mirrored so owner-write works regardless of role name" — the mechanism is unproven for the actual writer role; the GO-gate has no positive-write assertion.

### H4 — B-SEC · `public_select USING(true)` on `product_media` leaks pre-launch/unavailable/`available=false` media and `meta.frameKeys[]`/`storage_key` for every tenant, with no `available` filter at the RLS layer
**Finding.** Mirroring `products.public_select USING(true)` is cited as proven, but `products` exposes only menu fields. `product_media` rows carry `storage_key`, `poster_key`, `meta` (frame manifests), `bytes`, and an `available` flag. `USING(true)` exposes **all** of them — including `available=false` (draft/hidden) media and media for products that are `is_available=false` — to any reader, including the C1-granted `anon` via PostgREST. The design relies on the *app endpoint* to filter by availability/tier, but RLS itself grants blanket read. A direct PostgREST query (`/rest/v1/product_media?select=storage_key,meta`) bypasses the app filter entirely.
**Broken invariant.** B-SEC: public read must not expose unpublished/internal storage internals; tier-gating must not be app-only when the row is Data-API-reachable.

---

## MEDIUM

### M1 — B-FAIL/B-DATA · FK race: deleting a media row that is currently `primary_media_id` — `ON DELETE SET NULL` fires a `products` UPDATE → **menu_version bump as a side-effect of a secondary delete**
**Finding.** The design's core lever is "secondary edit → no bump, primary swap → bump." But `primary_media_id ... ON DELETE SET NULL` means: when an owner deletes the media row that *happens to be the current primary*, Postgres performs `UPDATE products SET primary_media_id = NULL` to satisfy the FK → this fires `trg_bump_menu_version_products` (021:55). So a *delete in `product_media`* (which the design classifies as "secondary, no bump") transitively bumps the version. Conversely, a `DELETE FROM product_media` of a non-primary row touches only `product_media` → no bump (correct). The bump behavior is therefore **non-uniform and surprising**: identical user action (delete a media tile) bumps or not depending on whether it was primary. Phase-2's GO-gate ("set-primary bumps, reorder does NOT") never tests the *delete-primary* path.
**Broken invariant.** Deterministic menu_version semantics. Not catastrophic (the bump is arguably correct here — primary changed) but it contradicts the design's stated taxonomy and is untested.

### M2 — B-ANTIPATTERN · the registry refactor (phase1 §C) re-touches `MenuPage.tsx` — the repo's #2 churn hotspot (26 commits/90d, health 4.1/10) — for **zero Phase-1 behavior**, risking a real regression on a proven-fragile file
**Finding.** Phase 1 ships *no* rich rendering; `primary_media_id` is NULL everywhere; the registry "always falls through to `image_key`." Yet §C rewrites the image-render path (`getImageUrl`, `MenuPage.tsx:441-452`) into a `kind`-keyed registry. This is a non-functional refactor of the highest-risk client file (`get_risk` territory: MenuPage is a 99.6%ile churn hotspot) for a seam whose only Phase-1 job is to do exactly what the current code does. The byte-identical claim is asserted, not enforced — there's no rendered-DOM snapshot gate, only "public-menu E2E green." A subtle change in the data:/http/relative-key branching (444-451) ships dark and surfaces on the storefront.
**Broken invariant.** "Runtime minimal in Phase 1" / YAGNI. The schema seam is the cheap-window justification; the *client* refactor has no schema-window economics and adds churn to a fragile file with no positive proof of pixel-identity.

### M3 — B-SCALE/B-DATA · `product_media_product_idx (product_id, sort_order)` has **no `location_id` lead** — the per-location budget sweep and the RLS predicate both scan without an index
**Finding.** §9 operability promises a per-`location_id` `SUM(bytes)` budget check and an 80%-cap alert; §8 enforces budget pre-presign by summing the whole location. Both queries filter/group by `location_id`, but the only index is `(product_id, sort_order)`. The RLS `USING (location_id IN (...))` predicate also benefits from a `location_id` index. At the design's own mature scale (50 media rows/location × N locations), the budget sum is a seq-scan-per-presign. Not fatal at small N, but the design explicitly sells "indexed (location_id, ts)" as a B-DATA virtue and then omits the location index it needs.
**Broken invariant.** B-DATA: index supports the actual access path. Back-of-envelope: pre-presign budget check is O(total media rows) not O(location's rows) until a `location_id` index exists.

---

## LOW

### L1 — B-DATA · `bytes bigint DEFAULT 0` with budget enforced app-side means a row can claim `bytes=0` and evade the per-location cap
**Finding.** §5 deliberately omits a row CHECK ceiling ("budget is per-location aggregate"). But `bytes` defaults to 0 and is client/confirm-supplied; if the confirm step writes the row before/without authoritative R2 `HEAD` size, an owner (or a bug) inserts `bytes=0` rows that sum to 0 → cap never trips. The design says the server re-validates magic-bytes on confirm but does not state it re-reads object size authoritatively. Low because it's a Phase-2 concern and blast radius is the owner's own capped prefix.

### L2 — B-OPS · `down()` no-op is safe for the schema but the FK + enum are now permanent — a true rollback of a *bad* Phase-1 ship (e.g. C2 function corruption) requires a forward-fix migration, not a revert
**Finding.** "Forward-only, inert when flag off" is fine for the table. But the risk vector in Phase 1 is the `CREATE OR REPLACE FUNCTION read_public_menu` (C2): if that copy is wrong, the flag (`MEDIA_RICH_ENABLED`) does **not** gate it — the function is replaced unconditionally, flag-independent. So the headline "instant rollback = flip the flag" is false for the one Phase-1 change that can actually break the hot path. Rollback of a bad function body needs another migration round-trip on the live DB.
**Broken invariant.** "Rollback per phase = flip flag, no deploy." The function replacement is outside the flag's reach.

### L3 — B-CONSIST · GO-gate "byte-identical for NULL primary_media_id" is satisfiable while C2's whole-function transcription is wrong
(See C2.) The gate tests one NULL-row output; it cannot detect drift in the other ~124 hand-copied plpgsql lines (the 033 body), nor the non-NULL key-interleave. The verification is necessary-but-grossly-insufficient for the change it gates.

---

## Regression note for RE-ATTACK
The three load-bearing falsifiable claims to retest after any revision: (1) does `role_table_grants WHERE table_name='products'` on the live staging DB include `anon`/`authenticated`/`service_role` (C1) — run it, don't assume; (2) does a same-tenant `INSERT INTO product_media` succeed under the deployed writer role after the mirror (H3) — positive-write proof, currently absent from every GO-gate; (3) is the `read_public_menu` body a verified verbatim copy of 033 (C2/L3) — diff the deployed function source, not a single-row output.

---

## Round 2 — regression (re-attack on resolution.md + hardened phase1-implementation.md)

**Verdict:** the resolution closed C1's Data-API leak, made the migration idempotent (H2),
deferred C2/M2 cleanly, and added the M3 index + counsel-#3 CHECK — those hold. **BUT the
role-topology premise the whole disposition rests on is factually wrong, and it re-opens H3 as a
hard Phase-2 failure under the new "explicit grants" design.** One CRITICAL, one HIGH below;
everything else verified clean.

### Grounding (live `.env` + migrations, this round)
- `***REDACTED***` = **`deliveryos_api_user`** (`.env:4`, transaction pooler :6543). This
  is `fastify.db` / `server.db` — **the pool every route handler uses for both reads AND writes**
  (`auth.ts:126` does `UPDATE … via fastify.db`; `public/menu.ts:16` does
  `server.db.query('SELECT read_public_menu…')`).
- `***REDACTED***` and `***REDACTED***` = **`postgres`** (`.env:5-6`, :5432). `postgres`
  is used only for LISTEN/NOTIFY (MessageBus), pg-boss, migrations, and backup `pg_dump`
  (`workers/backup/index.ts:95` → `***REDACTED***`). It is **NOT** in any request path.
- `db/src/index.ts:28-34` guardrail **FATAL-rejects** the operational pool if it ever connects as
  `postgres` — so the runtime writer is *contractually never* the table owner.
- `deliveryos_api_user` is `BYPASSRLS` (`1780691681296:8`) but is **never** an owner and has **no**
  `GRANT SELECT/INSERT/UPDATE/DELETE ON public.*` in any migration (grep confirms only the
  *aspirational* `deliveryos_operational_user` gets `GRANT SELECT ON ALL TABLES`, 015:33). Its live
  read/write on `products`/`orders`/`locations` is carried entirely by **Supabase baseline default
  privileges**, not by repo migrations. (BYPASSRLS bypasses *policies*, not table-level ACLs.)

### [CRITICAL] B-SEC/B-DATA · resolution mislabels the runtime writer as `postgres`; the "explicit GRANT SELECT to deliveryos_api_user" replaces a working write path with a SELECT-only one → Phase-2 owner upload `INSERT INTO product_media` gets `permission denied`
**Finding.** resolution.md:5-7 and phase1 §A step-4 comment assert *"Writer = `postgres` (owner) …
needs no grant"* and grant the runtime pool **SELECT only**. The runtime writer is
`deliveryos_api_user` (the operational pool), proven above — `postgres` never touches a request. The
old grant-mirror loop (round-1 C1), for all its leak, at least mirrored `products`' full DML
(including whatever INSERT/UPDATE the operational role inherits from Supabase defaults). The new
design **drops that and grants exactly `SELECT`** — so the table the operational pool can read it
**cannot write**.
**Demonstration / scenario.** Phase 2 ships the owner upload-confirm endpoint. It runs in a Fastify
route → `fastify.db` → `deliveryos_api_user`. `INSERT INTO product_media (...)` → Postgres checks
table ACL → `deliveryos_api_user` holds only `SELECT` → **`ERROR: permission denied for table
product_media`**. RLS `WITH CHECK` is never even reached. This is round-1 **H3 reincarnated**, and
the resolution explicitly marked H3 "FIXED — writer = postgres owner, always can." It can't: the
guardrail at `db/src/index.ts:34` *forbids* the writer from being `postgres`. The GO-gate's new
"same-tenant insert succeeds" assertion (phase1 §GO-gate step 3) will **pass on the throwaway local
DB only because the test connects as the migration/superuser role**, not as `deliveryos_api_user`
— so the positive-write proof is run under the wrong role and gives false green.
**Broken invariant.** "Writer-role can actually write the new table" (the H3 invariant, re-opened).
Also: the GO-gate must exercise the *operational* role, not the owner, or it proves nothing about
production.
**Severity rationale.** CRITICAL not by Phase-1 blast radius (Phase 1 is inert — nothing writes
product_media yet) but because the resolution **certified H3 closed on a false premise** and baked
the wrong role into both the grant and the GO-gate; the failure is guaranteed at the first Phase-2
write and the very test meant to catch it is mis-roled.

### [LOW→note] B-SEC · the C1 `GRANT SELECT … TO deliveryos_api_user` is *necessary* (BYPASSRLS ≠ table ACL) and *insufficient alone* — same root as above
For the record: even the **read** side is not automatic. `deliveryos_api_user` reads other tables via
Supabase baseline default privileges, but `product_media` is a *new* table and
`1780421100065:37-44` revokes default `anon/authenticated` table privileges (not the api_user, so
the api_user default-grant, if Supabase applies one, may or may not land). The explicit
`GRANT SELECT TO deliveryos_api_user` is therefore correctly load-bearing for reads — confirming the
read path was right to make it explicit, which makes the **omission of the write grant** (above) the
asymmetric defect, not an over-grant.

### Verified-clean (fixes hold)
- **C1 Data-API leak — CLOSED.** `ALTER DEFAULT PRIVILEGES` (1780421100065:37-44) revokes new-table
  grants for `anon`/`authenticated` but **NOT `service_role`** → Supabase still auto-grants
  `service_role` on new `public` tables. The resolution's explicit `REVOKE ALL … FROM …
  service_role` is thus **load-bearing and correct** — it removes the one PostgREST role that would
  otherwise still see `product_media`. The round-1 cross-tenant *read* leak is genuinely closed.
- **REVOKE-from-service_role does NOT break backup/migration.** Backup `pg_dump` and migrations run
  as `postgres` (`***REDACTED***`, the owner), never `service_role`. No legitimate path
  uses `service_role` (ADR-006: custom Fastify pooler, not PostgREST). REVOKE is inert to ops.
- **H2 idempotency — HOLDS, including the post-CREATE-TABLE/pre-grant crash edge.** A retry after a
  crash between `CREATE TABLE` and the grant block re-enters the *same* migration transaction from
  the top: `CREATE TABLE IF NOT EXISTS` skips, `CREATE INDEX IF NOT EXISTS` skips, `DROP POLICY IF
  EXISTS`+`CREATE POLICY` re-applies clean, the `pg_type` DO-guard skips the enum, `REVOKE`/`GRANT`
  are idempotent, `ADD COLUMN IF NOT EXISTS` skips. node-pg-migrate runs each `up()` in one
  transaction and records success only on commit, so a partial state cannot be half-recorded — the
  whole `up()` re-runs and every statement is individually re-runnable. No partial-state trap.
- **`ADD COLUMN IF NOT EXISTS … CHECK` — no real hole.** The "IF NOT EXISTS skips → CHECK never
  added" edge is real *in general*, but `locations.plan` does not pre-exist in any migration or code
  (grep clean). First apply = genuine ADD → CHECK lands. The only path where IF-NOT-EXISTS fires is
  a *retry*, where the column already carries the CHECK from the first (rolled-back-then-retried, or
  committed) attempt. No window where the column exists without its CHECK.
- **Phase-1 inertness (defer of read_public_menu/registry) — CONFIRMED, no hidden tear.** Grep for
  `product_media`/`primary_media_id` across all non-migration `apps/`+`packages/` source = **empty**.
  Nothing reads the new column or table. `read_public_menu` (033) is untouched → byte-identity
  trivially holds. Phase 1 is genuinely inert; the defer creates no dangling reader.
- **M3 index + counsel-#3 CHECK — present and correct** (`product_media_location_idx` on
  `location_id`; `CHECK (plan IN ('free','business'))`).

### Net
1 new **CRITICAL** (writer-role mislabel → SELECT-only grant blocks the Phase-2 write the resolution
certified as fixed, and the GO-gate is mis-roled so it can't catch it). Everything else the
resolution claimed — C1 leak closure, service_role REVOKE safety, H2 idempotency incl. the crash
edge, the CHECK semantics, and the deferral's inertness — **verified and holds.**
