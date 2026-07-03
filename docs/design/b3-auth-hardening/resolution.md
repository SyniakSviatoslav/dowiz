# B3 Deep Auth Hardening — RESOLVE (Council round 1)

Status: **REVISION — pending re-attack.** This resolution is NOT self-certified "done"; the conductor
re-runs Breaker + Counsel against the revised proposal. Author: System Architect. Date: 2026-07-03.

Inputs resolved: `breaker-findings.md` (C1, C2, H1–H3, M1–M5, L1–L2), `counsel-opinion.md` (ES-1, ES-2,
open question). Every load-bearing breaker fact was re-verified against live source before disposition
(see "Verification" below) — the findings are grounded, not hypothetical.

---

## 0. Headline decision — option (b): NARROW Phase-0, DEFER the flip

**NOBYPASSRLS-on-prod is not safely shippable yet. I am NOT choosing (a) "materially revised enforcement
design shipped now." I am choosing (b): ship the truly-safe Phase-0 wins + decoupled incident response,
and DEFER the enforcement flip behind a proper pre-flip enumeration + proof program.**

Reasoning (failure-first, boring-wins):
- The breaker proved three *already-in-code* facts that break Option B's model, not hypotheticals:
  C1 (role-grant chain never established → login lockout), C2 (`is_local=false` on a transaction-mode
  pool in two live provisioning paths), H2 (money-writes key on `app.current_tenant`, which `withTenant`
  never sets). These are structural, not cosmetic.
- H3 quantifies the real blocker: **~123 raw `.db.query`/`pool.query` DML call-sites vs ~116 `withTenant`**
  — roughly half the write surface never enters the enforcement seam. You cannot honestly flip a
  defense-in-depth layer while half the surface is outside it and un-enumerated.
- Counsel's own long-horizon lens agrees: B3 is **open-source-/scale-blocking, not first-paid-order-blocking.**
  There is no deadline pressure forcing an unsafe flip now. The cheap-certain wins (close the leak, lock
  the safe guardrails) carry almost all the near-term risk reduction; the flip carries almost all the risk.

The role-model bug (C1/M2) IS fixable on paper (see C1 disposition), and I specify the fix so the design
is coherent when the flip returns. But fixing the role model does not close C2/H2/H3/M3/M4 or ES-2 —
those are the gate. So the flip leaves Phase-0.

---

## 1. Phase-0 scope (SAFE-NOW — shippable, each independently revertible)

Ordered. Every item is deterministic and does NOT depend on any NOBYPASSRLS flip.

| # | Item | Disposition source | Ship note |
|---|---|---|---|
| P0-1 | **Incident response (decoupled):** rotate BOTH leaked creds — **postgres SUPERUSER** (named) + `deliveryos_api_user` — + git-history scrub + CI secrets gate | ES-1 revise, H1 fix | standalone, human-go recorded separately from any B3 sign-off, do FIRST |
| P0-2 | **C2 fix:** convert the two `set_config(...,false)` provisioning paths (`onboarding.ts:75`, `spa-proxy.ts:771`) to explicit-txn `is_local=true` (or route through the canonical seam) | C2 fix | inert-neutral today (BYPASSRLS), removes latent transaction-mode hazard; pre-flip prerequisite |
| P0-3 | **JWT off WS URL:** migrate FE `client/status/ws.ts` to message-auth; add a dated forcing deadline; close socket immediately (1008) on failed URL-token verify | L2 fix | telemetry informs, deadline forces; dual-accept until deadline |
| P0-4 | **W1 boot-guard (two-mode):** assert enforcement-role `rolbypassrls=false` via catalog (`pg_roles`), gated on role existence; post-convergence assert `current_user` | M1 fix | catalog-based, not `current_user`-based; dark until role exists |
| P0-5 | **Guardrail locks:** W2 argon2 params, W3 refresh reuse-detection, W4 no-cookie grep-gate, W5 kid-rotation runbook, W6 TTL review | proposal §6 (confirmed) | red→green guardrails only; no behavior change |
| P0-6 | **Rate-limit:** KEEP process-local `Map` limiter; **DEFER** the pg-backed shared store | M5 defer + accept-risk | see M5 |

Everything else — the role machinery, the seam unification, the enumeration, the policies re-verify,
the flip itself — is **Phase-1+ (deferred)**, gated on the program in §3.

---

## 2. Per-finding disposition

Legend: **FIX** (concrete design change applied to proposal.md) · **ACCEPT-RISK** (justified + owner) ·
**DEFER-FLAG** (marked MISSING; the flip cannot proceed until closed).

### C1 — role-grant chain → total login lockout — **FIX (design) + DEFER (flip)**
Verified: `withTenant` sets only `app.user_id`; RC2 policies target `TO dowiz_app`; no
`GRANT dowiz_app TO dowiz_app_rls` exists anywhere; §3 states the grant in the **reverse** direction.
**Root cause:** the enforcement role (`dowiz_app_rls`) is a *different* role from the policy-target role
(`dowiz_app`). Postgres applies a `TO <role>` policy to `current_user` **and to any role of which
current_user is a member.** So the fix is precise and boring:
- Create `dowiz_app_rls NOLOGIN NOBYPASSRLS` **and** `GRANT dowiz_app TO dowiz_app_rls` (membership).
  Membership makes the `TO dowiz_app` policies (RC2 `USING(true)`, RC4, RC6) apply to `dowiz_app_rls`,
  and `dowiz_app_rls` inherits `dowiz_app`'s grants — while role-attribute `BYPASSRLS` is **not**
  inherited via membership, so `dowiz_app_rls` stays enforcing. This is the lockout firebreak actually
  connected.
- This must be **proven on staging** (impersonate: `SET ROLE dowiz_app_rls` + `set_config('app.user_id',…)`
  → read `users`/`auth_refresh_tokens` → expect rows, not zero) **before** any owner-lane flip.
The flip stays **DEFERRED** regardless (blocked by C2/H2/H3/M3/M4/ES-2). C1's paper-fix removes the
lockout-by-construction so the deferred program builds on a coherent role model.

### C2 — `set_config(…,false)` on transaction-mode operational pool — **FIX (Phase-0)**
Verified: `onboarding.ts:75` and `spa-proxy.ts:771` both `set_config('app.user_id',$1,false)` on a
`db.connect()` client from the Supavisor **transaction-mode** (`:6543`) pool, relying on a comment
("persists across statements on a single client"). Transaction mode does not guarantee the same PG
backend across autocommit statements. Two failure branches (onboarding-break, or stale-GUC cross-tenant
leak) both bite under enforcement. **Fix:** wrap each provisioning write-sequence in an explicit
`BEGIN…COMMIT` and use `is_local=true` (matching `withTenant`), or route through the canonical seam
(§3). Today under BYPASSRLS this is inert-neutral, so it ships in Phase-0 as a correctness prerequisite
with its own proof, ahead of any flip.

### H1 — "dual-valid overlap window = idempotency" is impossible for a single role — **FIX**
Verified logic: one PG role cannot hold two valid passwords; keeping the leaked role alive for "overlap"
means the leaked credential stays exploitable for the whole window — contradicting the rotation's goal.
**Fix:** overlap is only coherent across **two distinct role identities** (old leaked → new clean, app
cut over, then leaked role disabled/dropped). For the **superuser** password (which the runtime does not
use) there is no overlap need at all — rotate immediately and accept a coordinated redeploy blip. The
proposal's §5 "made idempotent by an overlap window (both passwords valid)" wording is removed. This is
folded into the decoupled incident-response (ES-1 / P0-1).

### H2 — money-writes key on `app.current_tenant`, `withTenant` sets only `app.user_id` — **FIX (design) + DEFER (money-lane flip)**
Verified: RC4 policies key on `app.current_tenant`; `withTenant` sets `app.user_id`; courier/webhook
money paths set `app.current_tenant` on a **raw** checked-out client, outside `withTenant`, with no role
switch. **Fix:** introduce a single canonical context seam `withTenantContext(pool, {userId?, tenantId?})`
that (a) opens the txn, (b) applies the enforcement role, (c) sets the correct GUC(s) `is_local=true`,
and (d) for money-table writes **asserts `rowcount > 0` → else throw** (a silent 0-row money write is a
red-line failure, never a no-op). All money paths migrate to it before the money lane may flip. Until
every money path is on the seam, the **money-lane flip is DEFERRED** (marked MISSING in §3).

### H3 — ~half of DML surface never enters the enforcement seam — **DEFER-FLAG (MISSING: enumeration)**
Verified count: ~123 raw `.db.query`/`pool.query` vs ~116 `withTenant`. The "owner→courier→anon 3-lane"
framing hides that most raw-pool write paths are not in any lane. **Disposition:** the flip **cannot
proceed** until a complete, checked-in enumeration of all raw-pool tenant DML call-sites exists, and each
is either (a) migrated to the canonical seam (§3/H2) or (b) explicitly **quarantined** (documented as
running under the login role during ramp, with app-layer `WHERE` as its sole guard, listed by name). Add
a CI gate forbidding *new* out-of-seam DML on FORCE-RLS tenant tables. **This enumeration is not yet done
— it is the gating pre-flip work item.**

### M1 — W1 boot-guard is dead during the ramp — **FIX (Phase-0)**
Verified: boot-guard checks `current_user==='postgres'` at connect; under Option B the login role stays
BYPASSRLS, so a `NOT rolbypassrls` check on `current_user` would either FATAL forever or be disabled, and
would never see `dowiz_app_rls`. **Fix (two-mode, P0-4):** during ramp, assert the *enforcement* role's
attribute via catalog — `SELECT rolbypassrls FROM pg_roles WHERE rolname='dowiz_app_rls'` must be false,
plus assert the membership grant exists; post-convergence, revert to the `current_user`-based assertion
(now `dowiz_app`, NOBYPASSRLS). Ship the catalog assertion in Phase-0, gated on role existence so it is a
no-op until the (deferred) role is created.

### M2 — blanket grant-mirror re-opens migration 080 hardening — **FIX**
Verified: `080` REVOKEs `TRUNCATE, TRIGGER, REFERENCES` from `dowiz_app` and `INSERT/UPDATE/DELETE ON
platform_admins`. A blanket `GRANT ALL … TO dowiz_app_rls` would hand the enforcement role *more* than
the audited login role (TRUNCATE is not governed by RLS → tenant-table wipe; platform_admins write →
self-promotion). **Fix:** do **not** blanket-mirror grants. Use **role membership** (`GRANT dowiz_app TO
dowiz_app_rls`, per C1) so `dowiz_app_rls` inherits *exactly* `dowiz_app`'s post-080 grant set — no more,
no less. §5.2's "mirror the login role's table/sequence grants" is replaced by the membership grant.

### M3 — SECURITY DEFINER function owner unspecified (owner-resolve + worker sweeps) — **FIX (design) + DEFER**
Verified: RC3 `app_owner_location` (077) is `SECURITY DEFINER`; the plan references ~19 Phase-2 sweep fns
(078) also DEFINER. A DEFINER function bypasses RLS **only while its owner retains BYPASSRLS.** At
convergence (`ALTER ROLE dowiz_app NOBYPASSRLS`), any DEFINER fn owned by `dowiz_app` becomes RLS-subject
→ `app_owner_location` returns 0 → owner locked out of own dashboard; worker sweeps return 0 → dispatch/
reconciliation silently stops. **Fix:** pin DEFINER-fn ownership to a dedicated **bypass-class**
`dowiz_definer NOLOGIN BYPASSRLS` role (or keep them owned by `postgres`); assert owner-is-bypass in
`verify:rls`. This is a hard convergence precondition → the convergence step stays **DEFERRED** until
asserted.

### M4 — drift reconciliation built on an unverified, self-contradictory claim — **FIX (introspection-first) + flag claim UNVERIFIED**
Verified: `grep "RENAME COLUMN"` across migrations = 0; base + 077 + 080 all reference `owner_id`. The
claim "prod keys on `user_id` (rename applied on staging, skipped on prod)" has **no in-tree artifact**
and §3 states the drift direction inconsistently (if the rename ran on staging, `user_id` would be on
*staging*, not prod). **Fix:** (1) the claim is marked **UNVERIFIED** — the first action is to actually
introspect BOTH environments and record the real column state; write no rename migration against a claim.
(2) The reconciliation must be re-run-safe across all three states: only `owner_id` → no-op; only
`user_id` → rename; **both columns present** → a rename *fails* (cannot rename into an existing column) →
handle explicitly (operator-gated data merge + drop-stale, never an auto-rename). The proposal's DO-guard
(`IF user_id EXISTS THEN RENAME`) does not cover the both-columns case → corrected.

### M5 — pg rate-limit store: adversarial BoE wrong + hot-row contention = DoS amplifier — **DEFER-FLAG (pg store) + ACCEPT-RISK (in-memory 2×)**
Verified logic: the counter upserts **per attempt before the limit decision**, so a credential-stuffing
flood writes on every request; a single-account target = one hot `(key, window)` row → serialized upserts
under row-lock → self-serialization consuming operational-pool connections = the 2026-06-20 pool-starvation
class. And degrade-to-in-memory removes cross-instance limiting *exactly* during DB stress.
**Disposition:** this is over-engineering that turns a control into an amplifier → **DEFER** the pg store
(boring wins). Phase-0 **keeps the process-local `Map` limiter**. **ACCEPT-RISK** the cross-instance gap:
at the current fleet (2 API machines) the worst case is `N_machines × per-instance budget` = **2×** the
intended auth budget — bounded and small. Revisit only if the fleet grows materially or a real
distributed brute-force is observed, and if so prefer a *sloppy* per-instance-batched aggregate (no hot
per-attempt row), not a synchronous per-attempt upsert. Owner: Architect.

### L1 — convergence (Option A) is not flag-reversible — **ACCEPT-RISK + FIX (honest wording)**
Verified: convergence is `ALTER ROLE … NOBYPASSRLS` + removing the `SET LOCAL ROLE`; rollback is another
migration, not a flag. **Accept** — convergence is a deliberate end-state reached only after full soak +
proof, and it is far-future given the flip is deferred. **Fix the wording:** §9's "every risky item
reverts by flag" is scoped to the ramp only; the doc must state plainly that the converged end-state
trades flag-reversibility for simplicity (migration-level rollback). Owner: Architect + DB owner.

### L2 — WS URL-token removal has no forcing function; socket not closed on failed verify — **FIX (Phase-0)**
Verified: `logTokenDeprecation` (`websocket.ts:350`) counts usage but cached PWAs + FE `client/status/ws.ts`
(still URL-based) make usage asymptotic, not 0; failed URL verify (`websocket.ts:352-354`) relies on the
5s authTimeout instead of closing. **Fix (P0-3):** migrate `client/status/ws.ts` to message-auth (removes
the largest live URL source), set a **dated deadline** after which URL-accept is removed regardless of the
asymptote (telemetry informs, the deadline forces), and close the socket immediately (1008) on failed
URL-token verify.

---

## 3. ETHICAL-STOP dispositions

### ES-1 — decouple credential rotation from B3; name the superuser — **REVISE (design) + human-go recorded separately**
Accepted in full. Revisions:
- Credential **rotation** is pulled out of B3 item #4. It becomes standalone incident response (P0-1):
  rotate **both** leaked credentials — the **postgres SUPERUSER** (explicitly named; it was absent from
  the proposal and is the most dangerous leaked cred) **and** `deliveryos_api_user` — plus the git-history
  scrub and CI secrets gate. It proceeds independent of B3 approval, its rollout, and the re-role decision.
- The rotation **mechanism** is corrected per H1 (two distinct role identities for any overlap; direct
  immediate rotation + coordinated redeploy for the superuser). No "both passwords valid on one role."
- The **re-role** decision (retire `deliveryos_api_user`, promote `dowiz_app` as the canonical login role)
  is a genuine B3 design decision and stays in B3.
- The human's **go on the rotation is recorded separately** from any B3 sign-off. The doing is operator-
  gated as it already is. Design side: fully revised — **not** escalated as needs-human-decision.

### ES-2 — no prod lane flip until silent-denial is detectable + reversible live, and orphan audit re-run at the flip — **REVISE (satisfied by scope + hard gates)**
Accepted. In Phase-0 there is **no prod lane flip at all**, so ES-2 is honored by scoping. When the flip
returns as its own proposal, ES-2's conditions become **hard, non-defaultable preconditions**:
- (a) the 0-row anomaly metric AND per-lane flag-revert are **proven live on prod** (not merely designed);
- (b) the NULL-key orphan audit is **re-run at the flip moment**, and a **continuous** NULL-keyed-insert
  gate runs on FORCE-RLS tables **throughout** the ramp (a point-in-time snapshot cannot guard a live
  moving system — counsel's residual harm);
- (c) **[open question, incorporated]** a fail-closed tenant denial must be **user-distinguishable** — the
  owner sees a "temporarily unavailable / contact support" state, **not** a silent empty list, so a
  security denial can never masquerade as "you have no orders." This is a UI-tells-the-truth / server-
  authoritative red line; it is added to the deferred flip's DoD, not left as an ops-only metric.
The flip act itself remains **human-set**. Design side: revised — **not** escalated as needs-human-decision.
The residual human decision that *does* remain is purely go/no-go timing on the (already human-gated) flip.

---

## 4. Deferred pre-flip program (all must be provably complete before any NOBYPASSRLS lane flips on prod)

Marked **MISSING** until each is checked-in + proven on staging:
1. Role model proven: `dowiz_app_rls NOBYPASSRLS` + `GRANT dowiz_app TO dowiz_app_rls`; membership-based
   policy application confirmed by staging impersonation (C1, M2).
2. Canonical `withTenantContext` seam covering `withTenant` + all raw-pool tenant/money paths; money
   writes rowcount-guarded (H2).
3. Complete checked-in enumeration of the ~123 raw DML call-sites → each migrated or quarantined; CI gate
   against new out-of-seam DML (H3).
4. C2 provisioning paths converted to explicit-txn `is_local=true` (C2) — *this one ships in Phase-0*.
5. DEFINER-fn ownership pinned to a bypass-class role + `verify:rls` assertion (M3).
6. Drift reconciliation: real introspection output recorded for both envs; migration re-run-safe across
   all three column states incl. both-columns (M4).
7. ES-2 gates live-proven on prod (anomaly metric, per-lane revert, continuous orphan gate, user-facing
   distinguishable denial).
8. Convergence (Option A) documented as the non-flag-reversible end-state, reached only post-soak (L1, M3).

Only after 1–8 does the flip return as its own gated proposal.

---

## 5. Verification (breaker facts re-checked against live source, 2026-07-03)
- `packages/platform/src/auth/tenant.ts` — `withTenant` sets only `app.user_id`, `is_local=true`; no role
  switch, no `app.current_tenant`. (C1, H2 grounded.)
- `apps/api/src/routes/owner/onboarding.ts:75` + `apps/api/src/routes/spa-proxy.ts:771` — both
  `set_config('app.user_id',$1,false)` on a `db.connect()` transaction-mode client. (C2 grounded.)
- `packages/db/migrations/1790000000077…` — RC2 `ops_all … FOR ALL TO dowiz_app USING(true)`; RC4
  courier writes key on `app.current_tenant`; RC3/RC6 SECURITY DEFINER fns present. (C1, H2, M3 grounded.)
- `packages/db/migrations/1790000000080…` — REVOKEs `TRUNCATE,TRIGGER,REFERENCES` + `platform_admins`
  writes from `dowiz_app`. (M2 grounded — blanket mirror would re-open these.)
- `packages/db/src/index.ts` — operational-pool `on('connect')` guard checks only
  `current_user==='postgres'`; no `rolbypassrls` assertion. (M1, W1 grounded.)
- Counts (116 `withTenant` vs 123 raw DML) taken from breaker grep; not independently re-counted — the
  disposition (enumeration is MISSING) holds regardless of the exact number.

---

## 6. Not self-certified
Per RESOLVE discipline: this is a **revision**, not a closure. The conductor re-runs Breaker + Counsel
against the revised `proposal.md`. Open residual for the human: go/no-go timing on the (human-set,
deferred) flip, and the recorded go on the decoupled credential rotation (P0-1).
