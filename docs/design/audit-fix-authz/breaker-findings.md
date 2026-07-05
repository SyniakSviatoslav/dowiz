# Breaker findings — Audit-fix authz + data-access seam (Council STEP 2 · ATTACK)

Status: **BREAKER (adversarial)** — proving the proposed fixes break / regress / miss. Date: 2026-07-03.
Author: System Breaker (Triadic Council STEP 2). Grounded line-by-line against live source (not the proposal's
own citations). No fixes proposed here — findings only. Inputs: `docs/design/audit-fix-authz/proposal.md`,
`docs/adr/ADR-audit-fix-authz.md`.

Verdict: the four *point* fixes (LC2, F5) are essentially sound; **F3/F4, the two guardrails
(`no-unscoped-tenant-query`, `require-auth-hook`), the resolver-collapse, and the LC5 sink are each
broken, under-specified, or unfixed as written.** Most important: the batch **enumerates ≥5 LIVE
cross-tenant write/read IDORs (products + modifier-groups subroutes) and then does NOT fix them** — they
are handed to a warn-level lint that, as specified, cannot even flag them.

---

## Count by severity
- **CRITICAL: 1** (B1)
- **HIGH: 3** (B2, B3, B4)
- **MEDIUM: 4** (B5, B6, B7, B8)
- **LOW / CONFIRMED-OK: 3** (B9, B10, B11)

---

## CRITICAL

### B1 — The batch enumerates ≥5 LIVE cross-tenant IDORs and leaves them UNFIXED
**Severity: CRITICAL.** **Invariant violated: no cross-tenant read/write on a tenant table.**

The proposal's §5a lists these as `LIVE-IDOR` (client-supplied id, tenant table, no location predicate,
exploitable today):

- `products.ts:211-217` PUT translations — `INSERT INTO product_translations (product_id,…) VALUES ($1=id,…)
  ON CONFLICT … DO UPDATE`; `id` is the URL `:id`, **never verified against `location_id`**. The only guard
  (`:206` `SELECT supported_locales FROM locations WHERE id=$locationId`) validates the *locale*, not the
  product. `product_translations` has **no `location_id` column** → RLS cannot scope it even post-flip.
  → overwrite any tenant's product translation. (verified /root/dowiz/apps/api/src/routes/owner/products.ts:204-218)
- `products.ts:239` GET translations — `SELECT * FROM product_translations WHERE product_id=$1`. Foreign read.
- `products.ts:256` DELETE translations — `DELETE … WHERE product_id=$1 AND locale=$2`. Foreign delete.
- `products.ts:282` **DELETE `FROM product_modifier_groups WHERE product_id=$1`** — **destructive
  cross-tenant write**: wipes a competitor's product→modifier-group links with **no location guard at all**
  (the sibling INSERT `:286` sets `location_id`, the DELETE does not). (verified products.ts:281-291)
- `products.ts:307-313` GET product modifier-groups — `WHERE pmg.product_id=$1`. Foreign read.
- `modifier-groups.ts:156-160` POST modifiers — `INSERT INTO modifiers (location_id,group_id,…) VALUES
  ($1=locationId,$2=groupId,…)`; `groupId` unverified (see B8 for the corrected severity).

**Now read §3 (the fixes) and §8.1 (Tier-1 point fixes): the point-fix set is `LC2, F3, F4, LC5, F5`.**
None of the six sites above are in it. Their only "coverage" is the `no-unscoped-tenant-query` lint —
which is (a) shipped at **warn**, so it neither blocks CI nor fixes anything, and (b) unable to flag them at
all (B2). So the batch ships with at least one **destructive** cross-tenant write (`products.ts:282`) and
five other cross-tenant read/writes **still live**. The proposal found the class and then treated
enumeration as remediation. This is the headline gap: a fix batch that closes 4 findings while knowingly
shipping ≥5 others of equal class.

---

## HIGH

### B2 — `no-unscoped-tenant-query` lint fails its OWN red→green proof and misses every LIVE site
**Severity: HIGH.** **Invariant violated: a guardrail must go RED on the bug it exists to catch (no cheat-green).**

The lint (proposal §4B) flags a `.query(\`…\`)` that "hits a known tenant table and lacks a tenant predicate
token (`location_id`, a `JOIN memberships`, `courier_id`, `customer_id = $`) … unless the call is inside a
`withTenant(...)` callback." Three independent defeats, each verified against source:

1. **Bare `location_id` token matches the SELECT/INSERT *column list*, not a predicate.** §6 claims the lint
   "must flag `orders.ts:862`, `compute.ts:86`, `anonymizer/index.ts:119`." But:
   - `orders.ts:863` = `SELECT id, status, location_id, type FROM orders WHERE id=$1` — **contains
     `location_id`** (a selected column) → token present → **NOT flagged**.
   - `anonymizer/index.ts:119` = `SELECT anonymized_at, location_id FROM customers WHERE id=$1 FOR UPDATE` —
     **contains `location_id`** → **NOT flagged**.
   Only `compute.ts:86` (`SELECT no_show_count, completed_count, last_no_show_at … WHERE id=$1`, no
   `location_id`) would flag. **Two of the three §6 red→green targets are unflaggable by the token set the
   proposal specifies** → the proof plan is internally inconsistent. Every `INSERT` that lists `location_id`
   as a column (modifier-groups:157, products:286) is likewise auto-exempt.
2. **The `withTenant` exemption whitelists exactly the LIVE-under-BYPASSRLS sites.** Every §5a LIVE site
   (modifier INSERT, all products subroutes, gdpr, signals) runs **inside a `withTenant(...)` callback**. The
   proposal's own §1 premise is that under the deployed **BYPASSRLS** pool `withTenant` "protects nothing."
   So the exemption exempts precisely the queries that are live. `withTenant` seats a GUC for RLS; it does
   **not** validate a child FK (`group_id`, `product_id`) — the exact defect in B1.
3. **Interpolated-clause blindness → false pos AND false neg.** `gdpr.ts:117`, `signals.ts:36-81`,
   `promotions.ts:56-65` build the WHERE in a *separate* `clauses`/`setClauses` string and interpolate
   `${clauses}`. The predicate text is **not in the `.query()` template literal** the lint parses. Treat
   `${…}` as "unknown" → false-positive on safe queries (escape-hatch spam, → the very author-friction that
   makes the escape hatch a rubber stamp); treat it as safe → false-negative on a genuinely unscoped
   interpolated clause. Dynamic SQL (the `setClauses.join(', ')` UPDATEs at modifier-groups:108/201) is
   invisible either way.

Net: the lint would flag ~1 of the ~6 live sites, fail its documented red→green, and be trivially evaded by
the escape hatch it needs constantly because of (3). It is not a class-killer; it is a warn-level smell with
a hole shaped like the bug.

### B3 — `require-auth-hook` tightening cannot detect its own fix and cannot burn down to error
**Severity: HIGH.** **Invariant violated: the gate must recognize the corrected code and reach 0 to ratchet.**

Verified against the rule (`tools/eslint-plugin-local/src/index.js:228-272`) and an auth-pattern census of all
27 owner files:

1. **The matcher only sees a bare `Identifier` named `verifyAuth`/`requireRole` in `addHook`** (`:245-246`).
   But `requireRole` is a **factory** — the fix adds `requireRole(['owner'])` (a `CallExpression`) or
   `fastify.requireRole(['owner'])` (a `MemberExpression` call, as gdpr.ts:29 already does). Neither is a bare
   Identifier → **the AND-tightened rule would not detect the fix's own hook** on couriers.ts /
   courier-invites.ts → the file never turns green.
2. **10 of 27 owner files carry auth ONLY via per-route `preValidation: [server.verifyAuth,
   server.requireRole(['owner']), …]` arrays** (categories, locations, menu-availability, menu-confirm,
   menu-import, menu-translate, modifier-groups, product-media, products, promotions) — **no `addHook`, no
   `register`-with-preHandler**, the only two shapes the current matcher inspects. They are already invisible
   to the rule. Ratcheting to `error` makes all 10 permanent false-positives → forces either a large
   allow-list (author friction, and the allow-list itself becomes the bypass) or a full matcher rewrite the
   proposal doesn't scope.
3. **File-level granularity ≠ route-level.** The rule reports once per file at `Program:exit`. A multi-route
   file where a *single* route omits `requireRole` still passes if any other route has it. That is the exact
   shape of LC2 (one route in a multi-route file missing its guard) — the tightening does not close it. And
   `require-auth-hook` scopes to `/routes/owner/` + `/routes/courier/` only (`:235-237`), so the LC2 route
   (`routes/orders.ts`) is out of scope entirely.

### B4 — Resolver-collapse (Tier-2) breaks first-run onboarding and drops `userId` — not just the flagged 500→401
**Severity: HIGH.** **Invariant violated: behavior-preservation for legitimate callers.**

The ADR flags only OR-2 (`promotions.ts` authz 500→401). The collapse to one `getOwnerLocationId` returning
`string|null` with a "single `null → sendError(401)` convention" is more dangerous than admitted. Enumerated
+ verified (5 clones; callers traced; global `setErrorHandler` at server.ts:443 confirms a plain `throw`
genuinely yields 500 + Sentry capture):

- **`spa-proxy.ts` `getLocationId` (clone 3) has 3 callers that deliberately map `null → 200`, not 401**
  (firsthand-verified spa-proxy.ts:668-676): `/api/owner/settings` returns **`{id:null}`** so a fresh owner
  routes to `/admin/onboarding` (the comment explicitly warns 401 here bounces first-run to `/login` — "O1");
  the courier-invite path (`:743`) returns a **200 `pending:true`** placeholder; the storefront upsert
  (`:762`) **falls through to `bootstrap_owner` provisioning**. A blanket `null→401` unification **regresses
  onboarding**.
- **Clones 4 & 5 (`spa-proxy getOwnerContext`, `product-media getOwnerLocation`) return
  `{locId, userId}`** and their 9 callers consume `ctx.userId` for `withTenant`. A `string|null` canonical
  resolver **drops `userId`** — not a shape-preserving swap.
- **The spa-proxy clones authenticate off the raw `Authorization` header** (`request.user` is unpopulated in
  the catch-all proxy). The canonical `getOwnerLocationId(request, db)` reads `request.user` → **would not
  resolve at all** if swapped in there.

So "collapse the 5 clones" is safe for clone 1 only; for the other 4 it changes status/envelope/logging/Sentry
(promotions), regresses onboarding to 401 (spa-proxy ×3), or drops a return field callers depend on
(spa-proxy, product-media). The proposal treats this as trivial Tier-2 debt-paydown.

---

## MEDIUM

### B5 — F3 defense-in-depth ("scope courier mutations to `courier_locations`") is NOT code-only — contradicts ADR §7
**Severity: MEDIUM.** **Invariant violated: stated migration/contract impact ("code-only, no schema change").**

`couriers.status` is a **global** column on the identity table (`packages/db/migrations/1780421029538_couriers.ts:12`,
`CHECK (status IN ('active','deactivated','suspended'))`). `courier_locations` has **no status/active column**
(`:21-27` — PK `(courier_id, location_id)`, only a `role`). So `owner/couriers.ts:99-102`
`UPDATE couriers SET status='deactivated' WHERE id=$courierId` **necessarily deactivates the courier for
every location** (and `:112-115` revokes ALL their sessions). Proposal §3.2 / ADR OR-3 want this "scoped to
`courier_locations`, not the global row" — but per-location deactivation requires a **new column / migration**,
which ADR §7 explicitly excludes ("the four point fixes are code-only — no schema change"). It is one or the
other: either the shared-courier cross-tenant deactivation side-effect stays **live**, or the batch needs a
migration it says it doesn't. OR-3's "design says no [A shouldn't affect B]" is not achievable as scoped.

### B6 — LC5 anonymizer sink `AND location_id=$2` is a tautology — real protection is the entry gate, not the "second layer"
**Severity: MEDIUM.** **Invariant violated: "two independent layers" (the sink is not independent).**

`anonymizer/index.ts:131`: `const locationId = options.subject?.locationId || row.location_id`. The proposed
sink predicate `UPDATE customers … WHERE id=$1 AND location_id=$2` uses this `locationId`. Two live callers:
- **GDPR** (`workers/anonymizer-gdpr.ts:64`) threads `row.location_id` = the *request's* location → the
  predicate IS meaningful **only because the worker supplies it**, not because the sink self-protects.
- **Retention** (`index.ts:89`) threads **each customer's OWN `location_id`** → the predicate is a **pure
  tautology** (`WHERE location_id = <its own location>`), providing zero isolation. (This is the legitimate
  cross-location system path — adding the predicate does **not** break it, but it also does nothing.)

The landmine: the `|| row.location_id` fallback means **any future caller that omits `subject.locationId`
silently self-satisfies the predicate** (`AND location_id = <the row's own>`) → PII erased anyway. The
proposal's claim that the sink alone means "even a poisoned queue row cannot cross tenants" is false unless
the fallback is **deleted** — which §3.3 does not say. The load-bearing fix is the **entry gate** (§3.3.1);
the sink is sold as an independent second layer but is inert-or-tautological in both live paths. Present it
as belt-only, not belt-and-suspenders, or delete the fallback.

### B7 — LC5 sink enumeration includes dead code (`:57`/`:64` dry-run has zero callers)
**Severity: MEDIUM (accuracy).** §3.3 lists `:57`/`:64` (the dry-run existence oracle) among the sink sites
to scope. `grep` shows `dryRun:true` is **never passed to `anonymize()` anywhere** (only referenced inside
`anonymizer/index.ts` itself + an unrelated `scripts/restore.ts`). The entire `if (options.dryRun)` branch
(`:55-70`) is unreachable in production. Scoping it is harmless but pointless, and signals the sink-site
enumeration was not reachability-validated — undercutting the "cross-plane sweep found the unscoped set is
{:57,:64,…}" rigor claim.

### B8 — modifier-groups.ts:156 severity is overstated (does NOT reach checkout pricing)
**Severity: MEDIUM (severity-inflation).** §5a claims the injected modifier "surfaces … in **order-pricing
joins** (`orders.ts:223/434`) → cross-tenant menu/price injection." Verified: **both** pricing joins filter
`m.location_id = $2` (order's location) — `orders.ts:225` and `orders.ts:436`. An injected modifier carries
`location_id = attacker` (the INSERT hardcodes the caller's own `locationId`), so it is **filtered OUT** of
the victim's checkout. The real cross-tenant surface is limited to owner-GET `modifier_count` inflation
(`modifier-groups.ts:60-64` joins `m.group_id = mg.id` with no `m.location_id`) and possibly the public
storefront (unverified). Still a real cross-tenant write, but "cross-tenant price injection into orders" is
not demonstrable — the finding's severity rationale is wrong even though the write is real.

---

## LOW / CONFIRMED-OK (the fixes that survive attack — recorded so the council knows what NOT to re-litigate)

### B9 — LC2 JOIN fix: sound on the probed axes
No 404-vs-403 existence leak: `orders.ts:862` fix returns **404 for both nonexistent and cross-tenant**
(`throw {statusCode:404}` on 0 rows), mirroring the shipped GET sibling (`:758` → 404) — no existence oracle.
`assertOwnerTargetAllowed`'s 403 runs only AFTER the JOIN confirms ownership, so the 403 never leaks foreign
existence. Covers the transition: `locationId` for `updateOrderStatus`/`attemptHonestDispatch` is taken from
the **JOIN-verified** row (`:871`), so the mutation inherits the boundary. Legit multi-location owners resolve
(JOIN matches any active owner membership at the order's location). Caveat: the class-killer guardrail does
**not** cover `routes/orders.ts` (B3.3 scope), so a future unscoped mutation there is ungated.

### B10 — F5 fix: sound
`compute.ts:86` add `AND location_id=$2` + thread `params.locationId`; `signals.ts:118` already passes the
`requireLocationAccess`-verified `:locationId`. Foreign `customer_id` → 0 rows → empty signals. Closes F5.

### B11 — gdpr.ts already has the role gate; F3/F4 scope is correct
`owner/gdpr.ts:29` already runs `requireRole(['owner'])`. So the missing-role class is genuinely only
`couriers.ts` + `courier-invites.ts` (as the proposal states). F3's "customer reads decrypted roster" is real
(`couriers.ts:18` GET is behind file-level `requireLocationAccess` but no role gate; a customer with
`JWT.locationId==L` passes `requireLocationAccess`). Adding `requireRole` before `requireLocationAccess`
closes the primary vector; the F4 `role` allow-list is additive. (The defense-in-depth half is B5.)

---

## MISSED LIVE IDOR sites (headline deliverable): **NONE — the §5a enumeration is complete**

Two independent passes converge: (a) a fresh line-by-line adversarial re-sweep of **all 27**
`routes/owner/*.ts` files + the shared resolvers + the `withTenant` primitive + the RLS migration layer, and
(b) my own reads of the highest-risk files. **Neither found a single owner-plane site the audit marked SAFE
(or omitted) that is actually a live cross-tenant IDOR.** Every route the audit cleared binds each
tenant-table query to a `location_id = <server-verified>` predicate, a `getOwnerLocationId`/`getLocationId`
membership resolution, or a preceding same-tx ownership SELECT. The nearest miss-candidates are each
disqualified:

- `couriers.ts:100/113` global `couriers`/`courier_sessions` UPDATE — **membership-gated** (the `:89-92`
  `courier_locations` check blocks any cross-location target), so a cross-tenant *side-effect* (B5), not a
  param-injection IDOR.
- `dashboard.ts:556-582` `/verify` sibling reads by `order_id` with no location predicate — **output-gated**
  by the `:585` `orderRes.rowCount===0 → 404` (discarded for foreign orders). Latent, not live.
- `menu-translate.ts` — **fails closed** (`requireLocationAccess` reads `params.locationId` but the route
  param is `:id` → always 400; audit F10), and every entity read is `location_id`-filtered anyway.
- `settlements.ts:91`, `refunds.ts:70`, `product-media.ts` — every mutating child-by-parent query is
  preceded by and covered by a payout/payment/media ownership check. SAFE.

**This is the important result, and it cuts against the proposal, not for it:** because the LIVE surface is a
*complete, small, enumerated set* (the ~11 §5a sites + the anonymizer sink), there is no excuse for B1 —
they had the entire list and chose to point-fix 4 of ~11 while shipping the rest live behind a warn-level
lint that cannot flag them (B2). The completeness of the enumeration is exactly what makes the
incompleteness of the *remediation* indefensible. The seam (`no-unscoped-tenant-query`) is being sold as the
reason the tail is safe, but the tail is *already* provably safe (this sweep); the seam's real job would be
to catch the **LIVE** sites — and per B2 it catches ~1 of ~6.

Corroboration of B8: the independent sweep also downgrades `modifier-groups.ts:156` to "writes into the
caller's own tenant; impact is a dangling/foreign-group reference + a global-FK existence oracle rather than
victim-data mutation" — consistent with the pricing-join filter finding.

---

## What the council must decide (ranked asks for STEP 3)
1. **B1 (CRIT):** either add point fixes for the 6 products/modifier subroute IDORs to Tier-1, or explicitly
   accept shipping a destructive cross-tenant write (`products.ts:282`) — do not let the warn-lint stand in
   for a fix.
2. **B2/B3 (HIGH):** the two guardrails as specified do not go red on the bugs. Re-spec the lint token to a
   *predicate* form (`location_id\s*=\s*\$`, `JOIN memberships`) not bare `location_id`, drop the
   `withTenant` exemption while the pool is BYPASSRLS, and rewrite `require-auth-hook` to parse per-route
   `preValidation` arrays + `requireRole(...)` calls — or the red→green proofs (§6) cannot pass honestly.
3. **B4 (HIGH):** the resolver-collapse must special-case the 3 spa-proxy `null→200` onboarding callers and
   preserve `userId` return shape — it is not the trivial Tier-2 debt-paydown the ADR implies.
4. **B5/B6 (MED):** F3 per-location courier deactivation needs a migration (contradicts §7) or must be
   dropped; the LC5 sink must delete the `|| row.location_id` fallback or stop being called a second
   independent layer.
