# RESOLVE — Dispatch Auto-Recovery (B2) + Reconciliation Re-enable (B5)

- Status: **RESOLVED (design)** — every Breaker finding + every Counsel item dispositioned.
- Date: 2026-06-29
- Author: System Architect (DeliveryOS)
- Inputs: `proposal.md` (rev 2026-06-29), `breaker-findings.md`, `counsel-opinion.md`, conductor steer.
- Output: this file + updated `proposal.md` + updated `docs/adr/ADR-dispatch-recovery.md`.
- No production code in this round. Verified against live source (citations below). A focused
  re-attack follows.

Disposition legend: **FIX** (resolved structurally in this design) · **ACCEPT-RISK(owner)** ·
**DEFER-FLAG(owner)** · **NEEDS-HUMAN** (staged for STOP-ETHICS ratification).

---

## A. Verification ledger (what I re-checked against the tree before dispositioning)

| Claim | Verified | Source |
|---|---|---|
| `ORDER_DISPATCH_FAILED` has 0 subscribers | TRUE | `bootstrap/messaging.ts` wires `ORDER_ASSIGNMENT_CREATED`/`ORDER_REJECTED`/`ORDER_STATUS`/`BACKUP_FAILED`/… — **not** `ORDER_DISPATCH_FAILED`. Grep: only registry const + the publisher + verify-orphans + docs. |
| Escalation DELETEs the journal row, never touches `orders` | TRUE | `courier-dispatch.ts:68-71` publishes then `DELETE FROM courier_dispatch_queue`; no `orders` UPDATE anywhere in `handleDispatch`. |
| `this.boss` undefined; 30s retry self-destructs | TRUE | constructor `:10-14` has `pool, queue, messageBus`; `:76` `this.boss.send(...)`. |
| §5 FORCE-RLS migration is a no-op | TRUE | `1780421100051_force-rls.ts:14` already `ALTER TABLE courier_dispatch_queue FORCE ROW LEVEL SECURITY` (051 > 044, applied). |
| A6 trim == `CRITICAL_WORKERS`; backup-hourly watched by nothing | TRUE | `reconciliation.ts:217-218` (8 ids); `workers.ts:98-103` heartbeats 4; `liveness-checker.ts:11` `CRITICAL_WORKERS` == those same 4. |
| `WorkerHeartbeat` is a 15s timer, cadence-independent | TRUE | `lib/worker/heartbeat.ts:28-38` `setInterval(intervalMs=15000)` beating `'healthy'` regardless of job runs. |
| Shift-pick omits `'offered'`; uniq covers it | TRUE | `courier-dispatch.ts:55-58` excludes `('assigned','accepted','picked_up')`; `courier_one_active_assignment` (mig `073`) covers `('offered','assigned','accepted','picked_up')`. |
| `order_status` has no held/needs-attention value | TRUE | enum = `PENDING,CONFIRMED,PREPARING,READY,IN_DELIVERY,DELIVERED,REJECTED,CANCELLED,SCHEDULED,PICKED_UP` (`1780310044710:14`). |

---

## B. Breaker findings — disposition

### [HIGH] B-FAIL · `ORDER_DISPATCH_FAILED` zero subscribers + journal-delete → silent drop  →  **FIX**
(Also Counsel ETHICAL-STOP-1.) The headline failure is relocated past the escalation, not closed.
Resolved structurally in **three** linked parts; all three become **DoD-gated**:

1. **Wire the consumer.** Subscribe `BUS_CHANNELS.ORDER_DISPATCH_FAILED` in `bootstrap/messaging.ts`,
   mirroring the `ORDER_ASSIGNMENT_CREATED` handler (`messaging.ts:72`, claim-check-clean — opaque
   `orderId`/`locationId` only) → owner ops outbox via `tgSend('order.dispatch_failed', orderId,
   locationId)` (Telegram-ops) **and** an honest customer push. The owner is actually told; the
   customer is told honestly.
2. **Change the ORDER state (no longer "untouched").** On exhaustion, in the **same transaction** as
   the journal handling, persist a held / needs-attention marker on the order so the DB itself
   carries the durable truth, and the customer push reflects it. Mechanism decision — §C below
   (persisted held-marker column on `orders`, `order_status` stays truthful, no enum-ripple). The
   customer push is an honest "we're arranging your courier / slight delay", **not** a false "on its
   way".
3. **Do not erase the trace before it is durable.** Reorder `handleDispatch` exhaustion so the order
   held-marker is **committed** before/with the journal-row delete; the owner alert + customer push
   fire after commit (same post-commit pattern as `ORDER_ASSIGNMENT_CREATED`, `courier-dispatch.ts:99`).
   The durable owner-visible trace is now the committed order marker — not a void event.

DoD #3 is rewritten: the consumer must **fire** (spy on `NOTIFY_TELEGRAM_SEND` + customer push
enqueue) and the **order/customer state must change** (assert the held-marker is set + ORDER_STATUS/
customer push emitted). "Publishes into the void" is RED.

### [HIGH] B-OPS/B-DATA · A6 trim leaves `backup-hourly` (+ liveness-checker) monitored by nothing  →  **FIX**
The proposal's R3 trim (8→4) is a Goodhart regression: the kept 4 == `CRITICAL_WORKERS` (live 60s),
so A6 adds zero coverage while deleting its only unique value. **Do not narrow by deleting coverage.**
Instead **instrument the genuinely-unmonitored workers** so A6 watches the TRUE expected set with no
false DRIFT. `WorkerHeartbeat` is a cadence-independent 15s timer (`heartbeat.ts:32`), so an hourly/
nightly worker can heartbeat every 15s while its *job* runs hourly — the heartbeat proves the
**process/registration is alive**, not that a backup ran. A6's `now() - interval '1 hour'` staleness
window is therefore satisfied by every added worker.

**Final A6 set = the existing 8** (`EXPECTED_WORKERS` unchanged) — each gains/keeps a death-detection
path:

| Worker | P31 hb (15s) | Live LivenessChecker (60s, CRITICAL) | A6 nightly | Other |
|---|---|---|---|---|
| dispatcher | keep | yes | yes | — |
| settlement-cron | keep | yes | yes | — |
| dwell-monitor | keep | yes | yes | — |
| anonymizer-retention | keep | yes | yes | — |
| **backup-hourly** | **ADD** | **ADD to `WORKER_CRITICAL_LIST`** | yes | `BACKUP_FAILED` on run-fail + `BackupVerifyWorker` restore-test |
| **signal-raiser** | **ADD** | no (not safety-critical to 60s) | yes | — |
| **courier-stale_check** | **ADD** (from `CourierCronWorker`) | no | yes | — |
| **liveness-checker** | **ADD** | n/a (cannot watch itself) | **yes — A6 is the watcher-of-the-watcher** | — |

`backup-hourly` (data-recovery red-line) gains both a live 60s path (CRITICAL) and the nightly A6
path. `liveness-checker`'s death is now caught by A6 (the separate recon worker) — closing the
"who watches the watcher" gap. No worker is left with "no detection path"; the ADR records each path
above. R3 is replaced by **R3′ — instrument-all-8** (no trim). R-INHERIT's "trim to 4" mitigation is
withdrawn; the false-DRIFT it feared is now prevented by *adding heartbeats*, not by blinding A6.

### [HIGH] B-CONSIST · 23505 handled as generic error → re-pump churn → false Recon O3  →  **FIX**
The pre-check/INSERT TOCTOU is real; the DB unique correctly blocks double-assignment (Breaker could
not break that). The defect is treating the benign race as an error. Fix the catch in `handleDispatch`
to special-case 23505 **by constraint**:
- `courier_assignments_order_active_uniq` (order already bound by another path) → **DELETE the journal
  row, COMMIT, return success** (resolved; no throw, no pg-boss retry, no re-pump, no false O3).
- `courier_one_active_assignment` (a picked courier was racing) → the order still needs a courier →
  **do NOT delete**; return without re-throwing so the next pump tick re-picks a *different* courier.
  (After the MED offer-holding fix below, this branch is near-dead, but kept defensively.)
- Any other error → unchanged `ROLLBACK; throw`.

Optionally narrow the window: add `SELECT … FOR UPDATE` on the **orders row** in the Q6 pre-check
(the queue row is already `FOR UPDATE`, `courier-dispatch.ts:33`). Recommended as belt-and-suspenders;
the 23505 catch is the load-bearing fix. DoD #5 rewritten to assert "no 23505 crash-loop, no false
O3" — not just "no second assignment".

### [MEDIUM] B-FAIL · `singletonKey` suppresses the in-flight 30s retry → BOE wrong  →  **FIX (by deletion)**
The 30s self-retry `send` fires while the same-key job is still `active` → suppressed → the 30s path
is dead; recovery actually rides the 60s pump. Rather than invent a distinct retry key (a second,
redundant recovery cadence on top of the pump), **drop the in-worker self-retry entirely**. The
no-courier, not-yet-exhausted branch becomes: increment `attempts`, COMMIT, **return** — the journal
row persists and the **60s pump is the single, honest retry cadence**. This also **deletes the
`this.boss` bug** outright (the broken `this.boss.send` line is removed, not patched) — boring,
proven, one cadence to reason about. `COURIER_DISPATCH_RETRY_MS` is retired (note in ADR).

**BOE corrected (§2):** escalation ≈ `MAX_ATTEMPTS × 60s pump ≈ 5 × 60s ≈ 5 min` (one attempt per
tick), **not 2.5 min**. Stated honestly. Q2 in the ADR changes from "fix `this.boss`" to "delete the
self-retry; pump is the sole cadence".

### [MEDIUM] B-ANTIPATTERN · §5 FORCE-RLS migration is already done (mig 051)  →  **FIX (delete from design)**
Confirmed `1780421100051:14`. **Delete the proposed `ALTER TABLE … FORCE` migration** — it is a
no-op. DoD #9 is **already satisfied** (mark it "already-GREEN; keep the existing `verify:rls`
assertion as a standing guard"). Honest correction: §5's "ENABLE but not FORCE" premise was
stale-grounded. The drain itself is **code-only** (no schema). However the honest-tail fix (B-FAIL
above) introduces **one genuinely-needed additive migration** — the order held-marker — so the design
is *not* zero-migration; it is one *load-bearing* migration instead of one no-op (§C).

### [MEDIUM] B-CONSIST · flag-ON: offer-holding courier picked → perpetual 23505  →  **FIX**
Align the candidate-pick predicate with the uniq's status set: the shift-pick subquery
(`courier-dispatch.ts:55-58`) must exclude couriers holding **any** of
`('offered','assigned','accepted','picked_up')` — add `'offered'`. An offer-holding courier (shift
still `available` until accept, `assignments.ts:151`) is then never picked → no
`courier_one_active_assignment` 23505. Correct in BOTH handshake-flag states. New DoD item.

### [LOW] B-CONSIST · accept-timeout boundary rug-pull + slow-courier re-pick loop  →  **DEFER-FLAG + ACCEPT-RISK**
- (a) Boundary accept racing the sweep: row-lock-guarded → no double-assign; the loser gets a clean
  "assignment expired" (rowcount=0) — define `COURIER_ASSIGN_ACCEPT_TIMEOUT_MS` to comfortably exceed
  the FE accept timer so the race is rare → **DEFER-FLAG (R-OPEN-1, owner Architect+FE)**: verify the
  FE accept-timer value before launch; default proposed 5 min.
- (b) Re-pick the same slow courier in a loop, burning `attempts` → **ACCEPT-RISK (owner Architect)**:
  bounded by max-attempts → escalation, which is now *visible* (B-FAIL fix). Optional later
  mitigation (exclude the just-timed-out courier from immediate re-pick) noted, not built.

### [LOW] B-SEC · bare queue RLS policy (no NULLIF/member branch) → latent 22P02  →  **DEFER-FLAG**
Dormant: all 4 producers set the courier GUC or run on the BYPASSRLS operational pool (verified by
Breaker). With FORCE now live (mig 051), a *future* NOBYPASSRLS owner-context producer with an empty
GUC would `22P02`. **DEFER-FLAG (R-FLAG-1, owner Lead)** — add dual-context parity with
`courier_assignments` (`073:46-49`) when/if such a producer is introduced; not load-bearing today.

### [LOW] B-SCALE · Recon M1 non-sargable scan  →  **ACCEPT-RISK (owner Architect)**
Pilot ~few-k rows → sub-100ms; 10× lifetime ~1–2s seq scan, one connection, 03:00 UTC off-peak. The
deferred `(enqueued_at)` index does not serve this query (computed comparison). Accepted; revisit at
10×. (R-ACC-3 covers the broader nightly read burst.)

### [LOW] B-DATA · `attempts` pre-inflation → escalate on first tick  →  **ACCEPT-RISK (owner Architect)**
"Earlier escalation is safe" now genuinely holds, because B-FAIL makes escalation **visible** (owner
alert + order held-marker + customer push). A producer-churned order escalating on its first dispatch
tick lands in a *seen* terminal, not a silent drop. Accepted (R-ACC-1 extended).

---

## C. The held-marker mechanism (decision for B-FAIL part 2)

**Requirement:** on dispatch exhaustion the ORDER must move to a customer-honest, owner-visible,
**persisted** state — not "untouched", not a false "on its way".

**Options considered:**
- **(i) New `order_status` enum value** (`NEEDS_ASSIGNMENT`). Honest state-machine model; reuses the
  status-keyed customer-push path. **Rejected as primary:** an enum value on a red-line state machine
  ripples through every FE/dashboard status switch (large regression radius) and `order_status` today
  is *truthful* already (`READY`/`PREPARING` are not lies — the order genuinely is prepared/cooking;
  the gap is that the **stuck** condition is untold).
- **(ii) Persisted held-marker column on `orders` (CHOSEN):** one additive, forward-only, nullable
  `orders.dispatch_exhausted_at timestamptz` (ADD COLUMN, no default → metadata-only, no table
  rewrite). Set in the exhaustion transaction. This is the durable owner-visible trace, the input the
  grace-window worker reads, and the field Recon `O1`/owner-dashboard filter on. `order_status` stays
  truthful. The **customer** is told via the wired consumer's honest push (delay/arranging-courier) —
  state change is the committed column, customer-truth is the push.
- **(iii) Owner alert row only (no order change).** Rejected: violates "transition the ORDER" — the
  durable trace would live off the order; owner-inaction → no order-level record for the grace worker.

**Chosen: (ii).** One additive migration (`orders.dispatch_exhausted_at`). This **replaces** the
deleted no-op FORCE-RLS migration: net migrations after this resolution = **1, load-bearing** (was: 1
no-op). Integer-money untouched. RLS: `orders` is already tenant-scoped + FORCE; an additive nullable
column needs no policy change.

> Honest correction to the conductor's MED-5 steer: the **drain** is code-only (true). The
> newly-scoped **honest-tail** fix needs this one additive column. "No migration at all" holds for
> the drain; the design as a whole carries one load-bearing migration.

---

## D. Counsel — disposition

### ETHICAL-STOP-1 — honest-failure to customer AND owner at the exhaustion tail  →  **REVISE (resolved structurally)**
Resolved by the B-FAIL FIX above (consumer wired → owner Telegram + dashboard; order held-marker
committed → durable + dashboard-visible; honest customer push → no false "on its way"). The trace is
no longer erased before it is durable. The **residual human decision** is narrowed to the
grace-window policy only (below) → staged for STOP-ETHICS. The DoD now makes "publish into the void"
RED.

### A6 monitoring — instrument all, don't trim  →  **FIX** (see B-OPS above; R3 → R3′).

### Steel-man — Option C drain fold-in is load-bearing + untethered  →  **ACCEPT-RISK (owner Architect) + guard**
Accept C's named risk explicitly: a future refactor dropping the fold-in pass silently re-introduces
stranding with no compile-time tether. **Guard it:** DoD-1 (seed journal row → tick pump → assignment
created + row deleted) is registered as a **standing regression test** so removing the fold-in goes
RED. New risk row R-ACC-4.

### Non-blocking — accept-timeout must carry no courier reliability penalty  →  **ACCEPT (recorded constraint)**
There is no courier scoring system today; an accept-timeout under bad signal must never silently
become a courier-quality mark. Recorded in the ADR as a standing constraint: keep it scoring-free.

### Open question — grace-window when the owner doesn't act  →  **NEEDS-HUMAN (pre-staged default)**
Pre-staged recommended default for STOP-ETHICS ratification:

> **YES — a bounded owner grace, then auto honest-terminal.** After exhaustion sets
> `dispatch_exhausted_at` + alerts the owner, a grace worker (folded into the same sweep, reading the
> marker) waits `DISPATCH_OWNER_GRACE_MS` (recommended default **15 min**, configurable). If the owner
> has not manually assigned/acted within the window, the order auto-transitions to a customer-honest
> **terminal** state — reuse existing `CANCELLED` + `cancellation_reason='dispatch_exhausted'` +
> refund-path — and the customer gets an honest terminal push. The customer's truth never depends
> solely on a human noticing an alert.

Rationale: owner-inaction must not equal permanent customer silence. Reuses `CANCELLED` (no new enum),
the refund-path already exists, and the grace worker is one more read on the existing sweep marker.
**Owner of the human call:** Owner/operator at STOP-ETHICS. Until ratified, the grace auto-transition
ships **flag-OFF** (`DISPATCH_OWNER_GRACE_ENABLED=false`); exhaustion still sets the marker + alerts
(that half is in-scope and on). This is the only residual human gate.

---

## E. Net changes pushed into proposal.md + ADR

1. §2 BOE: escalation ≈ 5 min (5 × 60s pump), 30s self-retry deleted. Honest.
2. §4 Q2: delete the self-retry (removes `this.boss` bug); pump is sole cadence. Retire
   `COURIER_DISPATCH_RETRY_MS`.
3. §4 Q5 / R3 → **R3′**: instrument all 8 heartbeats (no trim); A6 watches the true set; per-worker
   detection paths recorded (incl. backup-hourly live + nightly, liveness-checker via A6).
4. §4 new Q7 — **exhaustion-tail honesty**: wire `ORDER_DISPATCH_FAILED` consumer (owner + customer),
   set `orders.dispatch_exhausted_at` in the exhaustion tx, don't erase the journal row until the
   order marker is committed.
5. §4 Q6 hardening: 23505 special-cased by constraint; offer-holding courier excluded from shift-pick.
6. §5: delete the no-op FORCE-RLS migration; add the one additive `orders.dispatch_exhausted_at`
   column; DoD #9 already-GREEN.
7. §7/§9: customer-honest degradation explicit; the held-marker + alert are the visible fallback.
8. §10 risks: add R-ACC-4 (fold-in untethered → DoD-1 regression), R-NEEDS-HUMAN-1 (grace-window),
   no-scoring-penalty constraint; withdraw R-INHERIT trim mitigation.
9. DoD: rewritten #2 (no-courier branch returns, pump retries, no `this.boss`), #3 (consumer fires +
   order/customer state changes — void = RED), #5 (no 23505 crash-loop, no false O3), #9 (already
   satisfied), + new items: offer-holding courier never picked; held-marker set on exhaustion;
   grace-window auto-transition (flag-gated, human-ratify).

---

## F. Residuals (honest)

- **R-NEEDS-HUMAN-1 (grace-window):** one human decision remains, pre-staged with a recommended
  default; ships flag-OFF until ratified. Owner: operator at STOP-ETHICS.
- **R-OPEN-1 (accept-timeout vs FE timer):** verify the FE accept-timer value before launch. Owner:
  Architect + FE.
- **R-OPEN-2 (recon N1/R1 PII-free):** confirm before re-enable. Owner: Architect.
- **R-FLAG-1 (dual-context queue RLS policy):** latent; add when a NOBYPASSRLS owner-context producer
  appears. Owner: Lead.
- **R-ACC-1/2/3/4, R-DEFER-1:** accepted, owners recorded in §10.
- These are **not** marked "resolved" — they are accepted/deferred/human-gated with named owners, per
  the no-self-clearing rule. A focused Breaker re-attack follows.
</content>
</invoke>
