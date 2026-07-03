# Council RESOLUTION R2 — Audit-fix authz: disposition of the re-attack findings

Status: **RESOLVE round 2 complete** (Triadic Council STEP 3, on the rev-2 re-attack). Date: 2026-07-03.
Inputs: `breaker-findings-r2.md` (R2-1..R2-6), `proposal.md` (now rev 3 — revised in place by this round),
`resolution.md` (round-1 dispositions — **the 14-site §2 fix list is FROZEN and untouched; a parallel
implementation lane is building it**). Every rev-3 delta is **purely ADDITIVE**. Design-only, no
production code, no lint implementation. **Conductor may re-attack (round 3); this document does not
self-certify.**

---

## 1. Disposition table (R2 findings)

| ID | Sev | Disposition | Resolution (where in rev-3) |
|----|-----|-------------|------------------------------|
| **R2-1** — 15th LIVE cross-tenant IDOR: `menu_schedules` storefront-availability tampering; enumeration incomplete | **CRIT** | **FIX (additive Tier-1) + FIX (operator-gated migration) + FIX (mechanical enumeration)** | **Writer** (§3.7, code-only, additive site #15): `menu-availability.ts:115` INSERT → `INSERT…SELECT … WHERE EXISTS(products/categories WHERE id=body-fk AND location_id=$verified)`, 0 rows → 404 — the same fold-in as frozen §2 #5/#9/#11. **Sibling audit done:** kitchen-busy PATCH (`UPDATE locations WHERE id=$1`) + schedule DELETE (`WHERE location_id=$1 AND id=$2`) are SAFE (no body FK); no schedule UPDATE endpoint. **Read path** (§3.8, track M-menu-sched-read, red-line/operator-gated): forward-only `CREATE OR REPLACE` adds `AND s.location_id = v_location_id` to `read_public_menu` (all 3 subqueries) + self-scopes `product_available_now`; L2-mechanics checklist (signature/DEFINER/DEFAULT/GRANT-verify/forward-only-down/equivalence). **Enumeration** (§3.9): replaced manual convergence with the mechanical FK-sweep. |
| **R2-2** — lint false-positive WALL (~87 forced-UNKNOWN vs budgeted ~15-25) + structurally blind to the FK class | **HIGH** | **FIX (honest re-cost) + ACCEPT (explicit scoping)** | §4B re-cost per the money-council **N4 precedent** (relabel the speed-bump honestly): the "~15-25, `error` day-1" claim is **falsified**. Split into **Tier-A `error` day-1** (SELECT/UPDATE/DELETE predicate proof + dynamic-`${}` fail-closed — the adoptable set the breaker confirmed flaggable) and **Tier-B `warn`→count-floor-ratchet→`error`** (`INSERT…VALUES`, ~60 sites — transitional, not oversold as an instant gate). The **FK-injection class is explicitly OUT of the lint's model** (a correct `location_id` + trusted child FK is invisible to a predicate lint); the **§3.9 mechanical FK-sweep is the named companion gate**. Optional Tier-B INSERT-column-FK heuristic specified (not assumed-covered). Cons re-cost in Option B. |
| **R2-3** — residual matcher evasions (A alias-correlation, B DML-CTE, C views) | **MED** | **FIX (three matcher hardenings)** | §4B(a): **(A)** require ≥1 `location_id = $n` (or `JOIN memberships … = $n`) **server-bound anchor**; alias-to-alias only ⇒ UNKNOWN→allowlist. **(B)** any DML-CTE (`WITH … AS (INSERT/UPDATE/DELETE)`) or multi-statement ⇒ UNKNOWN→allowlist (fail-closed on structural complexity). **(C)** extend the `TENANT_TABLES` manifest generator to enumerate views over tenant base tables (`information_schema.view_table_usage ⋈ TENANT_TABLES`) — mechanical, same SoT. |
| **R2-4** — B4 core signature drops the ADR-0004 `activeLocationId`-verify branch (5/6 clones) | **MED** | **FIX (widen signature)** | §7: core widened to **`resolveOwnerMembership(db, userId, activeLocationId?)`**; implements the two-branch P-d logic verbatim from `get-owner-location.ts:11-25` (baked location → verify against live active owner membership, else `LIMIT 1`). Preserves the insider-removal-window-to-zero property for all 5 clones; menu-import (6th) gains it. Tier-2, onboarding-regression-proven. |
| **R2-5** — provenance fix omits the worker's SECOND audit-insert (`anonymizer-gdpr.ts:74-78`) | **MED** | **FIX (covered-by-implementation)** | §3.3.3: BOTH audit writes (anonymizer `index.ts:291` + worker `anonymizer-gdpr.ts:74-78`) must stamp `location_id = subject's true tenant` + `metadata.{actor_location_id, subject_location_id, request_id}`. **The parallel implementation lane was instructed to fix the worker insert — recorded covered-by-implementation, spec folded into rev-3.** (Post entry-gate fix the two converge for NEW requests; the dual-stamp keeps HISTORICAL/attack rows + STOP-1 query-2 consistent.) |
| **R2-6** — allowlist hash-keys the SQL string, not the call site (adjacent-guard removal keeps green) | **LOW** | **ACCEPT-RISK (mitigated)** | Register §3 below. Mitigations: (a) the standing **rls-adversarial CI sweep** tests the authz *outcome*, not the string — it catches an adjacent-guard removal; (b) allowlist `reason` MUST cite the load-bearing guard (`file:line`), making removal review-visible; (c) prefer folding the guard INTO the statement (INSERT…SELECT/JOIN) so safety is IN the hashed string — which the §2/§3.7 fixes already do. Residual accepted: a byte-identical refactor that removes an *adjacent* guard is caught by rls-adversarial, not the lint. Owner: guardrail maintainer. |

**Additional mechanical-sweep finding (not in R2, surfaced by §3.9):** `products.ts:432` UPDATE
`category_id = COALESCE($7, category_id)` binds body `category_id` unverified → **DEFER-FLAG, LOW**. Writes
only the caller's OWN product (`WHERE id AND location_id=$2`); a foreign `category_id` merely drops that
product from the caller's own storefront (categories join is location-scoped) — self-foot-gun, **no
cross-tenant read or write**. Companion hygiene predicate (`EXISTS(categories WHERE id=$7 AND
location_id=$2)`) recommended; tracked, NOT a ship-blocker.

## 2. Verdict on the re-attack's round-1 re-litigation

| R2 verdict on round-1 finding | Resolution |
|---|---|
| B1 **STILL-OPEN** (15th site falsifies "complete surface") | **CLOSED-in-design** by §3.7/§3.8 (#15 fix) + §3.9 (mechanical enumeration replaces the falsified premise). |
| B2 **PARTIALLY-FIXED** (post-FROM works, FK-class blind) | **Honestly re-scoped** (R2-2): lint covers the predicate class (Tier-A/B), FK-sweep is the companion gate — the two together are the class-killer, not the lint alone. |
| B3 FIXED-IN-DESIGN, B4 MOSTLY-FIXED, B6 CONFIRMED-FIXED | Unchanged; B4 residual closed by R2-4 (signature widen). |
| 8 CONFIRMED-FIXED items (LC2, LC5 entry/sink, F3/F4, products #5-#10, modifier-groups #11, categories oracle #12, forensic claim) | **Do not re-litigate** — breaker-ratified; frozen in `resolution.md §2`. |

## 3. What the mechanical FK-sweep found beyond site #15 (the completeness artifact)

Method (§3.9): FK manifest (child column → sibling tenant parent, from migrations / `information_schema`)
× write-site trust classification. **Structural result: every child→sibling FK is plain single-column —
no parent has `UNIQUE(id, location_id)`, so no composite FK is expressible; the schema cannot self-defend,
the ownership predicate MUST live at each write site.** That is precisely why a `location_id`-predicate lint
is blind to this class.

- **VULNERABLE (live cross-tenant):** exactly **1** — `menu-availability.ts:115` = **#15** (→ §3.7/§3.8).
- **NEW mechanical hit, triaged LOW:** `products.ts:432` `category_id` (self-scoped, no cross-tenant harm).
- **Already fixed (frozen §2):** product_translations (#5-#7), product_modifier_groups (#8-#10),
  modifiers.group_id (#11).
- **SAFE by construction (reason recorded):** `menu-translate.ts` (FK ids re-derived from `WHERE
  location_id=$1`; body filter only narrows); `dashboard.ts:331/343` courier_assignments (both FKs
  pre-verified same-location at `:230`/`:241`); `order-persistence.ts` (server-authoritative pricing +
  same-tx order); system dispatch (server-derived); `menu-import.ts` (same-tx external_key rows).
- **Out of scope:** seed/dev-only inserts.

**No second live cross-tenant IDOR exists in the child-FK class.** The completeness claim now rests on this
reproducible join, and the sweep is the standing companion gate for R2-2's lint blind spot.

## 4. Updated red→green proof list (delta over `resolution.md §3`)

Additive proofs the conductor should re-attack (frozen §2 proofs unchanged):

1. **#15 writer (§3.7, code-only):** owner-A `POST …/{A}/menu-schedules {product_id:<B_product>,
   available:false}` ⇒ **404**, **no `menu_schedules` row** (bypass-pool ground truth); same for
   `{category_id:<B_category>}`. Legitimate same-tenant schedule ⇒ 201. Kitchen-busy PATCH + schedule
   DELETE unchanged (200/204).
2. **#15 read path (§3.8, migration):** pre-seed a cross-tenant `menu_schedules` row (A's `location_id`,
   B's `product_id`, `available:false`); tenant-B `/s/:slug` (`read_public_menu`) **still renders B's
   product**. Equivalence fixture asserts honest menus render byte-identically pre/post (extends
   `packages/db/tests/read-public-menu-availability-equivalence.sql`). RED before predicate, GREEN after.
3. **Mechanical FK-sweep (§3.9):** the sweep output is the checked-in completeness artifact; CI re-run on a
   schema/route change that adds an unverified child-FK write ⇒ a new VULNERABLE row (companion gate).
4. **Lint Tier-A `error` (R2-2):** MUST flag `orders.ts:863`, `anonymizer:119/133/148/196/210`,
   `compute.ts:86`, `products.ts:239/308`; post-fix + allowlist ⇒ 0 Tier-A errors. **Tier-B `warn`/ratchet:**
   warn-flags `products.ts:212/286`, `modifier-groups.ts:157`, `menu-availability.ts:115` at the floor.
5. **Lint R2-3 self-suite:** `$n`-anchor MUST-flag (alias-only) · DML-CTE MUST-flag · view-manifest hit ·
   column-list decoy MUST-flag · `INSERT…SELECT` pass · dynamic-`${}` MUST-flag.
6. **Resolver core widen (R2-4):** a multi-membership owner with a stale/removed baked `activeLocationId`
   ⇒ core returns `null` (P-d verify branch), NOT a different location via `LIMIT 1`; fresh-owner
   onboarding contracts (`{id:null}` 200) preserved.
7. **Worker provenance (R2-5, covered-by-implementation):** a legitimate GDPR erasure ⇒ BOTH audit rows
   (anonymizer + worker) carry the subject's true tenant in `location_id` and `metadata.actor_location_id`;
   STOP-1 query-2 returns them consistently.
8. **IDOR_TABLES** extended to include `menu_schedules` (on top of the resolution.md §3.6 additions).

Each guardrail → a `docs/regressions/REGRESSION-LEDGER.md` row with the red→green citation.

## 5. Explicit ACCEPT-RISK / DEFER-FLAG register (for the human's eyes)

1. **R2-6 (ACCEPT-RISK):** allowlist attests "this SQL string was audited once," not "this call site is
   still safe." An adjacent-guard removal that leaves the SQL byte-identical keeps the green. Mitigated by
   the rls-adversarial behavioral sweep + `reason`-cites-guard + fold-guard-into-statement; residual owner:
   guardrail maintainer.
2. **`products.ts:432` `category_id` (DEFER-FLAG, LOW):** unverified body child-FK on an UPDATE; blast
   radius self-scoped (no cross-tenant harm). Companion hygiene predicate recommended; not a ship-blocker.
3. **Lint Tier-B (ACCEPT, transitional):** `INSERT…VALUES` is `warn`/ratchet, not `error` day-1 — a NEW
   unscoped insert warns (does not wall CI) until the floor burns down. Honestly labeled; the FK-sweep is
   the hard gate for the exploitable subset.
4. **Rev-2 register carried forward:** B5 shared-courier deactivation (until M-F3); B7 dead dry-run branch;
   lint FN residue (SQL built outside the call site).

## 6. What remains open for round 3 / operator

- **#15 read-path migration (§3.8) is operator-gated (red-line).** Needs staged-DB apply + equivalence
  proof before prod; the writer fix (§3.7) closes NEW injections independently, so the two are not
  co-blocking, but the operator must schedule the migration to cover historically-injected rows.
- **STOP-1 (carried from round 1):** the forensic "was LC5 exploited?" decision is still a needs-human
  ship-gate; #15 adds a parallel question — **"was #15 exploited?"** — answerable by scanning
  `menu_schedules s JOIN products p ON p.id=s.product_id WHERE p.location_id <> s.location_id` (and the
  category variant) before the read-path migration lands. Operator records outcome (folds into the STOP-1
  record).
- **Lint Tier-B floor calibration** (~60 `INSERT…VALUES`) is a measured number to lock at implementation
  time; the ~87 total is the breaker's count — implementation must record the exact floor.
- **`products.ts:432` hygiene predicate** — accept-as-LOW now, or fold into the frozen batch as a 15th
  code fix? Recommend: track separately (LOW), do not expand the frozen lane.

*Not self-certified. Round-3 re-attack targets: #15 writer 404 + read-path exclusion, the mechanical
FK-sweep's SAFE-by-construction classifications (esp. order-persistence server-authoritative + dashboard
pre-verify), and the lint Tier-A/Tier-B severity split.*
