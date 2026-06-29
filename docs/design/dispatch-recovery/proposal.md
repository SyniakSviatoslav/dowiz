# Design Proposal — Dispatch Auto-Recovery (B2) + Reconciliation Re-enable (B5)

- Status: **RESOLVED (design)** — Breaker + Counsel dispositioned in `resolution.md`; decisions below
  updated accordingly. No production code in this round. A focused Breaker re-attack follows.
- Date: 2026-06-29
- Author: System Architect (DeliveryOS)
- Companion ADR: `docs/adr/ADR-dispatch-recovery.md` (DRAFT)
- Relates / does-not-contradict: ADR-deliver-v2-cash-as-proof (the `courier_dispatch_queue`
  re-offer mechanism, mig `1790000000073`), ADR-golive-remediation (C3 removed
  `ReconciliationWorker`; R9 tracks re-enable), monolith-first, integer-money, RLS ENABLE+FORCE,
  JWT RS256, Postgres-backed idempotency.

---

## 1. Problem + non-goals

### Problem (two launch-blockers, both red-line: dispatch state-machine + monitoring)

**B2 — dead dispatch / silent re-offer.** `courier_dispatch_queue` is the durable journal the
deliver-v2 design (`ADR-deliver-v2-cash-as-proof.md:97-98`) intends as the auto-re-offer mechanism.
It has **4 producers** and **0 consumers**:

- producers: reject (`routes/courier/assignments.ts:213-217`), decline (`:547-551`),
  binding-release (`lib/bindingRelease.ts:30-34`), offer-timeout sweep
  (`workers/courier-offer-sweep.ts:50-54`) — all `INSERT … ON CONFLICT … attempts+1`.
- consumer: **none.** Nothing turns a queued row into a `COURIER_DISPATCH` pg-boss job.
  `workers/courier-dispatch.ts` `.work(COURIER_DISPATCH, …)` (`:17`) waits for jobs that are
  never enqueued; its `handleDispatch` only runs AFTER a job is already dequeued.

Three secondary defects compound it:

1. **`this.boss` is undefined.** The only retry path, `courier-dispatch.ts:76`
   `await this.boss.send(...)`, references a field the constructor never sets (`:10-14` has
   `pool`, `queue`, `messageBus` — no `boss`). It throws `TypeError` → caught `:101-103` →
   `ROLLBACK`. Even if a job ever reached the worker, the "no courier available, retry later"
   branch self-destructs. Correct accessor is `this.queue.boss` (the pattern at bootstrap
   `workers.ts:59`).
2. **No `'assigned'` acceptance timeout.** `courier-offer-sweep.ts:40-45` expires only
   `status='offered'`. An auto-assigned `status='assigned'` binding a courier never accepts (nor
   rejects) never expires — shift stuck `on_delivery`, order stuck, no recovery. (`'assigned'` is
   the accept-required state: the reject endpoint matches `status='assigned'`,
   `assignments.ts:194`.)
3. **False signals.** `courier-offer-sweep.ts:47` logs "→ re-offered" and `bindingRelease.ts`
   returns `reoffered:true` while having done nothing but insert into the **undrained** table. The
   monitoring story lies: it reports recovery that never happens.

**B5 — ReconciliationWorker unregistered.** `workers/reconciliation.ts` is code-complete (12
read-only checks incl. `A6` worker-liveness that watches `'dispatcher'`) but absent from
`bootstrap/workers.ts`. It was **intentionally removed in prod** by ADR-golive breaker C3 — the
nightly worker was NOT the right executor for sub-minute lost-job detection, so that one detection
was folded into a live 1-min sweep instead, and the full worker was deferred to R9. The
consequence: `O1` (orphan PENDING>1h), `O2` (stuck shifts>24h), `M1–M4` money-drift, `N1`, `R1`,
`F1`, `T1`, and `A6` worker-liveness run **nowhere**. With B2 fixed, `A6` is exactly the gauge that
would have caught a dead dispatcher.

### Non-goals

- **Not** turning on `COURIER_OFFER_HANDSHAKE_ENABLED` (offer→accept). It stays dark; this design
  must be correct in BOTH flag states.
- **Not** building a smart courier-assignment algorithm (distance/ETA/fairness). The existing
  "first available shift by freshest heartbeat" (`courier-dispatch.ts:51-62`) is unchanged.
- **Not** moving money. Dispatch never touches integer-money fields. Recon stays read-only.
- **Not** re-introducing the round-2 migrations C3 rejected (`CONCURRENTLY` index, partial UNIQUE
  on the attempt-log). The only schema change is **one additive nullable column**
  (`orders.dispatch_exhausted_at`, the honest held-marker); the proposed FORCE-RLS migration was
  **deleted** as a verified no-op (already done at mig `1780421100051:14`).
- **Not** a new pg-boss queue or a new DB pool.

---

## 2. Back-of-envelope

Pilot scale (ADR-golive context): **1 active tenant, 3–10 locations, peak ≈ 0.5 orders/sec**.

| Quantity | Pilot | 10× growth | Notes |
|---|---|---|---|
| Orders/min (peak) | ~30 | ~300 | 0.5–5 orders/sec |
| Re-dispatch enqueues/min | ~9 | ~90 | assume worst-case 30% of orders hit ≥1 reject/decline/timeout |
| Pump tick interval | 60 s | 60 s | reuse existing 1-min courier sweep cadence |
| Jobs sent / tick | ≤ ~75 | ≤ ~750 | bounded by rows resident in the journal |
| Cost / send | ~1 ms | ~1 ms | pg-boss `send`, singletonKey dedup |
| Pump wall-time / tick | < 100 ms | < 1 s | one advisory-locked connection |

**Max-attempts math (corrected, RESOLVE).** `COURIER_DISPATCH_MAX_ATTEMPTS=5`. The in-worker 30 s
self-retry is **deleted** (it was suppressed by `singletonKey` — see RESOLVE B-FAIL/MED): the
**60 s pump is the sole retry cadence**, one attempt per tick. An order with no available courier
therefore retries **5 × 60 s ≈ 5 min**, then escalates: persists `orders.dispatch_exhausted_at`,
fires the (now-wired) `ORDER_DISPATCH_FAILED` consumer → owner alert + honest customer push, and
**only then** deletes the journal row. No infinite loop; the failure is **visible**, not silent.

**Queue growth under total courier shortage.** Inflow ~30 rows/min; each row lives ≤ ~5 min before
escalation-delete → steady-state journal ≈ `30 × 5 ≈ 150 rows`. The table is **self-bounding**:
the escalation path drains it into owner alerts + the order held-marker. At 10× (~1500 rows) a seq
scan is still sub-ms off the hot path; add a covering index only then (R-DEFER-1).

**Connection budget (API + worker + analytics + migrations, aggregate).** Pool `max: 8`
**unchanged**. The drain pass folds into the **existing** `CourierOfferSweepWorker.run()` — same
advisory lock, **same single connection**, zero new connections. The nightly recon worker is one
read-only burst at 03:00 UTC (~12 sequential `pool.query` over a single pooled connection for a
couple of seconds). Net new steady-state connections: **0**.

---

## 3. Options (≥2) with tradeoffs + concept

### Q1 — how does a journal row become a real re-dispatch?

**Option A — dedicated periodic pump worker.** New `CourierDispatchPumpWorker`: 1-min cron,
advisory lock, `SELECT` due rows → `boss.send(COURIER_DISPATCH, …)` per row.
- Concept: durable journal + sweeper-relay (transactional-outbox relay shape).
- Tradeoff: a second worker, second advisory-lock id, second connection acquisition each minute —
  surface unjustified at pilot when an identical 1-min courier sweep already runs on the same table.

**Option B — enqueue a pg-boss job directly at each of the 4 INSERT sites.** Skip the
table-as-queue; pg-boss IS the queue.
- Concept: direct transactional enqueue.
- Tradeoff: must `send` **inside** each caller's tx to stay atomic (else job-without-row or
  row-without-job); 4 call sites to touch; and you lose the **self-healing journal** — a job lost
  by pg-boss (failed past its own retries without hitting our max-attempts) is gone forever, no
  re-pump. The table + 4 producers already exist; B would orphan them.

**Option C — hybrid: durable journal + idempotent sweep-relay (CHOSEN).** The table stays the
durable journal (already written by all 4 producers); a **drain pass folded into the existing
`CourierOfferSweepWorker.run()`** pumps due rows into `COURIER_DISPATCH` jobs, deduped by pg-boss
`singletonKey: orderId`.
- Concept: durable journal + idempotent 1-min sweep ("data + sweep, no live timer to lose" — the
  same machinery shape as `OrderTimeoutSweep` and the offer-expiry pass, `courier-offer-sweep.ts:7-11`).
- Tradeoff: broadens one worker's responsibility from "expire offers" to "expire offers + drain
  dispatch journal" (two passes under one lock). Accepted: it is the lowest-surface,
  self-healing, pattern-consistent choice.

### Q5 — re-enable B5 reconciliation

**Option R1 — re-register the full `ReconciliationWorker` as-is (nightly, read-only).**
- Tradeoff: it ships 12 checks for free; but `A6.EXPECTED_WORKERS` (`reconciliation.ts:217-218`)
  lists 8 worker ids while bootstrap only heartbeats **4** (`workers.ts:98-103`) → nightly **false
  DRIFT** → alert fatigue → the social pressure that re-triggers C3. Not safe as-is.

**Option R2 — extract only the missing detectors (O1/O3) into a new minimal worker.**
- Tradeoff: more new surface than R1, and discards the already-built M/O/N/R/F/T/A6 coverage.

**Option R3 — re-register, but TRIM `A6.EXPECTED_WORKERS` to the 4 heartbeating ids (REJECTED in
RESOLVE).** Verified Goodhart regression: the kept 4 are **identical** to
`LivenessChecker.CRITICAL_WORKERS` (`liveness-checker.ts:11`), so A6-as-trimmed adds **zero**
worker-liveness coverage while deleting its only unique value — leaving `backup-hourly` (hourly DB
backup, a data-recovery red-line) and `liveness-checker` itself watched by **nothing**. Removing
noise by removing the monitor, not by fixing the gap.

**Option R3′ — re-register the full worker AND instrument the missing heartbeats so A6 watches the
TRUE set of 8 (CHOSEN, RESOLVE).** `WorkerHeartbeat` is a cadence-independent 15 s timer
(`lib/worker/heartbeat.ts:32`), so an hourly/nightly worker can heartbeat every 15 s while its job
runs hourly — the heartbeat proves the **process is alive**, satisfying A6's 1-hour staleness window.
Add `backup-hourly`, `signal-raiser`, `courier-stale_check`, `liveness-checker` to `heartbeatConfigs`
(`workers.ts:98-103`); add `backup-hourly` to `WORKER_CRITICAL_LIST` for a live 60 s path too. A6
keeps all 8 ids and yields **no false DRIFT** because every named worker now genuinely beats. Nightly
read-only **detection + alert** complements the sweep's sub-minute **recovery**; closes R9; restores
real coverage incl. backup-hourly + the watcher-of-the-watcher (`liveness-checker` caught by A6).
Per-worker detection paths are tabulated in the ADR.

---

## 4. Decision + rationale (ADR-format → `docs/adr/ADR-dispatch-recovery.md`)

**Drain (Q1): Option C** — durable journal + drain pass folded into the existing 1-min courier
sweep; idempotency via pg-boss `singletonKey: orderId` with the DB partial-active unique as
backstop. Matches the proven codebase pattern, reuses the table + 4 producers that already exist,
self-heals lost jobs, adds zero connections/queues.

**`this.boss` bug + dead 30 s retry (Q2): DELETE the in-worker self-retry (RESOLVE).** The
`startAfter:30s` `send` was suppressed anyway — it executed while the same-`singletonKey` job was
still `active`, so the 30 s path never materialized. Rather than invent a second recovery cadence,
**remove `courier-dispatch.ts:76` entirely**: the no-courier, not-yet-exhausted branch increments
`attempts`, COMMITs, and **returns** — the journal row persists and the **60 s pump is the single,
honest retry cadence**. This deletes the `this.boss` undefined bug outright (no patch needed) and
removes a whole class of singleton/double-send reasoning. `COURIER_DISPATCH_RETRY_MS` is retired.

**`'assigned'` acceptance timeout (Q3):** add a third pass to the same sweep — expire
`status='assigned' AND assigned_at < now() - COURIER_ASSIGN_ACCEPT_TIMEOUT_MS` → set
`status='cancelled', cancellation_reason='assign_accept_timeout'`, free the shift
(`courier_shifts → 'available'`), re-enqueue to the journal. Reuse **`'cancelled'`** (already in the
status vocabulary, mig `073:11-12`) so no migration is needed and the
`courier_assignments_order_active_uniq` guard frees the order for a fresh `'assigned'`. Window
applies in BOTH flag states (it targets the auto-assigned, not-yet-accepted binding); default
proposed below.

**False signals (Q4):** the journal write is honest as "re-**enqueued**", not "re-offered". Rename
`bindingRelease.ts` return `reoffered` → `requeued`; change the offer-sweep log to "→ re-enqueued
for dispatch". The genuine re-offer is now owned by the pump + worker, so the honest claim
("a courier was re-offered") is emitted by `handleDispatch` on a successful new `'assigned'`
(`ORDER_ASSIGNMENT_CREATED`, `courier-dispatch.ts:99`).

**B5 (Q5): Option R3′ (RESOLVE)** — re-register the full read-only nightly `ReconciliationWorker`
and **instrument the 4 missing heartbeats** (`backup-hourly, signal-raiser, courier-stale_check,
liveness-checker`) so `A6.EXPECTED_WORKERS` (all 8) watches a set that genuinely beats — no trim, no
false DRIFT, no loss of coverage. Add `backup-hourly` to `WORKER_CRITICAL_LIST` for a live 60 s path.
No migration (pg-boss `createQueue` at runtime). Per-worker death-detection paths in the ADR.

**Exhaustion-tail honesty (Q7, RESOLVE — closes ETHICAL-STOP-1):** the terminal escalation must be
seen by a human and reflected to the customer. (a) **Wire the consumer:** subscribe
`ORDER_DISPATCH_FAILED` in `bootstrap/messaging.ts` (mirror the `ORDER_ASSIGNMENT_CREATED` handler
`:72`, claim-check-clean) → owner Telegram-ops + an honest customer push. (b) **Change the order
state:** on exhaustion set `orders.dispatch_exhausted_at = now()` (persisted held / needs-attention
marker; `order_status` stays truthful) **in the same transaction**; the customer push says
"arranging your courier / slight delay" — never a false "on its way". (c) **Don't erase the trace
before it's durable:** the order marker is committed before/with the journal-row delete; owner alert
+ customer push fire post-commit. The durable owner-visible trace is now the committed order marker,
not a void event. (The residual grace-window decision is human-gated — §10 R-NEEDS-HUMAN-1.)

**Idempotency / handleDispatch hardening (Q6):** `handleDispatch` MUST pre-check before assigning —
if the order already has an active binding (`status IN ('offered','assigned','accepted','picked_up')`)
or is terminal (`orders.status IN ('DELIVERED','CANCELLED')`), **DELETE the journal row and return**
(resolved). The shift-pick must ALSO exclude couriers holding an `'offered'` assignment (RESOLVE
MED — align `courier-dispatch.ts:55-58` with `courier_one_active_assignment`'s
`('offered','assigned','accepted','picked_up')`), else an offer-holding courier is picked → 23505
loop. And the catch block MUST **special-case 23505 by constraint** (RESOLVE HIGH): on
`courier_assignments_order_active_uniq` (order already bound elsewhere) → DELETE the journal row,
COMMIT, return success (no throw, no pg-boss retry, no re-pump, no false Recon O3); on
`courier_one_active_assignment` (a picked courier raced) → do NOT delete, return so the next tick
re-picks a different courier; any other error → `ROLLBACK; throw`. Optionally `SELECT … FOR UPDATE`
the orders row in the pre-check to narrow the TOCTOU. This is the "already re-dispatched / now
assigned must not be re-offered" rule, hardened so the benign race is treated as benign.

---

## 5. Data / migrations (forward-only, atomic, RLS FORCE, integer)

**FORCE-RLS migration DELETED (RESOLVE).** The proposal originally added
`ALTER TABLE courier_dispatch_queue FORCE ROW LEVEL SECURITY`. **Verified already done** at committed
mig `1780421100051_force-rls.ts:14` (051 > 044, applied) → the proposed migration is a **no-op**.
Removed from the design. DoD #9 (`relforcerowsecurity = true`) is **already GREEN**; keep the
existing `verify:rls` assertion as a standing guard. The §5 "ENABLE-not-FORCE" premise was
stale-grounded; corrected.

**The drain is code-only — no schema.** The one genuinely-needed migration is for the **honest-tail**
fix (Q7), not the drain:

```
ALTER TABLE orders ADD COLUMN dispatch_exhausted_at timestamptz;
```

- Additive, forward-only, nullable, **no default** → metadata-only (no table rewrite). Set in the
  exhaustion transaction as the durable held / needs-attention marker; read by the owner dashboard,
  Recon `O1`, and the (flag-gated) grace-window worker. `order_status` stays truthful — no enum
  ripple on the red-line state machine (the new-enum-value option was considered and rejected for
  regression radius; see resolution.md §C).
- `orders` is already tenant-scoped + FORCE RLS; an additive nullable column needs **no** policy
  change.
- **No** status-enum migration: `'assigned'`-expiry reuses `'cancelled'` (already in `073` CHECK);
  the grace-window terminal also reuses `'cancelled'` + `cancellation_reason`.
- The bare-policy `NULLIF`/member-branch parity with `073` on the journal stays a SEPARATE optional
  follow-up (DEFER-FLAG R-FLAG-1) — couriers reach the journal only via the API tenant GUC / BYPASSRLS
  pool, never directly; latent 22P02 only for a future NOBYPASSRLS owner-context producer.
- **No** new index at pilot (≤~150 rows seq-scan). Covering index on `(enqueued_at)` is **R-DEFER-1**
  at 10× volume.
- Integer-money: untouched — dispatch moves no monetary field.
- Net migrations after RESOLVE: **1 load-bearing** (`orders.dispatch_exhausted_at`), replacing the
  deleted no-op.

---

## 6. Consistency + idempotency (no double-offer)

- **Application dedup:** every `COURIER_DISPATCH` send (pump AND `handleDispatch` retry) carries
  `singletonKey: orderId`. pg-boss permits only one job per key in `created/active/retry` — a
  scheduled retry (`startAfter`) occupies the key, so the pump cannot double-send while a dispatch
  is pending or waiting.
- **DB backstop:** `courier_assignments_order_active_uniq` (mig `073:22-24`) — partial unique on
  `order_id WHERE status IN (active set)` — guarantees **at most one active binding per order** at
  the database level; a racing second INSERT throws `23505`. `courier_one_active_assignment`
  (`073:32-33`) guarantees at most one active binding per courier. The worker's
  `FOR UPDATE SKIP LOCKED` shift pick (`courier-dispatch.ts:60`) prevents two ticks grabbing the
  same shift.
- **Idempotent journal:** the 4 producers use `INSERT … ON CONFLICT (order_id) DO UPDATE attempts+1`
  (PK on `order_id`) — one journal row per order, ever.
- **Self-cleaning drain:** `handleDispatch` DELETEs the row on success, on max-attempts escalation
  (after the order marker commits), and (Q6 guard) on already-bound / terminal order. The pump only
  sees genuinely-pending rows.
- **Benign race handled as benign (RESOLVE):** the shift-pick excludes couriers holding any of
  `('offered','assigned','accepted','picked_up')` (aligned with `courier_one_active_assignment`), so
  the only remaining 23505 is the order-level TOCTOU on `courier_assignments_order_active_uniq`; that
  is special-cased → DELETE the row + return success (the order is already bound), **not** a generic
  `throw` → re-pump. No crash-loop, no false Recon `O3`.
- **`attempts` semantics:** incremented by both the producers (enqueue collision) and
  `handleDispatch` (dispatch attempt). It is a circuit-breaker counter, not an exact retry count —
  documented; erring toward earlier escalation is acceptable.

---

## 7. Failures + degradation (every external call: timeout + fallback, zero cascade)

- **No courier available** (the expected degradation): the 60 s pump re-attempts up to
  `MAX_ATTEMPTS` (≈5 min), then escalates **honestly at both ends (RESOLVE)**: persists
  `orders.dispatch_exhausted_at` (durable held-marker, owner-dashboard-visible), the now-wired
  `ORDER_DISPATCH_FAILED` consumer fires the owner Telegram-ops alert **and** an honest customer push
  ("arranging your courier / slight delay"), and only then deletes the journal row. The owner
  manually assigns/cancels; if they don't act within a bounded grace window, the order auto-cancels
  to a customer-honest terminal (flag-gated, human-ratify — R-NEEDS-HUMAN-1). **Never silent (the
  failure is owner- AND customer-visible), never an infinite loop, never a false "on its way".**
- **Pump DB error:** `run()` is wrapped (the existing `try/catch` at `courier-offer-sweep.ts:63`);
  a failed tick logs and the next 1-min tick retries. The advisory lock prevents overlap. No live
  timer to lose.
- **Job lost by pg-boss** (failed past its own retries without our escalation): the journal row
  survives, the singletonKey frees on terminal job state, and the next pump tick **re-pumps** —
  this is the self-healing property B chose against.
- **Worker process down:** `A6` (recon) flags `dispatcher` as dead/stale within the nightly run;
  the heartbeat (`workers.ts:99`) is the live signal `LivenessChecker` already watches each 60 s.
- **`'assigned'` never accepted:** the new acceptance-timeout pass expires it and re-enqueues — no
  stuck shift, no stuck order.
- **No cascade:** dispatch failures terminate at the owner alert; they do not roll back orders,
  money, or other tenants' work. The drain pass is tenant-agnostic but each row carries its own
  `location_id`; one tenant's shortage cannot starve another's pump (each row sends its own job).

---

## 8. Security + tenant isolation

- `courier_dispatch_queue` is **already `FORCE ROW LEVEL SECURITY`** (committed mig
  `1780421100051:14` — RESOLVE corrected §5; this design adds no RLS migration), policy isolates by
  `app.current_tenant`. The pump runs as the BYPASSRLS operational role doing a deliberate
  cross-tenant pass (same posture as `courier-offer-sweep`), and each pumped job carries its own
  `location_id` → `handleDispatch` operates within one tenant.
- Re-dispatch carries **no PII** to any external surface (no menu/customer data to AI; ops alerts
  via the existing Telegram-ops outbox carry order/location ids only — claim-check clean).
- `ReconciliationWorker` is read-only; its drift alert (`ops.reconciliation_drift`) publishes
  counts + truncated detail, no raw PII rows leave the process (`evidence` stays in-process logs).
  Confirm the `N1`/`R1` detail strings carry no customer identifiers (DoD item).
- No new secrets, no cookies, no JWT-surface change.

---

## 9. Operability (health degraded-vs-down, observability <1 min, rollback, flag)

- **Observability:** a dead worker surfaces to `LivenessChecker` within 60 s (now incl.
  `backup-hourly`) and to nightly A6; the pump logs per-tick drain counts. The dispatch-shortage
  *escalation* is bounded at ≈5 min (`MAX_ATTEMPTS × 60 s pump`) — slower than 1 min by design (one
  attempt/tick), but it lands on a **wired** consumer + a persisted order marker, so the operator
  *sees* it (the <1 min bar applies to component-death detection, which holds).
- **Health:** a dead dispatcher does NOT flip `/health` to `down` (dispatch is a soft/`degraded`
  dependency, not a serving dependency — the menu must still serve, per the boot-budget rule); it
  surfaces via heartbeat + nightly `A6`. Document this as `degraded`, not `down`.
- **Rollback:** all behavior is data-driven and flag/threshold-tunable; reverting the worker code
  reverts behavior with no schema rollback needed (the one additive `orders.dispatch_exhausted_at`
  column is nullable and inert if unwritten — safe to leave). The recon worker can be unregistered to
  silence it instantly; the grace-window auto-cancel is flag-OFF by default.
- **Flags / scaling-gate:** envs with safe defaults — `COURIER_DISPATCH_MAX_ATTEMPTS=5`;
  `COURIER_DISPATCH_RETRY_MS` **retired** (self-retry deleted, RESOLVE);
  `COURIER_ASSIGN_ACCEPT_TIMEOUT_MS` (default **300000 / 5 min**; must exceed the courier UI accept
  window — confirm against the FE accept timeout in DoD, R-OPEN-1);
  `WORKER_CRITICAL_LIST` extended to include `backup-hourly`;
  `DISPATCH_OWNER_GRACE_ENABLED` (**default OFF** — the grace-window auto-cancel ships dark until
  human ratification at STOP-ETHICS, R-NEEDS-HUMAN-1) + `DISPATCH_OWNER_GRACE_MS` (default 900000 /
  15 min). The drain + exhaustion-tail run unconditionally (independent of
  `COURIER_OFFER_HANDSHAKE_ENABLED`); only the grace auto-cancel is flag-gated.
- **pg-boss queue existence:** ensure `COURIER_DISPATCH` queue is created before the pump sends
  (the dispatch worker's `.work` registers it at boot; add a defensive `boss.createQueue` in the
  sweep `start()` if ordering is not guaranteed) — operability note, not a blocker.

---

## 10. Open / accepted risks (justification + owner)

| Id | Risk | Disposition | Owner |
|---|---|---|---|
| R-DEFER-1 | No `(enqueued_at)` index on the journal | DEFER — pilot seq-scans ≤~75 rows sub-ms; add at 10× | Architect |
| R-ACC-1 | `attempts` double-counts enqueue-collision + dispatch-attempt | ACCEPT — circuit-breaker counter, earlier escalation is safe | Architect |
| R-ACC-2 | `'assigned'`-expiry reuses `'cancelled'` (no distinct status) | ACCEPT — avoids enum migration; `cancellation_reason` disambiguates; frees active-uniq | Architect |
| R-FLAG-1 | Optional dual-context RLS policy parity with `073` on the journal table | DEFER-FLAG — couriers never reach the table directly; FORCE is the load-bearing fix | Lead |
| R-OPEN-1 | `COURIER_ASSIGN_ACCEPT_TIMEOUT_MS` default must exceed the FE accept window | OPEN — verify FE accept-timer before setting; DoD-gated | Architect + FE |
| R-OPEN-2 | Recon `N1`/`R1` detail strings must be PII-free in the ops alert | OPEN — confirm before re-enable | Architect |
| R-ACC-3 | Re-enabling recon adds nightly read load at 03:00 | ACCEPT — read-only, ~12 queries, off-peak, one connection | Architect |
| R-ACC-4 | Option-C drain fold-in is load-bearing + untethered (a refactor could drop it, silently re-stranding) | ACCEPT + GUARD — DoD-1 integration test registered as a standing regression so dropping the fold-in goes RED | Architect |
| R-INHERIT | C3's original "noisy worker" failure mode | MITIGATED — **by instrumenting the 4 missing heartbeats (R3′)**, NOT by trimming A6; all 8 named workers genuinely beat → no false DRIFT and no loss of coverage | Architect |
| R-NEEDS-HUMAN-1 | After owner alert, owner inaction → permanent customer silence unless the order auto-transitions | NEEDS-HUMAN — recommended default staged (bounded grace then auto honest-terminal `CANCELLED`); ships **flag-OFF** until ratified at STOP-ETHICS | Owner/operator |
| R-ACC-5 | Accept-timeout must carry **no** courier reliability penalty | ACCEPT (standing constraint) — no scoring system today; keep it scoring-free | Architect |
| R-ACC-6 | `'assigned'`-expiry can re-pick the same slow courier in a loop | ACCEPT — bounded by max-attempts → now-**visible** escalation; optional later "exclude just-timed-out courier" not built | Architect |

---

## DoD — red → green (must be programmatically proven, not asserted)

A change is done only when each is RED before the fix and GREEN after, with pasted proof:

1. **Drain works.** Seed a `courier_dispatch_queue` row for an order with one `available`
   courier-shift, tick the pump → a `COURIER_DISPATCH` job is sent → `handleDispatch` creates a
   `status='assigned'` `courier_assignments` row for that order and **DELETEs the journal row**.
   *(integration test against the worker + a real/ephemeral pg-boss.)*
2. **No-courier branch returns cleanly; pump is the retry (RESOLVE).** Force the no-courier,
   not-yet-exhausted branch; assert it increments `attempts`, COMMITs, and **returns** with **no
   `TypeError`** (the `this.boss.send` line is deleted, not patched) and **no** `ROLLBACK`; the next
   pump tick re-attempts. RED on the current `this.boss` self-retry.
3. **Exhaustion is HONEST at both ends — void = RED (RESOLVE, ETHICAL-STOP-1).** With zero shifts,
   after `MAX_ATTEMPTS` the worker: (a) sets `orders.dispatch_exhausted_at` (assert the column is
   non-null for that order); (b) the **wired** `ORDER_DISPATCH_FAILED` consumer FIRES — assert a
   `NOTIFY_TELEGRAM_SEND` (owner) **and** a customer push are enqueued (spy); (c) only THEN is the
   journal row deleted. A test that asserts merely "publishes the event" while no consumer fires and
   no order state changes must be **RED**. *(integration: subscriber invoked + order column set + both
   sends enqueued.)*
4. **Idempotency / no double-assignment.** Two concurrent ticks / a double-pump for one order →
   exactly one active `courier_assignments` row (singletonKey + `order_active_uniq` proven; the
   second attempt is a no-op, not a 23505 crash-loop).
5. **23505 race is benign — no crash-loop, no false O3 (RESOLVE).** A journal row whose order is
   bound by another path between pre-check and INSERT → the `courier_assignments_order_active_uniq`
   23505 is special-cased → DELETE the row, COMMIT, **return success** (no throw, no pg-boss
   retry, no re-pump). Assert the pgboss failed-job count does not climb (Recon `O3` would otherwise
   false-DRIFT). RED today (generic `ROLLBACK; throw` → churn).
6. **`'assigned'` acceptance timeout.** An `assigned` past
   `COURIER_ASSIGN_ACCEPT_TIMEOUT_MS` → expired to `cancelled/assign_accept_timeout`, shift freed,
   re-enqueued. RED today (no such sweep).
7. **Offer-holding courier is never picked (RESOLVE).** With a courier holding an `'offered'`
   assignment (shift still `available`), the pump must NOT pick them → no
   `courier_one_active_assignment` 23505. Correct in both handshake-flag states. RED today
   (shift-pick omits `'offered'`).
8. **Recon runs + A6 watches the TRUE set of 8 — no false DRIFT (RESOLVE R3′).**
   `ReconciliationWorker.run()` executes all checks; with `backup-hourly`, `signal-raiser`,
   `courier-stale_check`, `liveness-checker` now heartbeating (15 s timer), `A6` returns PASS for all
   8 when alive and DRIFT only on a genuinely-stale worker. Assert **no false DRIFT** for
   `backup-hourly` while its process is up. RED if A6 is trimmed (zero coverage) or if backup-hourly
   has no heartbeat.
9. **No false `reoffered`.** `bindingRelease` returns `requeued` (renamed) and the offer-sweep log
   says "re-enqueued for dispatch"; the only "re-offered/assignment-created" signal is emitted by
   `handleDispatch` on a genuine new binding. *(unit assertion on the return shape + log string.)*
10. **RLS FORCE already present (RESOLVE — already-GREEN).** `verify:rls` already asserts
    `courier_dispatch_queue.relforcerowsecurity = true` (mig `1780421100051:14`). No new migration;
    keep the assertion as a standing guard.
11. **Held-marker migration is additive + safe.** `orders.dispatch_exhausted_at` added nullable, no
    default (metadata-only); `orders` RLS FORCE unaffected; integer-money untouched.
12. **Grace-window auto-transition (flag-gated, human-ratify).** With `DISPATCH_OWNER_GRACE_ENABLED`
    ON, an order with `dispatch_exhausted_at` older than `DISPATCH_OWNER_GRACE_MS` and no owner action
    → auto `CANCELLED` + `cancellation_reason='dispatch_exhausted'` + honest customer terminal push.
    Ships **default-OFF**; not GREEN-required for this change, but the test exists dark. Human gate at
    STOP-ETHICS (R-NEEDS-HUMAN-1).
13. **Tenant isolation.** A second tenant's journal row is never assigned to the first tenant's
    courier (each pumped job carries its own `location_id`; cross-tenant E2E from the
    realtime-isolation suite).
