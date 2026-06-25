# Resolution — Owner Access-Token Revocation

Role: System Architect (DeliveryOS). RESOLVE round over the Breaker findings (`breaker-findings.md`)
and the Counsel opinion (`counsel-opinion.md`) against `proposal.md` + `docs/adr/0004-owner-token-revocation.md`.

Date: 2026-06-23. Forward-only, flag-gated where relevant, reversible. No product code.

---

## 0. Verdict — the lean design DOMINATES; the DECISION is pivoted

The bespoke `token_version`-on-`users` per-request revocation layer (cache + Redis pub/sub +
sticky-deny + fail-open + RLS self-read policy + 5 mint-site claim changes) is **withdrawn as the
chosen design**. Three independent lines of evidence converge:

1. **Breaker C1 + L2:** the layer's own recommended hardening (R1: ENABLE+FORCE+self-read on `users`)
   is a guaranteed total-owner lockout on the NOBYPASSRLS operational pool with an unset `app.user_id`
   GUC, and is *not* reversible by the flag (the RLS is forward-only). The mitigation detonates the
   thing it protects.
2. **Breaker C2 + C3 + L1:** the "≤30s guaranteed floor" does not exist. Fail-open + at-most-once
   Upstash pub/sub + per-machine LRU + cold-start ⇒ a revoked token can be served *indefinitely* under
   the store degradation that is *correlated* with the incident that triggered the revoke. The one
   fail-closed escape hatch (sticky-deny) is itself conditional on a healthy DB.
3. **Counsel §4 + §3.1 (verified against live source):** access TTL and refresh TTL are **decoupled**
   (`apps/web/src/lib/apiClient.ts:120-127` silently refreshes on 401), so cutting the **access** TTL
   7d→24h shrinks the leak window 7× with **zero new infra, zero fail-open dilemma, zero RLS landmine,
   zero new way to be locked out or to lie to an operator.** The membership recheck the layer "invented"
   already runs in `/auth/refresh` (`apps/api/src/routes/auth.ts:289`).

The lean design closes the dominant share of the gap (window 7d→≤24h, plus the H3/H4 correctness
bugs — and, after REV-2, the insider write-window per-request via P-d) at a fraction of the surface
area on the most safety-critical path in the system. **It dominates.** The full per-request
immediate-revocation layer is **DEFERRED** (documented, flag-ready, accepted-risk), with a recorded
promotion trigger.

The proposal and ADR-0004 are rewritten around the lean design (PRIMARY a/b/c). The FRAMING ERROR
(roll-forward "indefinite") is corrected. The two ETHICAL-STOPs are carried into the rewrite.

---

## 1. The resolved (PRIMARY) design — what we DO now

> **REV-2 update (below, §7).** P-a is now **4 sites** (dev-bypass excluded — R2-3); P-b is
> **authenticated user-wide** (R2-4); P-c also **preserves `activeLocationId`** (R2-2); and a **new
> P-d** (`status='active'` in the scoping helpers — the insider-removal fix, R2-1) is added IN SCOPE.
> The table below reflects the resolved REV-2 set.

| # | Change | Surface | Reversible |
|---|---|---|---|
| **P-a** | Cut owner **ACCESS** token TTL `'7d'` → `'24h'` on the **4 refresh-backed mint sites**: argon2 (`local.ts:146`), OAuth (`auth.ts:148`), Telegram (`auth.ts:223`), refresh re-mint (`auth.ts:294`). **EXCLUDE dev-bypass** (`local.ts:68`) — no refresh token (`local.ts:69`), so 24h = daily staging relogin for zero security gain; keep 7d (R2-3). **KEEP** refresh family at 7d (and normalize the two 30d first-login inserts to 7d — see F-N below). Silent refresh (`apiClient.ts:120-127`) preserves no-relogin UX. | literal ×4, no schema, no infra | yes (revert the literal) |
| **P-b** | Add the MISSING server route **`POST /api/auth/logout`** (H3), **authenticated** (`verifyAuth`): derive `userId` from the access token (the credential the client actually sends — `ui/auth.ts:101`, no body), `DELETE FROM auth_refresh_tokens WHERE user_id = $1` — **user-wide "log out all devices"** (R2-4, the honest default). Per-device needs the refresh token in the body ⇒ deferred. Wire `packages/ui/src/lib/auth.ts:99-104` (currently 404s). Reuse-detection ALREADY exists (`auth.ts:262-283`). | one authenticated route + one client wire | yes (route is additive) |
| **P-c** | Fix `/auth/refresh` (H4 + R2-2): (1) re-derive `role` from the **fresh** memberships read, **401/relogin** if no active **owner** membership remains; stop hard-coding `role:'owner'` (`refreshedOwnerClaims`, `auth.ts:20`). (2) **Preserve the caller's CURRENT `activeLocationId`** if it is still a valid active owner membership; deterministic fallback (`ORDER BY created_at, id`) only if gone — never the tiebreaker-less `ORDER BY (role='owner') DESC LIMIT 1` (`auth.ts:288-292`), which silently swaps a multi-loc owner's tenant. | tighten existing handler | yes |
| **P-d** | **(NEW, R2-1 — the insider-removal fix.)** Add `AND status='active'` to the owner-scoping helper membership reads: `requireLocationAccess` (`plugins/auth.ts:147`), `getOwnerLocationId` (`get-owner-location.ts:11`), inline `getOwnerLocation` (`product-media.ts:51`), `getLocationId` (`promotions.ts:19`) — all filter `role='owner'` but **not** `status`, and three trust the baked JWT `activeLocationId` with no DB re-read. Also route the JWT-trust branch through a per-route `status='active'` re-check. Denies a removed/downgraded owner **per-request, immediate** on the owner-write routes (promotions/menu-import/product-media/orders). Index-backed (`core-identity.ts:64-65`), migration-free, **not** on the global hot path. | one predicate ×4 + JWT-branch re-read | yes (revert the predicate) |

Net effect: leaked-access-token window **7d → ≤24h**; membership-revoke / role-downgrade enforced at
the **refresh boundary (≤24h)** (P-c) **and per-request, immediately, on the owner-write routes** (P-d),
and refresh can no longer re-mint owner for a demoted user nor silently swap a multi-loc owner's tenant;
"log out all devices" actually kills the roll-forward path synchronously, user-wide (P-b). **No
per-request DB read on the global hot path. No fail-open. No `users`-RLS migration. No cache/pub-sub
runtime.**

---

## 2. Per-finding disposition

Disposition vocabulary: **fix** (resolved by the rewritten proposal/ADR) · **accept-risk**
(justification + owner) · **defer-flag** (carried into the DEFERRED layer, MISSING until promoted).

### Breaker findings

| ID | Sev | Finding (one line) | Disposition | Resolution |
|----|-----|--------------------|-------------|------------|
| **C1** | CRIT | R1 `users` RLS ENABLE+FORCE+self-read = total-owner lockout on NOBYPASSRLS pool + unset GUC | **fix (by removal)** | The lean design **adds no `users` RLS migration** and does **no per-request `users` read**. The detonator is removed from scope entirely. If the DEFERRED layer is ever promoted, R1 is re-classified as a **load-bearing migration-correctness CRITICAL** (not a footnote) and the read must run through `withTenant` (sets `app.user_id`) or via a role that the policy actually admits — never a bare operational-pool `pool.query` against a FORCE'd `users`. Recorded in §3 DEFERRED gate. |
| **C2** | CRIT | fail-open + at-most-once pub/sub + per-machine LRU ⇒ revocation gap **unbounded**, not ≤30s | **fix (by removal)** | No cache, no pub/sub, no fail-open in the lean design. The leak window is the **access exp (≤24h)** — a *hard, deterministic* bound that does not depend on store health. The "≤30s floor" claim is deleted from the proposal/ADR. Synchronous kill for the acute case = `DELETE FROM auth_refresh_tokens` (P-b) + wait out ≤24h. |
| **C3** | CRIT | sticky hard-revoke can't survive a cold cache ⇒ "known-compromised" guarantee is also fail-open | **fix (by removal)** | The sticky-deny mechanism and `auth_revoked_at` column are **dropped**. Acute "known-compromised owner" handling = refresh-family delete (synchronous, store-health-independent) + ≤24h access exp. No conditional escape hatch to fail. |
| **H1** | HIGH | `tv`-on-`users` conflates "revoke one leaked token" with "log out all devices"; can't revoke attacker without nuking victim's live session | **fix (by removal)** | `tv` is dropped, so the conflation is gone. P-b gives **two distinct** operations: per-family logout (kills *one* device/session) and per-user "log out all devices" (`WHERE user_id`). The operator can revoke the leaked family without touching the victim's live family, *if* sessions are distinguishable; the blunt all-devices bump is opt-in, not the only lever. Per-device granularity beyond family-scope is in DEFERRED. |
| **H2** | HIGH | folded membership recheck is either per-request DB load (the starvation it avoids) OR cached+fail-open with the same gap | **fix (by relocation)** | The membership recheck is **not on the per-request hot path** at all in the lean design. It lives in `/auth/refresh` (P-c) — a handler that **already runs the memberships SELECT** (`auth.ts:289`), so it adds **zero** new query and **zero** per-request checkouts. The dilemma (immediacy vs starvation) dissolves: enforcement cadence = refresh cadence (≤24h), paid only at refresh, never per request. |
| **H3** | HIGH | flag default-off + no owner-logout endpoint ⇒ operators believe they revoked when nothing happened; "logout" button POSTs to a 404 | **fix** | P-b **creates** `POST /api/auth/logout` and wires `auth.ts:99-104`. There is no enforcement flag in the lean design (P-a/c are always-on behaviour changes, not dark-deployed gated reads), so the "believed-but-false revocation" dark-window cannot exist. The headline trigger ("log out") is now wired to a real, synchronous family-delete. |
| **H4** | HIGH | `/auth/refresh` re-mints with a stale fail-open `tv`, hard-codes `owner`, reads on the broken pool | **fix** | P-c rewrites the refresh close *without* `tv`: re-derive role from the fresh memberships read; **401 if no active owner membership**; stop hard-coding `role:'owner'`. The "shares the broken pool" and "re-reads but never re-stamps tv" holes are moot — there is no tv. The downgraded-owner-with-residual-non-owner-membership hole (H4 bullet 3) is closed by requiring an **active *owner*** membership to re-mint an owner token (else mint the lower role or 401). |
| **M1** | MED | legacy/absent-`tv` = permanent bypass class (pre-migration tokens immune to bump for 7d) | **fix (by removal)** | No `tv` claim, no Zod union change, no absent-⇒-0 grandfather class. The ≤24h access exp bounds *all* tokens uniformly, including any in-flight at deploy. |
| **M2** | MED | `auth_revoked_at` set but never read by the cached path; no source-of-truth for sticky-deny | **fix (by removal)** | `auth_revoked_at` column is dropped (no dead-weight column, no migration). |
| **M3** | MED | cache hit-rate math hides a cold-start/deploy spike that becomes Option-B load on the guarded pool | **fix (by removal)** | No cache ⇒ no cold-start miss-storm ⇒ no correlated checkout spike on deploy/restart. The hot path is unchanged from today (pure signature+exp). |
| **L1** | LOW | pub/sub declared "non-load-bearing" but the whole <1s SLA depends on it | **fix (by removal)** | No pub/sub. The proposal's backwards resilience story is deleted. |
| **L2** | LOW | "trivially reversible (flip flag)" false — R1 RLS ENABLE+FORCE on `users` is forward-only, not flag-reversible | **fix (by removal)** | No `users` RLS migration. P-a/b/c are all reversible (revert a literal / drop an additive route / revert a handler tightening). The reversibility claim is now true. |

**Every breaker C-finding is answered by removing its substrate, not by hand-waving.** C1's
users-RLS lockout and C2/C3's fail-open are *structurally impossible* in the lean design because the
per-request `users` read and the cache/fail-open semantics no longer exist.

### Counsel ETHICAL-STOPs

| ID | Stop | Disposition | Resolution |
|----|------|-------------|------------|
| **ES-1** | silent-no-op "Remove owner" / "Log out" button an operator mistakes for "revoked" (flag off ⇒ tv ignored ⇒ no-op for up to 7d) | **fix + HUMAN-decision flag** | In the lean design there is **no enforcement flag** for P-a/c, so a "logout" button is wired to P-b's **real synchronous family-delete** — it cannot silently no-op. The residual stop applies to the *future* **staff-management "Remove owner" UI** (membership delete / role-change endpoints, which **do not exist yet**): when that UI ships, the membership-revoke must be enforced at the refresh boundary (P-c already does this) **and** the UI copy must not claim instant kill of an in-flight access token (it's ≤24h). **HUMAN decision required before the staff-management UI ships** — recorded in §4. |
| **ES-2** | success copy out-running the guarantee ("Signed out everywhere ✓" while a fail-open window keeps the thief in) | **fix** | The fail-open window is **deleted** (no cache). The honest semantics are now: refresh-family delete is **synchronous and guaranteed**; the in-flight access token expires within **≤24h** (not instant). The "log out everywhere" UI copy must say "Signed out on this device; other devices sign out within 24 hours" (or similar) — **not** "✓ signed out everywhere" with an implied instant kill. Runbook documents that the *only* synchronously-guaranteed kill is `DELETE FROM auth_refresh_tokens` + waiting out the ≤24h access exp. Captured as a copy/runbook requirement in the rewritten proposal §9. |

---

## 3. DEFERRED — the full per-request immediate-revocation layer

**What:** per-request immediate (<1min) kill of an in-flight **access** token — either the
`token_version`/auth-epoch approach (with the cache/pub-sub correctly bounded) or an `owner_sessions`
session-lookup mirroring couriers.

**Accepted risk (recorded):** a leaked owner **access** token is valid up to **≤24h** (not the prior
7d, and not the <1min that the full layer would buy). Immediate kill of an *already-minted, not-yet-
refreshed* access token is **out of scope** until a real incident or compliance mandate justifies the
infrastructure.

**Owner:** System Architect (scope call) + Product (prioritization).

**Promotion trigger (any one):**
1. A real or imminent **owner-token-compromise incident** where a ≤24h window is unacceptable.
2. A **compliance / contractual mandate** requiring sub-hour session revocation.
3. Sustained scale where the **refresh-boundary enforcement (≤24h) is too coarse** for an observed
   abuse pattern (e.g. a removed owner exfiltrating during the residual window at volume).

**Gate when promoted (the breaker C-findings become hard pre-conditions):**
- **C1/L2:** any `users` RLS change is a load-bearing migration-correctness CRITICAL; the per-request
  read must run through `withTenant` (sets `app.user_id`) or a role the policy admits — **never** a
  bare operational-pool `pool.query` against a FORCE'd `users`. Reversibility of the RLS must be proven
  before merge (separate down-migration or an explicitly-accepted forward-only side effect).
- **C2/C3/L1:** the revocation guarantee must be **fail-closed-capable for known-compromised** without
  depending on store health (no "sticky-deny conditional on a prior healthy read"); the SLA floor must
  be a deterministic bound, not a TTL re-read that fail-opens.
- **M3:** cold-start/deploy miss-storm must be modeled against the guarded pool before enabling.

The DEFERRED layer is **flag-ready**: P-a/b/c/d do not preclude it (a `tv` claim or session lookup can
be added additively on top of the 24h access token; P-d's per-route `status` re-check is a strict
subset of what a per-request layer would do).

---

## 4. Items requiring a HUMAN decision

> **REV-2 note:** these items are carried and **sharpened** in §7 ("Still needs a HUMAN decision") after
> the second critic pass — in particular, P-d narrows the residual the human must accept (the insider
> **write** sub-case is now closed per-request), and a new Architect-level routing decision is recorded.

1. **Scope call — build the full layer now vs accept ≤24h?**
   Recommendation: **accept ≤24h, defer the full layer** (§3). This matches the threat model (small,
   no-compliance-mandate, ~50-location pre-launch SaaS), Counsel §5, and the Ship-Discipline launch
   trigger (first real paid order — none of the full layer serves it). **Open question to the human
   (Counsel §5):** *has an owner token actually leaked, or are we hardening a HIGH audit finding with
   no incident?* If a real incident is on the table, promote per §3 trigger 1 and build the synchronous
   kill today with tools that already exist (`DELETE FROM auth_refresh_tokens` + the ≤24h cut). Owner:
   System Architect + the human who owns the audit finding.

2. **ETHICAL-STOP-1 — staff-management "Remove owner" UI (when it ships).**
   The membership-delete / role-change endpoints do not exist yet. When they ship, a recorded human
   decision is required that the "Remove owner" action (a) is enforced at the refresh boundary (P-c)
   and (b) does **not** present UI copy implying an instant access-token kill (it's ≤24h). This is
   friction, not a veto — it requires a recorded decision, not a blocked one. Owner: Product +
   System Architect, at the time that UI is designed.

3. **ETHICAL-STOP-2 copy — already actionable now.** The "log out everywhere" confirmation copy must
   reflect the ≤24h eventual semantics for other devices, and the runbook must state the only
   synchronously-guaranteed kill. This is a fix (no human gate), captured in proposal §9; flagged here
   only so the copywriter/UX owner sees it. Owner: System Architect → UX.

---

## 5. Additional grounded corrections folded into the rewrite

- **FRAMING ERROR (Counsel §0):** proposal §1.2 and ADR Context.2 said a removed owner "rolls forward
  **indefinitely**." **False.** Verified: refresh re-mints from `memberships WHERE status='active'`
  (`auth.ts:289`) and refresh families expire (`now() + interval '7 days'` at `auth.ts:300`,
  `local.ts:153`). The roll-forward is **bounded by the 7d refresh family** (re-mint resets it, so the
  *rolling* bound is 7d-from-last-refresh), and membership status is **already** re-read on refresh.
  Corrected in proposal §1.2 and the ADR.

- **F-N — 30d-vs-7d refresh-family inconsistency (new finding, mine):** the OAuth (`auth.ts:154`) and
  Telegram (`auth.ts:228`) first-login inserts use `now() + interval '30 days'`, while password-login
  (`local.ts:153`) and the refresh re-mint (`auth.ts:300`) use `'7 days'`. The roll-forward bound is
  only honestly "7d" if the family TTL is uniform. **Disposition: fix** — normalize the two 30d inserts
  to `'7 days'` so the stated bound is true on every login path. (Continuous refresh already caps at
  7d-from-last-refresh because the re-mint is 7d, so this is a tightening of the *idle* upper bound, not
  the rolling one.) Captured as a proposal §5 migration-free code fix. Owner: Architect.

- **F-comment — stale "1h"/"1h)" comments:** `apps/web/src/lib/apiClient.ts:7,119` and
  `packages/ui/src/lib/auth.ts` comments describe a "1h" access token that the code does not mint (it's
  7d today, 24h after P-a). **Disposition: fix** — update the comments to "24h" when P-a lands so the
  decoupled-TTL rationale reads true. Owner: Architect.

---

## 6. Proof obligations (Mandatory Proof Rule — carried to implementation)

Not executed in this design round (no product code), but specified so "done" is unambiguous:

- **P-a:** unit/assert that a freshly-minted owner access token's `exp - iat ≈ 24h` on the **4
  refresh-backed mint sites** (and that the **dev-bypass** token stays `≈ 7d` — R2-3); Playwright
  owner-login on staging, then assert the session survives past 1h (silent refresh) to prove no UX
  regression.
- **P-b:** E2E — owner logs in on two devices/families, calls (authenticated) `POST /api/auth/logout`,
  asserts **both** families now 401 on `/auth/refresh` (user-wide kill); unauthenticated POST is
  rejected (no no-auth force-logout — R2-4 sub-2).
- **P-c:** E2E — (a) owner whose active owner membership is removed/downgraded calls `/auth/refresh`,
  asserts 401/relogin (or a non-owner token), NOT a fresh `role:'owner'` token; (b) **multi-location**
  owner working in L2 refreshes and asserts the new token still carries `activeLocationId=L2` (no
  silent swap to L1 — R2-2). Red→green regression + `docs/regressions/REGRESSION-LEDGER.md` row (AUTH).
- **P-d:** E2E — removed/downgraded owner (membership `status='revoked'`) holding a still-valid access
  token with baked `activeLocationId=L` hits an owner-write route (e.g. `POST .../promotions` or
  product-media) and asserts an **immediate** 403/404 (denied per-request), NOT a successful write —
  proving the insider write-window is closed without waiting ≤24h. Red→green regression + ledger row
  (AUTH red-line).
- **F-N:** assert OAuth/Telegram login refresh-family `expires_at - now ≈ 7d`.

---

## 7. REV-2 RESOLVE — residual dispositions over the second Breaker + Counsel pass

Round 2 over the REV-2 sections of `breaker-findings.md` (R2-1…R2-6) and `counsel-opinion.md`
(R2.1…R2.5, R2-new). The pivot is confirmed sound by both critics (no CRITICAL in the lean design);
these are residuals on the three new changes. Every disposition grounded in live source this round.

### Disposition table

| ID | Sev | Residual (one line) | Disposition | Resolution (grounded) |
|----|-----|---------------------|-------------|------------------------|
| **R2-1** | HIGH | Owner-scoping helpers filter `role` but **not** `status='active'`; three trust the baked JWT `activeLocationId` with no DB re-read ⇒ a removed owner keeps tenant **write** access for the full ≤24h on the owner-write routes (promotions/menu-import/product-media/orders), independent of P-c. | **FIX → new P-d (IN SCOPE).** | Verified live: `get-owner-location.ts:8-14` (returns `user.activeLocationId` at :8 before any DB read; fallback :11 has no `status`), `product-media.ts:49-55` (:49 JWT-trust, :51 no `status`), `promotions.ts:14-24` (:15-16 JWT-trust, :19 no `status`), `plugins/auth.ts:146-149` (:147 `role='owner'`, no `status`). Fix: add `AND status='active'` to all four (index-backed — `core-identity.ts:64-65` partial indexes already cover `WHERE status='active'`) **and** route the JWT-trust branch through a per-route `status='active'` re-read so the removed owner is denied **per-request, immediate**, not ≤24h. The false "tenant-staleness closed" claim is **dropped** (proposal §8, ADR Consequences) and replaced with this concrete enforcement. Cheap (a status predicate), closes the write-window the DEFERRED ≤24h risk left open. **Open sub-decision (Architect):** route the JWT-trust branch through the re-read (preferred) vs accept-risk that one branch at ≤24h with a named owner — recommendation: the re-read (in-budget, owner-write routes already write the DB). |
| **R2-2** | HIGH | `/auth/refresh` re-derives `activeLocationId` via `ORDER BY (role='owner') DESC LIMIT 1` with **no tiebreaker** (`auth.ts:288-292`) ⇒ a multi-location owner's working tenant silently swaps on refresh; P-a's 24h cadence amplifies the rate ~7×. | **FIX → folded into P-c.** | Verified: `auth.ts:289-293` re-derives from scratch, never carries the incoming token's `activeLocationId`. Fix: **preserve the caller's CURRENT `activeLocationId`** if it is still a valid active owner membership; deterministic fallback (`ORDER BY created_at, id`) only if gone; 401/relogin only when no active owner membership remains. **Never silently swap a working tenant.** Specified in proposal §4 P-c, ADR Decision P-c, §10 R2-2/R3. |
| **R2-3** | MED | Dev-bypass mint (`local.ts:68`) returns no refresh token (`local.ts:69`) ⇒ 24h there = daily staging relogin for zero security gain (prod rejects the dev keypair anyway). Both critics agree. | **FIX → exclude from P-a.** | Verified: `local.ts:66-69` mints `signDevToken(payload, '7d')` and returns only `{ access_token, userId, activeLocationId }` — no refresh-token issuance. P-a now touches the **4 real mint sites only**; dev-bypass stays `'7d'`. Applied across proposal §1/§2/§4, ADR Decision P-a, §1 PRIMARY table. |
| **R2-4** | MED | Client sends Bearer **access** token + no body (`ui/auth.ts:99-102`); P-b spec'd hashing "the caller's refresh token" ⇒ no input to hash (sub-1); a no-auth force-logout keyed on a presented token is a DoS (sub-2). | **FIX → P-b redesigned.** | Verified: `ui/auth.ts:95-107` POSTs `/api/auth/logout` with `Authorization: Bearer ${access_token}`, no body. P-b is now **authenticated** (`verifyAuth` required — closes sub-2 DoS), derives `userId` from the token, and does a **user-wide** `DELETE FROM auth_refresh_tokens WHERE user_id=$1` ("log out all devices" — the honest, simple default). Per-device logout needs the refresh token in the body ⇒ **deferred future enhancement** (sub-1), noted in §10 R2 + Non-goals. |
| **R2-5** | LOW | Logout vs in-flight refresh-rotation race can resurrect the just-deleted family for one token. | **Accept (LOW).** | Mitigated by the rotation's `used=true` single-use guard + family reuse-detection (`auth.ts:262-283`); the resurrected family is a single live token the racing client just minted (not a re-opened attacker path), bounded by ≤24h. A future tombstone (delete-that-blocks-reinsert) makes "log out = synchronous guaranteed kill" hold against a concurrent refresh — not needed at this stage. ES-2 copy notes the caveat. Captured in proposal §6. Owner: Architect. |
| **R2-6** | LOW | F-N (30d→7d OAuth/TG idle family) ⇒ a genuinely-idle OAuth/TG owner re-logins where they didn't. | **Accepted (already R4).** | Matches the password path; active sessions unaffected (re-mint is 7d). Honesty of the "bounded by 7d" claim is worth the idle-edge re-login. No change. |

### Counsel REV-2 notes

| Item | Disposition | Resolution |
|------|-------------|------------|
| Delete the fabricated "~85%" | **FIX.** | Removed from the ADR Consequences and softened in proposal §4 + resolution §0 to a qualitative true statement ("the dominant share of the gap — window 7d→≤24h, the insider write-window per-request via P-d, plus the H3/H4 correctness bugs"). The "~85%" survives only in `counsel-opinion.md` as the critic's own quoted words. |
| Confirm F-N (normalize OAuth/Telegram 30d→7d) stays | **CONFIRMED.** | F-N stays — both critics endorse it (Counsel "improves honesty"; Breaker R2-6 accepted). Without it the headline "bounded by 7d" is false on the OAuth/Telegram paths. Unchanged in proposal §5, ADR Decision, §10 R4. |
| ES-1 / ES-2 (rev-1 stops) | **CONFIRMED resolved** (Counsel R2.1). | ES-1 is structurally absent in the lean design (no enforcement flag, logout wired to a real DELETE); residual carried as a recorded **future** human gate for the staff-management "Remove owner" UI (§4 item 2). ES-2 downgraded to a fix (copy + runbook). One sharpening from Counsel R2.1: the future UI's honest copy is "Access ending — signs out within 24 hours," and the gate should check the **words**, not just the wiring. |

### The resolved PRIMARY set (REV-2)

**P-a** (24h access, **4** refresh-backed sites — dev-bypass excluded) · **P-b** (authenticated
**user-wide** logout) · **P-c** (refresh role re-derive + `activeLocationId` **preservation**) ·
**P-d** (`status='active'` in the owner-scoping helpers — the **insider-removal fix**, immediate
per-request on the owner-write routes). All **forward-only, reversible, migration-free** (code-only
predicate/TTL changes; the `memberships.status` column + partial indexes already exist). F-N
(30d→7d OAuth/TG family) folded in.

### Still needs a HUMAN decision (carried + sharpened)

1. **Scope call (unchanged, R5):** build the full per-request <1min layer now vs accept ≤24h on the
   non-owner-write surfaces? And the underlying fact question — *has an owner token actually leaked, or
   are we hardening a HIGH audit finding with no incident?* (Counsel §5). Recommendation: accept ≤24h,
   defer (§3). With **P-d added**, the highest-value sub-case (insider write to a removed-from tenant)
   is now closed per-request, so the residual the human must accept is **narrower** than before: a
   leaked-but-not-refreshed access token used for **reads** (or writes outside the owner-write routes)
   for ≤24h. Owner: human audit-owner + Architect.
2. **P-d JWT-trust-branch routing (Architect, new):** route the `getOwnerLocationId` /
   `getOwnerLocation` / `getLocationId` JWT-trust branch through a per-route `status='active'`
   re-read (preferred — closes the insider write-window immediately) vs accept-risk that branch at
   ≤24h with a named owner. Recommendation: the re-read (index-backed, scoped to owner-write routes).
   This is an Architect call, not a human-gate, but recorded so it isn't lost in implementation.
3. **ETHICAL-STOP-1 (unchanged):** the future staff-management "Remove owner" UI must not assert an
   instant access-token kill; copy = "Access ending — signs out within 24 hours" until the DEFERRED
   layer can make it instant; the gate checks the words, not just the wiring (Counsel R2.1). Owner:
   Product + Architect, when that UI ships.
4. **ETHICAL-STOP-2 (fix, no gate):** "log out all devices" copy + runbook must reflect ≤24h eventual
   semantics for other devices and note the R2-5 same-instant-refresh caveat. Owner: Architect → UX.
