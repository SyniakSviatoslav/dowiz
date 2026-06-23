# Design Proposal — Owner Access-Token Revocation (lean: decouple TTLs + real logout + refresh-boundary enforcement)

Slug: `owner-token-revocation`
Status: PROPOSED (design only — no product code). **RESOLVED to the lean design** (see `resolution.md`).
Red-line: AUTH. Forward-only (code-only; no schema migration). Reversible.
Author: System Architect (DeliveryOS)
Date: 2026-06-23 (rev 2 — pivoted after Breaker + Counsel)
Relates to: ADR-0004 (this change), ADR-0003 (dev-kid segregation), ADR-0001 (queue-in-Postgres).

> **rev 2 note.** rev 1 proposed a bespoke `token_version`-on-`users` per-request revocation layer
> (cache + Redis pub/sub + sticky-deny + fail-open + a `users` RLS self-read policy + 5 mint-site claim
> changes). The Breaker (`breaker-findings.md`) and Counsel (`counsel-opinion.md`) converged: that layer
> is over-engineered AND dangerous for this stage (C1 users-RLS lockout, C2/C3 unbounded fail-open),
> and a much leaner design dominates. This rev is rewritten around the lean design; the full per-request
> layer is **DEFERRED** with a recorded promotion trigger (§10). Per-finding dispositions: `resolution.md`.

---

## 1. Problem + non-goals

### Problem (audit HIGH)
Owner auth is **pure-JWT RS256** verified by **signature + `exp` only** (`apps/api/src/plugins/auth.ts:51-56`, `packages/platform/src/auth/jwt.ts:82-115`). There is no `jti`, no session row, no blacklist on the owner path. Access TTL is **7 days** on all owner mint sites (`auth/local.ts:146`, `auth/local.ts:68` dev-bypass, `auth.ts:148/223/294`). The per-request owner-scoping helpers (`get-owner-location.ts:11`, `product-media.ts:51`, `promotions.ts:19`, `plugins/auth.ts:147`) filter membership by `role='owner'` but **not** `status='active'` — verified live (P-d). Consequences:

1. **Theft window = 7 days.** A leaked/stolen owner access token is fully valid for 7 days. There is no "log out" the server honours — and the web client's "log out" button POSTs to `/api/auth/logout`, which **has no server route** (`packages/ui/src/lib/auth.ts:99-104`; only `routes/courier/auth.ts:478` has a `/logout`). The headline revocation trigger is wired to a 404.

### 1.2 Privilege roll-forward — CORRECTED framing (was overstated)
`role` + `activeLocationId` are baked at mint. A membership revoke or role downgrade does **not** invalidate an already-minted **access** token until its `exp`. `/auth/refresh` re-mints a fresh token from the refresh family (`auth.ts:288-302`).

**Correction (verified against live source — Counsel §0):** rev 1 said a removed owner "rolls forward **indefinitely**." That is **false**. The refresh handler already:
- mints from `memberships WHERE ... status = 'active'` (`auth.ts:289`) — so a *membership-revoke* is **already honoured at the next refresh**, with `activeLocationId` resolving to `null` for a removed owner; and
- writes refresh tokens with **`now() + interval '7 days'`** (`auth.ts:300`, `local.ts:153`) — the refresh *family* expires; a continuously-refreshing session is re-stamped to **7d-from-last-refresh**, not forever.

So the true blast radius is **bounded by the 7d refresh family** (the rolling bound is 7d-from-last-refresh), not indefinite, and membership status is *already* re-read on refresh. The real gap is narrower and slower than rev 1's prose. **Caveat (F-N, §5):** the OAuth (`auth.ts:154`) and Telegram (`auth.ts:228`) first-login inserts use `now() + interval '30 days'`, not 7d — so the *idle* (non-refreshing) family upper bound on those two paths is 30d. We normalize them to 7d so the "bounded by 7d" statement is true on every login path.

Three real gaps remain after the correction: **(i)** the in-flight **access** token survives up to 7d regardless of refresh-side state, and the only refresh-side guard that closes the roll-forward (`auth.ts:288-292`) has holes (H4): it re-reads memberships but hard-codes `role:'owner'` (`refreshedOwnerClaims`, `auth.ts:20`), so a *downgraded* owner who still holds *any* active membership re-mints a fresh **owner** token. **(ii)** there is no server-honoured logout to kill the family early. **(iii)** the **insider-removal** gap (R2-1): the per-request owner-scoping helpers don't filter `status='active'` and several owner-write routes trust the baked JWT `activeLocationId` with no membership re-read at all, so a removed/downgraded owner keeps **write** access (create promotions, import a menu, mutate media) to the tenant they were removed from for the **full ≤24h access life** — independent of the refresh boundary (they never need to refresh within 24h). This is the load-bearing insider write-window; P-d closes it per-request.

### Goal
Bound the blast radius with the **cheapest correct** changes for a ~50-location, no-compliance-mandate, pre-launch SaaS:
- shrink the leaked-**access**-token window from 7d to ≤24h;
- make "log out all devices" actually kill the roll-forward path **synchronously**;
- enforce membership-revoke / role-downgrade at the **refresh boundary** (≤24h);
- deny a **removed/downgraded owner** tenant **write** access **per-request, immediately** on the owner-write routes (P-d — the insider-removal fix), via an index-backed `status='active'` predicate;
- **without** (a) per-request DB latency / pool starvation on the **global hot path** (P-d's read is confined to the owner-write routes that already touch the DB, not every authenticated request), (b) regressing the 7d **no-relogin** UX, (c) any `users`-RLS migration, fail-open window, cache, or pub/sub.

### Non-goals
- Not touching the **courier** path (already DB-bound sessions).
- Not touching the **customer** token (order-scoped, low blast radius).
- Not building the owner-staff-management UI (membership delete / role-change endpoints do **not exist yet** — confirmed by grep). This design enforces *at the refresh boundary* (P-c) **and per-request on the owner-write routes** (P-d) whatever those endpoints will write to `memberships.status`; it does not create them. **ETHICAL-STOP-1 (§9) gates that UI when it ships.**
- Not building **per-device** logout (the client sends only the access token, no refresh token in the logout body — `ui/auth.ts:99-102`). P-b ships the honest user-wide "log out all devices"; per-device is a deferred future enhancement (R2-4 sub-1).
- Not building the full **per-request immediate (<1min) access-token revocation** layer — **DEFERRED** (§10) with a recorded trigger.

---

## 2. Back-of-envelope

### Owner request volume
- Scale (Context-Handoff v4.5 envelope): **N = 50 locations**, growth target **500**. Owners are operators, not the public.
- Active owner sessions in a busy hour: ≈ 1.5 × locations = **~75** at N=50, **~750** at N=500.
- Owner request rate ≈ **0.2 req/s/owner** sustained ⇒ aggregate **~15 req/s** today, **~150 req/s** at 10× growth. WS upgrades amortize (one verify per connect).

### The crux the envelope settles
The whole rev-1 edifice (cache + pub/sub + fail-open) existed to avoid a **per-request DB read** at the *hypothetical 10×* (150 checkouts/s). **The lean design adds ZERO per-request work** — the hot path stays exactly today's pure signature+exp verify. So the envelope no longer has to defend a cache: there is nothing on the hot path to optimize. The connection budget (API + worker + analytics + migrations) is **unchanged from today**.

The only added DB work is at the **refresh boundary** (P-c) — which **already runs the memberships SELECT** (`auth.ts:289`), so it is **0 new queries**; and at **logout** (P-b) — one `DELETE` per explicit logout (rare, operator/owner-initiated, not on any request hot path). Both are off the per-request path entirely.

### Leak-window math
| Lever | Before | After (lean) | Cost |
|---|---|---|---|
| Leaked **access** token validity | 7d | **≤24h** (P-a, 4 refresh-backed sites; dev-bypass stays 7d) | revert a literal to undo |
| Removed/downgraded owner — tenant **write** access on owner-write routes | full ≤24h (status-blind helpers, JWT-trusted) | **denied per-request, immediate** (P-d `status='active'`) | indexed predicate, no schema |
| Membership-revoke / role-downgrade — token role/tenant | next refresh, but re-mints `owner` anyway (H4) | **next refresh, ≤24h**, correctly downgraded/401, working tenant preserved (P-c) | tighten existing handler |
| "Log out (all devices)" | 404 no-op | **synchronous user-wide family delete** (P-b) | one authenticated additive route |
| Idle refresh-family upper bound | 7d (pw/refresh) / **30d** (OAuth/TG) | **7d uniform** (F-N) | code-only literal |

Silent refresh (`apiClient.ts:120-127`) means cutting **access** 7d→24h costs **zero** relogin prompts **on the 4 refresh-backed mint sites** — access and refresh TTLs are **decoupled** (Counsel §3.1, §4; verified). The **dev-bypass** mint is excluded from P-a precisely because it issues **no** refresh token (R2-3): cutting it to 24h would force a daily staging relogin with no silent-refresh recovery for zero security gain, so it stays at 7d.

---

## 3. Options (≥2, with concept names + tradeoffs)

### Option D′ — **Decouple TTLs + real logout + refresh-boundary enforcement** (CHOSEN)
**Concept:** *short-access + silent-refresh* (the access/refresh-TTL-decoupling pattern) + a real **session-family revocation** endpoint (using the existing `auth_refresh_tokens` family + reuse-detection) + **authorization re-evaluation at the refresh boundary**. No per-request server state.
- **Concept lineage:** this is the standard "short access token, long rotating refresh, revoke the refresh family" pattern (OAuth2 refresh-token rotation). The no-relogin UX comes from silent refresh, not access longevity.
- **Tradeoffs**
  - + **Zero** hot-path cost (hot path unchanged). + No schema migration, no `users` RLS, no cache, no pub/sub, no fail-open dilemma. + Reuses the **already-existing, already-tested** refresh-family + reuse-detection (`auth.ts:262-283`). + Reversible (revert literals / drop an additive route). + Closes the membership/role roll-forward at the refresh boundary with **0 new queries**.
  - − Residual: a leaked **access** token is valid up to **≤24h** (not <1min). Accepted (§10).
  - − Coarse: logout is per-family / per-user, not per-arbitrary-device beyond family scope. Accepted/deferred.

### Option A — **Token-version / auth-epoch column + per-request cached read** (REJECTED — was rev 1's choice)
**Concept:** monotonic `token_version` on `users`; access JWT carries `tv`; verify compares against current value via a per-machine 30s-TTL LRU + Redis pub/sub eviction; fail-open on store error with a sticky hard-revoke escape hatch.
- **Why rejected (Breaker C1/C2/C3/L1/L2, Counsel §3-4):** the recommended `users` RLS self-read policy is a **total-owner lockout** on the NOBYPASSRLS operational pool with an unset GUC (C1), and is **not** flag-reversible (L2). The "≤30s floor" is **false**: fail-open + at-most-once Upstash pub/sub + per-machine LRU + cold-start ⇒ a revoked token can be served **indefinitely** under the store degradation *correlated* with the revoke-triggering incident (C2). The fail-closed escape hatch is itself conditional on a healthy DB (C3). Four moving parts on the most safety-critical path, for ~23.99h of additional tightening over D′ on one sub-case. **Dominated.**

### Option B — **Per-session `jti` + `owner_sessions` table** (mirror couriers)
**Concept:** every owner mint inserts a session row; verify does a `SELECT ... WHERE id = jti AND revoked_at IS NULL AND <membership valid>` per request — the courier path.
- **Tradeoffs:** + per-device granularity, + immediate (0s). − **one PG checkout on every owner request** on a pool already guarded with 503s (~150/s at 10× growth). Rejected as the hot-path mechanism. **Kept as the DEFERRED upgrade path** (§10) if per-device immediate revocation is ever mandated.

### Option C — **Redis deny-list on revoke** (TTL = remaining token life)
**Concept:** pure-JWT tokens; on revoke, add `userId`+`iat`-cutoff to Redis; verify checks every request.
- **Tradeoffs:** + no PG cost, but − a hard Redis read on **every** owner request + the same fail-open/closed dilemma as A, paid per request. Rejected on the hot path.

---

## 4. Decision + rationale (ADR-0004)

**Decision: Option D′ — decouple TTLs + real logout + refresh-boundary enforcement. No per-request server state. The full per-request immediate-revocation layer (A or B) is DEFERRED (§10).**

Specifically (all code-only; no schema migration):

- **P-a — cut owner ACCESS TTL `'7d'` → `'24h'`** on the **4 real (refresh-backed) mint sites**: argon2 (`local.ts:146`), OAuth (`auth.ts:148`), Telegram (`auth.ts:223`), refresh re-mint (`auth.ts:294`). **EXCLUDE the dev-bypass mint** (`local.ts:68`) — it returns **no refresh token** (`local.ts:69`, verified), so 24h there means a forced **daily** staging re-login with **zero** silent-refresh recovery (`apiClient.ts` skips the refresh branch when no `dos_refresh_token` is stored) for **zero** security gain (the dev token runs under the dev keypair a prod verifier rejects, ADR-0003). Keep the dev-bypass at `'7d'` (R2-3; both critics agree). **KEEP** the refresh family at 7d. The web client silently refreshes on 401 (`apiClient.ts:120-127`), so the owner **never sees a relogin prompt** — access/refresh TTLs are decoupled. Net: leaked-access window **7d → ≤24h**, reversible by reverting the literal.

- **P-b — add `POST /api/auth/logout`** (closes H3), **authenticated**. The route **must** require a currently-valid credential (`verifyAuth`); a no-auth force-logout keyed on a presented (possibly replayed/already-rotated) token is a DoS vector (R2-4 sub-2). Derive `userId` from the **access token** (the credential the client actually sends — `ui/auth.ts:101` sends `Authorization: Bearer ${access_token}` with **no body**, verified) and do a **USER-WIDE** `DELETE FROM auth_refresh_tokens WHERE user_id = $1` — "log out all devices" semantics, the honest, simple default. This is the credential-honest design: the client sends no refresh token, so per-family ("this device") logout is **not implementable** from the current call and is **deferred** as a future enhancement (it requires the client to send the refresh token in the body — R2-4 sub-1). Wire `packages/ui/src/lib/auth.ts:99-104` (currently 404s). After the family is deleted, the in-flight access token still expires within ≤24h (P-a); the **synchronous, guaranteed** kill is the family delete, the access exp is the bounded tail.

- **P-c — fix `/auth/refresh`** (closes H4 + R2-2 misroute). Two coupled fixes to `auth.ts:288-294`:
  1. **Role:** re-derive `role` from the fresh memberships read; **401/relogin if no active *owner* membership remains** (do not re-mint owner for a user whose only surviving membership is non-owner); stop hard-coding `role:'owner'` (`refreshedOwnerClaims`, `auth.ts:20`).
  2. **activeLocationId (R2-2):** **preserve the caller's CURRENT `activeLocationId`** (carried on the incoming token) if it is still a *valid active owner membership* for that user. Only fall back to a **deterministic** pick (`ORDER BY created_at, id` — never the current tiebreaker-less `ORDER BY (role='owner') DESC LIMIT 1`, which returns an arbitrary row) if the carried location is gone. **Never silently swap a multi-location owner's working tenant.** 401/relogin only when **no** active owner membership remains. P-a cuts refresh cadence 7d→24h (~7× more frequent), which is exactly why the non-deterministic re-derive must be fixed now — it would otherwise misroute ~7× more often.

  Both enforce membership-revoke / role-downgrade at the **refresh boundary (≤24h)** with **0 new queries** (the SELECT already runs).

- **P-d — add `AND status='active'` to the shared owner-scoping helpers** (the load-bearing insider-removal fix, R2-1). The per-request tenant-scoping helpers filter by `role` but **not** `status`, so a removed/downgraded owner (membership flipped to `status='revoked'`/`'suspended'`) keeps tenant write access for the full ≤24h access life — **independent of P-c** (they never need to refresh within 24h). Verified live, four sites:
  - `requireLocationAccess` (`plugins/auth.ts:146-149`): `SELECT 1 FROM memberships WHERE location_id=$1 AND user_id=$2 AND role='owner'` — **no `status`** ⇒ a revoked membership still returns rowCount 1.
  - `getOwnerLocationId` (`lib/get-owner-location.ts:8-14`), inline `getOwnerLocation` (`product-media.ts:49-55`), `getLocationId` (`promotions.ts:14-24`): all **(a)** trust `user.activeLocationId` from the JWT with **zero** DB check, and **(b)** fall back to `... WHERE user_id=$1 AND role='owner' LIMIT 1` — **no `status`**.

  **Fix:** add `AND status='active'` to every one of those four membership reads. This is index-backed — the partial indexes `memberships_user_id_active_idx` and `memberships_location_role_active_idx` (`core-identity.ts:64-65`) already cover `WHERE status='active'`, so the predicate is **free**. This denies a removed/downgraded owner **PER REQUEST (immediate, not ≤24h)** on the helper-protected routes — the actual money/tenant write surface (product-media, promotions, menu-import, orders).

  **The unresolved gap (decide):** the **JWT-trust branch** — `getOwnerLocationId`/`getOwnerLocation`/`getLocationId` return `user.activeLocationId` straight from the token **before** the fallback query ever runs (`get-owner-location.ts:8`, `product-media.ts:49`, `promotions.ts:15-16`). Adding `status` to the *fallback* does nothing for the common case where the JWT carries `activeLocationId`. Two routings:
  - **(P-d preferred — route through the helper):** make the JWT-trust branch **also** do the `status='active'` membership re-read for the carried `activeLocationId` (one indexed PK-ish read per owner request **only on these specific owner-write routes** — not the global hot path; these routes already do per-request DB writes, so one indexed membership SELECT is in-budget). This closes the insider write-window **immediately** on every owner-write surface.
  - **(accept-risk ≤24h):** if even that per-route read is judged too costly, accept-risk the JWT-trust branch at ≤24h with a named owner — but then the insider write-window the DEFERRED ≤24h risk leaves open stays open on these routes. **Recommendation: P-d preferred** — the read is index-backed, scoped to a handful of owner-write routes (not the hot path), and closes the highest-value insider gap for the cost of one indexed predicate.

**Where enforcement lives:** the **refresh boundary** (≤24h cadence, P-c) + **explicit logout** (synchronous, P-b) + a **per-request `status='active'` membership re-check on the owner-write routes** (immediate, P-d). The **global hot path** (`plugins/auth.ts:44-92` owner branch) is **unchanged** — P-d's per-request read is confined to the owner-write surfaces that already touch the DB, not every authenticated request.

**Fail behaviour:** there is **no new fail-open/fail-closed dilemma** — the hot path adds no dependency. The refresh handler's existing posture is preserved (a DB error on refresh fails that one refresh; the client retries / re-logins; no fleet-wide effect). The synchronous logout `DELETE` either succeeds (family gone) or errors (client surfaces it and the ≤24h access exp still applies) — no silent success.

**Why not A/B/C platform-wide:** A is a total-owner-lockout + unbounded-fail-open landmine for ≤24h of tightening over D′ (Breaker C1/C2/C3). B/C cost a per-request checkout/RTT on the global hot path against a guarded pool. D′ closes the dominant share of the gap (window 7d→≤24h, the insider-removal write-window per-request via P-d, plus the H3/H4 correctness bugs) at near-zero surface area on the most safety-critical path.

**Minimal + reversible:** P-a is a literal revert; P-b is an additive route (drop to undo); P-c is a handler tightening (revert to undo); P-d is a one-predicate addition (`AND status='active'`) per helper (revert to undo) — index-backed, no schema. **No migration, no flag needed** for the enforcement (these are honest, always-on behaviour changes — there is no dark-deploy window in which an operator could believe they revoked when they didn't, because there is no gated read; cf. ES-1).

---

## 5. Data / migrations (forward-only) — NONE for the chosen design

**No schema migration.** The chosen design is **code-only**. The `auth_refresh_tokens` table, its `family_id`, and reuse-detection **already exist** (`auth.ts:234-305`). No new column, no `tv`, no `auth_revoked_at`, **no `users` RLS change** (this is what removes the C1/L2 landmine entirely). **P-d** is also migration-free — it only adds an `AND status='active'` predicate to existing membership SELECTs; the `memberships.status` column and the `WHERE status='active'` partial indexes already exist (`core-identity.ts:60,64-65`), so the predicate is index-backed and free.

Code-only fixes folded in:
- **F-N — normalize refresh-family TTL.** Change the two first-login inserts from `now() + interval '30 days'` (`auth.ts:154` OAuth, `auth.ts:228` Telegram) to `'7 days'`, matching `local.ts:153` and the refresh re-mint `auth.ts:300`, so the "bounded by 7d" statement holds on every login path. No migration (these are inline SQL literals).
- **F-comment — fix stale "1h" comments.** `apps/web/src/lib/apiClient.ts:7,119` describe a "1h" access token the code does not mint; update to "24h" when P-a lands so the decoupled-TTL rationale reads true.

**Integer / money / RLS FORCE:** N/A — no schema touched.

If the DEFERRED layer (§10) is ever promoted, *that* change carries the migration and its breaker gate (C1 becomes a load-bearing migration-correctness CRITICAL — read via `withTenant`, never a bare operational-pool query against a FORCE'd `users`; reversibility proven before merge).

---

## 6. Consistency + idempotency

- **Logout (P-b) idempotency:** `DELETE FROM auth_refresh_tokens WHERE user_id = $1` is naturally idempotent — a second logout deletes zero rows, harmless. No exactly-once needed.
- **Logout vs in-flight refresh race (R2-5, LOW):** a P-b `DELETE ... WHERE user_id=$1` racing a `/auth/refresh` rotation can be partly resurrected — if the refresh's atomic claim (`auth.ts:262-265`) lands *after* the logout DELETE, it INSERTs a fresh row for the same family (`auth.ts:298-301`), surviving until its own 7d expiry. **Mitigation (already present):** the rotation's single-use guard (`used=true` flip) + family reuse-detection (`auth.ts:266-283`) means the resurrected family is a *single live token* the racing client just minted — not a re-opened attacker path; and it is still bounded by the ≤24h access exp. **Accepted as LOW.** A future tombstone (delete-that-blocks-reinsert) would make "log out = synchronous guaranteed kill" hold against a concurrent refresh; not needed at this stage. Note for ES-2 copy: "log out all devices" is synchronously guaranteed *except* against a same-instant concurrent refresh from one of the caller's own devices.
- **Refresh atomicity (P-c):** unchanged — the existing guarded single-use `UPDATE ... WHERE used=false` (`auth.ts:262-283`) and its benign-concurrent-vs-replay distinction are preserved. P-c only **tightens the claims minted** after the atomic claim succeeds (re-derive role + 401 if no owner membership). A logout racing a refresh: if the family `DELETE` lands first, the refresh's `SELECT ... WHERE token_hash` returns 0 rows ⇒ 401 (correct); if the refresh's atomic claim lands first, it mints a ≤24h token and the next logout deletes the rotated family (correct). Both bounded by ≤24h.
- **Money:** N/A.

---

## 7. Failures + degradation (every external call: timeout + fallback, zero cascade)

| Dependency | Failure | Degradation |
|---|---|---|
| **Hot-path verify** | — | **Unchanged from today** (pure signature+exp). No new dependency, no new failure mode, no fail-open question on the hot path. |
| **`/auth/refresh` memberships read (P-c)** | pool checkout reject / DB error | The single refresh fails (401 / client retries or re-logins). **No fleet-wide effect** — only the refreshing owner is affected, and silent-retry covers a transient blip. Same posture as today's refresh handler. No cascade. |
| **`POST /api/auth/logout` DELETE (P-b)** | DB error | Surface a 5xx to the client; the client has **already cleared local tokens** (`auth.ts:97`), and the ≤24h access exp still bounds the server-side tail. The UI copy must say "signing out" not "✓ signed out everywhere" (ES-2). No silent success. |
| **Owner-write route `status='active'` membership read (P-d)** | pool checkout reject / DB error | The single owner-write request fails (5xx); the client retries. **No fleet-wide effect** — confined to that one owner request on that one route; the global hot path has no such read. Fail-closed (an errored membership check denies the write), which is the correct posture for a tenant-write authorization gate. No cascade. |
| **Access token at ≤24h exp** | normal | Silent refresh re-mints (no relogin) unless the family was revoked, in which case the owner re-logins — correct. |

Timeouts: the refresh memberships SELECT uses the existing operational pool's short `connectionTimeoutMillis` (no head-of-line cascade). No new long-lived dependency added.

---

## 8. Security + tenant isolation

- **Leak window:** owner access token 7d → **≤24h** (P-a). The existing refresh reuse-detection (`auth.ts:262-283`) already kills the family on replay, so a thief trying to *extend* a stolen session past ≤24h via the refresh token trips family-revoke.
- **Insider-removal write-window CLOSED per-request (P-d, R2-1):** the load-bearing fix. The per-request owner-scoping helpers (`get-owner-location.ts:11`, `product-media.ts:51`, `promotions.ts:19`, `plugins/auth.ts:147`) today filter `role='owner'` but **not** `status='active'`, and three of them trust the baked JWT `activeLocationId` with no DB re-read — so a removed/downgraded owner keeps **write** access (promotions, menu-import, product-media, orders) to a tenant they were removed from for the **full ≤24h** access life, independent of refresh. P-d adds `AND status='active'` to all four membership reads (index-backed; `core-identity.ts:64-65`) and routes the JWT-trust branch through a per-route `status='active'` re-check, denying a removed owner **immediately, per request** on the owner-write surfaces. **Correction (do not claim more):** P-d closes the insider write-window on the **helper-protected / owner-write routes**; it does **not** make the in-flight access token universally inert (that is the DEFERRED <1min layer). The earlier draft's blanket claim that "tenant-isolation staleness is closed" was **false against live source** and is withdrawn — staleness is closed *on the owner-write routes via P-d* and *at the refresh boundary via P-c*, not on every conceivable surface.
- **Tenant isolation at refresh (P-c):** P-c re-derives `role` from a live membership and 401s a removed/downgraded owner at refresh, while **preserving** the caller's working `activeLocationId` if it is still a valid active owner membership (R2-2 — never silently swap a multi-location owner's tenant). No new cross-tenant surface.
- **RLS:** **no `users` RLS change** — the C1 total-owner-lockout vector is removed from scope. No FORCE-bypass introduced.
- **No new bearer secret, no cookies** (RS256-only, zero-cookies invariant preserved).
- **Dev-kid (ADR-0003):** unaffected — P-a only shortens the dev-bypass token's TTL (`local.ts:68`), which already runs under the dev keypair a prod verifier rejects.
- **Force-logout authority:** the owner themselves (P-b — authenticated, user-wide, scoped to the caller's own `user_id` derived from the token) or an operator (runbook: `DELETE FROM auth_refresh_tokens WHERE user_id = $compromised`) or the system (membership/role enforced at refresh by P-c and per-request by P-d). No tenant can revoke another tenant's session — P-b requires a currently-valid credential and scopes strictly to the caller's own `user_id` (a no-auth force-logout would be a DoS, R2-4 sub-2; rejected).

---

## 9. Operability

- **Operator force-logs-out a compromised owner (runbook):**
  1. `DELETE FROM auth_refresh_tokens WHERE user_id = $compromised;` — **synchronous, guaranteed** kill of the roll-forward path (no cache, no store-health dependency).
  2. Wait out the in-flight access token: **≤24h** (P-a). This is the *honest* bound — there is **no** instant kill of an already-minted access token in this design (that is the DEFERRED layer, §10).
  → Document in `docs/runbooks/`. The runbook must state plainly: **the only synchronously-guaranteed kill is the refresh-family delete; the access token expires within ≤24h, not instantly** (ES-2).
- **ETHICAL-STOP-1 (silent-no-op "Remove owner" / "Log out"):** in this design there is **no enforcement flag** and the logout button is wired to a **real synchronous DELETE** (P-b), so it cannot silently no-op. The residual stop applies to the **future staff-management "Remove owner" UI**: when it ships, a **recorded human decision** is required that the action is enforced at the refresh boundary (P-c) and its copy does not claim an instant access-token kill. Friction, not veto. Owner: Product + Architect.
- **ETHICAL-STOP-2 (copy must not out-run the guarantee):** the "log out (everywhere)" confirmation copy must reflect eventual semantics — e.g. *"Signed out on this device; other devices sign out within 24 hours."* **Not** "✓ Signed out everywhere" with an implied instant kill. Owner: Architect → UX.
- **Metrics (observability < 1 min):**
  - `owner_auth_logout_total` — explicit "log out all devices" (user-wide).
  - `owner_auth_refresh_downgrade_401_total` — refreshes 401'd because the owner membership was removed/downgraded (P-c) — expected to spike right after a membership change, then settle.
  - `owner_auth_refresh_fail_total` — refresh DB errors (transient-blip visibility).
  - `owner_write_scope_denied_total{route}` — owner-write requests denied by P-d's `status='active'` re-check (removed/downgraded owner hitting a tenant they no longer belong to) — a spike here right after a membership change is the insider-removal fix working.
- **Health: degraded vs down.** No new hot-path dependency ⇒ **no new health signal**. Refresh/logout DB errors surface through the existing pool-health signal; they do **not** flip `/health` to down (a refresh blip is per-owner, not fleet-wide).
- **Rollback:** revert P-a literals (TTL back to 7d), drop the P-b route, revert the P-c handler tightening, revert the P-d `status='active'` predicates. No data to undo. Reversible in one PR revert.
- **Scaling gate / flag:** none required for P-a/b/c (honest, always-on). The **DEFERRED** layer (§10) is the flag-gated, staging-proof-gated future work.
- **Proof (Mandatory Proof Rule):** see `resolution.md §6` — Playwright owner-login + survive-past-1h (P-a no-regression), logout-then-all-families-401 (P-b user-wide), removed-owner-refresh-401 + multi-loc-no-swap (P-c / R2-2), removed-owner-blocked-on-owner-write-route (P-d / R2-1) on staging; red→green regression + ledger row (AUTH red-line).

---

## 10. Open / accepted / deferred risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | A leaked owner **access** token is valid up to **≤24h** (not <1min). Immediate kill of an already-minted, not-yet-refreshed access token, **on surfaces other than the owner-write routes**, is out of scope. (The insider-removal **write** window on the owner-write routes is now closed per-request by P-d — see R2-1.) | **Accepted-risk + DEFERRED.** Matches the threat model (small, no-compliance-mandate, ~50-loc pre-launch). **Promotion trigger:** (1) a real/imminent owner-token-compromise incident where ≤24h is unacceptable; (2) a compliance/contractual sub-hour-revocation mandate; (3) observed abuse during the ≤24h window at volume. When promoted, build Option A (correctly bounded) or B — *with* the breaker gate (C1: `users` RLS read via `withTenant`, never bare operational-pool query against FORCE'd `users`; C2/C3: fail-closed-capable without store-health dependency; M3: cold-start modeled). | Architect (scope) + Product (priority) |
| R2-1 | **Insider-removal write-window (HIGH).** Status-blind owner-scoping helpers + JWT-trusted `activeLocationId` ⇒ a removed/downgraded owner keeps tenant **write** access for the full ≤24h on owner-write routes (promotions/menu-import/product-media/orders), independent of P-c. | **FIX (P-d, IN SCOPE).** Add `AND status='active'` to all four helper membership reads (index-backed, free) **and** route the JWT-trust branch through a per-route `status='active'` re-check ⇒ removed owner denied **per-request, immediate**, on the owner-write surfaces. Cheap (a status predicate), closes the write-window the DEFERRED ≤24h risk left open. The false "tenant-staleness closed" claim is withdrawn (§8) and replaced with this concrete enforcement. **If the per-route JWT-trust re-read is judged too costly:** accept-risk that one branch at ≤24h with a named owner (Architect) — but recommendation is the re-read (in-budget on routes that already write the DB). | Architect (FIX) |
| R2-2 | **Refresh misroute (HIGH).** `/auth/refresh` re-derives `activeLocationId` via `ORDER BY (role='owner') DESC LIMIT 1` with **no tiebreaker** (`auth.ts:288-292`) ⇒ a multi-location owner's working tenant can silently swap on refresh; P-a's 24h cadence amplifies the rate ~7×. | **FIX (P-c).** Preserve the caller's CURRENT `activeLocationId` if it is still a valid active owner membership; fall back to a **deterministic** pick (`ORDER BY created_at, id`) only if it's gone; 401 only when no active owner membership remains. Never silently swap a working tenant. E2E: multi-location owner refresh keeps L2, never swaps to L1. | Architect (FIX) |
| R2-3 | **Dev-bypass at 24h (MED).** The dev-bypass mint returns no refresh token (`local.ts:69`), so 24h there = daily staging relogin, zero security gain. | **FIX (exclude from P-a).** P-a touches the **4 real refresh-backed sites only**; dev-bypass (`local.ts:68`) stays `'7d'`. Both critics agree. | Architect (FIX) |
| R2-4 | **Logout auth posture (MED).** Client sends Bearer access token + no body (`ui/auth.ts:99-102`); a no-auth force-logout keyed on a presented token is a DoS. | **FIX (P-b).** Route is **authenticated** (`verifyAuth` required); derive `userId` from the token; **user-wide** `DELETE ... WHERE user_id=$1` ("log out all devices"). Per-device logout needs the refresh token in the body ⇒ **deferred future enhancement** (R2 below). | Architect (FIX) |
| R2-5 | **Logout vs in-flight refresh race (LOW).** A concurrent refresh can resurrect the just-deleted family for one token. | **Accepted (LOW).** Mitigated by the rotation's `used=true` single-use guard + family reuse-detection; bounded by ≤24h. A future tombstone makes it synchronous-guaranteed; not needed now. ES-2 copy notes the caveat. | Architect |
| R2 | Coarse logout granularity: **user-wide ("all devices") only**, not per-device. Per-device requires the client to send the refresh token in the logout body (it currently sends only the access token). | **Accepted / deferred future enhancement.** Per-device beyond family-scope ⇒ client wiring change + the `owner_sessions` table (Option B). YAGNI now; the honest default is "log out all devices." | Product |
| R3 | P-c predicate mis-scope could 401 a *valid* owner or **misroute** a multi-location owner (R2-2). | **FIX-spec'd (R2-2).** The predicate: preserve the carried `activeLocationId` iff it is an active `role='owner'` membership for this user; else deterministic fallback (`ORDER BY created_at, id`); 401 only when no active owner membership; if a non-owner membership survives, mint the *lower* role rather than 401-loop. Cover with E2E (downgrade → non-owner token or relogin; multi-loc → no silent swap; never a silent owner re-mint). | Architect |
| R4 | F-N (30d→7d OAuth/TG family) shortens the *idle* family on those two paths; an owner idle >7d on OAuth/TG must re-login. | **Accepted.** 7d idle re-login matches the password path already; the rolling/active session is unaffected (re-mint is 7d). Honesty of the "bounded by 7d" claim is worth the idle-edge re-login. | Architect |
| R5 | **HUMAN scope call:** build the full per-request layer now vs accept ≤24h? And: *has an owner token actually leaked, or are we hardening a HIGH audit finding with no incident?* (Counsel §5). | **Needs human decision.** Recommendation: accept ≤24h, defer (R1). If a real incident is on the table, promote per R1 trigger 1 and use the synchronous `DELETE FROM auth_refresh_tokens` + ≤24h cut *today*. | Human (audit owner) + Architect |
| R6 | **ETHICAL-STOP-1 (HUMAN):** the future staff-management "Remove owner" UI must not assert an effect the server delivers only at the ≤24h refresh boundary. | **Needs human decision when that UI ships.** Recorded gate in §9. | Product + Architect |
| R7 | **ETHICAL-STOP-2:** "log out everywhere" copy must reflect ≤24h eventual semantics for other devices. | **Fix** (copy + runbook, §9). No human gate. | Architect → UX |

---

## Concept summary (named, per mandate)
- **D′ = short-access + silent-refresh + session-family revocation + refresh-boundary authorization + per-request tenant-write re-authorization** (CHOSEN) — OAuth2 refresh-rotation pattern; access/refresh TTLs decoupled; no per-request server state on the global hot path. **P-a** (24h access, 4 refresh-backed sites) · **P-b** (authenticated user-wide logout) · **P-c** (refresh role re-derive + `activeLocationId` preservation) · **P-d** (`status='active'` in the owner-scoping helpers — the insider-removal fix, immediate per-request on owner-write routes).
- **A = token-version / auth-epoch + cached per-request read** — REJECTED (Breaker C1 users-RLS lockout, C2/C3 unbounded fail-open, L2 non-reversible RLS). Kept only as a DEFERRED option.
- **B = per-session jti + `owner_sessions` table** — courier-mirror; DEFERRED upgrade path for per-device immediate revocation (per-request checkout cost).
- **C = Redis deny-list** — rejected (hard Redis read every owner request + fail-open dilemma).
- Posture: **no hot-path dependency; synchronous family-delete for the acute case; ≤24h deterministic bound for the in-flight access token** (no cache, no pub/sub, no fail-open, no `users` RLS).
