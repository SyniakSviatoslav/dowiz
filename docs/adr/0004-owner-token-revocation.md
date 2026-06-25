# ADR-0004 — Owner access-token revocation: decouple TTLs + real logout + refresh-boundary enforcement

- Status: PROPOSED (design only; code-only — no schema migration). **Pivoted from rev 1** after Breaker + Counsel.
- Date: 2026-06-23 (rev 2)
- Red-line: AUTH. Forward-only (code-only). Reversible (revert literals / drop additive route).
- Supersedes/relates: ADR-0001 (queue-in-Postgres), ADR-0003 (dev-kid fail-closed). Does not contradict either.
- Full design: `docs/design/owner-token-revocation/proposal.md` · Resolution: `docs/design/owner-token-revocation/resolution.md`

> **rev 2.** rev 1 chose a bespoke `token_version`-on-`users` per-request revocation layer (cache +
> Redis pub/sub + sticky-deny + fail-open + a `users` RLS self-read policy + 5 mint-site claim changes).
> The Breaker proved that layer detonates the thing it protects (C1: the recommended `users` RLS
> self-read policy is a total-owner lockout on the NOBYPASSRLS operational pool; C2/C3: the "≤30s floor"
> is actually unbounded under the store degradation correlated with the incident; L2: the RLS is not
> flag-reversible). The Counsel showed a leaner design dominates. This ADR is the lean design; the full
> per-request layer is **DEFERRED**.

## Context

Owner auth is pure-JWT RS256, verified by signature + `exp` only (`apps/api/src/plugins/auth.ts:51-56`, `packages/platform/src/auth/jwt.ts:82-115`). Access TTL is 7 days on all owner mint sites (`auth/local.ts:146`, `auth/local.ts:68`, `auth.ts:148/223/294`) — a deliberate **no-relogin** UX win delivered by *silent refresh on 401* (`apps/web/src/lib/apiClient.ts:120-127`), **not** by access longevity. Access and refresh TTLs are **decoupled**.

Two HIGH audit findings, with the roll-forward framing **corrected** against live source:
1. A leaked/stolen owner access token is valid for a full 7 days — and the web "log out" button POSTs to `/api/auth/logout`, which **has no server route** (`packages/ui/src/lib/auth.ts:99-104`). The headline revocation trigger is wired to a 404.
2. `role`/`activeLocationId` are baked at mint. **Correction (Counsel §0, verified):** rev 1 said a removed owner "rolls forward **indefinitely**." That is **false** — `/auth/refresh` already mints from `memberships WHERE status='active'` (`auth.ts:289`) and refresh families expire at `now() + interval '7 days'` (`auth.ts:300`, `local.ts:153`). The roll-forward is **bounded by the 7d refresh family** (re-mint resets it, so the rolling bound is 7d-from-last-refresh), not indefinite. **But** the refresh close has holes (H4): it hard-codes `role:'owner'` (`refreshedOwnerClaims`, `auth.ts:20`), so a *downgraded* owner with any surviving membership re-mints a fresh **owner** token; and the OAuth/Telegram first-login inserts use `now() + interval '30 days'` (`auth.ts:154/228`), so the *idle* family bound on those paths is 30d, not 7d.
3. **Insider-removal (R2-1, the load-bearing gap):** the per-request owner-scoping helpers (`get-owner-location.ts:11`, `product-media.ts:51`, `promotions.ts:19`, `plugins/auth.ts:147`) filter membership by `role='owner'` but **not** `status='active'`, and three of them trust the baked JWT `activeLocationId` with no DB re-read — verified live. So a removed/downgraded owner (membership flipped to `status='revoked'`/`'suspended'`) keeps tenant **write** access (promotions/menu-import/product-media/orders) for the full ≤24h access life, **independent of refresh**. This is the insider write-window P-d closes per-request.

Couriers solve immediate revocation with DB-bound `courier_sessions` — but that does a PG checkout **per request**, unacceptable for owners on a pool already guarded with 503s (`auth/local.ts:77-83`) at the projected 15→150 req/s owner load.

## Decision

**Lean design — no per-request server state. Three code-only changes; no schema migration.**

- **P-a — cut owner ACCESS TTL `'7d'` → `'24h'`** on the **4 real (refresh-backed) mint sites** (argon2 `local.ts:146`, OAuth `auth.ts:148`, Telegram `auth.ts:223`, refresh re-mint `auth.ts:294`). **EXCLUDE the dev-bypass mint** (`local.ts:68`) — it returns no refresh token (`local.ts:69`), so 24h there = a daily staging relogin with no silent-refresh recovery for zero security gain (dev keypair, prod-rejected per ADR-0003); keep it `'7d'` (R2-3). **KEEP the refresh family at 7d.** Silent refresh preserves the no-relogin UX (access/refresh decoupled). Leaked-access window: **7d → ≤24h.**
- **P-b — add `POST /api/auth/logout`** (closes H3), **authenticated** (`verifyAuth` required — a no-auth force-logout is a DoS, R2-4). Derive `userId` from the access token (the credential the client actually sends — `ui/auth.ts:101`, Bearer access token, no body) and do a **user-wide** `DELETE FROM auth_refresh_tokens WHERE user_id = $1` ("log out all devices" — the honest, simple default). Per-device logout requires the client to send the refresh token in the body ⇒ **deferred future enhancement** (R2-4 sub-1). Wire `packages/ui/src/lib/auth.ts:99-104` (currently 404s). The user-wide delete is the **synchronous, store-health-independent, guaranteed** kill of the roll-forward path.
- **P-c — fix `/auth/refresh`** (closes H4 + R2-2): (1) re-derive `role` from the fresh memberships read, **401/relogin if no active *owner* membership remains**, stop hard-coding `role:'owner'`; (2) **preserve the caller's CURRENT `activeLocationId`** if it is still a valid active owner membership, falling back to a **deterministic** pick (`ORDER BY created_at, id`) only if it's gone — never the tiebreaker-less `ORDER BY (role='owner') DESC LIMIT 1`, which silently swaps a multi-location owner's working tenant (P-a's 24h cadence amplifies that misroute ~7×). Enforces membership-revoke / role-downgrade at the **refresh boundary (≤24h)** with **0 new queries** (the SELECT already runs).
- **P-d — add `AND status='active'` to the owner-scoping helpers** (the insider-removal fix, R2-1): `requireLocationAccess` (`plugins/auth.ts:147`), `getOwnerLocationId` (`get-owner-location.ts:11`), inline `getOwnerLocation` (`product-media.ts:51`), `getLocationId` (`promotions.ts:19`) all filter `role='owner'` but **not** `status='active'` — verified live — and three trust the baked JWT `activeLocationId` with no DB re-read. So a removed/downgraded owner keeps tenant **write** access (promotions/menu-import/product-media/orders) for the full ≤24h, independent of P-c. **Fix:** add `AND status='active'` to all four membership reads (index-backed; `core-identity.ts:64-65`) **and** route the JWT-trust branch through a per-route `status='active'` re-check, denying a removed owner **per-request, immediate** on the owner-write surfaces. This is migration-free (the `status` column + partial indexes exist). The per-route re-read is confined to the owner-write routes that already touch the DB — **not** the global hot path.
- **F-N — normalize** the OAuth/Telegram first-login refresh-family inserts from `'30 days'` to `'7 days'` so the "bounded by 7d" statement holds on every login path.

**Where enforcement lives:** the refresh boundary (≤24h cadence, P-c) + explicit logout (synchronous, P-b) + a per-request `status='active'` membership re-check on the owner-write routes (immediate, P-d). The **global hot path** (`plugins/auth.ts:44-92` owner branch) is **unchanged** — pure signature+exp, so there is **no new fail-open/fail-closed dilemma, no cache, no pub/sub, no `users` RLS change.** P-d's per-request read is on the owner-write surfaces only, not every authenticated request.

**Flag:** **none required.** P-a/b/c are honest, always-on behaviour changes — there is no gated read in which an operator could believe they revoked when they didn't (cf. the rev-1 silent-no-op trap, ETHICAL-STOP-1).

## Alternatives considered

- **A — token-version / auth-epoch + cached per-request read (rev 1's choice):** REJECTED. Breaker C1 (the recommended `users` RLS self-read policy is a total-owner lockout on the NOBYPASSRLS operational pool with an unset `app.user_id` GUC), C2/C3 (fail-open + at-most-once Upstash pub/sub + per-machine LRU ⇒ revocation gap unbounded under the store degradation correlated with the incident; the sticky-deny escape hatch is itself conditional on a healthy DB), L2 (the RLS is forward-only, not flag-reversible). Four moving parts on the most safety-critical path for ~23.99h of additional tightening over the lean design. **Dominated.** Kept only as a DEFERRED option.
- **B — per-session jti + `owner_sessions` table (mirror courier):** per-device + immediate, but a PG checkout per owner request (~150/s at 10× growth). Rejected as the hot-path mechanism; **kept as the DEFERRED upgrade path** if per-device immediate revocation is ever mandated.
- **C — Redis deny-list (TTL = remaining token life):** a hard Redis read on every owner request + the same fail-open dilemma. Rejected on the hot path.
- **"Do nothing":** rejected — the 404 logout and the H4 owner-re-mint-on-downgrade are real correctness gaps independent of the leak window.

## Consequences

- + Leaked owner access-token window: **7 days → ≤24h** — a 7× tightening with **zero new infra, zero schema, zero `users` RLS, zero fail-open window.**
- + "Log out all devices" now **synchronously** kills the refresh family user-wide (was a 404 no-op). Authenticated, scoped to the caller's own `user_id`.
- + Membership-revoke / role-downgrade enforced at the **refresh boundary (≤24h)** by P-c (role re-derive, no owner re-mint for a demoted user, working `activeLocationId` preserved) **and per-request, immediately, on the owner-write routes** by P-d (`status='active'` predicate). **Honest scope:** tenant-isolation staleness (baked `activeLocationId` outliving a membership) is **closed on the owner-write routes (P-d) and at the refresh boundary (P-c)** — *not* on every conceivable surface; the in-flight access token is not universally inert (that is the DEFERRED <1min layer). The earlier blanket "is closed" claim was false against live source (the scoping helpers don't filter `status`) and is withdrawn.
- + The hot path is **unchanged** — no per-request DB read, no cache, no pub/sub, no new health signal, no new way to be locked out.
- + Reversible: revert literals (P-a), drop the additive route (P-b), revert the handler tightening (P-c).
- − **Residual accepted risk:** a leaked, not-yet-refreshed **access** token is valid up to **≤24h** (not <1min). Immediate kill of an in-flight access token is **DEFERRED** (see below). The runbook and "log out everywhere" copy must state this honestly (ETHICAL-STOP-2): the *synchronously-guaranteed* kill is the refresh-family delete; the access token expires within ≤24h.
- − Coarse logout granularity (per-family / per-user, not per-arbitrary-device) — accepted; per-device deferred (Option B).
- − F-N shortens the *idle* family on OAuth/Telegram from 30d to 7d — accepted (matches the password path; active sessions unaffected).

## Deferred — full per-request immediate (<1min) access-token revocation

Out of scope until a recorded **promotion trigger** fires: (1) a real/imminent owner-token-compromise incident where ≤24h is unacceptable; (2) a compliance/contractual sub-hour-revocation mandate; (3) observed abuse during the ≤24h window at volume. When promoted, build Option A (correctly bounded) or B **with the Breaker findings as hard pre-conditions**: C1 — any `users` RLS change is a load-bearing migration-correctness CRITICAL, read via `withTenant` (sets `app.user_id`), never a bare operational-pool query against a FORCE'd `users`, reversibility proven before merge; C2/C3 — the revocation guarantee must be fail-closed-capable for known-compromised without depending on store health; M3 — cold-start/deploy miss-storm modeled against the guarded pool. Owner: System Architect (scope) + Product (priority).

## Open items / human decisions before implementation

- **HUMAN scope call (Counsel §5):** build the full per-request layer now vs accept ≤24h? And — *has an owner token actually leaked, or are we hardening a HIGH audit finding with no incident?* Recommendation: accept ≤24h, defer. If a real incident is on the table, use `DELETE FROM auth_refresh_tokens` + the ≤24h cut today and promote the layer per the trigger above. Owner: human audit-owner + Architect.
- **HUMAN — ETHICAL-STOP-1:** when the staff-management "Remove owner" UI ships, a recorded decision is required that the action is enforced at the refresh boundary (P-c) and its copy does not assert an instant access-token kill. Owner: Product + Architect.
- **ETHICAL-STOP-2 (fix, no gate):** "log out everywhere" copy + runbook must reflect ≤24h eventual semantics for other devices.
- **R3 (Architect):** spec the exact P-c membership predicate (R2-2): preserve the carried `activeLocationId` iff it is an active `role='owner'` membership for this user; else deterministic fallback (`ORDER BY created_at, id`); 401 only when no active owner membership; non-owner-only ⇒ mint the lower role, don't 401-loop. Cover with E2E (multi-loc owner → no silent tenant swap).
- **P-d JWT-trust-branch routing (Architect):** decide per R2-1 — route the `getOwnerLocationId`/`getOwnerLocation`/`getLocationId` JWT-trust branch through a per-route `status='active'` re-read (preferred — closes the insider write-window immediately, index-backed) vs accept-risk that branch at ≤24h with a named owner. Recommendation: route through the re-read.
- **Proof (Mandatory Proof Rule):** Playwright on staging — owner-login survives past 1h (P-a no-regression), logout-then-all-families-401 user-wide (P-b), removed/downgraded-owner-refresh-401 + multi-loc-no-swap (P-c / R2-2), removed-owner-blocked-on-owner-write-route immediately (P-d / R2-1); red→green AUTH-red-line regression + ledger row. See `resolution.md §6`.
