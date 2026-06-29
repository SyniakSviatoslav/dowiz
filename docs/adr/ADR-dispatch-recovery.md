# ADR — Dispatch Auto-Recovery (B2) + Reconciliation Re-enable (B5)

- Status: **ACCEPTED (design)** — RESOLVE round applied (Breaker + Counsel dispositioned in
  `docs/design/dispatch-recovery/resolution.md`); no production code yet, focused re-attack pending.
- Date: 2026-06-29
- Deciders: Owner, System Architect
- Companion design: `docs/design/dispatch-recovery/proposal.md`
- Relates / does-not-contradict: `ADR-deliver-v2-cash-as-proof.md` (the `courier_dispatch_queue`
  re-offer mechanism + mig `1790000000073` partial-active uniques), `ADR-golive-remediation.md`
  (C3 removed `ReconciliationWorker`; R9 tracks re-enable), monolith-first, integer-money, RLS
  ENABLE+FORCE, JWT RS256, Postgres-backed idempotency.

## Context

`courier_dispatch_queue` (mig `1780421100044`) is the durable re-offer journal deliver-v2 intends
(`ADR-deliver-v2-cash-as-proof.md:97-98`). It has 4 producers (reject / decline / binding-release /
offer-timeout) and **0 consumers** — no path turns a queued row into a `COURIER_DISPATCH` pg-boss
job, so re-dispatch is 0% in every flag state. Three compounding defects: (1) the only retry path
`courier-dispatch.ts:76` calls `this.boss.send` where `this.boss` is `undefined` (constructor has
no `boss`; correct accessor is `this.queue.boss`) → `TypeError` → `ROLLBACK`; (2) no acceptance
timeout for an auto-assigned `status='assigned'` a courier never accepts → stuck shift + order; (3)
the offer-sweep log and `bindingRelease`'s `reoffered:true` claim recovery that never happens.

Separately (B5), the code-complete read-only `ReconciliationWorker` (12 checks incl. `A6`
worker-liveness watching `dispatcher`) is unregistered — removed in prod by ADR-golive breaker C3
because the **nightly** worker was the wrong executor for sub-minute lost-job detection (that one
detection was folded into a live 1-min sweep; the full worker deferred to R9). Result: O1/O2/O3,
M1–M4, N1/R1/F1/T1, and A6 run nowhere — including the gauge that would have caught the dead
dispatcher.

Pilot scale: 1 tenant, 3–10 locations, ~0.5 orders/sec. Constraints held: integer-money, RLS
ENABLE+FORCE, JWT RS256, Postgres-backed idempotency, no raw runtime `CREATE` on `pgboss` (but
`boss.createQueue`/`schedule`/`send` work at runtime), pool `max:8` unchanged.

## Decision

**Drain mechanism — durable journal + idempotent sweep-relay (Option C).** The table stays the
durable journal (already written by all 4 producers); a **drain pass folds into the existing
`CourierOfferSweepWorker.run()`** (1-min cron, advisory-locked, same connection) and pumps due rows
into `COURIER_DISPATCH` jobs via `boss.send(COURIER_DISPATCH, {orderId, locationId},
{singletonKey: orderId})`. Idempotency is layered: pg-boss `singletonKey` (one job per order in
flight) + the DB partial-active unique `courier_assignments_order_active_uniq` (mig `073`) as the
hard backstop (≤ one active binding per order). Rejected: a dedicated pump worker (A — extra
worker/lock/connection unjustified at pilot) and direct enqueue at the 4 sites (B — non-atomic,
4-site change, loses the self-healing journal; orphans the existing table).

**`this.boss` bug + 30 s self-retry — DELETED (RESOLVE).** The `startAfter:30s` self-retry was a
no-op anyway (suppressed by the same `singletonKey` while the job was still `active`), and it
referenced an undefined `this.boss`. Rather than patch it to `this.queue.boss`, **remove the
self-retry entirely**: the no-courier, not-yet-exhausted branch increments `attempts`, COMMITs, and
returns; the **60 s pump is the single retry cadence**. This deletes the bug outright and gives one
cadence to reason about. `COURIER_DISPATCH_RETRY_MS` is retired. (BOE: escalation ≈ 5 × 60 s ≈ 5 min.)

**`handleDispatch` idempotency guard + benign-race handling (RESOLVE).** Before assigning, if the
order already has an active binding (`status IN ('offered','assigned','accepted','picked_up')`) or is
terminal (`DELIVERED`/`CANCELLED`), DELETE the journal row and return. The **shift-pick must also
exclude couriers holding an `'offered'` assignment** (align `courier-dispatch.ts:55-58` with
`courier_one_active_assignment`'s `('offered','assigned','accepted','picked_up')`) — else an
offer-holding courier is picked → perpetual 23505 (correct in both flag states). And the catch must
**special-case 23505 by constraint**: `courier_assignments_order_active_uniq` (order bound elsewhere
in the TOCTOU window) → DELETE row, COMMIT, return success (no throw → no pg-boss retry → no re-pump →
no false Recon `O3`); `courier_one_active_assignment` (a picked courier raced) → do not delete, return
so the next tick re-picks a different courier; otherwise `ROLLBACK; throw`. Optionally `SELECT … FOR
UPDATE` the orders row in the pre-check to narrow the window.

**Exhaustion-tail honesty (RESOLVE — closes Counsel ETHICAL-STOP-1).** The terminal escalation must
be seen by a human and reflected to the customer; `ORDER_DISPATCH_FAILED` had **zero subscribers** and
the customer order was left "untouched". Fix, in scope: (a) **subscribe `ORDER_DISPATCH_FAILED` in
`bootstrap/messaging.ts`** (mirror `ORDER_ASSIGNMENT_CREATED` `:72`, claim-check-clean) → owner
Telegram-ops + an honest customer push; (b) on exhaustion set `orders.dispatch_exhausted_at` (durable
held / needs-attention marker; `order_status` stays truthful — no enum ripple) in the same
transaction; (c) **do not delete the journal row until the order marker is committed** — the durable
owner-visible trace is the committed order field, not a void event. The customer push is honest
("arranging your courier / slight delay"), never a false "on its way".

**`'assigned'` acceptance timeout.** A third pass in the same sweep expires `status='assigned' AND
assigned_at < now() - COURIER_ASSIGN_ACCEPT_TIMEOUT_MS` → `status='cancelled',
cancellation_reason='assign_accept_timeout'`, free the shift, re-enqueue. Reuses `'cancelled'`
(already in the `073` status CHECK) — **no enum migration**; the active-uniq frees the order for a
fresh binding. Applies in both flag states. Default window 5 min, gated on confirming the FE accept
window.

**Honest signals.** `bindingRelease` return `reoffered` → `requeued`; offer-sweep log → "re-enqueued
for dispatch". The genuine "assignment created / re-offered" signal is emitted only by
`handleDispatch` on a successful new binding (`ORDER_ASSIGNMENT_CREATED`).

**B5 re-enable — Option R3′ (RESOLVE; supersedes R3).** Re-register the full read-only nightly
`ReconciliationWorker`, and **instrument the 4 missing heartbeats instead of trimming A6**. Trimming
`A6.EXPECTED_WORKERS` to the 4 heartbeating ids was a Goodhart regression: those 4 are **identical**
to `LivenessChecker.CRITICAL_WORKERS` (`liveness-checker.ts:11`), so the trim adds zero coverage and
leaves `backup-hourly` (data-recovery red-line) and `liveness-checker` itself watched by nothing.
`WorkerHeartbeat` is a cadence-independent 15 s timer (`lib/worker/heartbeat.ts:32`), so an
hourly/nightly worker can heartbeat every 15 s (proving the process is alive) within A6's 1-hour
window. Add `backup-hourly, signal-raiser, courier-stale_check, liveness-checker` to `heartbeatConfigs`
(`workers.ts:98-103`) and add `backup-hourly` to `WORKER_CRITICAL_LIST` (live 60 s). A6 keeps all 8
ids and yields no false DRIFT. No migration (`boss.createQueue` at runtime). Nightly **detection +
alert** complements the sweep's sub-minute **recovery**. Closes R9.

**Final A6 set (8) + death-detection path per worker:**

| Worker | P31 heartbeat (15 s) | Live `LivenessChecker` (60 s, CRITICAL) | A6 nightly | Other |
|---|---|---|---|---|
| dispatcher | yes | yes | yes | — |
| settlement-cron | yes | yes | yes | — |
| dwell-monitor | yes | yes | yes | — |
| anonymizer-retention | yes | yes | yes | — |
| **backup-hourly** | **add** | **add to `WORKER_CRITICAL_LIST`** | yes | `BACKUP_FAILED` on run-fail + `BackupVerifyWorker` restore-test |
| **signal-raiser** | **add** | no | yes | — |
| **courier-stale_check** | **add** (from `CourierCronWorker`) | no | yes | — |
| **liveness-checker** | **add** | n/a (cannot watch itself) | **yes — A6 is the watcher-of-the-watcher** | — |

No worker is left without a detection path; `backup-hourly` gains both live (60 s) and nightly;
`liveness-checker` death is caught by the separate A6 recon worker.

**Schema (RESOLVE).** The proposed `ALTER TABLE courier_dispatch_queue FORCE ROW LEVEL SECURITY` is
**DELETED** — verified already applied at committed mig `1780421100051:14` (a no-op). The drain is
code-only. The one genuinely-needed migration is for the honest-tail:
`ALTER TABLE orders ADD COLUMN dispatch_exhausted_at timestamptz;` — additive, forward-only, nullable,
no default (metadata-only, no rewrite); `orders` RLS FORCE unaffected; no enum change (the
`'assigned'`-expiry and grace-terminal both reuse `'cancelled'`); no index at pilot (R-DEFER-1).
Net migrations after RESOLVE: **1 load-bearing**, replacing the deleted no-op.

**Grace-window (RESOLVE — human-gated).** After exhaustion sets the marker + alerts the owner, if the
owner does not act within `DISPATCH_OWNER_GRACE_MS` (default 15 min) the order auto-transitions to a
customer-honest terminal (`CANCELLED` + `cancellation_reason='dispatch_exhausted'` + honest customer
push). Ships **flag-OFF** (`DISPATCH_OWNER_GRACE_ENABLED=false`) pending ratification at STOP-ETHICS
(R-NEEDS-HUMAN-1). Standing constraint: an accept-timeout / dispatch-exhaustion must carry **no**
courier reliability penalty (no scoring system today — keep it scoring-free).

## Consequences

- **Positive:** re-dispatch goes from 0% → working in both flag states; the broken self-retry is
  removed (one honest 60 s cadence); `'assigned'` no longer strands shifts/orders; courier shortage
  is **genuinely owner- AND customer-visible** — a wired `ORDER_DISPATCH_FAILED` consumer + a
  persisted `orders.dispatch_exhausted_at` marker + an honest customer push — bounded (max-attempts),
  never silent or infinite, never a false "on its way"; the benign 23505 race is treated as benign
  (no crash-loop, no false `O3`); offer-holding couriers are never picked; monitoring tells the truth
  (no false `reoffered`); A6 watches the **true set of 8** (incl. `backup-hourly` + watcher-of-the-
  watcher) with no false DRIFT; O1/O3 + money/retention drift detection restored; zero new
  pools/queues/connections; one additive metadata-only migration.
- **Negative / accepted:** `attempts` double-counts enqueue-collision + dispatch-attempt — now safe
  because escalation is *visible* (R-ACC-1); `'assigned'`-expiry + grace-terminal reuse `'cancelled'`
  (R-ACC-2); no journal index at pilot (R-DEFER-1); recon adds a small nightly read burst (R-ACC-3);
  Option-C drain fold-in is load-bearing + untethered → guarded by the DoD-1 regression (R-ACC-4);
  slow-courier re-pick loop bounded by visible escalation (R-ACC-6); `COURIER_ASSIGN_ACCEPT_TIMEOUT_MS`
  default must exceed the FE accept window (R-OPEN-1); recon `N1`/`R1` detail must be confirmed
  PII-free before re-enable (R-OPEN-2); dual-context RLS policy parity on the journal deferred
  (R-FLAG-1).
- **Human-gated (STOP-ETHICS):** the grace-window auto-cancel (R-NEEDS-HUMAN-1) ships flag-OFF until
  the operator ratifies the recommended default (bounded grace → auto honest-terminal).
- **Open before merge:** confirm the FE accept timeout (R-OPEN-1); confirm recon detail strings are
  PII-free (R-OPEN-2); the 13-item red→green DoD in the proposal must be proven (RED→GREEN, pasted) —
  notably DoD #3 (consumer fires + order/customer state changes; void = RED).

## Alternatives considered

- **Dedicated pump worker (A)** — rejected: a second worker/advisory-lock/connection per minute is
  unjustified surface when an identical 1-min courier sweep already runs on the same table.
- **Direct enqueue at the 4 INSERT sites (B)** — rejected: must enqueue inside each caller's tx to
  stay atomic, touches 4 sites, and loses the self-healing journal (a lost job is gone forever); it
  orphans the existing table + producers.
- **Re-enable recon as-is (R1)** — rejected: `A6.EXPECTED_WORKERS` (8) ≠ heartbeating workers (4) →
  nightly false DRIFT → re-triggers C3.
- **Extract only O1/O3 into a new minimal worker (R2)** — rejected: more new surface than R3′ and
  discards already-built M/O/N/R/F/T/A6 coverage.
- **Trim A6 to the 4 heartbeating ids (R3)** — REJECTED in RESOLVE: the kept 4 == `CRITICAL_WORKERS`
  → zero added coverage while blinding A6 to `backup-hourly` + `liveness-checker`. Goodhart. Replaced
  by R3′ (instrument the missing heartbeats; A6 watches the true 8).
- **Patch `this.boss` → `this.queue.boss` and keep the 30 s self-retry** — rejected: the retry was
  suppressed by `singletonKey` anyway; deleting it (60 s pump = sole cadence) removes the bug and a
  whole class of double-send reasoning.
- **New `order_status` enum value for the held state** — rejected: an enum value on the red-line
  order state machine ripples through every FE/dashboard switch; a nullable
  `orders.dispatch_exhausted_at` column carries the durable held-marker with no enum ripple while
  `order_status` stays truthful (resolution.md §C).
- **Add `'assignment_expired'` status + migration** — rejected: an enum migration for a state that
  `'cancelled' + cancellation_reason` already expresses; the active-uniq frees the order either way.
- **A backing index on the journal now** — rejected at pilot (≤~75 rows seq-scan off the hot path);
  R-DEFER-1 at 10×.
