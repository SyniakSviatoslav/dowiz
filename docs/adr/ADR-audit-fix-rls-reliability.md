# ADR — GUC/tx discipline (`withTenantTx`), latent-RLS completion, GDPR-erasure liveness, pg-boss queue policy

- **Status:** Proposed — RESOLVED-DRAFT v2 + **R2 re-attack applied** (breaker-r2 + counsel; see
  `docs/design/audit-fix-rls-reliability/resolution-r2.md`). **Lane-A worker-local safety subset
  IMPLEMENTED + PROVEN** (N1 fail-loud backstop + LC4 pending-reset in `workers/anonymizer-gdpr.ts`,
  guardrail `apps/api/tests/anonymizer-gdpr-backstop.test.ts` red→green, ledger #61). The structural
  post-flip *success* mechanism (DEFINER `gdpr_erase_customer`) + LC4-MIG + MIG-1..4 + the flip
  remain **operator-gated** (see §R2-sync). Awaiting operator.
- **Date:** 2026-07-03
- **Deciders:** System Architect (proposer), Triadic Council, DB owner, Operator
- **Related:** `docs/design/audit-fix-rls-reliability/proposal.md` (full design + verified fact
  sheets `site-inventory.md`, `rls-state.md`, `pgboss-state.md`), `ADR-b3-deep-auth-hardening`
  (the NOBYPASSRLS ramp this ADR is a precondition of), `ADR-pg-privilege-hardening`,
  `docs/design-review/AUDIT-SYNTHESIS-2026-07-03.md` (roots R-B, R-E; LC4; plan items A3/B-3/B-15/B-18),
  `docs/regressions/REGRESSION-LEDGER.md` row 50.
- **Red-lines:** RLS · PII/legal (GDPR) · `packages/db/migrations/**` (operator/protect-path gated).

## Context

The 2026-07-03 six-lane audit converged on two systemic roots this ADR owns:

1. **R-B — GUC/tx discipline is inconsistent.** The canonical correct shape (BEGIN +
   `set_config(k,v,true)` + COMMIT) exists in `packages/platform/src/auth/tenant.ts:3-21` and is
   documented in-code (`apps/api/src/lib/courier-room-authz.ts:9-13`), but a verified sweep of all
   49 `set_config` sites found **11 autocommit no-ops** (GUC dies before the query — including every
   customer status push, the courier settlements money reads via bare `pool.query`, all telegram
   order actions, customer push subscribe) and **2 session-scoped leaks**
   (`routes/owner/onboarding.ts:75`, `routes/spa-proxy.ts:771` — the GUC persists on the pooled
   physical connection after release: a cross-request identity bleed). All masked today by the
   `dowiz_app` role's BYPASSRLS; every one bites — silently — at the staged NOBYPASSRLS flip.
2. **§E — the flip would not isolate what matters.** `couriers` (password_hash, encrypted PII) and
   `courier_sessions` (token_hash) have **no RLS at all**; the anonymous policies on
   `orders`/`order_items`/`customers` are fail-open (`USING (app_current_user() IS NULL)`,
   OR-nullifying tenant_isolation on the public pool); `locations` is `USING(true)`; `backup_*` and
   five token-bearing tables are ENABLE-not-FORCE.

Plus two reliability criticals in the same blast area: **LC4** — GDPR erasure rows strand
`in_progress` forever after one transient failure (`workers/anonymizer-gdpr.ts:29/39/84-98`: retry
re-scans `pending` only and never reads `job.data`; the `failed` terminal is unreachable; legal
red-line) — and **R-E** — pg-boss v10 runtime under v12 types with 100% bare `createQueue` calls
(singletonKey dedup fleet-wide no-op, no retry/backoff/dead-letter, one boot throw amputates the
worker fleet including the detectors).

## Decision

1. **One transaction-scoped context helper — `withTenantTx(pool, ctx, fn)`** (generalizing the
   existing `withTenant`), covering both GUC families (`app.user_id`, `app.current_tenant`) plus an
   explicit `anonymous` variant; BEGIN + `set_config(...,true)` + COMMIT/ROLLBACK + release, client
   never escapes. It is the **single hook point** where B3's `SET LOCAL ROLE dowiz_app_rls` per-lane
   flag is prepended. Convert all 13 broken sites (session leaks first), then the hand-rolled-correct
   sites. Guardrail: `no-bare-set-config` ESLint rule — the literal `set_config` is forbidden outside
   `packages/platform/src/auth/tenant.ts` (+ migrations); lands red, goes green with conversion.
   (Alternative — named per-principal helpers over a shared core — kept as fallback veneer;
   ALS auto-injection rejected: hides the transaction boundary that caused the bug class.)
2. **Latent-RLS completion, forward-only, FORCE-first (v2)** — four migrations, each inert for the
   BYPASSRLS main pool, each operator-gated: **MIG-1 `couriers`/`courier_sessions` join the
   credential FIREBREAK** — ENABLE+FORCE + role-restricted `ops_all FOR ALL TO dowiz_app
   USING(true) WITH CHECK(true)` (the actual `users`/`auth_refresh_tokens` convention: mig
   `1780421100065` STEP A2 + mig 077 RC2 — NOT tenant policies; `couriers` has no `location_id`
   and login/session reads are pre-context, so tenant-RLS = total courier lockout at the flip
   [breaker F1]) + `courier_auth_read FOR SELECT TO dowiz_app` on `courier_locations`; gated on
   OPS-READ-CHECK (no operational-pool reads); tenant-scoped `TO dowiz_app_rls` policies are
   deferred **MIG-1b** with the B3 courier-lane flag. MIG-2 re-scope the anonymous policies to
   `app_current_user() IS NULL AND location_id = <anon tenant GUC>` (+ WITH CHECK on the customer
   UPDATE, + scoped anon INSERTs) — **gated by P10 (FORCE-access inventory UNCONVERTED=0) and
   GATE-ANON-E2E (staging anon-checkout+track E2E green with MIG-2/3 applied there)**, not by
   prose ordering; MIG-3 scope `locations.public_select` via a DEFINER slug resolver; MIG-4 FORCE
   completion for `backup_*` + token-bearing ENABLE-only tables and replacement of `USING(true)`
   self-mint/ops policies. The flip artifact itself
   (`docs/security/OPERATIONAL-ROLE-nobypassrls.migration.ts`) is untouched; this ADR hardens its
   preconditions. **Correction of record (ES-2):** the deferred flip as staged isolates nothing on
   `couriers`/`courier_sessions` (no RLS exists there today); the ADR-020 open-source flip is
   gated on MIG-1+MIG-4 landed + P1/P1b green (**GATE-OSS-RLS**).
3. **GDPR erasure liveness (v2 — ships standalone as Lane A per counsel ES-1):** cross-tenant
   scan/claim via SECURITY DEFINER `gdpr_claim_due()` (atomic claim stamping
   `metadata.claimed_at` + `metadata.claim_token`); per-row work in
   `withTenantTx({tenantId: row.location_id})` with **LC4-MIG** adding a missing-ok
   `app.current_tenant` arm to the two `app_member_location_ids()`-keyed policies (the v1
   `{tenantId}`-only compat claim was false [breaker F3]); retry jobs carry-and-use
   `job.data.requestId` (handler fixed to the v10 array shape); retryable failures CAS-reset to
   `pending`; stale reclaim keyed on `claimed_at` >30min (no `updated_at` column exists — the v1
   "no schema change" claim is dropped: one additive migration, zero new columns [breaker F4]);
   **all terminal writes are claim-token CAS-guarded and side effects (audit row, event) fire only
   on CAS win** — exactly-once per request [breaker F7]; cap → `failed` + `anonymizer.gdpr.dlq` +
   **level-triggered** O-GDPR check (re-fires until resolved) + bound resolution owner (operator,
   72h SLA, runbook) + owner-facing requires-action surface + controller-facing Art.12 receipt
   (direct-to-subject channel = needs-human). Invariant: every erasure request reaches
   `completed`/`failed`; nothing strands — and nothing terminal is unowned.
4. **pg-boss queue policy (v2):** pin types to the deployed v10 and remove `@ts-nocheck` —
   honestly scoped to include fixing the direct `boss.work` v10-array-shape sites it surfaces
   (settlement-cron, dwell, lifecycle-handlers, anonymizer-gdpr — runtime-wrong today [breaker
   F11]; anonymizer's fix rides Lane A); a single `QUEUE_POLICY` map beside `QUEUE_NAMES` drives
   `createQueue` **plus a boot-time policy reconciler** (prod queues already exist as `standard` —
   bare re-create does not alter stored policy [breaker F8]; OPEN-V1 spike verifies v10
   `updateQueue`, fallback = send-side `singletonSeconds`, the one dedup proven on `standard`);
   explicit `retryLimit/retryDelay/retryBackoff`, per-queue `deadLetter` → **`<queue>.dlq`** (the
   helper's convention; v1 `.dead` was a monitor-watches-nothing mismatch [breaker F10]) + one DLQ
   monitor driven by the same map; `expireInSeconds` for long handlers; delete the unreachable
   `err.data` retry illusion; per-registration boot isolation in `bootstrap/workers.ts` with the
   liveness/reconciliation **watch-set fed by the boot registry of actually-started workers**
   ("detectors first" dropped — it fought reconciliation's dependency and made false DRIFT
   [breaker F12]).

## R2-sync (2026-07-03 — conductor re-attack `resolution-r2.md` applied by lead)

The R2 re-attack of the v2 design surfaced one CRITICAL and one HIGH that amend the decisions above.
Binding amendments (each dispositioned in `resolution-r2.md §1`):

- **Decision #3 — correction of record (N1, CRITICAL):** the v2 claim "the erasure path itself
  survives the flip" is **false as designed** — the data erasure runs on the anonymizer's
  context-free connection (`lib/anonymizer/index.ts:131,220`), which `customers` RLS does not honour
  post-MIG-2 (`customers` has **no** `app.current_tenant` arm — the RC4 arm is orders-only,
  `1790000000077:44-67`), while the worker wrote `completed` regardless of `result.skipped`. **Amended
  to:** (i) terminal `completed`/audit/event are **conditional** — they fire only when a data-level
  re-read confirms `customers.anonymized_at IS NOT NULL`; otherwise `failed` + `ANONYMIZER_GDPR_FAILED`
  signal, **never** `completed` (reverses "regardless of `result.skipped`"; credits the idempotent
  already-anonymized case). (ii) The erasure **data path** must run under an enforcement-valid
  mechanism — a SECURITY DEFINER `gdpr_erase_customer(p_customer, p_location)` (search_path-pinned,
  ledger #33) returning the resulting `anonymized_at` — **not** the worker's `app.current_tenant`
  context. **IMPLEMENTED now (worker-local, no migration):** (i) — the fail-loud backstop + LC4
  pending-reset, guardrail `anonymizer-gdpr-backstop.test.ts` (red→green, ledger #61). **OPERATOR-GATED
  / GATE-FLIP-E2E:** (ii) — the DEFINER function is a `packages/db/migrations/**` red-line; drafted for
  the operator. The backstop makes the pre-DEFINER interim SAFE (post-flip a non-erasure fails loud,
  never silently completes) — it does not by itself make post-flip erasure *succeed*.
- **Decision #3 / LC4-MIG scope (N2, HIGH):** route the worker's terminal writes + audit INSERT
  through a DEFINER `gdpr_finalize(...)` so LC4-MIG adds **no** `app.current_tenant` arm to
  `gdpr_erasure_requests` / `anonymization_audit_log` (both `FOR ALL`, member-only today; an arm would
  grant the courier/webhook principal CRUD incl. DELETE on erasure requests + forge/erase of the
  append-only audit log). Fallback if an arm is retained: command-split (`FOR SELECT`+`FOR UPDATE`
  only; `FOR INSERT` only on the audit log — no DELETE). Operator-gated.
- **Decision #2 / MIG-2 (N1-b):** adding a `customers` `app.current_tenant` arm is **REJECTED** —
  it repeats N2 on the *primary* PII table (table-wide read/update for every courier-shift/webhook
  principal). Use the DEFINER route instead.
- **Decision #2 / MIG-1 (N3, MED):** extend MIG-1 with `courier_auth_write FOR INSERT TO dowiz_app`
  on `courier_audit_log` — post-flip the failed-login audit INSERT (`courier/auth.ts:269-273`, pre-context
  `db.connect()`, zero-UUID fallback) hits the RC5 policy WITH-CHECK → 42501 → a wrong-password 401
  becomes a 500. P1b/P9 must exercise the *failed* login path (expect 401, not 500). Operator-gated (MIG-1).
- **Decision #1 (`withTenantTx`) (N5, LOW):** the helper must issue transaction-local resets of both
  GUC families (`set_config('app.user_id','',true)`, `set_config('app.current_tenant','',true)`)
  before setting ctx keys, so `{anonymous:true}` cannot inherit a leaked session GUC. (Lane 0.)
- **Decision #4 (pg-boss) (N4, MED):** the queue-policy reconciler must pass the FULL policy from
  `QUEUE_POLICY` on every `updateQueue`; add a wrapper that requires `policy` (a partial `updateQueue`
  resets a `'short'` queue to `'standard'`, silently disabling `singletonKey` dedup). (Lane A′.)
- **Verification block:** P5 extended with the data-level erasure assertion + negative `failed`+DLQ
  case (the new P-proof — IMPLEMENTED as `anonymizer-gdpr-backstop.test.ts` at the unit level; the
  NOBYPASSRLS+MIG-2 rehearsal-DB variant remains a GATE-FLIP-E2E item); P1b/P9 failed-login (N3); P8
  asserts no `FOR ALL`/DELETE `app.current_tenant` arm on the two GDPR tables (N2); P3 anon-reset (N5);
  P6 short-stays-short after a partial reconcile (N4).
- **needs-human (R2 additions, `resolution-r2.md §3`):** the N1 correction-of-record; the re-scoped
  Lane-A DoD; the structural-fix mechanism choice (DEFINER vs anon-lane context — DEFINER recommended);
  the N3 firebreak + N2 DEFINER-ization as migration red-lines — each operator-gated.

## Ordering (v2 — three lanes; gates, not prose)

**Lane 0** (helper + `withCourierTx` money veneer + lint red + ALS dev/test tripwire) →
**Lane A** now, standalone, go recorded separately (LC4 + LC4-MIG + boot isolation + `.dlq` +
O-GDPR) → **Lane A′** (queue-policy sweep) — none of these wait for the flip.
**Lane B** (flip preconditions): session-leak fixes → semantic conversions incl. telegram
txn-boundary redesign → OPS-READ-CHECK → MIG-1/MIG-4 (P1+P1b) → public-lane conversion
(P9/P10 green) → GATE-ANON-E2E → MIG-2/MIG-3 → B3 per-lane ramp → GATE-FLIP-E2E + soak →
Phase-3/4 `ALTER ROLE dowiz_app NOBYPASSRLS`. Open-source: GATE-OSS-RLS.

## Verification (proof-or-it-didn't-happen; v2 list)

P1 RLS-adversarial isolation (firebreak = deny-by-default for non-`dowiz_app` roles; un-skipped in
CI per audit root R-C) · **P1b courier-auth-survives-the-flip** (login + session validation under a
NOBYPASSRLS'd `dowiz_app` probe — the key, not the lock) · P2 notify-push regression **pinned to
the flip-rehearsal DB** (both GUC families) · P3 pooled-connection GUC-leak probe · P4 lint gate
red→green · P5 GDPR liveness (pending-reset, targeted retry, cap→failed+`.dlq`+level-triggered
DRIFT, CAS-concurrency = exactly 1 audit row/event, reclaim both ways) · P6 dedup proof **against a
pre-existing `standard` queue + reconciler** · P7 boot isolation + registry watch-set (no false
DRIFT) · P8 policy-hygiene probe (tenant key OR allowlisted role-restricted firebreak) ·
**P9 flip-rehearsal suite** (anon checkout / track exchange / courier auth / owner auth under
enforcement — the falsifiable conversion-complete gate) · **P10 FORCE-access inventory scan**
(CI; UNCONVERTED=0) · **P11 telegram txn-boundary** (COMMIT strictly before external HTTP).
Ledger rows per P1/P1b/P2/P3/P5.

## Consequences

**Positive:** the deferred flip becomes real isolation instead of theater; B3's ramp gets its
required txn seam; a legal-red-line liveness hole closes; queue dedup/retry semantics match what the
code already assumes. **Negative/costs:** ~40 mechanical site conversions (review load); one BEGIN
per previously-autocommit read (marginal); four gated migrations; conversion order discipline until
the lint gate is green. **Rollback:** helper conversions revert per-site (multi-write flows keep today's partial-failure
semantics — no atomicity regression to unwind); migrations are additive policies inert for the
BYPASSRLS main pool (MIG-1 is immediately effective for non-BYPASSRLS roles — hence
OPS-READ-CHECK; drop-policy is break-glass); queue options revert per-queue; LC4 is worker-local
logic + **one additive migration** (two policy arms + one DEFINER function, zero new columns).
