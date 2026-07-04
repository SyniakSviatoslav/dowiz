# S3-CATALOG Port — Council Packet · BREAKER FINDINGS

> **Seat:** system-breaker (S3-catalog Triadic Council). **Charge:** prove the Rust/axum catalog port
> breaks. No fixes, no design (architect's job). Every finding is demonstrable — a concrete
> break-scenario or a policy-SQL fact — with `file:line` evidence against the live Node ground truth
> and the live migration policy catalog (`fix/audit-remediation@b28b1764`), matched to a mapped
> open-question / quirk-row.
>
> **Verdict up front: the packet breaks — not on its headline thesis (the `with_user` GUC-family
> finding is CORRECT and well-grounded), but on the *edges of the seam it declares load-bearing*.**
> The packet's own rule — "no tenant table on a context-free connection; correct independent of which
> pool role is live" — is violated by three S3-scope realities the packet either omits or mislabels as
> safe `PORT`: (1) the P-d membership *authorization* read runs context-free and returns ∅ post-flip;
> (2) `themes.ts` is a raw-pool, GUC-less path and `theme_versions` keys on a **third** GUC family the
> `with_user(app.user_id)` remedy does not satisfy; (3) the read path is RLS-free (`public_select
> USING(true)`), so the "belt AND suspenders" the packet leans on is belt-only on reads and the B3
> probe can false-green. **2 CRITICAL, 4 HIGH, 3 MED, 2 LOW.** Read C1 (owner lockout) and C2 (theme
> third-GUC) first.

Legend: **[BREAK]** the exact failure · **[EVID]** file:line · **[SCENARIO]** repro/number · **[MAPS]** OQ/quirk.

---

## CRITICAL

### C1 · B-SEC / B3 · The P-d membership *authorization* read is context-free — porting it "1:1" replays the anonymizer-N1/F1 pre-context break → total owner-write lockout post-flip

- **[BREAK]** Packet §3 clause 5 instructs: owner-write routes resolve location "from a **live**
  memberships row … **port 1:1**", naming `requireLocationAccess`, `getLocationId()`, and
  `getOwnerLocationId`. All three read `memberships` on the **raw operational pool with no GUC and no
  `withTenant`**:
  - `requireLocationAccess` runs `SELECT 1 FROM memberships WHERE location_id=$1 AND user_id=$2 AND
    role='owner' AND status='active'` on `request.server.db` (raw pool) — `plugins/auth.ts:148-151`.
  - `getLocationId` (menu-import) runs `db.query('SELECT location_id FROM memberships WHERE user_id=$1
    …')` on the raw `db` — `owner/menu-import.ts:16-19`.
  `memberships` is FORCE RLS with `tenant_isolation USING (location_id IN (SELECT
  app_member_location_ids()))` (`migrations/1780310071220_core-identity.ts:92-94`), and
  `app_member_location_ids()` = `SELECT location_id FROM memberships WHERE user_id = app_current_user()
  AND status='active'`, with `app_current_user()` = `NULLIF(current_setting('app.user_id', true),'')`
  (`:70-79`).
- **[SCENARIO]** Post-B3-flip (NOBYPASSRLS), this pre-context read has **no `app.user_id` seated** (it
  runs *before* any `with_user` txn — it is the gate that decides whether to open one). So
  `app_current_user()` = NULL → `app_member_location_ids()` = `WHERE user_id = NULL` = ∅ → the
  memberships policy `location_id IN (∅)` = FALSE for every row → the `SELECT 1` returns **0 rows** →
  `requireLocationAccess` returns **404 for every owner request**, on every catalog route. This is the
  exact mechanism the RLS-reliability council flagged as CRITICAL F1/F3 (wrong/absent GUC family →
  member-fn returns ∅ → 0 rows;
  `audit-fix-rls-reliability/breaker-findings.md:101-111,245-256`), now on the gate that authorizes
  *every* S3 write. The `with_user` seam does not help: it seats the GUC *inside* the write txn, but
  the authorization read fires first, on the raw pool, by construction.
- **[GOVERNANCE HOLE]** The packet's latent-class list (§3, classes 1/2/3) enumerates
  `set_config(false)` / no-BEGIN / cross-physical-conn, and asserts "S3 touches none of these files."
  It does **not** enumerate the *no-GUC-at-all context-free read* class, and it explicitly labels the
  membership resolvers "SAFE — port 1:1." The §8 DoD probe tests only that the **write txn** seats
  `app.user_id`; it does not exercise the pre-txn authorization read, so the probe passes while every
  owner is locked out. No quirk-register row, no threat-model scenario (S3-T1…T8 all concern the write
  txn, not the authz read).
- **[EVID]** `plugins/auth.ts:148-151`, `owner/menu-import.ts:16-19`;
  `migrations/1780310071220_core-identity.ts:70-79,92-94`; probe scope `proposal.md §8`.
- **[MAPS]** Q2 (🔴 B3 readiness), §3 clause 5. Port-blocking: the "correct independent of which pool
  role is live" claim (`threat-model.md §4`) is false for the authorization read as written.

### C2 · B-SEC / Q1 / Q7 · `themes.ts` is a raw-pool GUC-less path AND `theme_versions` keys on a THIRD GUC family (`request.jwt.claim.sub`) — the `with_user(app.user_id)` remedy is necessary-but-INSUFFICIENT; both the table and the GUC are omitted from the packet

- **[BREAK]** The packet scopes `themes.ts (3)` into S3 as a plain `PORT` with "no independent
  red-line" (§2) and asserts the S3 write path routes owner writes through the tenant-scoped
  `with_user` seam (§3 clause 1). **`themes.ts` does neither** — it reaches the raw pool directly:
  `const client = await db.connect()` at `themes.ts:23` (GET), `:62` (PUT, with a bare
  `client.query('BEGIN')` at `:64` and **no `set_config` anywhere**), and `:139` (logo). It touches
  two FORCE-RLS tables:
  1. `location_themes` — `tenant_isolation USING (location_id IN (SELECT app_member_location_ids()))`
     (`migrations/1780310075801_branding.ts:20-23`), the `app.user_id` root.
  2. `theme_versions` — `theme_versions_owner_write FOR ALL **TO authenticated** USING (location_id IN
     (SELECT location_id FROM memberships WHERE user_id = (current_setting('**request.jwt.claim.sub**',
     true))::uuid))` (`migrations/1780338982030_theme_versions.ts:33-42`).
- **[SCENARIO]** Two independent post-flip breaks, neither fixed by the packet's remedy:
  - Porting `themes.ts` **verbatim** (the default = CARRY) reproduces the context-free connection the
    packet's §3 clause 3 / threat-model S3-T2 claim is "structurally refused" ("raw pool unreachable
    from route code") — the Rust `Pools` fields being `pub(crate)` *forces* a rewrite, so verbatim
    port is impossible and the rewrite is an **un-flagged behavior change** with no quirk-register row.
  - Even the packet's remedy (`with_user(app.user_id)`) **does not make `theme_versions` writes
    correct**: its policy reads `request.jwt.claim.sub`, **not** `app.user_id` and **not**
    `app.current_tenant`, and is scoped `TO authenticated` (the app's runtime role is `dowiz_app`, not
    `authenticated`). Seating `app.user_id` leaves `request.jwt.claim.sub` NULL → the policy subquery
    is empty → `WITH CHECK` fails → the `INSERT INTO theme_versions` on every theme PUT (`themes.ts:96-101`)
    matches 0 rows / raises. The theme editor silently stops publishing CSS versions post-flip.
- **[GOVERNANCE HOLE]** Q7 (open-questions.md:76-85) enumerates the S3 tables to check for legacy
  GUCs — `products/categories/modifier_groups/modifiers/product_modifier_groups/product_translations/
  location_themes/menu_schedules/import_sessions` — and **omits `theme_versions` entirely**. The
  packet's GUC analysis is binary (`app.user_id` vs `app.current_tenant`); the third root
  (`request.jwt.claim.sub`, census §2 line 140) is never mentioned for any S3 write table. The
  inventory itself already flags it exists ("request.jwt.claim.sub … 3 refs") — the packet did not
  carry it forward.
- **[EVID]** `themes.ts:23,62-64,96-101,139`; `migrations/1780338982030_theme_versions.ts:33-42`;
  `migrations/1780310075801_branding.ts:20-23`; Q7 table list `open-questions.md:78-79`.
- **[MAPS]** Q1, Q7, Q-DUAL-GUC. Port-blocking for the theme surface: the `with_user` contract as
  specified is provably insufficient for it.

---

## HIGH

### H1 · B-SEC / B3 · `public_select FOR SELECT USING(true)` on products/categories/locations/menu_schedules means owner READS are RLS-free — the packet's "SELECT needs app.user_id" / "belt AND suspenders for reads" is FALSE, and a SELECT-visibility B3 probe false-greens

- **[BREAK]** §3 ("Post-flip: `SELECT/UPDATE/DELETE` need `app.user_id` seated to see the row") and
  `threat-model.md §2 TB-2` ("post-flip RLS is authoritative *iff* `app.user_id` is seated … Belt AND
  suspenders — neither alone") assert RLS is a live second boundary on reads. It is not, for the
  hottest catalog reads: `CREATE POLICY public_select ON categories/products/locations FOR SELECT
  USING (true)` — **no `TO` clause** (`migrations/1780338741329_public-menu-rls.ts:6-15`), and the same
  on `menu_schedules` (`1790000000062_menu-schedules.ts:63`). Permissive policies OR-combine, so for
  SELECT: `public_select(true)` OR `tenant_isolation(member)` = **TRUE for every tenant's rows**.
- **[SCENARIO]** Post-flip, an owner `GET /products` with a wrong/unset `app.user_id` still returns
  its rows — because the explicit `WHERE location_id=$1` (products.ts:72) filters them, and RLS adds
  nothing. So a B3-readiness probe built the natural way ("owner A must not SEE owner B's rows under
  NOBYPASSRLS") **passes even when the GUC is wrong/absent** — it cannot detect the exact GUC-family
  bug C1/C2 are about, because the WHERE clause already produces the correct visibility. GUC
  correctness is observable **only** via INSERT (`WITH CHECK`) and UPDATE/DELETE (`tenant_isolation`,
  which *is* `FOR ALL` with no `TO`). The packet's own §8 probe wording ("INSERT/UPDATE affects
  exactly the intended rows") happens to dodge this — but the framing in §3 / TB-2 that reads are
  RLS-scoped is wrong, and any council member or porter who writes a read-visibility isolation assert
  ships a green over the hole. (Exception: `modifier_groups`/`modifiers` have **no** `public_select`
  — reads there *do* need the GUC — so the surface is inconsistent, sharpening the trap.)
- **[EVID]** `migrations/1780338741329_public-menu-rls.ts:6-15`,
  `1790000000062_menu-schedules.ts:63`, `1780310072731_menu.ts:37-46` (tenant_isolation FOR ALL);
  contradicts `proposal.md §3` and `threat-model.md §2 TB-2`.
- **[MAPS]** Q2, S3-T1. The DoD must pin the probe vectors to INSERT/UPDATE/DELETE and drop the
  "reads are RLS-scoped" claim.

### H2 · B-SEC · Clause-4 "every owner write ALSO carries an explicit `WHERE location_id`" OVERCLAIMS — top-level INSERTs carry none, and they are the exact statements where RLS `WITH CHECK` is the sole post-flip boundary

- **[BREAK]** §3 clause 4 and the DoD ("extend `rls-adversarial` 'privileged pool queries have WHERE
  location_id' from `workers/` to `routes/owner/**`") treat the explicit `WHERE location_id=$N` as a
  universal belt that "holds independent of which pool role is live." A `VALUES` INSERT has no WHERE
  and cannot: `INSERT INTO products (location_id, …) VALUES ($1, …)` (products.ts:42-46), `INSERT INTO
  categories … VALUES ($1,…)` (categories.ts:40-44), `INSERT INTO modifier_groups … VALUES ($1,…)`
  (modifier-groups.ts:37-41), and the menu-alias `INSERT INTO products …` (products.ts:431-434).
- **[SCENARIO]** The DoD guardrail is therefore either **impossible** (it false-*fails* on every
  legitimate top-level INSERT, which has no WHERE to assert) or **vacuous** (it special-cases INSERTs,
  leaving the INSERT tenant boundary un-verified). And INSERT is precisely where the packet itself
  says the flip bites hardest ("INSERT needs `app.user_id` seated to satisfy `WITH CHECK`", §3). So
  the *only* pre-flip tenant boundary on a top-level product/category INSERT is the `requireLocationAccess`
  **middleware** (auth.ts:148) — which is itself the context-free read that breaks post-flip (C1).
  Net: the packet leans on a "belt" (WHERE) that is absent on the statements where the "suspenders"
  (RLS WITH CHECK) is the sole boundary, and the named guardrail cannot cover them. (Child-write
  INSERT…SELECT fold-ins *do* carry ownership — products.ts:324-335, menu-availability.ts:120-125,
  modifier-groups.ts:157-164 — so the claim holds for those, but not for the top-level rows.)
- **[EVID]** products.ts:42-46, categories.ts:40-44, modifier-groups.ts:37-41, products.ts:431-434;
  DoD `proposal.md §8`; clause-4 `proposal.md §3`.
- **[MAPS]** Q-EXPLICIT-PREDICATE, S3-T8. The register row asserts "verbatim WHERE location_id + gate";
  the gate does not apply to the INSERT half.

### H3 · B-OPS / B3 · The B3 probe cannot use the real flip subject — `dowiz_app` (BYPASSRLS, created out-of-band, NOT in migrations); a substitute NOBYPASSRLS role has different grants + `TO`-scoped policy applicability → the proof exercises a different object than production

- **[BREAK]** Q2(a)/§8 promise a "live NOBYPASSRLS probe … independent of when B3 actually flips" that
  proves S3 correct. The flip subject is the runtime role `dowiz_app`, which "is created **out-of-band
  in Supabase (not in migrations)**" and whose BYPASSRLS bit is "asserted from docs, not from a live
  `pg_roles` read" (inventory/12 §2 line 178, gap #4). The sqlx offline/CI shadow DB is built **from
  migrations** (inventory/12 §9 line 448). So `dowiz_app` **does not exist** in the probe's DB; the
  probe must connect as some *other* NOBYPASSRLS role — the migration-created
  `deliveryos_operational_user WITH LOGIN NOBYPASSRLS` (`migrations/1790000000015_operational-pool-role.ts:19`).
- **[SCENARIO]** That substitute is not behavior-equivalent to post-flip `dowiz_app`: (a) grants flow
  overwhelmingly `TO dowiz_app` (inventory/12 §2 line 178, "29 grant refs") — a different role may
  fail-closed for a *grant* reason, masking or faking a *policy* result; (b) `TO`-scoped policies
  resolve by role — `theme_versions_owner_write` is `TO authenticated` (C2), so a probe as
  `deliveryos_operational_user` matches a **different policy set** than post-flip `dowiz_app` will.
  A probe passing under the substitute role therefore does not establish that the *actual* flip is
  safe — the exact "the proof exercises a different object than production" false-green class the
  RLS-reliability council raised as F8 (`audit-fix-rls-reliability/breaker-findings.md:183-191`). Q2's
  "provably correct under NOBYPASSRLS **before** the flip" is not achievable as specified without
  reproducing the real role attributes, which the migration-derived DB cannot.
- **[EVID]** inventory/12 §2 line 178 + gap #4 (`docs/design/rebuild-plan/inventory/12-data-layer.md`),
  §9 line 448; `migrations/1790000000015_operational-pool-role.ts:19`.
- **[MAPS]** Q2 (🔴). The probe's role construction is unstated and load-bearing.

### H4 · B-CONTRACT / ADR-0004 · §3 clause 5 binds a non-existent S2 `OwnerAt<Loc>` extractor — S2 shipped `OwnerClaimsExt` (role-narrow only, NO membership re-read); P-d is not provided by any existing component

- **[BREAK]** §3 clause 5 states owner-write routes "bind the **S2 `OwnerAt<Loc>` extractor** (the
  live `status='active'` membership re-read, ADR-0004 P-d)." No such extractor exists. The S2 Rust
  surface shipped `OwnerClaimsExt`, whose entire body is `Claims::Owner(o) => Ok(OwnerClaimsExt(o))`
  (`rebuild/crates/api/src/auth/extractors.rs:97-115`) — pure role-narrowing from the JWT claim, **no
  DB membership re-read, no location binding.** The only DB-binding extractor is `CourierSession`
  (extractors.rs:158-210); there is no owner analog.
- **[SCENARIO]** ADR-0004 P-d ("a removed owner is blocked immediately, not at ≤24h token expiry") is
  enforced in Node by the per-request `requireLocationAccess` membership `SELECT` (auth.ts:148). If the
  port "binds `OwnerAt<Loc>`" as instructed, it binds nothing (the component is absent) → a removed
  owner keeps writing for up to the ≤24h access-token TTL — a silent regression of the exact residual
  risk `threat-model.md §5` claims is "unchanged by the port." Building the re-read fresh (the honest
  path) collides with C1: wherever it lands it reads `memberships` context-free and breaks post-flip.
- **[EVID]** `rebuild/crates/api/src/auth/extractors.rs:97-115` (OwnerClaimsExt, no re-read); Node P-d
  `plugins/auth.ts:146-154`; claim `proposal.md §3` clause 5; residual-risk `threat-model.md §5`.
- **[MAPS]** ADR-0004, §3 clause 5, S2 handoff. The packet assumes an S2 deliverable that S2 did not build.

---

## MED

### M1 · B-CONTRACT / Q6 · locations PATCH tri-state nullability — `.nullish()` + dynamic SET encode "present-null = CLEAR" vs "absent = keep"; a plain Rust `Option<i64>` collapses both → the clear is silently dropped

- **[BREAK]** Q6 frames the SET-clause port purely as injection-safety ("fixed column allowlist vs
  interpolation"). It misses the load-bearing tri-state: `min_order_value` / `free_delivery_threshold`
  are `z.number().int().min(0).nullish()` (locations.ts:24-25) and the dynamic SET emits a `k=$n` only
  for keys **present** in the body (locations.ts:53-57), so `{"min_order_value": null}` → `SET
  min_order_value = NULL` (**clear**), while an absent key → column untouched (**keep**).
- **[SCENARIO]** Node's `Object.entries(updates)` distinguishes present-null from absent natively. A
  Rust struct with `min_order_value: Option<i64>` cannot — `null` and absent both deserialize to
  `None`. Whichever the porter picks (skip `None` → the owner's "clear my free-delivery threshold" is
  silently ignored; or write `None` → every unspecified field is nulled) is a parity break on a
  money-adjacent field, invisible to a happy-path test that only sets values. Distinguishing needs
  `Option<Option<i64>>` / `double_option` — a decision Q6 never raises.
- **[EVID]** locations.ts:24-25 (`.nullish()`), :53-57 (dynamic SET only for present keys); Q6
  `open-questions.md:68-74`.
- **[MAPS]** Q6, Q-DYNAMIC-SET, Q-LOCALE-PARTIAL (same present-only body semantics).

### M2 · B-DATA / R3 · The money-newtype invariant is NOT applied on the fee READ path — S1 already types the three flat-fee columns as bare `Option<i64>`, not `domain::Lek`; S3 writing them as `Lek` splits the type across surfaces on the same columns

- **[BREAK]** Concern 2 asserts "Integer-money invariant (`domain::Lek`, i64 minor units) applies to
  the three flat-fee fields," and R3 (inventory/12 §10) says "any Rust type other than a checked
  integer newtype is a regression." But the **already-shipped S1** `LocationInfoRow` types
  `delivery_fee_flat`, `free_delivery_threshold`, `min_order_value` as plain `Option<i64>`
  (`rebuild/crates/api/src/repo.rs:62-64`) — no `Lek`, no negative-rejection. The column types are
  confirmed `integer` (`migrations/1780338982014_location_commerce.ts:10-12`), `tax_rate numeric`
  (`:8`) — so the packet's *type* claims are correct, but the *invariant* is aspirational.
- **[SCENARIO]** If S3 writes fees through `Lek` (checked) while S1 keeps reading them as bare `i64`,
  the same three columns carry two Rust types across surfaces — a coherence split R3 exists to
  prevent, and the write-side `Lek` guarantee (reject negatives) is never enforced on read-back or in
  the storefront-info payload. Separately: inventory/12 §1 lists `locations` Money-cols = `-`, so the
  "16 money tables, all integer minor units" invariant test **excludes `locations`** — the census SSOT
  the packet cites does not track these columns as money at all.
- **[EVID]** `rebuild/crates/api/src/repo.rs:62-64`; `migrations/1780338982014_location_commerce.ts:8-12`;
  inventory/12 §1 line 46 (`locations … Money -`), §10 R3.
- **[MAPS]** Q-TAX-RATE-FLOAT, R3, money-newtype council.

### M3 · B-CONSIST / Q5 · The menu-import DEFER premise ("different GUC families") is FALSE — but a real two-writer hazard survives: S3-CRUD products have `external_key = NULL`, and Node `replace`-commit DELETEs exactly those rows

- **[BREAK]** The council's target concern for the DEFER is "two writers, one table, different GUC
  families." Verified false: menu-import commit uses `withTenant(db, user.userId, …)`
  (menu-import.ts:253) → seats `app.user_id`, the **same** family as the S3 `with_user` CRUD writes.
  So the DEFER introduces no GUC split. The real coherence hazard is different: the S3 CRUD `INSERT
  INTO products` omits `external_key` (products.ts:42-46, :431-434) → CRUD-created products have
  `external_key = NULL`; Node menu-import `replace` mode runs `DELETE FROM products WHERE location_id=$1
  AND (external_key IS NULL OR external_key != ALL($2))` (menu-import.ts:461).
- **[SCENARIO]** During the DEFER window (Node still owns menu-import, Rust owns CRUD writes), an
  owner who edits the menu in the S3 editor (products with `external_key NULL`) then runs a Node
  menu-import in `replace` mode has **every S3-editor-created product silently deleted** by that
  DELETE — the historical-order guard (menu-import.ts:442-451) only protects products referenced by
  `order_items`, not freshly-created ones. This is pre-existing behavior, but the DEFER keeps both
  writers live across the strangler boundary and the packet's Q5 rationale never flags the
  cross-writer interaction (it argues only pipeline/threat-model separation).
- **[EVID]** menu-import.ts:253 (withTenant → app.user_id), :461 (replace DELETE), products.ts:42-46,431-434
  (no external_key); Q5 rationale `proposal.md §6` / `open-questions.md:57-66`.
- **[MAPS]** Q5, Q-REPLACE-MASSDELETE, S3-T6.

---

## LOW

### L1 · B-CONTRACT / Q8 · `.sub` vs `.userId` — the Rust `OwnerClaims` already carries both, constructed equal; the only exposed accessor is `Claims::sub()`, no `owner_id()`

- **[BREAK]** menu-confirm reads `request.user.sub` (menu-confirm.ts:17) where every other owner route
  reads `.userId`. In Rust, `OwnerClaims` has both `user_id` (`#[serde(rename="userId")]`) and `sub`,
  and `OwnerClaims::new` sets `sub: user_id` (`rebuild/crates/api/src/auth/claims.rs:54-79`) — equal by
  construction. `Claims::sub()` exists (claims.rs:167-172); no `owner_id()` accessor exists yet.
- **[SCENARIO]** No live divergence today (equal by construction), so the port is safe if it passes
  either field to `with_user`. The residual is a future-edit hazard Q8 already names — low, and the
  Rust struct's `sub=user_id` invariant is the mitigation.
- **[EVID]** menu-confirm.ts:17; `rebuild/crates/api/src/auth/claims.rs:54-79,167-172`.
- **[MAPS]** Q8. Confirms R3 recommendation (single accessor) is cheap and correct; not a break.

### L2 · B-OPS / §8 · The S1 menu cache is a faithful per-instance `(slug,locale)` TTL/SWR port — so an S3 write→storefront read is stale ≤30s on BOTH stacks; not a regression, but §8 omits any read-after-write / menu_version coherence gate for the S3-write→S1-read path

- **[BREAK]** Priority-target "cache invalidation drift for the S1 SSR/TTL cache" is **not a port
  regression**: the Rust `TtlSwrCache` (cache.rs:62-101, keyed by slug) mirrors Node's in-process
  `(slug,locale)` TTL/SWR cache exactly — both are time-based (30s TTL / 300s stale,
  `menu.ts:89-101`), **neither** keyed on `menu_version` (the FE polls that separately). So concurrent
  Node/Rust catalog writes create identical ≤30s staleness on both stacks; the `menu_version` bump
  trigger fires stack-agnostically at the DB.
- **[SCENARIO]** The residual is a DoD gap, not a break: §8 lists no read-after-write assertion for
  the S3-write → S1-storefront-read path, so a catalog E2E that writes via the editor and asserts the
  storefront reflects it **within 30s** can flake against a warm S1 cache — a flake that also exists
  on Node today, hence a carried constraint the parity oracle must already respect.
- **[EVID]** `rebuild/crates/api/src/cache.rs:62-101`; `apps/api/src/routes/public/menu.ts:89-101`.
- **[MAPS]** §8 DoD.

---

## Vectors probed and NOT broken (kept honest — no severity inflation)

- **Core Q1 thesis (`with_user` vs `with_tenant` GUC family): CORRECT.** Every catalog `tenant_isolation`
  policy keys on `app_member_location_ids()` ← `app.user_id` (`migrations/1780310072731_menu.ts:37-46`,
  `1780338982010_menu_modifiers.ts:45-46`, `1780310075801_branding.ts:22-23`,
  `1780310071220_core-identity.ts:86-94`), and `db.rs::with_tenant` seats `app.current_tenant` from a
  `TenantId` (`rebuild/crates/api/src/db.rs:62,155-159`). The wrong-family break is real; the packet
  got its headline right. `set_config(..., true)` (is_local) is correct and pinned (db.rs:184-199).
- **`tax_rate` float / flat fees integer: CORRECT.** `tax_rate numeric`, `delivery_fee_flat /
  min_order_value / free_delivery_threshold integer`, `delivery_radius_km numeric`
  (`migrations/1780338982014_location_commerce.ts:8-12`, `1780310071220_core-identity.ts:40`) — the
  packet's Q-TAX-RATE-FLOAT typing matches ground truth (the *invariant* gap is M2, not the type).
- **menu-confirm write-set = `{allergens_confirmed}`: CORRECT.** The UPDATE mutates exactly that
  column, never `source` (menu-confirm.ts:20); the proposed guardrail is appropriate. (The
  `trg_bump_menu_version` trigger on `products` also fires — expected cache invalidation, not a break.)
- **S3 has no order-total computation: CORRECT.** `location_info` reads the fee/tax columns into a DTO
  (`repo.rs:380-396`) but computes no total; no S3 read path applies `tax_rate`. "S3 only stores
  pricing inputs" holds.
- **Cross-stack "both directions" for catalog writes: largely SAFE.** Unlike S2's token seam, a
  catalog write is a stack-agnostic DB row — RLS, the explicit WHERE, and the `menu_version` triggers
  are identical for a Node or Rust writer; there is no minted artifact that must round-trip between
  stacks. The concurrent-write seam does not break at the DB level (the hazards are M3's external_key
  interaction and L2's carried staleness, not a double-write anomaly).
- **A NOBYPASSRLS probe IS constructible in principle** — `deliveryos_operational_user` exists as a
  NOBYPASSRLS role and `provision-rls.test.ts`/`claim-rls.test.ts` already run under one. The break is
  H3 (it is not the *same* role as the flip subject), not that no probe can be built.

---

## SHARPEST — the council must not ship without addressing these

**C1 (context-free P-d membership read → owner lockout post-flip)** and **C2 (`theme_versions` third
GUC family → the `with_user` remedy is insufficient, and the table is omitted from Q7).** Both are the
same failure the packet declares it exists to prevent — "no tenant table on a context-free connection;
correct independent of which pool role is live" — surfacing on S3-scope code the packet marks SAFE/PORT.
Neither has a quirk-register row, and the §8 NOBYPASSRLS probe (which tests only the write txn's
`app.user_id`) covers **neither** — while H1 shows a naïvely-written read-visibility probe false-greens
and H3 shows the probe can't even use the real flip role. No 🔴 S3 row should build until C1's
authorization read and C2's theme-surface GUC are dispositioned with register rows and a probe that
tests INSERT/UPDATE/DELETE (not SELECT visibility) under the *actual* runtime role attributes.
