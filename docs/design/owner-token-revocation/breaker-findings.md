# Breaker Findings — Owner Access-Token Server-Side Revocation

Attacker: System Breaker DeliveryOS. Axis: where does this design fail to revoke, leak, or break.
Target: `docs/design/owner-token-revocation/proposal.md` + `docs/adr/0004-owner-token-revocation.md`.
Grounding (read, READ-ONLY): `apps/api/src/plugins/auth.ts`, `apps/api/src/routes/auth.ts`, `apps/api/src/routes/auth/local.ts`, `packages/platform/src/auth/jwt.ts`, `packages/platform/src/auth/tenant.ts`, `packages/shared-types/src/legacy.ts:161-175`, `packages/db/migrations/1780310071220_core-identity.ts`, `1780421100051_force-rls.ts`, `1790000000015_operational-pool-role.ts`.

Date: 2026-06-23. Zero fixes proposed — only proofs of breakage. Severity is calibrated, not inflated.

---

## CRITICAL

### C1 · B-SEC / B-DATA — R1 self-read RLS policy is a guaranteed total-owner lockout, NOT an open item
**Vector:** the design (§5, §8, R1) says: if `users` lacks RLS, "add ENABLE + FORCE + self-read policy (`USING (id = app_current_user())`) in the same forward-only migration." This is presented as a safe action item. It is a self-inflicted outage detonator.

**Grounded facts:**
- `users` has **no RLS today** — `core-identity.ts:84-100` enables RLS only on `locations`/`memberships`/`organizations`; `force-rls.ts:5-14` forces only courier tables. `users` is never ENABLE'd.
- The verify path reads through `request.server.db`, which is the **operational pool** (`server.ts:249-251`, `createOperationalPool`), whose role is `deliveryos_operational_user WITH ... NOBYPASSRLS` (`1790000000015:19`).
- The verify path does a **bare `pool.query(...)`** — see the courier mirror it copies at `auth.ts:74-83`. It does **NOT** go through `withTenant`. Only `withTenant` runs `set_config('app.user_id', ...)` (`tenant.ts:11`). `app_current_user()` is `NULLIF(current_setting('app.user_id', true),'')::uuid` (`core-identity.ts:70-72`) ⇒ **returns NULL** when the GUC is unset.

**Break:** with `FORCE` + `USING (id = app_current_user())`, the NOBYPASSRLS operational role + unset GUC ⇒ predicate `id = NULL` ⇒ **zero rows for every owner on every request**. The cache-fill SELECT returns no `token_version` row. Best case the verify path then fail-opens (revocation silently dead — see C2); worst case "row absent ⇒ tv mismatch ⇒ 401" ⇒ **every owner on every machine is logged out the instant the flag is flipped**. The "≤30s revoke" promise inverts into "100% revoke of all legitimate owners, permanently."

**Invariant violated:** "no self-inflicted total-owner outage" (the design's own stated reason for fail-open, §4/R3) and RLS-correctness. The mitigation the design recommends (R1) is the exact thing that breaks it. R1 must be treated as a **load-bearing migration-correctness CRITICAL**, not an "Open" footnote.

---

### C2 · B-FAIL / B-SEC — FAIL-OPEN makes the control structurally unable to deliver its core promise; worst-case gap is unbounded, not ≤30s
**Vector:** the entire purpose of this control is *revocation*. The design's crux decision (§4, R3, §7 table) is **fail-open** on the `token_version` read: "if no cached value, FAIL-OPEN to token valid." Quantify what a removed owner / attacker keeps during any store degradation.

**Worst-case timeline (compounding, all individually plausible):**
1. Operator/membership trigger bumps `tv` and `PUBLISH auth:tv:bump`.
2. Machine M2's Upstash pub/sub subscription is flapped/disconnected at that instant. Upstash pub/sub is **at-most-once** — a disconnected subscriber **never receives** the message (no replay). M2 does not evict.
3. M2's cached `(userId → old tv)` entry was just refilled 1s ago ⇒ valid for ~29 more seconds on M2. The attacker's token keeps working on M2.
4. At TTL expiry, M2 attempts a cache-refill SELECT. If at *that* moment the PG read fails (pool checkout reject — the exact contention the design says is common enough to need 503 guards at `auth/local.ts:77-83`), the design's rule is **"no cached value ⇒ FAIL-OPEN to valid."** The revoked token is served again, and a fresh (stale) entry is cached for another 30s.
5. Each subsequent 30s window that coincides with a pool blip **re-opens** the gap. On a flapping Upstash connection + a contended pool, M2 serves the revoked token **indefinitely**, not "≤30s."

**Back-of-envelope:** the design's own §2 says owner load is bursty and the pool is contention-sensitive enough to already carry 503 guards. A DB blip during an incident (e.g. the very incident that motivated the revoke — a compromised owner hammering the API) is *correlated* with the moment you most need revocation. Fail-open + at-most-once pub/sub + per-machine LRU ⇒ the revocation gap is bounded by **min(pub/sub delivery, TTL) only when the store is healthy**, and **unbounded when it is not**. The "≤30s guaranteed floor" claim (§4 point 3, "TTL is authority") is **false**: TTL only forces a re-*read*, and the re-read fail-opens.

**Invariant violated:** "a security control whose entire purpose is revocation must fail safe for the asset it protects." Fail-open is defensible for *availability* controls, not for *revocation* controls. The design inverts the threat model: it optimizes against a self-inflicted outage (a real but operationally-recoverable event) at the cost of the one guarantee it exists to provide.

---

### C3 · B-CONSIST — sticky hard-revoke (the only fail-CLOSED escape hatch) cannot survive a cold cache, so the "known-compromised" guarantee is also fail-open
**Vector:** the design admits fail-open is unacceptable for a *known-compromised* owner and carves an exception (§4, R3, §7): a "sticky hard-revoke" — "if the last *successful* cached read for a user carried a hard-revoked flag, that sticky-deny survives a subsequent DB blip until TTL."

**Break — the sticky flag depends on a prior successful read that may never exist on the machine that matters:**
- The sticky-deny is described as surviving *"a subsequent DB blip"* **only if** "the last successful cached read ... carried a hard-revoked flag." That requires the machine to have **already successfully read the row after `auth_revoked_at` was set.**
- Concrete: operator sets `auth_revoked_at` on M-primary. Machine M2 is mid pool-outage and has **never** successfully read the post-revoke row (its cached entry predates the revoke, or it cold-started during the outage with an empty LRU). M2 has no sticky flag to be sticky about. Per the fail-open rule, "no cached value ⇒ allow." **The known-compromised owner sails through M2 for the full outage.**
- Worse: a machine that cold-starts (Fly redeploy, OOM-restart, autoscale-in then back) during the incident begins with an empty LRU. Its first read of the compromised user, if the DB is degraded, fail-opens. The "fail-closed-capable" exception is therefore **conditional on the DB being healthy** — i.e. it is fail-closed only when fail-open wasn't needed.

**Back-of-envelope:** with 2+ machines and `auto_stop_machines=false`, the probability that *every* live machine has a fresh successful post-revoke read of the compromised user *before* its next blip is exactly the probability that the store is healthy — the case where you didn't need the sticky flag. The escape hatch closes only the door that wasn't open.

**Invariant violated:** "force-logout of a known-compromised owner must be fail-CLOSED-capable" (the design's own §4 exception). The mechanism does not achieve fail-closed under the only conditions (store degradation) where the distinction matters.

---

## HIGH

### H1 · B-CONSIST / B-SEC — `tv`-on-`users` conflates "revoke one leaked token" with "log out all devices"; you CANNOT revoke the attacker without logging the victim out
**Vector:** the design (Option A, §3, R4) bumps `token_version` on `users` — global per-user, not per-session. It claims this is "exactly right." For the *primary motivating threat* (a leaked/stolen owner token, §1 point 1) it is wrong.

**Break:** owner Alice's token is phished. Alice is still actively using her real device (tablet at the counter taking orders). To kill the attacker's token, the only lever is `UPDATE users SET token_version = token_version + 1`. That bump invalidates **Alice's own live device too** — mid-service. Alice is forced to re-login (and if her refresh family was also cleared per the operator runbook §9 step 2, she must re-enter credentials) **while taking orders**. There is **no way** to revoke the leaked token while preserving the legitimate session — the design explicitly has no per-device granularity (R4, "Deferred").

**Consequence:** operators will hesitate to revoke (revoking nukes the victim's working session during peak service), so the control's *human* usability undermines its security purpose. "Theft window drops from 7d to ≤30s" (ADR Consequences) is only true if the operator accepts logging the victim out of a live session — a cost the design never prices.

**Invariant violated:** "revoke the compromised credential, not the user's livelihood." The design conflates two distinct security operations and ships only the blunt one for the threat that actually motivated it.

---

### H2 · B-SCALE / B-CONSIST — folded membership recheck either re-introduces per-request DB load (the starvation it claims to avoid) OR is cached, in which case role-downgrade revocation has the SAME ≤30s+fail-open gap
**Vector:** §4 point 4 borrows the courier membership recheck into "the same cache-fill query." Pick a horn:

- **Horn A (correctness):** to kill role-downgrade roll-forward *immediately*, the membership predicate must be evaluated **per request**. But the design caches the whole fill for 30s ⇒ the membership state is also 30s-stale **and fail-open**. So a downgraded/removed owner keeps elevated `activeLocationId` access for the same ≤30s (unbounded under C2). The design claims this "kills role-change roll-forward" — it only delays it by the cache window, with the same fail-open hole.
- **Horn B (immediacy):** if instead the membership check is *not* cached (evaluated per request to be correct), then every owner request does the join SELECT ⇒ **one PG checkout per owner request** = exactly Option B's cost (§3, "150 checkouts/s at 10× growth") that §4 ("Why not B platform-wide") spends a page rejecting. The design cannot both fold membership into the 30s cache *and* claim immediate role revocation.

**Back-of-envelope:** at the design's own 150 req/s (N=500), Horn B = 150 PG checkouts/s of pure auth overhead on the contention-sensitive pool. Horn A = a removed owner reads/writes tenant data for up to 30s (or unbounded under fail-open) after being removed.

**Invariant violated:** "no per-request pool starvation" (design goal a) AND "membership-revoke takes effect ~immediately" (design goal) are mutually exclusive under a single cached fill. The design claims both.

---

### H3 · B-OPS / B-FAIL — flag default-off + no existing owner-logout endpoint = operators believe they revoked when nothing happened
**Vector:** §4/§9 ship `OWNER_REVOCATION_ENABLED=false` by default; "off ⇒ verify ignores `tv` entirely." Separately, **there is no server-side owner logout endpoint today** (grep: only `apps/api/src/routes/courier/auth.ts:478` has `/logout`; the web's `authService.logout()` POSTs `/api/auth/logout` — `packages/ui/src/lib/auth.ts:99` — which has **no matching server route**, confirmed by grep over `server.ts`/`routes/*.ts`).

**Break — the silent-no-op window:**
1. Migration ships (column added), tokens start carrying `tv`, but `OWNER_REVOCATION_ENABLED` is still `false` (the documented default, deploy-dark phase).
2. An incident hits. Operator runs the §9 runbook: `UPDATE users SET token_version = token_version+1, auth_revoked_at = now()` + pub/sub publish. The dashboard shows the bump succeeded.
3. **The verify path ignores `tv` because the flag is off.** The compromised token keeps working for the full 7-day exp. The operator has *false confidence* they revoked — the worst operational state, strictly worse than knowing you can't revoke.
4. Even after the flag is flipped, the web "logout" button POSTs to a **non-existent** `/api/auth/logout` (404, swallowed client-side per `auth.ts:104`) ⇒ **no `tv` bump is ever triggered by user-initiated logout.** The design says "explicit logout → bump tv" but the endpoint that would do it does not exist and is out of scope (§Non-goals: "membership delete / role change endpoints do not exist yet"). The most common revocation trigger (a user clicking "log out") is wired to nothing.

**Invariant violated:** "ops can tell the difference between revoked and not-revoked" + "the control is actually reachable." Shipping the seam without the endpoints that call it means the headline trigger ("explicit logout") is dead on arrival, and the dark-deploy window is a believed-but-false revocation.

---

### H4 · B-CONSIST / B-SEC — `/auth/refresh` does NOT close the roll-forward as designed; it re-mints with a 30s-stale fail-open `tv` and the membership read is on a DIFFERENT pool than the cache
**Vector:** §4 point 5 + §6 claim refresh re-mints "with the current `token_version`" and "refuses to mint if no active membership remains," closing the roll-forward.

**Break:**
- `/auth/refresh` (`auth.ts:234-305`) runs on `fastify.db` = the **operational read-only pool**. Per C1, if R1's self-read RLS policy lands, the refresh's `token_version` read **also** returns NULL/zero-rows ⇒ it would read `tv=0` (absent ⇒ 0 per §5) and re-mint a token that *passes* the verify check ⇒ **roll-forward fully preserved.**
- Even without C1: §6 ("Refresh atomicity") admits "if a tv bump races a refresh, the refresh either reads the old value ... or the new value." The refresh reads `token_version` directly from DB (not the 30s cache) — but a removed owner who refreshes in the **same 30s window** the operator bumps can read the *old* tv (replication lag / read-replica / the bump txn not yet visible) and mint a fresh **7-day** token stamped with the old tv that then validates for 30s on every machine — and re-refreshes again before the next bump. The design never bumps `tv` *on refresh*; it only *reads* it. A removed owner who keeps refreshing every <30s rides the consistency seam.
- The "refuse to mint if no active membership" guard depends on the membership SELECT at `auth.ts:288-292` returning empty. But that query returns *any* active membership ordered by `(role='owner') DESC` — a downgraded owner who still holds a **non-owner** membership (e.g. demoted to staff/courier on the same location) returns a row ⇒ refresh succeeds and re-mints `role:'owner'` (the endpoint hard-codes `role:'owner'` at `refreshedOwnerClaims`, `auth.ts:20`). **Role downgrade does not stop the refresh from minting an owner token.**

**Invariant violated:** "refresh cannot roll a revoked/downgraded owner forward." The refresh path re-reads but never re-stamps `tv`, hard-codes `owner`, and shares the broken pool — three independent holes in the close.

---

## MEDIUM

### M1 · B-SEC — legacy/absent-`tv` = permanent bypass class; absent⇒0 means any token minted before the bump, or with `tv` stripped, validates against `token_version=0` forever-until-exp
**Vector:** §5 + R5: `tv` is `.optional()` on the owner variant; "absent ⇒ treated as 0 ⇒ legacy in-flight tokens still validate against `token_version = 0`."

**Break — walk the verify path (`jwt.ts:82-115`):** the `AuthToken` owner variant is `.strict()` (`legacy.ts:164`), so an attacker cannot *add* junk claims — but `tv` being **optional** means a token with **no `tv` claim at all still parses and validates.** Now: any owner token minted *before* the migration deployed, OR during the dark-deploy window before mint sites add `tv`, carries no `tv`. After the first revocation bump sets `token_version=1`, those legacy tokens have `tv` absent ⇒ treated as `0` ⇒ compared against current `1` ⇒ this *would* mismatch... **unless** the comparison is "absent means skip the check" rather than "absent means literal 0." The design's wording is ambiguous ("treats absent tv as tv=0, so legacy tokens validate against token_version=0 *until they naturally expire*") — which states legacy tokens **keep validating**, i.e. a pre-migration stolen token is **never revocable by a bump** because its baseline is frozen at 0 while the column moved to 1. A 7-day pre-migration leaked token is immune to `tv` revocation for its entire life. The attacker doesn't even need to strip anything — the design hands them a 7-day grandfather clause.

**Invariant violated:** "a bump revokes all of a user's tokens." Tokens minted before the first bump (every token in the dark-deploy + flag-flip window) are structurally exempt.

### M2 · B-OPS — `auth_revoked_at` is set but never read by the cached verify path; sticky-deny depends on a "hard-revoked flag" that has no defined source-of-truth in the cache fill
**Vector:** §5 adds `auth_revoked_at timestamptz`; §9 step 1 sets it. But the cache-fill SELECT in §4 is specified to read `token_version` + membership. The "sticky hard-revoke flag" (C3) must come from reading `auth_revoked_at` and caching a deny — yet the design never states the cache key carries it, nor how a deny becomes "sticky" across the 30s TTL boundary (after TTL the entry is gone; the next fill must re-read `auth_revoked_at`, which under a DB blip fails ⇒ fail-open per C2). The column is dead weight unless the fill reads it every miss, and even then it does not survive the outage it was built for.

**Invariant violated:** "every column added has a reader on the revocation path that achieves its stated fail-closed effect." `auth_revoked_at` does not.

### M3 · B-SCALE — cache hit-rate math is optimistic; cold-start / deploy / scale events convert the whole owner fleet to "Option B" load transiently, on the pool that has 503 guards
**Vector:** §2 claims 83–97% hit rate and "~2.3 reads/s." But every Fly redeploy, OOM-restart (the public-menu OOM lesson is cited as precedent — restarts happen), and autoscale event starts a machine with an **empty LRU**. For the first 30s after each such event, **every** owner request is a miss ⇒ one PG checkout each. At 150 req/s (N=500) that is a 150 checkout/s spike on the contended pool **at exactly the moment** (deploy/restart) the system is least stable. The envelope's "negligible 2.3 reads/s" is the steady-state average and hides the cold-start spike.

**Invariant violated:** "no pool-starvation spike." The hit-rate average masks a correlated burst that lands the design back on the cost it rejected Option B for.

## LOW

### L1 · B-OPS — pub/sub described as "non-load-bearing optimization" but the entire <1s SLA depends on it; the "≤30s floor" is the fail-open re-read (C2), so realistically the only fast path IS the load-bearing one
The design repeatedly says "TTL is authority, pub/sub is speed." Given C2 proves the TTL floor fail-opens, the *only* mechanism that actually revokes fast is pub/sub — which is at-most-once and explicitly allowed to drop. The design's resilience story is backwards: it leans on the component it declared non-load-bearing.

### L2 · B-ANTIPATTERN — design claims "trivially reversible (stop reading it)" but the R1 RLS ENABLE+FORCE on `users` is forward-only and NOT reversible by a flag flip
§4 "Minimal + reversible: flip the flag, column stays inert." True for the column. **False for R1**: if the migration adds `ENABLE/FORCE ROW LEVEL SECURITY` + a policy on `users` (the identity table read on every auth path, owner *and* customer *and* the courier session join), flipping `OWNER_REVOCATION_ENABLED=false` does **not** undo the RLS — `users` reads are now RLS-gated permanently, and any other code path reading `users` via the operational pool (e.g. `local.ts:59`, `auth.ts:288`) is now subject to it. The "reversible in one secret change" claim does not cover the migration's most dangerous side effect.

---

## Top deliverable — the single attack that most undermines the core promise
**C2 (fail-open) compounded by C1 (R1 RLS lockout) and C3 (sticky-deny is also fail-open).** The design's core promise is *immediate, guaranteed* revocation with a "≤30s floor." That floor does not exist: the TTL only forces a re-read, and the re-read fail-opens on any pool blip — which is *correlated* with the incident that triggered the revoke. On 2+ machines with at-most-once Upstash pub/sub and per-machine LRUs, a flapping connection serves a revoked token indefinitely. The one fail-closed escape hatch (sticky hard-revoke) requires a prior successful post-revoke read that a degraded/cold machine does not have. Net: **the control cannot guarantee it revokes a known-compromised owner**, which is the exact thing it was built to do. Meanwhile its recommended hardening (R1) is a total-owner-lockout landmine. The design optimizes against a recoverable availability event at the cost of its only security guarantee.

---

# REV-2 RE-ATTACK — against the PIVOTED lean design (P-a/b/c)

Target: rewritten `proposal.md` + `resolution.md` + `docs/adr/0004-owner-token-revocation.md` (rev 2).
Grounding (READ-ONLY, real source this round): `apps/api/src/routes/auth.ts`, `apps/api/src/routes/auth/local.ts`,
`apps/api/src/plugins/auth.ts`, `apps/api/src/lib/get-owner-location.ts`, `apps/api/src/routes/owner/product-media.ts`,
`apps/api/src/routes/owner/promotions.ts`, `apps/web/src/lib/apiClient.ts`, `packages/ui/src/lib/auth.ts`,
`apps/api/src/server.ts:580-581`, `packages/db/migrations/1780310071220_core-identity.ts`.
Date: 2026-06-23.

## Verdict up front
**The pivot is sound.** Removing the token_version/users-RLS/fail-open layer closes C1/C2/C3 *structurally*
(no substrate left to break) and introduces **no regression of equal severity**. P-a/b/c are individually
defensible. I did NOT find a CRITICAL in the lean design. **But** the rewrite's own correctness claim has a
real gap the docs never name (R2-1, HIGH), the membership-revoke story is weaker than the proposal sells
because the *actual* tenant-scoping helpers don't filter `status` (R2-2, HIGH), and two MEDIUM honesty/UX
regressions land on P-a. Ranked below. Severity calibrated, not inflated.

---

## REGRESSION CHECK — did removing the layer reopen anything?
- **No.** The removed layer never shipped (rev-1 was design-only). The hot path (`plugins/auth.ts:44-92`)
  is **unchanged** — pure `verifyAuthToken` signature+exp for owners (the courier branch already does its
  own per-request `courier_sessions` read; owners hit none of it). Removing the proposed `users`-RLS read
  removes a vector that never existed in code. No regression from removal.
- The three NEW changes (P-a/b/c) are the only regression surface. Findings below are confined to them.

---

## HIGH

### R2-1 · B-CONSIST / B-SEC — P-c's "membership-revoke enforced at refresh" is HALF the story: the per-request tenant-scoping helpers trust the baked `activeLocationId` and DON'T filter `status`, so a removed owner keeps tenant access for the FULL ≤24h access life on the busiest owner surfaces — independent of refresh
**The proposal's load-bearing claim** (proposal §1.2, §8; ADR Consequences): "membership-revoke / role-downgrade enforced at the refresh boundary (≤24h)" and "tenant-isolation staleness (baked `activeLocationId` outliving a membership) is closed."

**Ground truth — it is only closed on routes that mount `requireLocationAccess`, and even there only by `role`, not `status`:**
- `requireLocationAccess` (`plugins/auth.ts:146-149`) does re-read per request: `SELECT 1 FROM memberships WHERE location_id=$1 AND user_id=$2 AND role='owner'` — **but there is NO `status='active'` predicate.** A membership row flipped to `status='revoked'`/`'suspended'` (the exact "remove owner" operation the future UI will perform — `memberships.status` is the column, `core-identity.ts:60`) **still returns rowCount 1** ⇒ access granted. The per-request guard that exists is blind to the membership-status revoke the whole design is about.
- Worse, the **routes that don't mount `requireLocationAccess` at all** resolve their tenant via `getOwnerLocationId` (`lib/get-owner-location.ts:8-14`) / the inline `getOwnerLocation` (`product-media.ts:49-55`) / promotions (`promotions.ts:15-21`). All three: **(a) trust `user.activeLocationId` from the JWT with zero DB check**, and **(b)** their fallback query is `... WHERE user_id=$1 AND role='owner' LIMIT 1` — **again no `status='active'`**. Confirmed no-location-access owner surfaces: `routes/owner/product-media.ts`, `routes/owner/promotions.ts`, `routes/owner/menu-import.ts`, plus `routes/orders.ts` (owner branch).

**Break (concrete):** Operator/future-UI sets a removed owner's `memberships.status='revoked'`. The removed owner still holds a valid access token (baked `activeLocationId=L`). They hit `POST /api/owner/.../product-media` or the promotions/menu-import endpoints. `getOwnerLocation` returns `L` straight from the JWT, no membership read at all ⇒ they **upload media / create promotions / import a menu into a tenant they were removed from**, for the entire **≤24h** access life — and P-c (refresh) never runs because they never need to refresh within 24h. On `requireLocationAccess` routes the `status`-blind query lets them through too. **P-c closes the refresh boundary; it does not close the in-flight access token against tenant data, which is the actual money/tenant surface.**

**Why this is HIGH not CRITICAL:** it requires the future staff-management "remove owner" UI to exist (it doesn't yet — proposal §Non-goals), and is bounded by ≤24h. But the proposal *asserts* tenant staleness "is closed" — and against the live helpers it is **not closed at all on the no-location-access routes, and only role-closed (not status-closed) elsewhere.** The accepted-risk table (R1/R3) silently assumes enforcement lands; the actual enforcement point (`status`-filtered per-request membership read on every owner surface) is neither present today nor added by P-a/b/c.

**Invariant violated:** "a removed owner cannot read/write the tenant they were removed from." The design's stated closure of this invariant is contradicted by the live scoping helpers, which P-c does not touch.

### R2-2 · B-CONSIST — P-c's own predicate (R3, "open") will silently MISROUTE a legitimate multi-location owner: refresh re-derives `activeLocationId` as the FIRST active membership, which is not necessarily the location the owner was working in
**Setup:** refresh re-derives location via `SELECT location_id FROM memberships WHERE user_id=$1 AND status='active' ORDER BY (role='owner') DESC LIMIT 1` (`auth.ts:288-292`). For a single-location owner this is fine. For a **multi-location owner** (two active owner memberships L1, L2), `LIMIT 1` over an `ORDER BY (role='owner') DESC` with **no tiebreaker** returns an *arbitrary* row (Postgres returns whichever the plan yields — effectively non-deterministic across the partial index `memberships_user_id_active_idx`).

**Break:** owner is actively managing L2 (their token carries `activeLocationId=L2`). Access token hits ≤24h, silent refresh fires (`apiClient.ts:120-127`). Refresh re-derives `activeLocationId` and returns **L1** (the arbitrary first row). The owner's next request is now scoped to L1; their dashboard/menu/orders for L2 **silently swap to L1's data** mid-session — no error, no relogin, just the wrong tenant's data. With P-a cutting access to 24h, this refresh now fires *every day per active session* instead of every 7 days, so the misroute frequency goes up ~7×. The proposal flags R3 as "open — spec the exact predicate" but frames it as a 401-loop risk; the real failure is **silent wrong-tenant data**, not a loop, and P-a makes it more frequent.

**Invariant violated:** "a refresh preserves the session's working tenant." The current re-derive does not carry the *incoming* token's `activeLocationId` through; it recomputes it from scratch with a non-deterministic `LIMIT 1`. (Note: today this already exists at 7d cadence — `refreshedOwnerClaims` carries `activeLocationId` through, but the *source* of that value at `auth.ts:293` is the same `LIMIT 1` re-derive, NOT the incoming token. So P-c doesn't introduce the bug, but P-a's 24h cut **amplifies its rate ~7×** and the R3 spec must fix the carry-through, not just the 401 case.)

---

## MEDIUM

### R2-3 · B-FAIL / B-OPS — P-a 24h on the dev-bypass path is a UX regress on staging: the dev branch returns NO refresh_token, so a 24h dev token has NO silent-refresh path ⇒ forced re-login every 24h (was every 7d)
**Ground truth:** the dev-bypass branch (`local.ts:58-70`) mints `signDevToken(payload, '7d')` and returns **only** `{ access_token, userId, activeLocationId }` — **no `refresh_token`, no `auth_refresh_tokens` insert.** The whole silent-refresh machinery (`apiClient.ts:14-15` returns null with no stored `dos_refresh_token`; `:120-123` skips the refresh branch entirely when there's no refresh token).

**Break:** P-a cuts this dev token `'7d'→'24h'`. On staging (where `ALLOW_DEV_LOGIN=true`, per the saved staging token / DEV_AUTH_SECRET), every dev/E2E session now hard-expires at 24h with **zero silent-refresh fallback** ⇒ forced re-login daily. Before P-a that was weekly. The proposal's headline promise — "cutting access 7d→24h costs **zero** relogin prompts" (proposal §2, §4; "silent refresh means ... zero relogin") — is **false for the dev-bypass mint site**, which is exactly one of the 5 sites P-a touches (`local.ts:68`). The proposal lists `local.ts:68` in the P-a target set without noting it has no refresh companion.

**Why MEDIUM:** staging/dev-only (prod boot-guard forbids dev-login, ADR-0003), so no prod-owner impact — but it IS a real regress of the E2E/staging loop the Ship-Discipline rule depends on (daily re-login in long-running test sessions), and it falsifies a stated invariant ("zero relogin on every mint path"). Either the dev path should mint a longer-lived token or the "zero relogin" claim must be scoped to the 4 refresh-backed paths.

### R2-4 · B-SEC — P-b logout requires no auth and is keyed on the bearer/refresh token: a stolen-refresh-token holder can force-logout the victim's family; and the client sends the wrong credential
**Two grounded sub-issues:**
1. **Wrong credential at the call site.** `packages/ui/src/lib/auth.ts:95-107` (`logout()`) POSTs `/api/auth/logout` with header `Authorization: Bearer ${access_token}` and **no body** — it does NOT send the refresh token. But P-b's spec (proposal §4, ADR P-b) says logout "hashes the **caller's refresh token**" and `DELETE ... WHERE family_id = (that token's family)`. The client never sends a refresh token here ⇒ P-b as specified has **no input to hash**. Either P-b must key off the access token's `userId` (then "this device" scope is impossible — it can only do all-devices), or the client wiring (`auth.ts:99-102`) must change to send the refresh token in the body. The proposal says "wire `auth.ts:99-104`" but doesn't note that the current call sends the access token, not the refresh token — so the per-family ("this device") logout P-b advertises **cannot work** with the credential the client actually sends.
2. **No-auth force-logout.** If P-b keys purely on a presented refresh token (no `verifyAuth`), then anyone holding a *stolen refresh token* can delete that family — which is benign (they already own the session). But if P-b also exposes the all-devices variant keyed on `user_id` derived from an *unauthenticated* presented token, replay of a captured (already-rotated, now-invalid) refresh token to force a victim logout is a minor DoS. Bounded/minor, but the auth posture of the new route is unspecified — it must require a *currently-valid* credential and scope strictly to the caller's own `user_id`.

**Why MEDIUM:** sub-issue 1 is a real "the feature won't function as designed" gap (per-family logout dead-on-arrival with the current client credential); sub-issue 2 is minor-DoS. Neither is CRITICAL.

---

## LOW

### R2-5 · B-CONSIST — logout vs in-flight refresh-rotation race leaves a usable family window (bounded, benign)
P-b `DELETE FROM auth_refresh_tokens WHERE family_id=$x` racing a `/auth/refresh` rotation: if the refresh's atomic claim (`auth.ts:262-265`) lands first, it INSERTs a fresh row for the **same family_id** (`auth.ts:298-301`) *after* a logout DELETE that already ran ⇒ the just-deleted family is **resurrected** by the in-flight refresh, and survives until its own 7d expiry. The proposal §6 claims "the next logout deletes the rotated family" — true, but it requires a *second* logout the user won't issue (they think they already logged out). The window is bounded by ≤24h access exp + the user not refreshing, so practically benign, but "log out = synchronously guaranteed kill" (ADR Consequences) is **not** guaranteed against a concurrent refresh — it needs a delete that also blocks re-insert (e.g. a tombstone), which the additive route as specified does not have.

### R2-6 · B-DATA — F-N (30d→7d) shrinks the OAuth/Telegram idle family, but the refresh re-mint already writes 7d, so the only *new* behaviour is more frequent re-login for genuinely-idle OAuth/TG owners — accepted in R4, noted here only as the regression it is (idle >7d on those paths now re-logins where it didn't). Bounded, accepted.

---

## DEFERRED accepted-risk (≤24h leaked access token) — is 24h too long for THIS threat model?
**Largely acceptable, with one concrete caveat the docs under-price.** For a *theft/phishing* of an owner token, ≤24h + synchronous refresh-family kill is proportionate at ~50 locations pre-launch. **The caveat is R2-1:** the residual ≤24h window is not just "read your own tenant a bit longer" — because the no-location-access owner routes (product-media, promotions, menu-import, orders) trust the baked `activeLocationId` with **no status check**, the ≤24h window is a **write window into a tenant the owner was removed from** (create promotions, import a menu, mutate media). For a *removed/disgruntled* owner (the insider-threat case, distinct from external theft), 24h of write access to a tenant they no longer belong to can mean price/menu sabotage. That is a money/tenant-integrity surface, and it is *not* bounded by the synchronous refresh-family delete (that only stops *new* access tokens; the in-flight one keeps writing for ≤24h). So: 24h is acceptable for external-theft; for the **insider-removal** threat it is too long *given the status-blind scoping helpers* — but the right fix is R2-1 (status-filter the per-request scoping), not the full per-request revocation layer. The DEFERRED disposition is fine; the proposal should stop claiming tenant-staleness "is closed."

---

## REV-2 top deliverable
**R2-1.** The pivot's correctness rests on "membership-revoke enforced at the refresh boundary," but the *actual* per-request tenant-scoping helpers (`get-owner-location.ts:11`, `product-media.ts:51`, `promotions.ts:19`, and even `requireLocationAccess` at `plugins/auth.ts:147`) filter by `role='owner'` and **never by `status='active'`**, and several busy owner routes trust the JWT's baked `activeLocationId` with no DB read at all. So a removed owner retains read/WRITE access to the tenant for the full ≤24h access life on the surfaces that matter (promotions, menu-import, product-media, orders), regardless of P-c. The lean design is sound and the pivot is correct, but the proposal's claim that "tenant-isolation staleness is closed" is false against live source — the enforcement point the claim assumes does not exist. This is a spec/scope gap, not a reason to revert the pivot.

---

# REV-3 RE-ATTACK — narrow regression check on the REV-2 fixes (P-a/b/c/d as applied)

Target: `proposal.md` + `resolution.md` with REV-2 dispositions (R2-1..R2-5) applied.
Grounding (READ-ONLY, live source this round): `apps/api/src/lib/get-owner-location.ts`,
`apps/api/src/routes/owner/promotions.ts:14-24`, `apps/api/src/routes/owner/product-media.ts:46-56`,
`apps/api/src/plugins/auth.ts:44-157`, `packages/db/migrations/1780310044710_extensions-and-enums.ts:12`,
`1780310071220_core-identity.ts:55-101`, `1780421100051_force-rls.ts`, `1790000000015_operational-pool-role.ts`,
`apps/api/src/routes/dev/mock-auth.ts:184-188`, membership-insert sites (server.ts:713, auth.ts:363,
spa-proxy.ts:754, onboarding.ts:89, seed*.ts), and a full `requireLocationAccess` caller grep.
Date: 2026-06-23.

## Verdict up front — HARD-EXIT-READY. 0 unresolved CRITICAL/HIGH introduced by the REV-2 fixes.
The REV-2 fixes do **not** introduce a new CRITICAL or HIGH. I attacked each per the brief and the design holds.
Two LOW notes are recorded for the implementer (not blockers, not new severity). The design is sound and
ready to exit. I did not manufacture findings to fill the matrix.

## P-d — does `status='active'` deny a LEGITIMATE owner? NO. (regression: clean)
- **The enum has no benign non-active owner state.** `membership_status AS ENUM ('active','suspended','removed')`
  (`extensions-and-enums.ts:12`). There is no `invited`/`pending`. The only non-active states are *exactly* the
  revoke/suspend states P-d is meant to deny. Adding `AND status='active'` cannot strand an `invited` owner because
  that state does not exist.
- **Every owner-membership write is `'active'`.** All insert sites omit `status` (DEFAULT `'active'`):
  `server.ts:713`, `auth.ts:363`, `spa-proxy.ts:754`, `onboarding.ts:89`, `seed*.ts`; the dev path is explicit
  `... 'active' ... DO UPDATE SET status='active'` (`mock-auth.ts:185-188`). And there is **no** `UPDATE memberships
  SET status=...` anywhere in the API (grep clean — the "remove owner" UI does not exist yet, per proposal §Non-goals).
  ⇒ today **100% of owner memberships are `status='active'`**, so P-d denies **zero** real owners on day one.
- **Shared-helper non-owner callers: not affected.** `requireLocationAccess` (`plugins/auth.ts:117-157`) is the only
  helper with non-owner callers, but its customer branch (`:127-132`) and courier branch (`:135-140`) `return` **before**
  the `memberships` query (`:146-149`). P-d's predicate lands only on the owner-path query, which all of
  `couriers.ts`, `courier-invites.ts`, `locations.ts`, `dashboard.ts`, etc. reach **as owners**. No non-owner caller
  touches the modified SELECT.

## P-d per-request cost — bounded; NOT a hot-path regression
- **Extra queries/request = at most ONE, and only on owner-write routes.** The JWT-trust branch
  (`get-owner-location.ts:8`, `product-media.ts:49`, `promotions.ts:15`) today returns `activeLocationId` with zero DB
  reads. P-d's "route the JWT-trust branch through a `status='active'` re-read" converts that 0-read into **1 indexed
  read** per owner-write request. The predicate is covered by the partial index `memberships_user_id_active_idx`
  (`core-identity.ts:64`), so it is an index probe, not a scan. The **global hot path** (`plugins/auth.ts:44-92` owner
  branch) is untouched — owners add no per-request read there. At the envelope's ~15 req/s today / ~150 req/s at 10×,
  the owner-WRITE subset is a small fraction, and those routes already do per-request DB writes — one extra indexed
  SELECT is in-budget and far below the pool-starvation threshold the rejected Options B/C would hit (one checkout on
  EVERY owner request). No regression of the pool budget.

## P-d × RLS — the C1-class lockout does NOT re-appear (verified, and pre-existing-state-neutral)
This was the sharpest thing to check, because `memberships` is **`ENABLE + FORCE ROW LEVEL SECURITY`**
(`core-identity.ts:91-92`) with policy `USING (location_id IN (SELECT app_member_location_ids()))` (`:93-94`), and the
helpers issue a **bare `pool.query`** on the operational pool **without `withTenant`** (no `app.user_id` GUC). On paper
that is the C1 shape. **But P-d does not re-introduce C1**, for a decisive reason:
- The policy function `app_member_location_ids()` **already** filters `WHERE user_id = app_current_user() AND
  status='active'` (`core-identity.ts:76-79`). So whatever RLS posture the bare helper query has **today**, P-d's added
  `AND status='active'` is a **strict subset of an already-enforced predicate** — it changes the lockout calculus by
  exactly nothing. If the operational pool runtime credential is NOBYPASSRLS, these helper queries are *already*
  status-and-tenant-gated by RLS (and any "owner sees rows" behaviour today proves the GUC/credential path already
  admits them); if the runtime credential is still the superuser/`postgres` (BYPASSRLS — the migration note at
  `1790000000015:9-11` says the cutover is a manual `***REDACTED***` change), then RLS is bypassed and P-d's
  literal predicate becomes the **actual** status enforcement. **Either way P-d adds no new failure mode** — it does not
  ENABLE/FORCE any RLS (unlike the rejected rev-1 R1 on `users`), it adds no `users` read, and it cannot lock out an
  owner that the existing identical-shape query doesn't already serve. This is the structural reason the pivot's C1
  removal stays true under P-d.

## P-c activeLocationId preservation — no new per-refresh failure, no stale-tenant carry
- **No new failure path.** "Preserve the caller's CURRENT `activeLocationId` if still a valid active owner membership"
  is a filter over **the same memberships SELECT that already runs** at refresh (`auth.ts:289`) — 0 new queries
  (confirmed in proposal §2/§6). The preserve-check is an in-memory match of the incoming-token `activeLocationId`
  against the returned active-owner rows; it cannot add a DB round-trip and so cannot add a per-refresh failure mode.
- **No stale-tenant carry.** The carried `activeLocationId` is honoured **only if** it appears in the fresh
  `status='active' AND role='owner'` result set; if the owner was removed from that location, the row is absent ⇒ the
  carry is rejected and the deterministic fallback (`ORDER BY created_at, id`) applies; if no active owner membership
  remains ⇒ 401. So the carry can never resurrect a removed tenant — it is gated by the live read, not trusted from the
  token. Correct.

## P-b user-wide DELETE — acceptable multi-device surprise, and unauthenticated trigger is closed
- **Multi-device logout surprise: acceptable and disclosed.** A legit multi-device owner clicking "log out" kills all
  their families. This is the *named, intended* "log out all devices" semantics (R2-4), gated behind ES-2 honest copy
  ("other devices sign out within 24h"). It is the credential-honest default (the client sends no refresh token, so
  per-device is not even implementable from the current call — `ui/auth.ts:99-102`). Bounded, disclosed, accepted —
  not a new finding.
- **Cannot be triggered unauthenticated — confirmed.** P-b requires `verifyAuth` and derives `userId` from a
  currently-valid access token, scoped strictly to the caller's own `user_id` (proposal §4 P-b, §8; resolution R2-4
  sub-2). No body, no presented-token replay surface ⇒ the no-auth force-logout DoS is closed by construction. A
  tenant cannot revoke another tenant's session. Correct.

## Integration (P-a+b+c+d) — the insider WRITE window is closed where it matters; residual is named and bounded
- With **P-d** routing the JWT-trust branch through a `status='active'` re-read, a removed owner (membership flipped to
  `'removed'`/`'suspended'`) is denied **per-request, immediately** on the owner-WRITE surfaces (product-media,
  promotions, menu-import via the helpers; products/categories via `getOwnerLocationId`; `requireLocationAccess`
  routes). The in-flight ≤24h access token can no longer WRITE to a removed-from tenant through these helpers. That is
  the load-bearing insider gap the whole pivot now hinges on, and P-d closes it.
- **Residual (already DEFERRED/accepted, not a new finding):** any owner-data surface that resolves its tenant by some
  path **other than these four helpers** would still trust the baked `activeLocationId` for ≤24h. The implementer's DoD
  must therefore be "**every** owner-write tenant resolution goes through a `status='active'` re-read," not just the
  four named helpers — if a fifth resolution path exists or is later added, the insider write-window reopens there. This
  is an implementation-completeness obligation (cover with the P-d E2E + a guardrail that greps for status-blind
  `WHERE ... role='owner'` membership reads), already implied by proposal §4/§8; flagged here so it is not lost. It is
  NOT a new HIGH against the design — the design's enforcement point is correct; it is a "apply it exhaustively" note.

## REV-3 LOW notes (implementer hygiene — not blockers, not new severity)
- **L3 · B-OPS** — P-d's per-request membership re-read is **fail-closed** (errored check ⇒ deny the write, per
  proposal §7), which is correct for an authorization gate, but it converts a transient pool blip on an owner-write
  route from "succeeds" (today, JWT-trust, 0 reads) into "5xx, retry." This is the intended trade and bounded to the
  owner-write subset (not the hot path), but the `owner_write_scope_denied_total{route}` metric (proposal §9) should
  distinguish *authorization-deny* from *DB-error-deny* so an operator doesn't read a pool blip as an insider-removal
  spike. Hygiene, not a design hole.
- **L4 · B-CONSIST** — restating REV-2 R2-5 (already accepted LOW): "log out all devices" is synchronously guaranteed
  *except* against a same-instant concurrent refresh from one of the caller's own devices (the resurrected family is a
  single live token the caller just minted, bounded ≤24h). Unchanged; no regression.

## REV-3 top line
**The REV-2 fixes are clean. 0 new CRITICAL, 0 new HIGH.** P-d denies no legitimate owner (no benign non-active owner
state exists; every owner membership is `'active'`; no non-owner caller hits the modified query), does not re-introduce
the C1 RLS-lockout (its predicate is a strict subset of the already-enforced `app_member_location_ids()` status filter,
and it ENABLEs/FORCEs no RLS), and adds at most one indexed read per owner-write request off the global hot path. P-c's
preserve-check adds no query and cannot carry a stale tenant (it is gated by the live read). P-b cannot be triggered
unauthenticated and its all-devices semantics are disclosed. The integration closes the insider WRITE window on the
helper-protected owner-write surfaces — with the single implementer obligation to apply the `status='active'` re-read
**exhaustively** across every owner-write tenant-resolution path (DoD + a grep guardrail), not just the four named
helpers. **Design is hard-exit-ready.**
