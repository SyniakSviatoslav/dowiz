# Council RESOLUTION — Audit-fix: cross-tenant authz + data-access seam

Status: **RESOLVE round complete** (Triadic Council STEP 3). Date: 2026-07-03.
Inputs: `proposal.md` (now rev 2 — revised by this round), `breaker-findings.md`, `counsel-opinion.md`.
Every breaker finding B1–B8 dispositioned (FIX / ACCEPT-RISK / DEFER-FLAG); the counsel ETHICAL-STOP
addressed (revise + needs-human). Design-only — no production code. **Conductor re-attacks next; this
document does not self-certify.**

---

## 1. Disposition table (breaker findings)

| ID | Sev | Disposition | Resolution |
|----|-----|-------------|------------|
| **B1** — ≥5 enumerated live IDORs left unfixed (incl. destructive `products.ts:282` DELETE) | CRIT | **FIX** | Tier-1 point-fix set expanded to the **complete enumerated live surface** — new proposal §3.5 lists all 8 remaining sites with per-site fixes (fold-ownership-into-statement pattern, 0 rows → 404). Enumeration-as-remediation withdrawn; no live site is left to a lint. Full list in §2 below. |
| **B2** — the lint can't flag the live sites (column-list token match, withTenant exemption, interpolation blindness) | HIGH | **FIX (redesign)** | `no-unscoped-tenant-query` **v2 = prove-or-allowlist, fail-closed** (proposal §4B): predicate-form tokens matched only in *post-FROM/INTO* text (column lists can never satisfy); `INSERT…VALUES` on a tenant table is UNKNOWN by construction (pushes the `INSERT…SELECT…WHERE` fold-in); any `${…}`/non-literal SQL is UNKNOWN; **no withTenant exemption** while pool is BYPASSRLS; UNKNOWN passes only via a hash-keyed checked-in allowlist (`{path, sql-hash, reason, verified-by}` — editing a query invalidates its entry); **`error` from day 1** (allowlist absorbs the audited-safe tail, no warn-window); allowlist count-floor ratchet in CI; adversarial fixture self-suite. Red on `orders.ts:863` is the canonical fixture (§3 below). This is the allowlist-inversion the round mandated. |
| **B3** — `require-auth-hook` can't see `requireRole([...])` factory calls or the preValidation-array form; can't reach 0 | HIGH | **FIX (rewrite matcher)** | v2 matcher (proposal §3.2): recognizes Identifier / MemberExpression / **CallExpression** shapes in `addHook`, **per-route `preValidation`/`preHandler` option arrays** (the shape 10/27 owner files use), and register-level options; **per-ROUTE granularity** (closes the one-bad-route-in-a-guarded-file hole = the LC2 shape); `// auth-exempt: <reason>` escape hatch counted in the same ratchet. `routes/orders.ts` stays out of hook-rule scope by design — covered by the tenant-query rule (its owner routes already carry inline `requireRole`, verified `:841`). Reaches 0 honestly → `error`. |
| **B4** — resolver-collapse regresses first-run onboarding (3 spa-proxy null→200 callers) + drops `{locId,userId}` | HIGH | **FIX (re-scope)** | Collapse re-specified as **one core resolution body + thin per-caller adapters** (proposal §7): `resolveOwnerMembership(db, userId) → {locationId,userId}|null` carries no HTTP semantics; each caller keeps its contract — get-owner-location null→401; spa-proxy keeps raw-header auth + its 3 deliberate null→200 mappings (settings `{id:null}` onboarding, invite `pending:true`, storefront bootstrap fall-through); product-media keeps `{locId,userId}`. The ONLY intentional delta: promotions 500→401 (changelog-noted). Onboarding regression test added to the proof plan (§6.7d). Remains Tier-2, no longer "trivial." |
| **B5** — F3 per-location deactivation needs a migration (`courier_locations` has no status column) — contradicts "code-only" | MED | **FIX (re-scope) + DEFER-FLAG** | Tier-1 F3 = role gate only (closes the actual privilege-escalation: customer reads decrypted roster, courier mutates co-workers). Per-location deactivation **extracted to track M-F3** with its migration (`courier_locations.status`), per-location session semantics, and its own red→green (*deactivate in A ⇒ active in B*) — per counsel, elevated as a courier-livelihood protection, not buried. **Named residual risk while deferred (human-visible):** an owner deactivating a legitimately-shared courier deactivates them for all locations; cross-location *targeting* is already blocked (`couriers.ts:89-92`); shared-courier population today ≈ operator-seeded demos. §7's "code-only" claim corrected. |
| **B6** — sink `AND location_id=$2` is a tautology unless the `\|\| row.location_id` fallback is deleted | MED | **FIX** | Fallback **DELETED** at `anonymizer/index.ts:131` and `:208`; explicit scope becomes **required** — missing scope → throw (fail-closed). Live callers already comply (GDPR worker threads the request row's location; retention threads each customer's own location explicitly). Layering re-labeled honestly: entry gate = load-bearing control; sink = dependent tamper-evidence layer, not "independent second layer." Proof: `anonymize()` without scope ⇒ throws; UPDATE with foreign scope ⇒ 0 rows (§6.1). |
| **B7** — sink enumeration included dead code (dry-run `:57`/`:64` has zero callers) | MED | **ACCEPT (accuracy corrected)** | Correct: `dryRun:true` is never passed in production. The rigor claim is corrected in §3.3 (enumeration now reachability-annotated). The dead branch is scoped anyway (uniformity, ~0 cost — dead code gets revived) and flagged to the cleanup/ponytail ledger for deletion. No design change beyond the honesty fix. |
| **B8** — modifier-groups:156 severity overstated (does NOT reach checkout pricing) | MED | **FIX (accuracy)** | §5a row corrected: both pricing joins filter `m.location_id = <order's location>` (`orders.ts:225/436`) — injected rows are filtered out of the victim's checkout. Real surface: owner-GET `modifier_count` inflation (`:60-64` join lacks a location guard — **companion-fixed** in §3.5.7) + FK existence oracle. Downgraded HIGH→MED; still Tier-1 point-fixed (a cross-tenant write is a cross-tenant write). |
| B9/B10/B11 — LC2 JOIN, F5, gdpr-role-gate confirmed sound | LOW/OK | **NO CHANGE** | Recorded; not re-litigated. |

## 2. The COMPLETE live-IDOR point-fix list (Tier-1, all of it)

Order = ship order (highest-harm-first per counsel). Every site verified against live source this round.

| # | Site | Endpoint / path | Fix (design) |
|---|------|-----------------|--------------|
| 1 | `owner/gdpr.ts:48` + `:81-86` (**LC5 entry**) | POST `/:loc/gdpr-requests` | direct-`customerId` branch gains same-tenant proof (`SELECT 1 FROM customers WHERE id=$ AND location_id=$`) → 404; gate-miss classified (`nonexistent` vs `cross_tenant_attempt`, server-side only) and cross-tenant attempts security-logged before the 404 |
| 2 | `lib/anonymizer/index.ts` (**LC5 sink**) | worker sink | `AND location_id=$2` on ALL by-id reads/writes (`:119, :133-140, :148, :196, :210-221`, + dead dry-run `:57/:64` for uniformity); **fallback `\|\| row.location_id` deleted** (`:131`, `:208`) — scope required, fail-closed; audit `location_id` = subject's true tenant + `metadata.actor_location_id`/`subject_location_id`/`request_id` provenance |
| 3 | `routes/orders.ts:862-865` (**LC2**) | PATCH `/orders/:id/status` | membership JOIN folded into the read (mirrors GET sibling `:747-756` + `dashboard.ts:626`); 0 rows → 404 before `assertOwnerTargetAllowed`; `locationId` from the JOIN-verified row; JOIN-miss attempt-logged |
| 4 | `lib/signals/compute.ts:85-88` (**F5**) | GET `/:loc/signals/compute` | `AND location_id=$2`, threaded from the `requireLocationAccess`-verified param |
| 5 | `owner/products.ts:211-217` | PUT `…/products/:id/translations/:locale` | `INSERT…SELECT FROM products p WHERE p.id=$ AND p.location_id=$ ON CONFLICT…` → 404 on 0 rows (`product_translations` has NO location_id column — parent fold-in is the only possible scope, even post-RLS-flip) |
| 6 | `owner/products.ts:239` | GET `…/products/:id/translations` | `JOIN products p ON p.id=pt.product_id AND p.location_id=$2` |
| 7 | `owner/products.ts:256` | DELETE `…/translations/:locale` | `DELETE … USING products p WHERE p.id=pt.product_id AND p.location_id=$3 …` → existing 404-on-0-rows now covers foreign products |
| 8 | `owner/products.ts:282` (**destructive DELETE**) | PUT `…/products/:id/modifier-groups` | same-tx product-ownership pre-check → 404, **plus** `AND location_id=$2` on the DELETE itself |
| 9 | `owner/products.ts:285-289` | (same endpoint, INSERT half) | `INSERT…SELECT FROM modifier_groups mg WHERE mg.id=$ AND mg.location_id=$` — foreign `group_id` → 400, tx rollback |
| 10 | `owner/products.ts:307-313` | GET `…/products/:id/modifier-groups` | `AND pmg.location_id=$2` (+ `mg.location_id=$2`) |
| 11 | `owner/modifier-groups.ts:156-160` | POST `…/modifier-groups/:groupId/modifiers` | `INSERT…SELECT FROM modifier_groups mg WHERE mg.id=$2 AND mg.location_id=$1` → 404; companion: count join `:62` gains `AND m.location_id = mg.location_id` |
| 12 | `owner/categories.ts:163-165` + `:244-245` (existence oracle) | DELETE `…/categories/:id` | pre-check SELECT gains `AND location_id=$2` — 409-vs-404 oracle closed |
| 13 | `owner/couriers.ts` (**F3**) | file-level hooks | `fastify.addHook('preValidation', fastify.requireRole(['owner']))` (mirrors gdpr.ts:29) |
| 14 | `owner/courier-invites.ts` (**F4**) | file-level hooks + body | role gate as #13 + body-`role` allow-list `['courier']` → 400 |

**CRITICAL/HIGH scorecard after this resolution:** breaker B1 (CRIT) → FIXED-in-design (all sites
Tier-1); B2, B3, B4 (HIGH) → FIXED-in-design (lint v2, hook-rule v2, core+adapters). Audit findings:
LC2/LC5/F3/F4/F5 + all §5a subroute IDORs → Tier-1; nothing live deferred. The only DEFERRED item is
the B5 per-location-deactivation *side-effect* (track M-F3, migration, named risk) and the Section-E
RLS migrations (B3/flip council, unchanged non-goal).

## 3. Red→green proof list (what the conductor should re-attack)

1. **Lint v2 vs `orders.ts:863` — the canonical RED fixture** (the exact query B2 proved unflaggable
   under v1): `SELECT id, status, location_id, type FROM orders WHERE id = $1` — `location_id` sits in
   the column list; post-FROM text has no `location_id =` → **MUST flag**. GREEN after the membership
   JOIN.
2. Lint v2 RED set (pre-fix tree): `anonymizer/index.ts:119, :133, :148, :196, :210` · `compute.ts:86`
   · `products.ts:212, :239, :256, :282, :286, :308` · `modifier-groups.ts:157`. GREEN post-fix +
   seeded allowlist ⇒ 0 errors at severity `error`.
3. Lint v2 adversarial self-suite: column-list decoy MUST-flag · predicate pass · `${clauses}`
   interpolation MUST-flag · concatenated SQL MUST-flag · `INSERT…VALUES` w/ location_id column but
   unverified FK MUST-flag · `INSERT…SELECT` w/ predicate pass. Allowlist count-floor ratchet wired.
4. `require-auth-hook` v2: RED on `couriers.ts` + `courier-invites.ts` pre-fix; GREEN post-fix; ZERO
   false positives on the 10 preValidation-array owner files (categories, locations,
   menu-availability, menu-confirm, menu-import, menu-translate, modifier-groups, product-media,
   products, promotions).
5. Per-fix adversarial HTTP assertions (owner-A token vs tenant-B object), each RED pre-fix:
   LC5 404 + B-customer PII unchanged post-drain (bypass-pool) · `anonymize()` w/o scope throws ·
   sink UPDATE 0-rows on foreign scope · attempt-log row `cross_tenant_attempt` · provenance fields on
   a legitimate erasure · LC2 404 + B-order unchanged · **PUT modifier-groups on B's product ⇒ 404 +
   B's `product_modifier_groups` rows INTACT** (the destructive-DELETE case) · translations PUT/GET/
   DELETE 404 · foreign `group_id` sync ⇒ 400, no row · POST modifiers into B's group ⇒ 404, no row ·
   categories oracle status-parity · F3 customer-GET 403 / courier-PATCH 403 · F4 courier-POST 403 +
   `role:'owner'` 400 · F5 empty.
6. CI: `test:unit` job wired (rls-adversarial runs; skipped setup = FAIL); `IDOR_TABLES` extended from
   `['orders','customers','courier_positions']` (test:30) with `courier_sessions`,
   `gdpr_erasure_requests`, `product_translations`, `product_modifier_groups`, `modifiers`.
7. Resolver contracts (Tier-2): fresh-owner `GET /api/owner/settings` ⇒ 200 `{id:null}` (NOT 401);
   promotions cross-tenant ⇒ 401; `{locId,userId}` shape preserved.
8. Track M-F3 (when it lands): deactivate courier in A ⇒ courier still active in B.

Each guardrail → `docs/regressions/REGRESSION-LEDGER.md` row with the red→green citation.

---

## §STOP-1 — Counsel ETHICAL-STOP: disposition **REVISE + NEEDS-HUMAN**

**Revised into the design** (proposal §3.3, §8): LC5 ships FIRST (irreversibility + broken backup >
audit HIGH); the audit stamp records the **subject's true tenant** + actor-vs-subject provenance in
`metadata` (JSONB — code-only, verified `anonymizer/index.ts:278-295`); blocked cross-tenant attempts
are **logged, not just 404'd** (LC5 entry gate + LC2). The data subject (customer, courier) is named a
first-class stakeholder in §8/ADR.

**NEEDS-HUMAN (recorded decision required before PROD ship of this batch, and unconditionally before
onboarding real tenant #2):**

**"Was LC5 exploited?" — and can the trail answer it?** Sharper than the counsel feared: the trail is
**not blind** for the GDPR path. `customers.location_id` **survives anonymization** (the UPDATE nulls
name/phone only) and `gdpr_erasure_requests` records the *requester's* location — so cross-tenant
erasure requests are directly detectable by cross-join, **as long as request rows are retained**
(operator: confirm `gdpr_erasure_requests` retention covers the exposure window). Run:

```sql
-- (1) Direct evidence: erasure requests whose target customer belongs to another tenant
SELECT r.id, r.location_id AS requester_loc, c.location_id AS victim_loc,
       r.requested_by_owner_id, r.status, r.created_at
FROM gdpr_erasure_requests r
JOIN customers c ON c.id = r.customer_id
WHERE c.location_id <> r.location_id;

-- (2) Audit-log divergence: anonymizations whose logged tenant ≠ the subject row's true tenant
SELECT a.id, a.subject_id, a.location_id AS logged_loc, c.location_id AS true_loc,
       a.actor_kind, a.actor_id, a.created_at
FROM anonymization_audit_log a
JOIN customers c ON c.id::text = a.subject_id
WHERE a.subject_kind = 'customer' AND c.location_id <> a.location_id;
```

Known residual blindness (state it in the record): a request row deleted/purged before the check, and
any hypothetical non-GDPR-path invocation, are not covered by (1); (2) covers completed erasures
regardless of path. Proportionality (counsel's anchor): today's tenants are operator-seeded demos —
external disclosure is near-vacuous *now*; the duty binds at real tenant #2, and the attempt-logging +
provenance in this batch is what makes it dischargeable from then on.

**OPERATOR RECORDS HERE (STOP-1 decision — one of):**
- [ ] "Ran (1)+(2) on prod on <date>: no cross-tenant rows; retention window confirmed to cover
      <exposure start>; proceed."
- [ ] "Ran (1)+(2): found <n> rows / cannot rule out; disclosure plan: <…>."
- [ ] "Proceed, demos-only, log-check <clean | blind-but-accepted>; capability confirmed in place
      before real tenant #2."

> Recorded by: ____________  Date: ____________

---

## Counsel advice adoption map (all 7)

| # | Advice | Adopted where |
|---|--------|---------------|
| 1 | LC5 first | §8 order |
| 2 | Log the attempt | §3.3.1 + LC2; proof §6.1c |
| 3 | Status-code changelog | §7 |
| 4 | Adversarially test the lint | §4B self-suite; residual FN surface → Option A queue |
| 5 | Escape-hatch count ratchet | §4B allowlist floor in CI |
| 6 | Elevate courier-scope fix | Track M-F3, own migration + proof, named risk |
| 7 | Name the data subject | §8 stakeholders + ADR |

Also adopted from the counsel's epistemic lens: the lint is no longer both the assumption and its own
mitigation — the tail's safety rests on the *enumerated allowlist* (audited entries), the lint's job is
only to fail everything NOT proven or enumerated, and its own soundness is fixture-tested. Option A's
steel-man residue is honored: repositories extract highest-harm aggregates first (Customer→Order).

## Explicit ACCEPT-RISK register (for the human's eyes)

1. **B5 residual (until M-F3):** shared-courier deactivation crosses tenant boundaries as a
   side-effect; population today ≈ operator demos; cross-location targeting already blocked.
2. **B7:** dead dry-run branch stays (scoped) until the cleanup ledger deletes it.
3. **Lint FN residue:** SQL assembled outside the `.query(` call site is invisible to the lint by
   design; bounded by the fail-closed non-literal rule + the sweep; permanently closed only by
   Option A migration of that aggregate.

*Not self-certified. Conductor re-attack targets: §3 proof list above — especially lint-v2 vs
`orders.ts:863`, the destructive-DELETE assertion (#8/§2), and the fallback-deletion throw.*
