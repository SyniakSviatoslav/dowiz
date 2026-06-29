# Breaker Findings — Dispatch Auto-Recovery (B2) + Reconciliation Re-enable (B5)

Attacker: System Breaker DeliveryOS. Axis: where does it break, not whether it is nice.
Target: `docs/design/dispatch-recovery/proposal.md` (rev 2026-06-29).
Method: READ-ONLY verification against live source + migrations. No fixes proposed.

Verdict up front: the design's two **headline safety claims are false against the code it
ships on** — (1) "max-attempts → ORDER_DISPATCH_FAILED → owner-visible, never silent" publishes to
a channel **nobody subscribes to**, and (2) §5's "queue is not FORCE, add a FORCE migration" is
**already done** in committed mig `051`. True double-assignment (two couriers / two cash custodians)
is **DB-blocked** by `courier_assignments_order_active_uniq` — I could not break it; the residual is
error-handling churn, not money-to-two. Ranked by exploitability below.

---

## [HIGH] B-FAIL · `ORDER_DISPATCH_FAILED` has ZERO subscribers — max-attempts = SILENT order drop, trace erased

The whole "no silent drop / owner-visible terminal" claim (§7, §B2, §9) rests on one publish:
`courier-dispatch.ts:68` → `BUS_CHANNELS.ORDER_DISPATCH_FAILED` (`registry.ts:11` = `'order.dispatch_failed'`).

Grep of the entire `apps/api/src` + `packages/`: the string appears **exactly twice** — the channel
definition and the single publish. **No `subscribe` / `on` / `work` consumer exists.** It is a
publish into the void.

Scenario (genuine courier shortage, the expected degradation): order O exhausts `MAX_ATTEMPTS`
in the no-courier branch → `courier-dispatch.ts:69` **`DELETE FROM courier_dispatch_queue`** runs,
then the void-publish. Result:
- journal row **deleted** → the only durable trace of the stranded order is gone (worse than
  pre-fix, where the row at least persisted);
- `handleDispatch` never touches `orders.status` → the order sits in `READY`/`CONFIRMED` forever
  with no courier, no binding, no alert;
- no owner notification fires. The customer sees "preparing" indefinitely.

This is the **exact B2 failure mode the design claims to close**, just relocated past the escalation.
DoD #3 ("publishes `ORDER_DISPATCH_FAILED`") passes a spy assertion while being operationally inert.
Violated invariant: *backend/dispatch failure → order survives with a visible fallback*; *visibility
of failure < 1 min*. Here failure is invisible and the trace is destroyed.

---

## [HIGH] B-OPS/B-DATA · A6 trim (8→4) leaves `backup-hourly` monitored by NOTHING — death of the backup worker is now invisible

Confirmed: `reconciliation.ts:217-218` `EXPECTED_WORKERS` lists 8 ids; bootstrap heartbeats only 4
(`workers.ts:98-103`: `dispatcher, settlement-cron, dwell-monitor, anonymizer-retention`). The
proposal (R3) trims A6 to those same 4. The **dropped** 4 are
`signal-raiser, liveness-checker, courier-stale_check, backup-hourly`.

The kill: `LivenessChecker.CRITICAL_WORKERS` (`liveness-checker.ts:11`) is **the identical 4**
(`dispatcher,settlement-cron,dwell-monitor,anonymizer-retention`). So after the trim, A6 watches
*exactly the same set the live 60s checker already watches* → A6 contributes **zero** new
worker-liveness coverage, while the proposal sells it as "exactly the gauge that would have caught a
dead dispatcher" (§B5) — but `LivenessChecker` already catches dispatcher death every 60s.

A6's only *unique* value was watching the OTHER four — and that is precisely what is removed.
Consequence by worker:
- **`backup-hourly`** (`BackupCronWorker`, `workers.ts:62/69` — hourly DB backup, a data-recovery
  red-line) is in **neither** `CRITICAL_WORKERS` **nor** the trimmed A6. A dead backup worker →
  silent backup gap → **unrestorable DB on disaster**, detected by nothing. (`BackupVerifyWorker`
  is likewise unmonitored.)
- **`liveness-checker`** itself: the design leans on it for sub-minute dispatcher-death detection
  (§7, §9). Trimming it from A6 means *the monitor-of-monitors is monitored by no one* — if
  `LivenessChecker` dies, dispatcher death is caught neither live nor nightly.

This is a monitoring **regression dressed as a fix**: the false-DRIFT noise is removed by deleting
the monitoring, not by adding the 4 missing heartbeats. Violated invariant: *visibility of a
component death < 1 min*; *backup restorable / monitored*.

---

## [HIGH] B-CONSIST · 23505 is handled as a generic error (throw → re-pump), not "already-resolved" — Q6 pre-check narrows but does not close the TOCTOU

The Q6 pre-check (SELECT active binding → DELETE row → return) runs **inside the same tx** as the
shift-pick and the `INSERT INTO courier_assignments` (`courier-dispatch.ts:84`), and the order row is
**not** locked. Window: between the pre-check SELECT (sees no active binding) and the INSERT, a
concurrent **owner manual-assign** (`dashboard.ts:330/342`), **courier accept**, or `lib/dispatch.ts`
path can COMMIT an active binding for the same order.

`courier_assignments_order_active_uniq` (mig `073:22-24`, `WHERE status IN
('offered','assigned','accepted','picked_up')`) then makes the pump's INSERT throw **23505** — so
**two couriers are correctly prevented** (no money-to-two; I could not break this). BUT the catch
block `courier-dispatch.ts:101-103` is `await ROLLBACK; throw err` — it does **not** special-case
23505 as "already handled / delete the row." The throw → pg-boss job failure → pg-boss retry +
the 60s pump re-pump. The journal row survives (rolled back), so it churns until a *later* tick's
pre-check happens to win the order against the INSERT.

So DoD #5's claim — "the second attempt is a no-op, not a 23505 crash-loop" — is **false in the race
window**: the unique violation is still routed through the failure path. Bounded (not infinite), but
it (a) pollutes `pgboss.job` failed-state, which (b) trips Recon **O3** (`>10 failed jobs/24h →
DRIFT`, `reconciliation.ts:237-252`) — a self-inflicted false alarm. Violated invariant: *parallel
status transitions guarded as "already handled", not surfaced as an error that re-pumps*.

---

## [MEDIUM] B-FAIL/B-ANTIPATTERN · `singletonKey` suppresses the in-flight retry send → 30s retry is dead, §2 escalation BOE is wrong

The no-courier branch COMMITs, then (per Q2) `this.queue.boss.send(COURIER_DISPATCH, …,
{startAfter: 30s, singletonKey: orderId})` — **but this send executes while the current
`COURIER_DISPATCH` job (same `singletonKey: orderId`) is still in `active` state** (the `.work`
handler has not returned). pg-boss singleton dedup spans `created/active/retry`; an active job holds
the key → the retry send is **suppressed (returns null, no job)**.

Effect: the configured `COURIER_DISPATCH_RETRY_MS=30000` self-retry path is **dead** — recovery for
a no-courier order falls entirely to the 60s pump. Therefore §2's "Max-attempts math: 5 × 30 s ≈
2.5 min" is wrong; real escalation ≈ **5 × 60 s ≈ 5 min** (attempts increment once per pump tick,
not per 30s). DoD #2 spies that `send` is *invoked* — it is — so it goes **green while the job never
materializes** (false-green). Violated: *every BOE must hold*; *DoD asserts the real effect, not a
spied call*.

---

## [MEDIUM] B-ANTIPATTERN · §5 premise is STALE — `courier_dispatch_queue` is ALREADY `FORCE ROW LEVEL SECURITY`

§5 states "`courier_dispatch_queue` (mig `1780421100044`) is ENABLE but **NOT FORCE**" and proposes a
new forward-only `ALTER TABLE … FORCE ROW LEVEL SECURITY`. But committed migration
`1780421100051_force-rls.ts:14` **already** runs exactly `ALTER TABLE courier_dispatch_queue FORCE
ROW LEVEL SECURITY` (committed in `84b95d66`, no STAGED header → a normal applied migration; `051 >
044`). DoD #9 (`relforcerowsecurity = true`) is therefore **already GREEN today**.

The proposed migration is a redundant no-op. More importantly, the §5 analysis ("the correct
hardening, no behavior change today") was written without reading the live schema state — the same
class of stale-grounding the project's own memory repeatedly flags. The load-bearing §5 reasoning is
built on a false premise. Violated: *verify against actual schema before asserting state*.

---

## [MEDIUM] B-CONSIST · flag-ON shift-pick status-set ≠ uniq status-set → an offer-holding courier 23505-loops the pump

`courier-dispatch.ts:55-58` excludes couriers with an assignment `IN ('assigned','accepted',
'picked_up')` — it does **not** exclude `'offered'`. But `courier_one_active_assignment` (mig
`073:32-33`) covers `IN ('offered','assigned','accepted','picked_up')`. In handshake-ON, an
`'offered'` binding leaves the courier's shift `'available'` (`assignments.ts:151` sets
`on_delivery` only on *accept*). So the pump can pick a courier who already holds an outstanding
offer → `INSERT 'assigned'` → **23505 on `courier_one_active_assignment`** → throw → re-pump (same
failure path as the HIGH above). At pilot with few couriers, if the available couriers all hold
offers, the order **can never be assigned** and the pump churns until offers resolve. The design
claims "must be correct in BOTH flag states" (§Non-goals); flag-ON is not. Violated: *correct in
both flag states*.

---

## [LOW] B-CONSIST · accept-timeout clock starts at auto-assign; boundary accept = rug-pull; slow courier re-picked in a loop

The new pass expires `status='assigned' AND assigned_at < now() - COURIER_ASSIGN_ACCEPT_TIMEOUT_MS`
(`assigned_at` set at `handleDispatch` INSERT). Verified the courier DOES need to accept in flag-OFF
too (`assignments.ts:158` legacy `assigned→accepted`; `picked_up` requires `status='accepted'`
`:255`), so the target state is genuinely "courier has not acknowledged" — not a live delivery. Two
residual nits: (1) a courier tapping accept at the 5-min boundary races the sweep — row-lock-guarded
so no double, but the loser gets a `rowcount=0` "assignment gone" error (UX rug-pull); (2) after
cancel+free+re-enqueue, the shift-pick can **re-pick the same slow courier** for the same order →
assigned → 5 min → cancelled → loop, burning `attempts` until escalation (into the no-op of finding
#1). Bounded, low impact.

## [LOW] B-SEC (latent) · queue policy is bare `current_setting('app.current_tenant')::uuid` — no NULLIF/member branch

Unlike `courier_assignments` (`073:46-49`, dual-context `NULLIF(...,'')` + `app_member_location_ids()`),
the queue policy (`044:15-16`) is the bare `location_id = current_setting('app.current_tenant')::uuid`.
With FORCE now on (mig `051`), any future **NOBYPASSRLS producer with an empty GUC** throws `22P02`
(`''::uuid`) and rolls back the caller's tx. Dormant today — verified all 4 producers either set the
courier GUC (`assignments.ts:81/422/486`, `bindingRelease` is courier-only: callers at
`assignments.ts:448/501`) or run on the BYPASSRLS operational pool (offer-sweep). R-FLAG-1 defers it;
acceptable, but it is a live tripwire for any new owner-context producer.

## [LOW] B-SCALE · Recon M1 is non-sargable — heaviest nightly query, accepted at pilot

`checkPricingIntegrity` (`reconciliation.ts:116-123`) filters
`total != subtotal + delivery_fee + tax_total - discount_total` — a computed comparison no index can
serve; planner scans the `created_at>7d` slice (or the whole table if `created_at` is unindexed).
BOE: pilot ~few-k orders/week → sub-100ms. 10× lifetime ~1–5M rows → ~1–2s seq scan, one connection,
03:00 UTC. Acceptable; R-DEFER-1's `(enqueued_at)` index does not help this query. Flagged only as
the genuine heaviest of the 12 checks; not a C3-class regression.

## [LOW] B-DATA · `attempts` pre-inflation can escalate on the FIRST tick — compounds finding #1

`attempts` is incremented by both the 4 producers (`ON CONFLICT … attempts+1`) and `handleDispatch`
(R-ACC-1, accepted). A churned order (multiple reject/decline/timeout before the pump runs) can reach
`attempts ≥ MAX_ATTEMPTS` from producers alone. Then the **first** pump tick with no courier
escalates immediately — not after the 2.5/5-min window — straight into the **no-op
`ORDER_DISPATCH_FAILED`** of finding #1, deleting the journal row on its very first dispatch attempt.
The accepted "earlier escalation is safe" assumption is only safe if escalation is *visible*; with
finding #1 it is silent, so this accelerates the silent drop.

---

## Regression / what I could NOT break (honest scorecard)

- **Two couriers for one order (money/cash to two custodians):** BLOCKED at the DB by
  `courier_assignments_order_active_uniq` (`073:22-24`) for every INSERT path
  (`courier-dispatch.ts`, `dashboard.ts`, `dispatch.ts`, `server.ts`). I found no path to two active
  bindings. The residual is error-handling churn (finding #3/#6), not double-assignment.
- **Two concurrent drain-passes:** prevented — `pg_try_advisory_lock(9)` + pg-boss `singletonKey`
  on the sweep queue (`courier-offer-sweep.ts:24/35`); a long drain just makes the next tick early-
  return. Holds.
- **Cross-tenant pump leak:** the sweep/pump run on the BYPASSRLS operational pool doing a
  deliberate cross-tenant pass, each pumped job carries its own `location_id`. No leak found; the
  FORCE flip (already live, finding #5) does not change pool behavior.
- **Infinite re-pump on genuine shortage:** bounded by `attempts >= MAX_ATTEMPTS` → delete. No
  infinite loop (the failure is *silent*, finding #1 — but not infinite).
