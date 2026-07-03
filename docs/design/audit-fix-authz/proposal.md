# Design Proposal — Audit-fix: cross-tenant authz + the missing data-access seam

Status: **DESIGN — REVISED post-RE-ATTACK (rev 3)** (no production code; artifacts + ADR draft only).
Date: 2026-07-03 (rev 3, council RESOLVE round 2). See `resolution-r2.md` for the R2-1..R2-6
dispositions (rev-2 dispositions in `resolution.md`, B1–B8 + STOP-1). **The 14-site frozen fix list in
`resolution.md §2` is untouched — a parallel implementation lane is building it now; every rev-3 delta
below is purely ADDITIVE.** Rev-3 headline deltas (all additive):
- **#15 — the un-enumerated 15th live IDOR** (`menu-availability.ts:115` INSERT `menu_schedules` trusts
  body `product_id`/`category_id`, amplified by `read_public_menu`/`product_available_now` scanning
  `menu_schedules` with **no `location_id` predicate**): a code-only writer fold-in (§3.7) **plus** an
  operator-gated forward-only read-path migration (§3.8). Corrects the "complete enumerated surface" premise.
- **A MECHANICAL FK-sweep** replaces the falsified manual-convergence completeness claim (§3.9): the child-FK
  class is enumerated from the schema FK topology × write-site trust; the sweep's output (not another manual
  pass) now carries the completeness claim.
- **Lint honestly re-costed** (§4B, per breaker R2-2): the "~15-25 allowlist, error day-1" claim is
  falsified (~87 forced-UNKNOWN measured) → split into Tier-A (error now: SELECT/UPDATE/DELETE predicate
  + dynamic-`${}`) and Tier-B (INSERT…VALUES: warn→ratchet→error); the FK-injection class is explicitly
  **out of the lint's model** and the FK-sweep is its named companion gate. Three matcher evasions closed (R2-3).
- **Resolver core widened** to carry `activeLocationId` (§7, R2-4) so the ADR-0004 P-d verify branch (5/6
  clones) survives the collapse.
- **LC5 provenance** extended to the worker's SECOND audit-insert (§3.3, R2-5 — covered-by-implementation).

Rev-2 deltas (retained): Tier-1 fixes ALL 14 enumerated live IDORs; `no-unscoped-tenant-query` redesigned;
`require-auth-hook` matcher rewritten; resolver-collapse → core+adapters; F3 per-location deactivation →
track M-F3; LC5 sink fallback DELETED + actor-vs-subject provenance + attempt-logging; LC5-first sequencing.
Author: System Architect (Triadic Council STEP 1 — FRAME + PROPOSE).
Inputs (verified first-hand against source, not just re-read):
`docs/design-review/audit-security-2026-07-03.md` (F1/F2/F3/F4/F5),
`docs/design-review/audit-architecture-2026-07-03.md` (F2 — no repository layer, THE ROOT),
`docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md` (LC2/LC5, root R-A).
Relates to (does NOT supersede): `ADR-security-hardening-2026-07` (front-loaded the GET `/orders/:id`
read #1, spa-proxy #6, invite predicate #7 — this batch is the **PATCH sibling that was left behind**
plus the class-killer seam), `ADR-pg-privilege-hardening` (B3 / NOBYPASSRLS flip),
ADR-0004 (owner-token revocation), ADR-0013 (courier realtime authz).
Red-line: **auth / RLS / PII** → every guardrail + migration edit is operator-gated (protect-paths);
Council + Breaker + Counsel gate before any code. Conductor runs breaker+counsel next.

---

## 1. Problem + non-goals

Four live cross-tenant findings and one architectural root. All share a single cause: **there is no
enforced data-access seam — the tenant predicate is written by hand, per query, and is therefore
sometimes forgotten.** Verified today:

- `withTenant` is imported by **22 of 65** route files; the other 43 hand-roll `db/pool/client.query`
  with no scoping helper (`grep` confirmed, §5).
- The ADR-0004 live-membership resolver is cloned **≥3×** with **divergent failure semantics**:
  `lib/get-owner-location.ts:17/25` returns `null` (→ clean 401); `owner/promotions.ts:25` **throws**
  `new Error(...)` (→ generic 500). Same authz condition, two HTTP outcomes — a fix to one does not
  propagate.
- The active tenant boundary today is the explicit SQL predicate + preHandler, because the operational
  pool connects as `dowiz_app` which holds **BYPASSRLS** (RLS inert on the hot path). So a query missing
  its predicate is a **live** cross-tenant read/write, not a latent one.

The **proof that this is a class, not four points**: `owner/dashboard.ts:626 transitionOrder` (owner
order-transition path A) reads `SELECT id,status FROM orders WHERE id=$1 AND location_id=$2` — correctly
scoped. `orders.ts:862 PATCH /orders/:id/status` (owner order-transition path B — the *same* logic,
duplicated per arch-F14) reads `SELECT id,status,location_id,type FROM orders WHERE id=$1` — **no
predicate**. Two copies of one operation; the predicate was added to one and forgotten on the other.
That divergence is the finding (LC2) *and* the argument for the seam (R-A).

**Non-goals.** (a) Does NOT flip the pool to NOBYPASSRLS — that is B3, operator-gated; this batch is
correct under **both** postures. (b) Does NOT fix the Section-E RLS-policy gaps (couriers/courier_sessions
NO RLS, unscoped anonymous policies) — those are handed to the B3/flip council with a sequencing contract.
(c) Does NOT rework courier/customer GUC seating. (d) Does NOT decompose spa-proxy or the contracts layer
(arch F1/F3) — separate tracks. This batch = the four authz bugs + the one seam that stops them recurring.

---

## 2. Verified findings (file:line, read against live source)

| ID | Sev | Where (verified) | Root defect |
|----|-----|------------------|-------------|
| **LC2 / F1** | CRIT | `routes/orders.ts:840` guard `[verifyAuth, requireRole(['owner'])]`; read `:862-865` `SELECT id,status,location_id,type FROM orders WHERE id=$1` — **no membership JOIN**; `locationId` taken from the order's own row `:871`; `withTenant :860` inert under BYPASSRLS | mutation not bound to caller's tenant |
| **F3** | HIGH | `routes/owner/couriers.ts:14-15` hooks = `verifyAuth` + `requireLocationAccess` only; `requireRole` **not imported** (`:6`). `requireLocationAccess` (`plugins/auth.ts:127-140`) admits a **customer** whose JWT `locationId==L` and a **courier** whose `activeLocationId==L` | missing role gate → customer reads decrypted roster (`:40-46` `decryptPII`), courier mutates co-workers (`:99-116`) |
| **F4** | HIGH | `routes/owner/courier-invites.ts:20-21` `verifyAuth`+`requireLocationAccess`, no `requireRole`; `role` taken verbatim from body `:30`, no allow-list | non-owner mints/revokes courier invites → account injection; chains with F7 (`courier/auth.ts:89-94` `ON CONFLICT … DO UPDATE password_hash`) into full ATO |
| **LC5 / F2** | HIGH (CRIT-impact) | Entry `routes/owner/gdpr.ts:48` `let resolvedCustomerId = customerId \|\| null` — body `customerId` verbatim; only the `phone` branch `:50-53` is location-scoped. Sink `lib/anonymizer/index.ts:118-141` `UPDATE customers SET phone=…,name=NULL… WHERE id=$1` — **no `location_id`** (`:195-222` `anonymizeOrder` same shape) | client-supplied child FK never proven same-tenant → irreversible cross-tenant PII erasure |
| **F5** | MED | Entry `routes/owner/signals.ts:118` passes client `customer_id` into `computeSignals`; sink `lib/signals/compute.ts:85-88` `SELECT no_show_count… FROM customers WHERE id=$1` — **no `location_id`** | cross-tenant customer-reputation read |
| **#15 / R2-1** | CRIT | Writer `routes/owner/menu-availability.ts:113-123` POST `…/menu-schedules` `INSERT INTO menu_schedules (…) VALUES [locationId, b.product_id, b.category_id, …]` — body FKs trusted verbatim (plain FKs, mig `062:28-29`); read amplifier `read_public_menu` (mig `064:139-160`) + `product_available_now` (mig `062:120-123`) scan `menu_schedules` with **no `s.location_id` predicate** under `public_select USING(true)` | owner-A hides/rewrites ANY tenant's product/category on the victim's **live public storefront** — cross-tenant integrity + availability DoS. Falsifies rev-2's "complete enumerated surface" |
| **ROOT / arch F2** | — | 43/65 route files no `withTenant`; resolver cloned with `null`(401) vs `throw`(500) divergence; `owner/dashboard.ts:626` scoped vs `orders.ts:862` unscoped = the same op written two ways | no enforced seam → the four bugs above + F10/F11 latents |

Guardrail gap that let F3/F4 ship: the existing `local/require-auth-hook` rule
(`tools/eslint-plugin-local/src/index.js:228-272`) sets `hasAuthHook=true` on `verifyAuth` **OR**
`requireRole` (`:245-248`) — since couriers.ts has `verifyAuth`, the rule is satisfied and never
demands `requireRole`. The rule that exists to catch exactly this is `verifyAuth || requireRole`, not
`verifyAuth && requireRole`.

---

## 3. The fixes (design, not code)

### 3.1 LC2 — PATCH `/orders/:id/status` (mirror the already-hardened GET sibling)
Fold authorization INTO the read as a JOIN — the JOIN **is** the tenant boundary — identical to the
GET `/orders/:id` owner branch shipped in the prior batch (`orders.ts:747-756`):
```
SELECT o.id, o.status, o.location_id, o.type
FROM orders o
JOIN memberships m ON m.location_id = o.location_id
WHERE o.id = $1 AND m.user_id = $2 AND m.role = 'owner' AND m.status = 'active'
```
0 rows → 404 **before** `assertOwnerTargetAllowed` (no existence leak; matches GET). `locationId` then
comes from the JOIN-verified row, not from an unauthorized read. One `withTenant` checkout (JOIN
authorizes inside the client already held — no second round-trip). Pool-agnostic (correct under both
BYPASSRLS and NOBYPASSRLS). Note: `owner/dashboard.ts:626` proves the codebase already knows this
shape — LC2 is bringing the second copy up to the first.

### 3.2 F3 / F4 — add the role gate + close the seam that let it slip
Two-line hook add on each file, between the existing hooks:
`fastify.addHook('preValidation', fastify.requireRole(['owner']));` (mirrors `gdpr.ts:29`). F4
additionally allow-lists `role` to `['courier']` (reject anything else 400) — the invite must not be
able to mint an owner.

**Rescoped per breaker B5 (was falsely claimed code-only):** the per-location courier deactivation
("deactivating a courier shared across tenants A+B must not disable them for B") **requires a
migration** — `couriers.status` is a global column on the identity table
(`packages/db/migrations/1780421029538_couriers.ts:12`) and `courier_locations` has no status column.
It is therefore **extracted to its own track M-F3** (see resolution.md §B5): migration adds
`courier_locations.status ('active'|'suspended') DEFAULT 'active'`, the owner deactivation UPDATE
re-targets that row, session revocation becomes per-location-aware, with its own red→green proof
(*deactivate courier in A ⇒ courier still active in B*). Per counsel, this is a **courier-livelihood
protection**, not a defense-in-depth footnote — deferring it is a named, human-visible decision with
the residual risk stated: until M-F3 lands, an owner deactivating a legitimately-shared courier
deactivates them for ALL their locations (the `couriers.ts:89-92` membership gate already blocks
cross-location *targeting*, so only genuinely-shared couriers are affected; today that population is
operator-seeded demos).

**Class fix (the reason it slipped) — matcher REWRITTEN per breaker B3** (the old spec could not see
its own fix): `local/require-auth-hook` v2 —
1. Recognizes auth in all three live shapes: (a) `addHook(hookName, X)` where X is a bare
   `Identifier`, a `MemberExpression` (`fastify.verifyAuth`), **or a `CallExpression`**
   (`requireRole([...])` / `fastify.requireRole([...])` — the factory form the fix itself uses);
   (b) **per-route option arrays** `{ preValidation: [...] } / { preHandler: [...] }` on
   `.get/.post/.put/.patch/.delete/.route(...)` — the shape 10/27 owner files already use
   (categories, locations, menu-availability, menu-confirm, menu-import, menu-translate,
   modifier-groups, product-media, products, promotions); (c) `register`-level options as today.
2. **Per-ROUTE granularity, not per-file**: every route registration in a scoped file must be covered
   by (file-level hooks ∪ its own route options) containing BOTH a verifyAuth-shape AND a
   requireRole-shape; report lands on the specific route node. This closes the LC2-shaped hole (one
   unguarded route hiding in a guarded file). Escape hatch: `// auth-exempt: <reason>` line comment
   above the route (grep-auditable, counted in the same ratchet as §4B's allowlist).
3. Scope stays `routes/owner/**` + `routes/courier/**`. `routes/orders.ts` (mixed-plane) is
   explicitly covered by the §4B tenant-query rule instead — its owner routes already carry inline
   `preHandler` with `requireRole` (verified `:841`); its risk is unscoped queries, not missing hooks.
4. Ratchet to `error` immediately after the two fixes: red on couriers.ts + courier-invites.ts
   pre-fix, green post-fix, **zero false positives on the 10 preValidation-array files** — all three
   are fixture-proven parts of the rule's own red→green (§6).

### 3.3 LC5 / F2 — validate the child FK is same-tenant, and harden the sink (SHIPS FIRST — §8)
Honest layering per breaker B6: the **entry gate is the load-bearing control**; the sink predicate is
a *dependent* tamper-evidence layer (it is only as good as the scope its caller threads), not an
"independent second layer." Both ship, correctly labeled:
1. **Entry gate (`owner/gdpr.ts`):** when `customerId` is supplied directly (`:48`), prove membership
   before the INSERT (`:81-86`): `SELECT 1 FROM customers WHERE id=$customerId AND
   location_id=$locationId` → 404 otherwise. (The `phone` branch `:50-53` is already scoped; this
   brings the `customerId` branch to parity.)
   **Plus attempt-logging (counsel advice 2 / STOP-1):** on gate miss, ONE additional server-side
   lookup (never surfaced to the caller) classifies `nonexistent` vs `cross_tenant_attempt`; a
   cross-tenant attempt writes a structured security log/audit row (actor userId, actor locationId,
   target customerId, subject locationId, request id, ts) **before** returning the same 404. A blocked
   cross-tenant erasure attempt is a signal, not just a denial — future attempts stay detectable after
   the hole is closed. The same pattern is applied to the LC2 PATCH (JOIN-miss → classify → log → 404).
2. **Sink predicate + scope-required (`lib/anonymizer/index.ts`):** add `AND location_id = $2` to
   **every** by-id read/write in the anonymizer — `:118-121` lock, `:133-140` UPDATE, `:148`
   `avatar_key` read (drives a storage delete), `:195-198`/`:210-221` (symmetric `anonymizeOrder`),
   and the `:57`/`:64` dry-run branch (uniformity — per breaker B7 that branch is currently
   **dead code**, `dryRun:true` has zero production callers; scoped anyway at ~0 cost and flagged to
   the cleanup ledger for deletion — the enumeration is now reachability-annotated).
   **AND delete the fallback (breaker B6 — this is what makes the predicate real):**
   `index.ts:131` + `:208` `const locationId = options.subject?.locationId || row.location_id` becomes
   a **required** explicit scope — no `|| row.location_id`. Missing scope → throw (fail-closed), so no
   future caller can silently self-satisfy the predicate with the row's own location. Live callers
   already comply: the GDPR worker threads the request row's `location_id`
   (`workers/anonymizer-gdpr.ts:64`); the retention sweep (`index.ts:89`) threads each customer's own
   location explicitly (legitimate cross-location system path — unchanged behavior, now explicit).
3. **Forensic provenance (STOP-1 / counsel §5 — code-only, `anonymization_audit_log.metadata` is
   JSONB):** the audit row's `location_id` column now stamps the **subject's true tenant**
   (`row.location_id` — the row's own), and `metadata` gains `actor_location_id` (the caller-verified
   tenant), `subject_location_id`, and `request_id`. An actor≠subject record is thereby
   **self-evidencing** — the data subject's per-record actor-vs-subject provenance the counsel asked
   for. Pre-fix the stamp recorded the attacker-supplied tenant; that blind spot is what STOP-1's
   forensic decision record (resolution.md) addresses retrospectively.
   **BOTH audit writes must be fixed (breaker R2-5 — covered-by-implementation):** there are TWO
   audit-log inserts per GDPR erasure — the anonymizer's (`lib/anonymizer/index.ts:291`) AND a SEPARATE
   worker insert (`workers/anonymizer-gdpr.ts:74-78`, currently stamped `row.location_id` = the request
   row's tenant). Both must stamp `location_id = <subject's true tenant>` and carry
   `metadata.actor_location_id` (the requester) + `subject_location_id` + `request_id`, else one erasure
   produces two audit rows with divergent tenants and the provenance is only half-applied. **The parallel
   implementation lane has been instructed to fix the worker insert; recorded here as covered-by-implementation.**
   (Note: post the LC5 entry-gate fix, the GDPR path can only enqueue `row.location_id` == the subject's
   true tenant, so the two converge for NEW requests; the dual-stamp fix is what keeps HISTORICAL/attack
   rows and the retrospective forensic query (STOP-1 query 2) consistent.)

### 3.4 F5 — thread `locationId` into the signals sink
`lib/signals/compute.ts:85-88` gains `AND location_id = $2`; `owner/signals.ts:118` passes `locationId`
(already in scope from the URL param, membership-checked by `requireLocationAccess`).

### 3.5 The remaining live IDORs — point-fix ALL of them (breaker B1, CRITICAL)
The prior revision enumerated these as LIVE and then handed them to a warn-lint that could not flag
them. **That was enumeration-as-remediation and is withdrawn.** The live surface is small and fully
enumerated (breaker confirmed §5a complete) — so every site gets a Tier-1 point-fix. Note these routes
already carry `verifyAuth + requireRole(['owner']) + requireLocationAccess` (verified per-route
preValidation) — the defect is purely the **missing product/group ownership predicate** on the child
id. The fix pattern folds ownership into the statement itself (the predicate/JOIN IS the boundary —
same shape as LC2), 0 rows → 404:

| # | Site | Fix |
|---|------|-----|
| 1 | `products.ts:211-217` PUT translations | `INSERT INTO product_translations … SELECT p.id,$2,$3,$4 FROM products p WHERE p.id=$1 AND p.location_id=$5 ON CONFLICT … ` — 0 rows → 404 (product not in caller's location). `product_translations` has no `location_id` column, so parent-ownership fold-in is THE fix (RLS can never scope this table) |
| 2 | `products.ts:239` GET translations | `SELECT pt.* FROM product_translations pt JOIN products p ON p.id=pt.product_id AND p.location_id=$2 WHERE pt.product_id=$1` |
| 3 | `products.ts:256` DELETE translation | `DELETE FROM product_translations pt USING products p WHERE p.id=pt.product_id AND p.location_id=$3 AND pt.product_id=$1 AND pt.locale=$2` — 0 rows → 404 (existing behavior) |
| 4 | `products.ts:282` DELETE product_modifier_groups (**destructive cross-tenant write**) | same-tx product-ownership pre-check (`SELECT 1 FROM products WHERE id=$1 AND location_id=$2` → 404) guarding the whole sync, **plus** `AND location_id=$2` on the DELETE itself (the table has the column — the sibling INSERT `:286` sets it) |
| 5 | `products.ts:285-289` INSERT product_modifier_groups | `INSERT … SELECT $1, mg.id, $3, $4 FROM modifier_groups mg WHERE mg.id=$2 AND mg.location_id=$4` — verifies each `group_id` is same-tenant; any miss → 400 (foreign/unknown group), tx rolls back |
| 6 | `products.ts:307-313` GET product modifier-groups | `AND pmg.location_id=$2` (+ `AND mg.location_id=$2` on the JOIN) |
| 7 | `modifier-groups.ts:156-160` POST modifiers | `INSERT INTO modifiers (location_id,group_id,…) SELECT $1, mg.id, … FROM modifier_groups mg WHERE mg.id=$2 AND mg.location_id=$1` — 0 rows → 404. **Companion cleanup:** the owner-GET count join `:62` gains `AND m.location_id = mg.location_id` so any historically-injected foreign row stops surfacing |
| 8 | `categories.ts:163-165` + `:244-245` existence oracle | the products pre-check SELECT gains `AND location_id=$2` — closes the 409-vs-404 oracle (the DELETE itself was already scoped) |

Together with §3.1–3.4 this closes **the entire enumerated live surface**: all 11 §5a LIVE-IDOR sites
+ the anonymizer sink + the 2 role-gate files. Nothing live is left to a lint.

### 3.6 ROOT — the single enforced seam (options in §4)
A `getOwnerLocationId`/membership-JOIN **mandate** + a lint/guardrail that **FAILS** on a raw
tenant-table query without a tenant predicate. This is the class-killer: it converts "remember the
predicate" from a per-author discipline into a build-time gate — but per breaker B1/B2 it is a
**recurrence-stopper for FUTURE queries, never a substitute for fixing known-live sites**. Design
options and tradeoffs in §4 (redesigned post-breaker). **Per R2-2 the lint models only the
`location_id`-predicate class; the child-FK-injection class (§3.9) is a SEPARATE gate.**

### 3.7 #15 / R2-1 writer — fold FK-ownership into the `menu_schedules` INSERT (code-only, ADDITIVE)
Verified live end-to-end (`menu-availability.ts:113-123`, mig `062:28-29`/`064:139-160`). The POST
`…/menu-schedules` INSERT trusts body `product_id`/`category_id`; `requireLocationAccess` verifies only
the URL `:locationId`, and the XOR precondition (`:107-111`) enforces only "exactly one target," not
ownership. Fix = the SAME write-side fold-in the frozen §2 sites #5/#9/#11 use — the predicate IS the
boundary; the seam cannot provide this (§3.9):
```
INSERT INTO menu_schedules (location_id, product_id, category_id, mode, start_minute, end_minute,
                            days_of_week, starts_at, ends_at, available)
SELECT $1, $2, $3, $4, $5, $6, $7, $8, $9, $10
WHERE ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM products   p WHERE p.id = $2 AND p.location_id = $1))
  AND ($3::uuid IS NULL OR EXISTS (SELECT 1 FROM categories c WHERE c.id = $3 AND c.location_id = $1))
RETURNING *
```
0 rows → the referenced product/category is not in the caller's location → **404** (mirror the existing
`sendError(404,'NOT_FOUND')` shape; the XOR 400 stays). **Sibling audit (breaker demand — done):**
the same file's other writers are SAFE and need no change — kitchen-busy PATCH (`:37-42`) is
`UPDATE locations … WHERE id=$1` (the verified `locationId`, no body FK); schedules DELETE (`:137-139`)
is `DELETE … WHERE location_id=$1 AND id=$2` (deletes only the caller's own schedule row, no body FK);
**there is no schedule UPDATE/PATCH endpoint**. So the POST INSERT is the *only* writer hole. Pool-agnostic
(correct under BYPASSRLS and post-flip). Additive site **#15**, not one of the frozen 14.

### 3.8 #15 / R2-1 read-path — forward-only `location_id`-predicate migration (RED-LINE, operator-gated)
The writer fix stops NEW injected rows; the read path must also stop scanning any historically-injected
row cross-tenant. Two SECURITY-DEFINER/STABLE functions scan `menu_schedules` with no `location_id`
predicate. A **forward-only** `CREATE OR REPLACE` migration (next number after `064`; red-line →
operator-gated apply, staged-DB first per ship discipline):

1. **`read_public_menu`** (mig `064:139-160`, the LIVE hot path — `v_location_id` is already resolved at
   `:52-53`): add `AND s.location_id = v_location_id` to all three `menu_schedules` subqueries (block
   `NOT EXISTS s`, allow-exists `s2`, allow-active `s3`).
2. **`product_available_now`** (mig `062:120-123`): currently **dormant on the live path** — the 064 UP
   replaced its call with the set-based predicate; only the emergency `down()` body (`FN_063_PRIOR`,
   `064:294`) still calls it. Harden it for the rollback path **without** changing its 3-arg signature
   (avoids a DROP+overload): self-scope the loop —
   `WHERE ((s.product_id = p_product_id AND s.location_id = (SELECT location_id FROM products   WHERE id = p_product_id))
      OR  (s.category_id = p_category_id AND s.location_id = (SELECT location_id FROM categories WHERE id = p_category_id)))`.

**L2 migration-mechanics checklist (money-council precedent — every box explicit):**
- [ ] **Signature EXACT** on `CREATE OR REPLACE`: `public.read_public_menu(p_location_id_or_slug text,
      p_locale text DEFAULT ''::text) RETURNS jsonb` — a signature change makes an *overload*, not a replace.
- [ ] **`SECURITY DEFINER` preserved** (the reader runs under `public_select`; losing definer breaks reads).
- [ ] **Parameter `DEFAULT ''` preserved** (Postgres forbids removing a default on replace).
- [ ] **REVOKE/GRANT re-emit / verify**: `CREATE OR REPLACE` keeps the ACL, so this is a *verify-no-EXECUTE-grant-dropped*
      step (documented); if any DROP is introduced, re-emit the grants that `062/063/064` rely on.
- [ ] **Forward-only** `down()` restores the exact prior body (current `064` UP for `read_public_menu`;
      current `062` body for `product_available_now`) — same pattern `064` already uses.
- [ ] **Behavior-preserving proof**: for legitimately-created rows (post §3.7) `s.location_id` == the
      product/category's location, so the added predicate excludes ONLY injected cross-tenant rows — honest
      menus render identically. Extend `packages/db/tests/read-public-menu-availability-equivalence.sql`
      with a cross-tenant-injected-row fixture asserting the victim's product survives.

### 3.9 The mechanical FK-sweep — enumeration by construction, not by manual convergence (R2-1 demand)
Rev-2's completeness rested on "two manual passes converge"; R2-1 falsified it (found #15). Rev-3 replaces
that with a **mechanical, reproducible two-column join** that any future audit re-runs:

**Class definition.** A *child-FK-injection* site is an `INSERT`/`UPDATE` `.query()` that binds a child-FK
column `C` of a tenant table `T`, where `C` REFERENCES a **sibling tenant-scoped parent** `P`
(products / categories / modifier_groups / modifiers / orders / customers / couriers — **not** `locations`,
which is the scope root), and `C`'s value comes from client-controlled input (`request.body` or a non-`:locationId`
param) **without** being (i) re-derived from a location-scoped `SELECT`, (ii) folded into `INSERT…SELECT…WHERE
P.location_id=$verified`, or (iii) pre-verified by a same-tx `SELECT 1 FROM P WHERE id=C AND location_id=$verified`.

**Method (encodable as a script / CI companion gate).**
1. **Build the FK manifest** from `packages/db/migrations/**` (or `information_schema.key_column_usage` ⋈
   `referential_constraints`): every column `T.C REFERENCES P(id)` where `P ∈ TENANT_TABLES ∧ P ≠ locations`.
   **Structural finding: EVERY such FK is plain single-column** — no parent carries a `UNIQUE(id, location_id)`,
   so no composite `(id, location_id)` FK is even expressible. *The schema cannot self-defend; the ownership
   predicate MUST live at each write site.* (This is why the lint's `location_id`-predicate model is blind here.)
2. **Cross-reference write sites**: for each `T.C`, grep `INSERT INTO T (…C…)` / `UPDATE T SET …C=…` in
   `apps/api/src/{routes,lib,workers}` and classify the binding source of `C`.

**Sweep output (run conceptually against live source — this is what carries the completeness claim):**

| Child-FK write site | Column → parent | Binding source | Verdict |
|---|---|---|---|
| `menu-availability.ts:115` INSERT | `product_id`/`category_id` → products/categories | **body verbatim** | **VULNERABLE = #15** → §3.7/§3.8 |
| `products.ts:432` UPDATE (`category_id=COALESCE($7,…)`) | `category_id` → categories | body, unverified | **NEW mechanical hit** — but writes only the caller's OWN product (`WHERE id AND location_id=$2`); a foreign `category_id` merely drops that product from the caller's own storefront (categories join is location-scoped) — self-foot-gun, **no cross-tenant read or write**. → LOW / referential-hygiene, DEFER-FLAG (add `EXISTS(categories WHERE id=$7 AND location_id=$2)`), NOT a ship-blocker |
| `products.ts:212/286`, `modifier-groups.ts:157` | product_translations/product_modifier_groups/modifiers | INSERT…SELECT fold-in | **FIXED** by frozen §2 (#5/#9/#11) |
| `menu-translate.ts:95/148/192` INSERT | category/product/modifier_translations | id re-derived from `SELECT … WHERE location_id=$1`; body `entity_filter` only narrows within scope | **SAFE by construction** |
| `dashboard.ts:331/343` INSERT courier_assignments | `order_id`/`courier_id` → orders/couriers | both pre-verified same-location (`:230` order FOR UPDATE, `:241` courier JOIN courier_locations) | **SAFE** |
| `order-persistence.ts:74/118` INSERT orders/order_items | `customer_id`/`product_id` | server-authoritative (price/product resolved from the location's menu; customer server-derived); order_id = same-tx | **SAFE** (customer plane, not owner-IDOR) |
| `dispatch.ts:49`, `courier-dispatch.ts:138`, `server.ts:679` | courier_assignments | server-derived (not client body) | **SAFE** |
| `menu-import.ts:395/401/422` | modifiers/product_modifier_groups | same-tx rows keyed by `external_key`, created in the location under `getLocationId` | **SAFE** |
| `menu-seed.ts`, `dev/mock-auth.ts` | seed/dev | server/dev-only | **OUT OF SCOPE** |

**Result beyond #15:** exactly **one** additional mechanical hit — `products.ts:432` `category_id` — which
triages to **LOW / self-scoped** (no cross-tenant harm). No second live cross-tenant IDOR exists in the
child-FK class. The completeness claim now rests on this join, not a manual pass; the method is the
standing companion gate for R2-2's lint blind spot.

---

## 4. Options for the DAL seam (≥2, with tradeoffs + effort)

The task asks for the single enforced seam. Two archetypes, plus the recommended sequenced hybrid.

### Option A — Repository layer (per-aggregate repos)
Extract `OrderRepo`, `CustomerRepo`, `CourierRepo`, `SignalsRepo`, … Every method takes `(tenantScope,
…)` and the predicate/JOIN lives **once** inside the repo; route handlers never write raw tenant SQL.
- **Pros:** strongest correctness — the predicate physically cannot be omitted because handlers don't
  author SQL; kills the ~30 copy-pasted order-SELECTs (arch F2) and the row-mapper dup as a side-effect;
  gives a natural home for `updateOrderStatus`-style invariants.
- **Cons:** **L effort** — ~777 raw `.query(` sites to migrate; high blast radius (touches money/state
  paths); a big-bang repo is itself a red-line change; realistically a multi-week staged migration, and
  until it is *complete* the class is only partially closed (a new raw query can still appear).
- **Enforcement:** needs a companion lint banning raw `.query(` in `routes/**` (else adoption stays
  partial — the exact failure mode that produced these findings). So Option A **still needs Option B's
  lint** to be a real gate.

### Option B — Mandatory preHandler + lint guardrail (enforced seam, no repo)
**REDESIGNED per breaker B2** — the v1 spec failed its own red→green (bare `location_id` token matched
SELECT/INSERT *column lists*, so `orders.ts:863` and `anonymizer:119` were unflaggable; the `withTenant`
exemption whitelisted exactly the live-under-BYPASSRLS sites; interpolated clauses were invisible).
v2 is **prove-or-allowlist, fail-closed**:
1. One canonical resolver — see §8 Tier-2 (re-scoped per breaker B4: one core resolution body +
   per-caller adapters preserving each caller's null-contract; NOT a single `null→401` collapse).
2. ESLint-local rule **`no-unscoped-tenant-query` v2** — scope: all of `apps/api/src/**` (routes AND
   lib sinks — anonymizer/signals live in `lib/`). Trigger: any `.query(…)` whose first arg references
   a table in `TENANT_TABLES` (checked-in constant generated from the RLS-FORCE migration list).
   A site passes ONLY via one of:
   - **(a) Predicate-form proof** — the SQL's *post-FROM* text (everything before the first
     `FROM`/`INTO` is stripped, so column lists can never satisfy the rule) matches a predicate token:
     `location_id\s*=\s*($n | <alias>.location_id)` inside WHERE/ON/AND/USING, `JOIN memberships`,
     `courier_id\s*=\s*$`, `customer_id\s*=\s*$` + `user.sub`-bound forms, or `token_hash\s*=\s*$`.
     `INSERT … VALUES` on a tenant table can never carry a predicate → it is UNKNOWN by construction
     (route (b)) — which deliberately pushes authors to the `INSERT … SELECT … WHERE <tenant
     predicate>` fold-in pattern the §3.5 fixes use.
     **Evasions closed (breaker R2-3):**
     - **(A) `$n`-anchor required** — an alias-to-alias equality (`a.location_id = b.location_id`) with
       NO server-bound `location_id = $n` (or `JOIN memberships … m.user_id = $n`) anchor in the same
       post-FROM text is **UNKNOWN → allowlist**, not a pass. The proof must include at least one
       `$n`-bound anchor; a pure two-table correlation cannot satisfy the rule (it is only safe when a
       *sibling* `= $n` anchors it, which the lint cannot distinguish — so it fails closed).
     - **(B) DML-CTE / multi-statement → UNKNOWN** — any SQL containing a data-modifying CTE
       (`WITH … AS (INSERT|UPDATE|DELETE …)`) or more than one statement is UNKNOWN → allowlist. The
       "strip before first FROM" heuristic mis-scopes a decoy outer predicate onto an unscoped inner
       write; fail-closed on structural complexity rather than trust it.
   - **(b) Allowlist entry** — `tools/eslint-plugin-local/tenant-scope-allowlist.json`: `{path,
     normalized-SQL-hash, reason, verified-by}`. Hash-keyed, not line-keyed: moving code doesn't churn
     entries; ANY text change to the query invalidates its entry → the rule goes red until the edited
     query is re-proven or re-audited. This is the inversion the RESOLVE round mandated: the ~verified
     tail is enumerated as known-safe; **anything new touching a tenant table fails by default**.
   - **Fail-closed rules:** any `${…}` interpolation in the SQL, or a non-literal first arg → UNKNOWN
     → allowlist required (even when a predicate is visible in the static text — the rule stays dumb
     and unspoofable; e.g. `modifier-groups.ts:108` gets an allowlist entry despite its literal
     `WHERE location_id = $1`). **NO `withTenant` exemption** while the pool posture is BYPASSRLS —
     `withTenant` seats a GUC, it does not validate a child FK; revisit only at the B3/flip council.
   - **(C) views (breaker R2-3):** a read `FROM <view>` over a tenant table is a silent false-negative
     because `TENANT_TABLES` is generated from FORCE-RLS *base* tables. Close it by extending the manifest
     generator to also enumerate any view whose definition selects from a tenant base table
     (`information_schema.view_table_usage ⋈ TENANT_TABLES`) and adding those view names to the trigger
     set. Mechanical, from the same SoT; documented as part of manifest generation, not a manual list.
   - **Severity — HONESTLY RE-COST per breaker R2-2 (the "~15-25 allowlist, `error` day-1" claim is
     FALSIFIED).** The breaker measured ~87 forced-UNKNOWN entries (~60 `INSERT…VALUES` on tenant tables
     + ~27 dynamic-`${}`) — a 4-6× under-count, because `INSERT…VALUES` is UNKNOWN by construction and
     the rev-2 budget omitted that whole population. "error from day 1 across all tenant writes" is
     therefore not deliverable without either a ~90-entry hand-audited allowlist (the rubber-stamp
     surface the design fears) or an ~87-error CI wall. Per the money-council **N4 precedent (relabel a
     speed-bump honestly)**, the rule ships as **two tiers with different severities**:
     - **Tier-A — `error` from day 1**: the `location_id`/membership **predicate-form proof** on
       `SELECT`/`UPDATE`/`DELETE` (the class that produced LC2/F5/LC5-sink) **plus** the dynamic-`${}`
       fail-closed rule. This is the adoptable set the breaker confirmed flaggable (post-FROM strip flags
       `orders.ts:863` + `anonymizer:119`); its allowlist is the bounded ~15-25 audited-safe SELECT/UPDATE/
       DELETE + dynamic tail. No live-IDOR grace window for THIS class.
     - **Tier-B — `warn` → count-floor ratchet → `error`**: `INSERT…VALUES` on a tenant table (~60 sites).
       Warn-level with a **monotonic floor that only decreases**, burning down as authors migrate the
       child-FK-bearing inserts to `INSERT…SELECT…WHERE` (the §3.7/§2 pattern). This avoids the day-1 wall
       AND avoids mass-allowlisting; the ratchet still prevents any NEW insert from escaping. Honest cost:
       Tier-B is a transitional warn, not an instant gate — stated plainly, not oversold.
   - **The FK-injection class is OUT of this lint's model (R2-2, explicit scoping).** A #9/#11/#15-shape
     bug carries a *correct* `location_id` + an *unverified child FK*, so a `location_id`-predicate lint
     cannot see it and would wave it through as "location_id scoped." The lint is therefore honestly scoped
     to "forgot the `location_id` predicate," and the **mechanical FK-sweep (§3.9) is its named companion
     gate** for the child-FK class — the two together, not the lint alone, are the class-killer. *Optional
     Tier-B extension:* an INSERT-column-FK heuristic that flags `INSERT…VALUES` binding a `*_id` column
     present in the §3.9 FK manifest (child→sibling-tenant) and NOT written as `INSERT…SELECT` — feeds the
     same ratchet, makes #15-class sites warn. Speculative; specified, not assumed-covered.
   - **Ratchet metric (counsel advice 5):** CI asserts allowlist entry-count ≤ the recorded floor and
     the floor only decreases; every entry carries `reason` + `verified-by`. A rising hatch count is
     the seam-erosion signal and fails the build. Tier-B carries its own floor.
   - **Adversarial self-test (counsel advice 4):** the rule ships with its own fixture suite —
     column-list decoy (MUST flag), predicate present (pass), `${clauses}` interpolation (MUST flag),
     string-concatenated SQL (MUST flag), `INSERT…VALUES` with `location_id` in the column list but an
     unverified FK (MUST flag), `INSERT…SELECT` with predicate (pass). The lint is trusted as a gate
     only after this suite is green; the residual false-negative surface (SQL built outside the call
     site) is bounded and handed to Option A's priority queue, not declared covered.
3. `require-auth-hook` v2 (§3.2 — factory-call + preValidation-array shapes, per-route granularity).
- **Pros:** **M effort**; Tier-A `error` immediately (no grace window for the `location_id`-predicate
  class); catches *future* regressions at author-time; no runtime behavior change → low deploy risk;
  reuses existing plugin infra.
- **Cons (honestly re-cost per R2-2):** the allowlist/ratchet floor is ~87 forced-UNKNOWN, not ~15-25 —
  so `error`-day-1 is Tier-A only; Tier-B (`INSERT…VALUES`, ~60 sites) is a warn-ratchet-to-error
  transition, not an instant gate. Dynamic SQL always costs an allowlist entry (deliberate friction).
  **The lint does NOT cover the child-FK-injection class** (§3.9 sweep is the companion gate). Does not
  eliminate the SQL/row-mapper duplication (that debt remains for Option A later).

### Option C — **RECOMMENDED: B now (class-killer), A incrementally (debt-paydown)**
Ship Option B as the enforcing gate in this batch — it stops the bleeding immediately and cheaply and
makes every future unscoped query a build failure. Then extract repositories per-aggregate *behind* that
gate, highest-churn/highest-risk first (Order → Customer → Courier), each migration provable because the
lint already guarantees no *new* unscoped query slips in during the multi-week paydown.
- **Why:** the four live bugs need the point fixes (§3.1-3.4) regardless; the *class* needs an enforced
  invariant *now* (B), and B is a prerequisite for A being safe (A without B leaves partial adoption — the
  present failure mode). A is the right long-term shape but is L-effort and must be staged; making it a
  gate-of-one is not achievable this batch. **Effort: M for B (this batch) + L for A (subsequent tracks).**

---

## 5. Enumerated raw-query IDOR surface

<!-- IDOR-SURFACE-ENUMERATION -->

Ground truth: `grep -rc '.query(' routes = 777` raw sites across 65 files; `withTenant` imported in 22.
The 43 files below carry no `withTenant` import and are the primary surface; the confirmed LIVE sites are
also present in withTenant-importing files whose raw `db.query` bypasses the callback.

**Verdict legend:** LIVE-IDOR = client-supplied id on a tenant table, no predicate, exploitable today.
RLS-ONLY-LATENT = scoped only by FORCE-RLS → IDOR the instant RLS relaxes or a raw pool query runs.
SCOPED-OK = bound to `user.sub`/`courier_id`/real predicate. PUBLIC-BY-DESIGN = storefront content.

Authz-verified boundary = `:locationId` in the path (checked by `requireLocationAccess` →
`SELECT 1 FROM memberships … role='owner' AND status='active'`) OR a server-resolved
`getOwnerLocationId`/`getLocationId`/`getOwnerLocation`. A site is SAFE only if it carries an explicit
`location_id=<verified>` predicate, JOINs through one, or is preceded by a same-tx ownership check.
Under BYPASSRLS `withTenant` protects nothing — so bare-child-id subroutes are **LIVE today**, not merely
latent (this upgrades audit F11's "closed by FORCE-RLS" to LIVE under the actual deployed pool posture).

### 5a. LIVE-IDOR (client-supplied id, tenant table, no predicate — exploitable today)
Owner-routes sweep result (exhaustive, verified line-by-line): **11 LIVE sites**, 9 genuine cross-tenant
data read/write + 2 existence-oracle.
| Site | Endpoint | Table · verb | Note |
|------|----------|--------------|------|
| `routes/orders.ts:862-865` | PATCH `/orders/:id/status` | orders · SELECT | **LC2** — no JOIN; GET sibling `:747` fixed, this left behind |
| `routes/owner/gdpr.ts:81-86` | POST `/:loc/gdpr-requests` | gdpr_erasure_requests · INSERT-FK | **LC5** — body `customerId` verbatim (`:48`) → enqueues foreign erasure |
| `lib/anonymizer/index.ts:118-141` | (worker sink) | customers · SELECT-FOR-UPDATE + UPDATE | **LC5** sink — no `location_id`; `:195-222 anonymizeOrder` symmetric |
| `lib/signals/compute.ts:85-88` | GET `/:loc/signals/compute` | customers · SELECT | **F5** — foreign customer reputation |
| `routes/owner/products.ts:211-217` | PUT `…/products/:id/translations/:locale` | product_translations · UPSERT | overwrites foreign product's translation |
| `routes/owner/products.ts:239` | GET `…/products/:id/translations` | product_translations · SELECT | reads foreign product's translations |
| `routes/owner/products.ts:256` | DELETE `…/products/:id/translations/:locale` | product_translations · DELETE | deletes foreign product's translation |
| `routes/owner/products.ts:282` + `:285-289` | PUT `…/products/:id/modifier-groups` | product_modifier_groups · DELETE+INSERT-FK | wipes/links foreign product's group links |
| `routes/owner/products.ts:307-313` | GET `…/products/:id/modifier-groups` | product_modifier_groups · SELECT | reads foreign product's modifier groups |
| **`routes/owner/modifier-groups.ts:156-160`** | POST `…/modifier-groups/:groupId/modifiers` | modifiers · INSERT-FK | **NEW (not in audit)** — unverified `:groupId`, single-col FK → injects a modifier row referencing a competitor's group. **Severity corrected per breaker B8 (was overstated):** does NOT reach checkout pricing — both order-pricing joins filter `m.location_id = <order's location>` (`orders.ts:225/436`), so the injected row (carrying the attacker's `location_id`) is filtered out of the victim's checkout. Real surface: victim's owner-GET `modifier_count` inflation (`:60-64` join lacks a location guard — companion-fixed in §3.5) + a global-FK existence oracle. Still a genuine cross-tenant write → still Tier-1 point-fixed; MED not HIGH |
| `routes/owner/categories.ts:163-165` + `:244-245` | DELETE `…/categories/:id` | products · SELECT | existence-oracle only (409 vs 404); the DELETE itself is loc-scoped/SAFE |

Plus the two role-gate gaps (privilege-escalation, within-tenant cross-role): `routes/owner/couriers.ts`
and `routes/owner/courier-invites.ts` — missing `requireRole(['owner'])` (**F3/F4**).

### 5b. LATENT (fails-closed today by a bug, or output-gated / inert FK — one edit from LIVE)
| Site | Why latent | Trap |
|------|-----------|------|
| `routes/owner/menu-translate.ts` (7 sites `:37,64,95-97,105,148-150,158,192-194`) | guard reads `params.locationId` but route param is `:id` → always 400, handler unreachable (audit F10) | rename `:id`→`:locationId` **without** adding a real predicate ⇒ venue-wide cross-tenant menu overwrite |
| `routes/owner/dashboard.ts:556-582` | GET `…/orders/:orderId/verify` — 3 sibling reads (order_items/products, courier_assignments/couriers/shifts, courier_audit_log) fetch cross-tenant but response is 404-gated by `orderRes` (`:585`) | remove/reorder the output-gate ⇒ leak |
| `routes/owner/menu-availability.ts:114-122` | POST menu-schedules INSERT — `location_id=mine` gates all reads, so a dangling foreign product_id/category_id FK is inert | reads that stop scoping by the row's own location_id ⇒ live |
| `routes/customer/orders.ts:309-312` | `UPDATE orders … WHERE id=$2` cancel (also LC3 phantom columns); ownership checked on an earlier read, not this UPDATE | courier/customer lane sweep pending |
| `routes/courier/me.ts:89,152,162` | `UPDATE couriers … WHERE id=$1` on the **global** courier row, self-bound via token but not `courier_locations`-scoped | shared-courier cross-tenant side-effect |

### 5c. SCOPED-OK (verified sound — do not touch)
Owner group: `order-meta.ts:29-31`, all of `themes.ts`, `dashboard.ts:626 transitionOrder`
(`WHERE id=$1 AND location_id=$2`) + snapshot/confirm/reject/assign/pickup/deliver, `activation.ts`,
`alerts.ts`, `dwell-settings.ts`, `fallback.ts`, `locations.ts`, `push.ts`, `notifications.ts`,
`onboarding.ts`, `menu-import.ts` (getLocationId + `id AND location_id` FOR UPDATE), `promotions.ts`,
`settlements.ts` (payout-ownership check precedes items), `product-media.ts`, `menu-confirm.ts`,
`refunds.ts`, `reveal-contact.ts`, and the products/categories/modifier-groups **primary** CRUD (the
`/:locationId/…:id` routes use `id AND location_id`) — only their bare-child-id subroutes are LIVE.
`gdpr.ts` everything except `:82`; `signals.ts` everything except compute. `couriers.ts`/
`courier-invites.ts` are SQL-safe (loc-scoped or `courier_locations`-preceded) — their only defect is the
missing role gate. Cross-plane: `order-messages.ts` (per-op membership check `:58/136/172`),
`customer/track.ts` (secret `token_hash`), `courier/me.ts:45` (`c.id=$1 AND cl.location_id=$2`).
PUBLIC-BY-DESIGN: all `routes/public/*`.

**Counts (owner-plane sweep, verified):** ~78 caller-supplied-id query sites on tenant tables →
**11 LIVE-IDOR** (9 data + 2 oracle) · **11 LATENT** (7 = menu-translate guard-bug, 3 = verify
output-gated, 1 = inert FK) · **~56 SAFE**. Files missing `requireRole(['owner'])`: **2**
(couriers.ts, courier-invites.ts). The courier/customer/admin-plane lane (independent sweep) confirms
the cross-plane SCOPED-OK set above — ~250+ raw `.query(` sites bound to `courier_id/customer_id=user.sub`,
a validated `location_id`, a preceding ownership SELECT, or a secret bearer `token_hash`; all ~15
`routes/public/*` files are PUBLIC-BY-DESIGN (slug/public-token storefront data); the 3 `routes/admin/*`
files are a separate platform-admin plane (`requirePlatformAdmin`, fleet-wide by design — not tenant
IDOR). Out of scope: otp/auth/webhook/dev credential flows. Only two minor non-IDOR notes surfaced:
`public/funnel.ts:62` inserts analytics on an arbitrary body `location_id` (write-amplification, not a
read/mutate leak) and `owner/settlements.ts:314 /regenerate` recomputes all locations by design-comment
(F13 tenant-smell, already known). **CORRECTION (breaker R2-1): this "two lanes converge / complete surface" claim was FALSIFIED — a 15th
live site (`menu-availability.ts:115` INSERT `menu_schedules`, §5b previously mislabeled LATENT) was
found post-hoc.** The §5b "inert FK" note for menu-availability was asserted without checking the read
path (`read_public_menu`/`product_available_now` scan `menu_schedules` with no `location_id` predicate),
so the FK is LIVE, not inert. **Rev-3 no longer rests completeness on manual convergence** — it rests on
the **mechanical FK-sweep (§3.9)**, whose child-FK-class join finds exactly #15 (CRITICAL, now Tier-1
via §3.7/§3.8) + one LOW self-scoped hygiene hit (`products.ts:432` `category_id`), and nothing else.
The full live cross-tenant surface = the frozen 14 §2 sites + **#15** + the anonymizer sink; **every one
gets a Tier-1 fix — the lint is not remediation** (breaker B1). What the redesigned `no-unscoped-tenant-query` **v2** (§4B) adds is the
recurrence gate: the audited tail becomes an explicit allowlist, anything NEW touching a tenant table
fails by default, and under the v2 predicate-form/post-FROM/INSERT-as-UNKNOWN rules all of today's LIVE
sites would have been flagged at author-time (the v1 spec could flag only ~1 of them — see
breaker B2 and the fixture suite that now proves this).

---

## 6. How each fix is proven (red→green) — wire the never-run test

The synthesis names the RLS-adversarial IDOR test as *"never wired"* (root R-C). Verified:
`apps/api/tests/phase5/rls-adversarial.test.ts` **exists** and is a genuine adversarial suite (owner-A
attempts cross-tenant SELECT/INSERT/UPDATE/DELETE on ~40 tenant tables via the RLS-enforced pool, with a
bypass-pool ground-truth check and a positive control). It is matched by the `test:unit` glob
(`package.json:35`) **but no CI job runs `test:unit`** — `ci.yml` runs build/typecheck/lint/gates/
verify-all/fresh-provision/deploy+E2E only. So the test is real and green-when-run but **never runs**.

**Proof plan (each fix gets a failing assertion that only its fix makes pass):**

1. **LC5 (FIRST — §8)** — owner-A `POST …/{A}/gdpr-requests {customerId:<tenantB uuid>}` ⇒ **404**,
   and after the worker drains, tenant-B customer `name`/`phone` **unchanged** (bypass-pool). Sink
   tests: (a) `UPDATE` touches 0 rows when `location_id` ≠ scope; (b) **fallback-deletion proof** —
   `anonymize()` invoked WITHOUT an explicit scope ⇒ **throws** (fail-closed), does not self-derive
   from the row; (c) **attempt-log proof** — the blocked cross-tenant request produces a security-log
   row classified `cross_tenant_attempt` with actor+subject tenant; (d) **provenance proof** — a
   legitimate erasure's audit row carries `metadata.actor_location_id` + `subject_location_id` and the
   `location_id` column equals the subject's true tenant.
2. **LC2** — owner-A `PATCH /api/orders/{tenantB_order}/status {"status":"CANCELLED"}` ⇒ **404** and
   tenant-B order status **unchanged** (bypass-pool ground truth). RED against `WHERE id=$1`, GREEN
   after the JOIN. Plus the cross-tenant-attempt log row (same pattern as LC5).
3. **§3.5 sites (one adversarial assertion each, owner-A token vs tenant-B object):**
   PUT/GET/DELETE `…/products/{B_product}/translations…` ⇒ **404** and B's translation unchanged/
   present (bypass-pool); PUT `…/products/{B_product}/modifier-groups` ⇒ **404** and B's
   `product_modifier_groups` rows **intact** (the destructive-DELETE case — the single most important
   assertion in the batch); PUT `…/products/{A_product}/modifier-groups [{group_id:<B_group>}]` ⇒
   **400** and no link row created; GET `…/products/{B_product}/modifier-groups` ⇒ **404/empty**;
   POST `…/modifier-groups/{B_group}/modifiers` ⇒ **404** and no row in B's group; categories oracle:
   DELETE `…/categories/{B_category_with_products}` ⇒ same status as a nonexistent id (oracle closed).
4. **F3** — customer-token `GET /api/owner/locations/L/couriers` ⇒ **403** (was 200 with decrypted
   roster); courier-token `PATCH …/couriers/:id` ⇒ **403**. (Per-location deactivation proof —
   *deactivate in A ⇒ still active in B* — moves to track M-F3 with its migration.)
5. **F4** — courier-token `POST …/courier-invites {role:'owner'}` ⇒ **403** (and `role` allow-list
   rejects non-`courier` ⇒ 400).
6. **F5** — owner-A `GET …/{A}/signals/compute?customer_id=<tenantB uuid>` ⇒ empty/`404`, no B counters.
6b. **#15 / R2-1 (writer, code-only §3.7)** — owner-A `POST …/{A}/menu-schedules {product_id:<B_product>,
   available:false}` ⇒ **404** (INSERT…SELECT 0 rows) and **no `menu_schedules` row created** (bypass-pool);
   same for `{category_id:<B_category>}`. Sibling coverage: kitchen-busy PATCH + schedule DELETE unchanged
   (no body FK) — asserted still 200/204 for legitimate same-tenant calls.
6c. **#15 / R2-1 (read path, migration §3.8)** — with a pre-seeded cross-tenant `menu_schedules` row (A's
   `location_id`, B's `product_id`, `available:false`), tenant-B `/s/:slug` (`read_public_menu`) still
   **renders B's product** (the added `AND s.location_id = v_location_id` excludes the injected row);
   equivalence fixture asserts honest menus render byte-identically pre/post (extends
   `read-public-menu-availability-equivalence.sql`). RED before the predicate, GREEN after.
6d. **Mechanical FK-sweep (§3.9)** — the sweep script/spec output is checked in as the completeness
   artifact: FK manifest (child→sibling-tenant, all plain) × write-site classification ⇒ exactly #15
   VULNERABLE + `products.ts:432` LOW-hygiene, rest SAFE-with-reason. Re-running it in CI on a schema/route
   change that adds an unverified child-FK write ⇒ a new VULNERABLE row (companion gate for R2-2's blind spot).
7. **ROOT** — (a) **wire `test:unit` (incl. rls-adversarial) into CI** as a job against the
   already-provisioned fresh-provision service DB — this is the load-bearing gate (R-C); a skipped
   should-succeed setup is a **FAIL**, not a green. `IDOR_TABLES` (currently
   `['orders','customers','courier_positions']`, rls-adversarial.test.ts:30) gains `courier_sessions`,
   `gdpr_erasure_requests`, `product_translations`, `product_modifier_groups`, `modifiers`,
   **`menu_schedules`** — the sink + injected tables, so the point fixes (incl. #15) are covered by the
   standing sweep.
   (b) **`no-unscoped-tenant-query` v2 red→green (Tier-A `error`; Tier-B `warn`/ratchet, per R2-2):**
   *Tier-A* pre-fix tree ⇒ MUST flag `orders.ts:863` (column-list decoy — the v1 failure case, now the
   canonical RED fixture), `anonymizer/index.ts:119, :133, :148, :196, :210`, `compute.ts:86`,
   `products.ts:239, :308`; post-fix + seeded allowlist ⇒ **0 Tier-A errors**. *Tier-B* (`INSERT…VALUES`)
   ⇒ warn-flags `products.ts:212, :286`, `modifier-groups.ts:157`, `menu-availability.ts:115` at the
   recorded floor; the R2-3 self-suite (`$n`-anchor MUST-flag, DML-CTE MUST-flag, view-manifest hit,
   column-list decoy MUST-flag, `INSERT…SELECT` pass) green. Honest note: the lint does NOT green #15
   (FK class is out-of-model) — §3.9's sweep is the gate that does.
   (c) **`require-auth-hook` v2 red→green:** RED on couriers.ts + courier-invites.ts pre-fix; GREEN
   post-fix; **zero false positives** across the 10 preValidation-array owner files (fixture-asserted,
   since factory-call + array shapes are exactly what the v1 matcher missed).
   (d) **Resolver contracts (Tier-2, per B4):** fresh owner (no membership) `GET /api/owner/settings`
   ⇒ **200 `{id:null}`** (onboarding NOT 401); promotions cross-tenant ⇒ **401** (the one approved
   delta, was 500); `{locId,userId}` shape preserved for spa-proxy/product-media callers.
   Each guardrail lands with a `docs/regressions/REGRESSION-LEDGER.md` row (red→green cited).

Never cheat green: no `.skip`/`.only`/inflated timeout/`assert.ok(true)`; a skipped provisioned-setup is
a failure. The rls-adversarial `IDOR_TABLES` list should gain `courier_sessions` and the sink tables so
the point fixes are covered by the standing sweep, not just bespoke tests.

---

## 7. Migration & contract impact

- **Migrations:** the Tier-1 *code* batch is **code-only** (predicates/JOINs/hooks/provenance-metadata —
  `anonymization_audit_log.metadata` is JSONB, verified), INCLUDING the #15 writer fold-in (§3.7,
  INSERT…SELECT — code-only). **Two migration tracks are honestly separated out (not claimed code-only):**
  (i) track **M-F3** (per-location courier deactivation, breaker B5: `courier_locations` has no status
  column); (ii) track **M-menu-sched-read** (#15 read-path, breaker R2-1, §3.8: forward-only
  `CREATE OR REPLACE` of `read_public_menu` + `product_available_now` adding the `location_id` predicate —
  red-line, operator-gated apply, staged-DB first, L2-checklist in §3.8). The Section-E RLS-policy gaps
  (couriers/courier_sessions NO RLS, unscoped anon policies) remain deferred to the B3/flip council
  (sequencing contract, §1 non-goals). **Sequencing note:** the #15 writer (§3.7) can ship in the code
  batch immediately; the read-path migration (§3.8) is defense-in-depth for historically-injected rows
  and ships operator-gated — the writer fix alone closes NEW injections, so the two are not co-blocking.
- **API contract:** no request/response shape change. Behavior change is **status-code only** on
  previously-exploitable cross-tenant calls: 200→404 (LC2, LC5, all §3.5 product/modifier subroutes),
  200→400 (foreign `group_id` in the modifier-groups sync), 200→403 (F3), success→403/400 (F4),
  data→empty (F5), oracle-409→404-parity (categories). Corrections of a leak, not breaking changes for
  legitimate same-tenant callers (who continue to get 200). The F4 `role` allow-list is additive
  validation (400 on bad input). **Per counsel advice 3, every status-code delta above ships as a
  one-line ADR/changelog note** so machine callers (demo-seeding scripts, settlements `/regenerate`)
  and future debuggers are not blindsided by an honest tightening.
- **Failure-semantics unification (re-scoped per breaker B4, signature widened per breaker R2-4):** ONE
  core membership-resolution body — **`resolveOwnerMembership(db, userId, activeLocationId?) →
  {locationId, userId} | null`**, no HTTP semantics. The `activeLocationId` parameter is **load-bearing,
  not cosmetic**: 5 of the 6 clones (`get-owner-location.ts:11-18`, `spa-proxy getLocationId:70-77` /
  `getOwnerContext:134-141`, `product-media:51-56`, `promotions:16-26`) implement the ADR-0004 **P-d verify
  branch** — when a baked `activeLocationId` is present, verify THAT location against a live active owner
  membership (`SELECT 1 FROM memberships WHERE user_id=$1 AND location_id=$2 AND role='owner' AND
  status='active'`) and return it or `null`; only when absent fall back to `LIMIT 1`. A `(db,userId)`-only
  core would silently DROP that branch, downgrade to `LIMIT 1`, and could return a *different* location for a
  multi-membership owner — regressing the insider-removal-window-to-zero property for 5/6 callers. The widened
  core implements both branches exactly as `get-owner-location.ts` does today; menu-import (the 6th clone,
  which currently lacks the branch) gains it for free. Thin per-caller adapters then PRESERVE each caller's
  contract: `get-owner-location` keeps null→401;
  the 3 spa-proxy callers keep their deliberate null→200 mappings (`/api/owner/settings` `{id:null}`
  onboarding routing, invite `pending:true`, storefront `bootstrap_owner` fall-through) — spa-proxy
  also keeps its raw-`Authorization`-header parse and feeds the derived userId into the core;
  `product-media`/`getOwnerContext` keep the `{locId,userId}` shape. The ONLY intentional behavior
  change is `owner/promotions.ts` cross/no-membership **500 → 401** (correctness fix, changelog-noted).
  First-run onboarding is regression-proven (§6.7d).
- **Guardrail/CI:** new ESLint rule + `require-auth-hook` tightening + a new CI job running `test:unit`
  against the service DB. Guardrail + migration paths are protect-path/operator-gated.

---

## 8. Rollout & sequencing

Order within Tier-1 is **highest-harm-first** (counsel: irreversibility + broken backup outranks
audit severity): **LC5 → LC2 → the remaining live IDORs (§3.5) → F3/F4 role gates → the seam.**

0. **STOP-1 gate (needs-human, before prod ship of this batch and before real tenant #2):** the
   operator runs the forensic queries in `resolution.md §STOP-1` ("was LC5 exploited?" — answerable
   via `gdpr_erasure_requests ⋈ customers` cross-join, since `customers.location_id` survives
   anonymization) and records outcome + disclosure decision. Design/build/staging may proceed in
   parallel; the recorded decision is a ship-gate, not a work-gate.
1. **Tier-1 (point fixes, ship together):** LC5 (entry gate + sink + fallback-deletion + provenance +
   attempt-log, **incl. the worker's second audit-insert per R2-5**), LC2, **all eight §3.5 sites** (incl.
   the destructive `products.ts:282` DELETE), **#15 writer fold-in (§3.7 — code-only)**, F3, F4, F5 — the
   **complete enumerated live surface** (now resting on the §3.9 mechanical sweep, not manual convergence),
   each with its red→green proof (§6) and a ledger row. Council-gated (authz/PII red-line); safe-direct per
   surface once approved. **No live site is left to a lint** (breaker B1/R2-1).
1b. **Track M-menu-sched-read (migration, operator-gated — R2-1 read path §3.8):** forward-only
   `CREATE OR REPLACE` adding the `location_id` predicate to `read_public_menu` + `product_available_now`;
   red-line apply, staged-DB first, L2-checklist + equivalence fixture (§3.8). Defense-in-depth over §3.7;
   not co-blocking with the code batch.
2. **Tier-1 guardrails (same batch — the lint is NOT separable/deferrable; deferring it recreates the
   bug, per counsel):** `require-auth-hook` v2 at **error** (reaches 0 honestly with the two fixes);
   `no-unscoped-tenant-query` v2 at **error** with the seeded allowlist + count-floor ratchet; the
   `test:unit` CI job with the expanded `IDOR_TABLES`. These lock the class closed.
3. **Track M-F3 (migration, operator-gated):** `courier_locations.status` + per-location deactivation
   + per-location session semantics + the *deactivate-in-A ⇒ active-in-B* proof (courier-livelihood
   protection — see §3.2).
4. **Tier-2 (debt-paydown, subsequent tracks):** resolver core+adapters per §7 (contract table in the
   ADR, onboarding regression-proven); then per-aggregate repositories (Customer→Order→Courier —
   highest-harm aggregates first, per counsel's steel-man), each provable because the lint prevents
   new unscoped queries meanwhile.
5. **Hand-off to B3/flip council:** Section-E RLS-policy migrations + the NOBYPASSRLS flip — a query
   that is predicate-correct today stays correct post-flip, so this batch is a strict prerequisite,
   not a conflict. The lint's `withTenant`-exemption question is re-opened ONLY there.

**Stakeholders (counsel advice 7):** the protected parties of this batch are not only tenant owners —
they are the **data subjects**: tenant B's *customer* (whose name/phone LC5 let a stranger erase,
irreversibly) and the *courier* (whose decrypted PII F3 exposed to customers, whose livelihood the
shared-deactivation side-effect threatens — hence M-F3). The provenance fix (§3.3.3) exists for them:
a wrongful erasure is at minimum attributable and disclosable to the affected party. The ADR names
them as first-class stakeholders.

Decision to ratify in the ADR: **Option C** (enforced-seam lint now + incremental repositories later),
with the v2 lint/hook designs and the complete-live-surface Tier-1 from this revision.
