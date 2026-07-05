# Breaker findings R2 — Council RE-ATTACK on the REVISED authz fix (rev 2)

Status: **BREAKER regression round (adversarial re-attack)** — proving whether rev-2 killed v1's
CRITICAL/HIGHs AND introduced no new break. Date: 2026-07-03. Grounded line-by-line against LIVE
source + migrations (not the proposal's own citations). No fixes proposed. Inputs: `proposal.md`
(rev 2), `resolution.md`, `breaker-findings.md` (round 1). **Note: rev 2 is DESIGN-ONLY — the v2
lint, `TENANT_TABLES` constant, allowlist, and hook-rule v2 are UNBUILT** (verified: no
`no-unscoped-tenant-query` / `tenant-scope-allowlist` / `TENANT_TABLES` anywhere outside
`node_modules`). So B2/B3 are judged as *specs*, not code.

---

## 6-line summary
1. Rev-2's point-fixes for the 14 enumerated sites are individually SOUND (verified each against source) — LC2, LC5 (entry+sink+fallback-delete), F3/F4, F5, all products/categories subroutes scope correctly, incl. the destructive `products.ts:282` DELETE (double-guarded) and the per-id `INSERT…SELECT` FK checks.
2. **But the enumeration is NOT complete: a 15th LIVE cross-tenant IDOR exists** — `menu-availability.ts:113` INSERT `menu_schedules` with an unverified body `product_id/category_id`, amplified by `read_public_menu` scanning `menu_schedules` with NO `location_id` predicate → an owner of A hides/rewrites ANY other tenant's products on that tenant's **live public storefront**. Neither the batch nor the class-killer lint fixes it.
3. B2 half-holds: the redesigned post-FROM matcher DOES now flag the v1 decoys `orders.ts:863` + `anonymizer:119` (column-list strip works), but it is structurally blind to the unverified-child-FK class (#9/#11/**#15**) — that bug carries a correct `location_id`, so a `location_id`-predicate lint never sees it.
4. The fail-closed lint creates a false-positive wall far larger than budgeted: **~60 `INSERT…VALUES` + ~27 dynamic-`${}` = ~87 forced-UNKNOWN allowlist entries** vs the proposal's "~15-25" — so "error from day 1, no warn window" either mass-allowlists (seam erosion) or wall-fails CI.
5. B6 fallback deletion is complete and safe (both live callers thread scope); B3 hook-v2 is feasible-in-design; B4 preserves the 3 null→200 + `{locId,userId}` contracts via adapters, but the core signature as literally written drops the ADR-0004 `activeLocationId`-verify branch (5/6 clones).
6. Forensic claim is CORRECT (`customers.location_id` survives on ALL paths — no hard-delete, no `location_id` mutation), but the provenance fix's enumeration omits the worker's SECOND audit-insert (`anonymizer-gdpr.ts:75-78`), leaving two audit rows per erasure stamped with DIFFERENT tenants post-fix.

## Count by severity
- **CRITICAL: 1** (R2-1 — the 15th live site; same class B1 rated CRIT, still unfixed)
- **HIGH: 1** (R2-2 — lint false-positive wall + structural FK-blindness undermine the class-killer)
- **MEDIUM: 3** (R2-3 lint evasions, R2-4 B4 activeLocationId regression, R2-5 provenance/forensic gap)
- **LOW: 1** (R2-6 allowlist hash-key staleness)
- **CONFIRMED-FIXED (survive re-attack): 8** (see §"What is now sound")

## Verdict on the round-1 findings
| ID | Round-1 sev | R2 verdict | Basis (verified) |
|----|------------|-----------|------------------|
| **B1** — live IDORs left unfixed | CRIT | **STILL-OPEN** | The 14 listed fixes each scope correctly, but the *premise* ("complete, enumerated live surface") is **FALSE** — 15th live site found (R2-1). Both round-1 "two passes converge / enumeration complete" AND proposal §5's completeness claim are falsified. |
| **B2** — lint can't flag live sites | HIGH | **PARTIALLY-FIXED** | The specific v1 blind spots ARE fixed: post-FROM/INTO strip flags `orders.ts:863` + `anonymizer:119` (column-list `location_id` no longer satisfies). BUT residual evasions remain (R2-3) AND the lint is blind by-design to the unverified-FK class that includes the un-enumerated #15 (R2-2). Load-bearing claim only half-true. |
| **B3** — hook rule can't reach 0 | HIGH | **FIXED-IN-DESIGN (unbuilt)** | v2 matcher (CallExpression + preValidation-array + per-route) is feasible and would green the 10 array-shape files + couriers/courier-invites post-fix. Cannot be proven — nothing is implemented. No new break introduced by the spec. |
| **B4** — resolver collapse regresses onboarding | HIGH | **MOSTLY-FIXED** | The 3 spa-proxy null→200 callers (`settings {id:null}` :668-676, invite `pending:true` :742-752, onboarding `bootstrap_owner` :762-788) and the `{locId,userId}` callers ARE preservable by per-caller adapters. Residual: core `resolveOwnerMembership(db,userId)` as written DROPS the ADR-0004 `activeLocationId`-verify branch present in 5/6 clones (R2-4). Deferred to Tier-2. |
| **B6** — sink predicate is tautological unless fallback deleted | MED | **CONFIRMED-FIXED** | `|| row.location_id` fallback lines verified at `index.ts:131` + `:208`; deleting them + require-scope-or-throw is safe — BOTH live callers already thread scope (gdpr worker `anonymizer-gdpr.ts:64` = request loc; retention batch loop `index.ts:89/:97` = each customer's own loc). Fail-closed throw never fires in production; entry gate (`gdpr.ts:48`) is the real layer. |

---

## CRITICAL

### R2-1 — 15th LIVE cross-tenant IDOR: `menu_schedules` storefront-availability tampering (enumeration INCOMPLETE)
**Severity: CRITICAL** (same class B1 rated CRIT: a live, unfixed cross-tenant write). **Invariant: no cross-tenant write on a tenant table.**

Verified end-to-end against source + two migrations:
- **Writer** — `routes/owner/menu-availability.ts:113-123` POST `/api/owner/locations/:locationId/menu-schedules`:
  `INSERT INTO menu_schedules (location_id, product_id, category_id, …) VALUES [locationId, b.product_id ?? null, b.category_id ?? null, …]`.
  `requireLocationAccess` verifies membership in the URL `:locationId` ONLY; body `product_id`/`category_id`
  (`z.string().uuid()`, no ownership check) are trusted verbatim. The `:107-111` precondition enforces
  only "exactly one of product/category," not ownership.
- **DB accepts it** — migration `1790000000062_menu-schedules.ts:28-29`: PLAIN FKs
  `product_id uuid REFERENCES products(id)` / `category_id REFERENCES categories(id)` — NOT composite
  `(id, location_id)`. RLS `WITH CHECK (location_id IN app_member_location_ids())` (`:59`) validates only
  the attacker's OWN `location_id`; the FK existence check bypasses RLS. So owner-A commits
  `{location_id: A, product_id: <B's product>, available: false, mode:'daily', 0..1440}` → 201.
- **Read amplifier makes it LIVE (not inert)** — the public storefront reader
  `1790000000064_read-public-menu-perf.ts:139-142`:
  `AND NOT EXISTS (SELECT 1 FROM menu_schedules s WHERE (s.product_id = p.id OR s.category_id = p.category_id) AND s.available = false AND menu_schedule_matches(…))`
  — **NO `s.location_id` predicate**, under `public_select … USING (true)` (062:63, world-readable). The
  plpgsql sibling `product_available_now` (062:123 `WHERE product_id = p_product_id OR category_id = p_category_id`)
  is equally unscoped. So when tenant B's `/s/:slug` renders, the scan sees the attacker's row → B's
  product is filtered OUT of B's live menu.

**Impact:** any owner can force ANY product/category of ANY other tenant to disappear from (or have its
availability windows rewritten on) the victim's live public storefront — cross-tenant integrity + an
availability DoS. UUID-unguessability does NOT mitigate: victim product/category ids are returned
verbatim in the public `read_public_menu` JSON, so the attacker harvests targets from the victim's own
`/s/:slug`. **This overturns round-1's "menu-availability is LATENT / the foreign FK is inert" —
inertness was asserted without checking the read path.** It is the SAME unverified-child-FK class the
batch DID point-fix at products #9 / modifier-groups #11, but at a site nobody enumerated. The batch
ships without fixing it, and (per R2-2) the class-killer lint would not catch it either.

---

## HIGH

### R2-2 — the class-killer lint has a false-positive WALL and is blind to the FK-injection class
**Severity: HIGH.** **Invariant: a guardrail must be adoptable at its stated severity AND catch the class it exists for.**

Two independent defeats of the v2 seam's value proposition, both verified:

1. **Fail-closed forces ~87 allowlist entries, not "~15-25."** The v2 rule triggers on any `.query()`
   referencing a `TENANT_TABLES` table (SoT = the ~50 FORCE-RLS tables, verified 30+ migrations). Rules:
   `INSERT…VALUES` on a tenant table is UNKNOWN-by-construction → allowlist; any `${…}`/non-literal SQL →
   UNKNOWN → allowlist. Measured against source (`apps/api/src/{routes,lib}`):
   **60 `INSERT INTO <tenant-table>…VALUES` sites + 27 dynamic-`${}` sites = ~87 forced-UNKNOWN entries**
   (before counting SAFE SELECT/UPDATE/DELETE scoped by a *preceding* ownership check rather than an inline
   predicate — those also flag). The proposal §4B budgets "~15-25 dynamic-SQL entries" — an under-count of
   4-6× because it omits the entire safe-`INSERT…VALUES` population. Consequence: "**error from day 1, no
   warn window**" (resolution B2) is not achievable without either a ~90-entry hand-audited allowlist (itself
   the rubber-stamp/seam-erosion surface the design fears, and a high count-floor the ratchet then locks in)
   OR a wave of ~87 build-breaking false positives. The load-bearing "no live-IDOR grace window" claim is
   not deliverable as scoped.
2. **The lint is structurally blind to the FK-injection class.** The v2 model proves a *`location_id`
   predicate*. But #9/#11/#15's bug is a correct `location_id` + an UNVERIFIED child FK
   (`group_id`/`product_id`/`category_id`) to a *different* tenant table. The INSERT carries `location_id`
   → it is UNKNOWN → allowlisted, and the auditor will (correctly, per the lint's own model) wave it through
   as "location_id scoped." So the lint neither flags #15 as a bug NOR would have prevented #9/#11. The seam
   catches "forgot the `location_id` predicate," never "trusted a child FK" — the exact class of half the
   enumerated live sites.

Net: the specific B2 red→green targets (`orders.ts:863`, `anonymizer:119`) ARE now flaggable (post-FROM
strip works — confirmed), but the seam is neither adoptable at `error` as budgeted nor able to catch the
FK-injection class that produced #15.

---

## MEDIUM

### R2-3 — residual lint evasions of the post-FROM matcher
**Severity: MEDIUM.** Three evasions survive the v2 "strip-before-first-FROM/INTO + predicate-token" design:
- **(A) table-to-table `location_id` correlation.** The accepted RHS form `location_id = <alias>.location_id`
  matches `WHERE a.location_id = b.location_id` where NEITHER side is bound to a server-verified `$n`. A
  self-join / two-tenant-table correlation with no `= $n` anchor PASSES while being unscoped. (The companion
  fix `AND m.location_id = mg.location_id` at `modifier-groups.ts:62` is only safe because a *sibling*
  `mg.location_id = $1` anchors it — the lint can't tell the anchored case from the unanchored one.)
- **(B) CTE / multi-statement decoy.** "Strip everything before the first `FROM`/`INTO`" mis-locates the
  predicate scope: a `WITH ins AS (INSERT INTO product_translations … VALUES(…)) SELECT … FROM x WHERE
  x.location_id=$1` puts a decoy predicate in the outer statement while the CTE INSERT is unscoped → PASSES.
- **(C) views.** A read `FROM <view>` over a tenant table isn't in `TENANT_TABLES` (generated from FORCE-RLS
  base tables) → the rule never triggers → silent false-negative. (Dynamic table names `FROM ${t}` ARE caught
  by the non-literal fail-closed rule.)

### R2-4 — B4 core signature drops the ADR-0004 `activeLocationId`-verify branch (5/6 clones)
**Severity: MEDIUM** (Tier-2/deferred). The onboarding regression B4 flagged IS fixed: the 3 spa-proxy
null→200 callers depend on separate `isValidOwnerToken`/`getOwnerUserId` probes (`spa-proxy.ts:674, 750, 763`)
that survive the collapse, and the `{locId,userId}` callers (getOwnerContext ×4, product-media getOwnerLocation
×5, `.userId`→withTenant) map faithfully. **Residual:** the core as literally specified —
`resolveOwnerMembership(db, userId) → {locationId,userId}|null` — keys only on `userId`, so it CANNOT verify a
baked `activeLocationId` (the ADR-0004 P-d property: pin/verify THAT location, deny a just-removed owner's
baked location immediately). 5 of 6 clones implement that branch (`get-owner-location.ts:11-18`,
`spa-proxy getLocationId:70-77` / `getOwnerContext:134-141`, `product-media:51-56`, `promotions:19-26`); only
menu-import lacks it. A `(db,userId)`-only core silently downgrades to `LIMIT 1` and can return a *different*
location for a multi-membership owner. The signature must widen to carry `activeLocationId`, or the
"contract-preserving adapter" claim regresses a security property for 5/6 callers.

### R2-5 — provenance/forensic fix omits the worker's SECOND audit-insert; two rows, two tenants
**Severity: MEDIUM.** The forensic claim itself is CORRECT (see §"Sound"). But two gaps:
- There are TWO audit-log writes per GDPR erasure: the anonymizer's (`index.ts:291`, stamped
  `locationId = subject?.locationId || row.location_id`) AND a SEPARATE worker insert
  (`anonymizer-gdpr.ts:75-78`, stamped `row.location_id` = the *request* row's tenant). Proposal §3.3.3's
  provenance fix enumerates ONLY `anonymizer/index.ts`. Post-fix it wants the anonymizer's audit `location_id`
  to become the *subject's true tenant* (the locked customer row), but the worker's row stays the requester's
  tenant → the two audit rows for one erasure would carry DIFFERENT `location_id`s, and the "subject's true
  tenant" provenance is only half-applied.
- The attempt-log classifier (§3.3.1: on gate-miss, look up the foreign customer to classify
  `cross_tenant_attempt`) is itself a NEW unscoped `customers` read → under the v2 lint it is another forced
  allowlist entry, and the audited "reason" would (correctly per the lint model) pass it — folding into R2-2's
  wall.

---

## LOW

### R2-6 — allowlist hash-keying certifies the SQL string, not the call site
**Severity: LOW.** The allowlist is `{path, normalized-SQL-hash, reason, verified-by}`; an entry is invalidated
only by a text change to the query. Safety of an allowlisted `… WHERE id=$1` often rests on a *preceding*
ownership SELECT in the surrounding code (many of the "~56 SAFE" sites). A refactor that REMOVES that guard but
leaves the SQL text byte-identical keeps the hash — and the green — so a genuine authz regression sails through
the seam. The allowlist attests "this string was audited once," not "this call site is still safe."

---

## What is now SOUND (survives re-attack — do not re-litigate)
1. **LC2** `orders.ts:863` → membership-JOIN fix mirrors the shipped GET sibling (`:747-756`, verified: `JOIN
   memberships m … WHERE o.id=$1 AND m.user_id=$2 AND m.role='owner' AND m.status='active'`, 0 rows → null/404).
2. **LC5 entry gate** `gdpr.ts:48` — the direct-`customerId` branch is the real load-bearing control; same-tenant
   proof + 404 is sound. **Sink + B6 fallback deletion** `index.ts:131/:208` complete; both live callers thread
   scope; require-or-throw never fires in prod.
3. **F3/F4** `couriers.ts:14-15` / `courier-invites.ts:20-21` genuinely lack `requireRole`; adding it (+ F4
   `role` allow-list) closes the customer-reads-roster / non-owner-mints-invite vectors. Verified GET returns
   `decryptPII` (`couriers.ts:44-46`), role taken verbatim (`courier-invites.ts:30`).
4. **F5** `compute.ts:85-88` (`WHERE id=$1`, no `location_id` even in the column list) → `AND location_id=$2` +
   thread `signals.ts` param — sound.
5. **Products #5-#10** — each folds ownership: translations INSERT…SELECT / JOIN / DELETE…USING;
   **destructive `products.ts:282` DELETE** double-guarded (same-tx `SELECT 1 FROM products WHERE id=$ AND
   location_id=$` pre-check → 404, PLUS `AND location_id=$` on the DELETE; the injected row can't carry a foreign
   loc since the sibling INSERT `:286` binds it); **INSERT `:285-289`** → `INSERT…SELECT FROM modifier_groups
   WHERE mg.id=$ AND mg.location_id=$` verifies EACH `group_id` per-iteration (the `$4` double-bind makes the
   inserted loc == the checked loc — no bypass).
6. **modifier-groups #11** `:156` FK fix + companion `:62` count-join `AND m.location_id` — sound; B8 downgrade
   correct (pricing joins `orders.ts:225/:436` both filter `m.location_id=$2` → injected rows filtered from
   checkout — verified).
7. **categories oracle #12** — BOTH sites present and fixed by `AND location_id=$2` on the pre-check SELECT
   (`:163-166` locationId-path variant AND `:244-245` menu-alias JWT variant). Missing one would leave the
   oracle on that path.
8. **Forensic claim (STOP-1)** — `customers.location_id` survives on ALL erasure paths: verified NO
   `DELETE FROM customers` and NO `UPDATE customers SET location_id` anywhere; anonymization only nulls
   phone/name/marketing (`index.ts:133-140`). So forensic queries (1)+(2) are answerable (modulo
   `gdpr_erasure_requests` retention, already stated). **Audit-provenance metadata write has no injection/trust
   issue** — parameterized `$7`, server-derived UUIDs, `reason` not in metadata.

---

## Single most important residual
**R2-1: the un-enumerated 15th live cross-tenant IDOR (`menu_schedules` storefront tampering).** It is a live,
unfixed cross-tenant write today; it falsifies the "complete enumerated surface" premise the entire B1
resolution rests on; and it is exactly the lint-evasion the re-attack was asked to find — the class-killer seam
would NOT stop this or any future FK-injection IDOR, because the vulnerable INSERT carries a correct
`location_id` and the seam only models `location_id` predicates, never the trust placed in a child FK. Fixing it
needs write-side FK ownership verification AND/OR a `s.location_id` predicate in `read_public_menu`/
`product_available_now` (a migration touch — so, like M-F3, not code-only). Recommend: re-open the completeness
claim, add #15 to Tier-1, and add an FK-ownership check (or an INSERT…SELECT…WHERE-in-location fold-in) as a
first-class fix pattern the seam cannot provide.

*Not self-certified. This is a regression-round attack surface, not a ratification.*
