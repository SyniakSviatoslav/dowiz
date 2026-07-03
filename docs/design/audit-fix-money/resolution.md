# Council RESOLVE — MONEY audit fixes (round 1 → revision v2)

- **Status:** RESOLVE round complete. `proposal.md` revised to **v2** in the same commit; conductor re-runs the breaker on the revision (no self-certification claimed here).
- **Inputs:** `proposal.md` (v1), `breaker-findings.md`, `counsel-opinion.md`, conductor RESOLVE directive.
- **Rule applied:** every breaker finding → FIX / ACCEPT-RISK / DEFER-FLAG; every counsel ETHICAL-STOP → design revision or needs-human-decision. Nothing dropped silently.

---

## 0. The one structural change that resolves most of the board

v1 hung the LC6 invariant on **fail-closed inline folds** — including inside the fleet-wide atomic sweep (C1) and grafted onto a context-free customer connection (H1). v2 replaces that single load-bearing wall with **four decoupled layers, none of which can block a batch and none of which can fail silently**:

| Layer | What | Failure behavior |
|---|---|---|
| **L-A** app-fold in `updateOrderStatus` | primary transactional recorder on funnel paths (per-order tx) | fail-closed **per order only** + Sentry/DRIFT alert + operator force-terminal escape (§ESC-2) |
| **L-B** webhook fold (pay-after-cancel) | unchanged from v1 | per-payment tx; idempotent unique |
| **L-C** minimal `SECURITY DEFINER` **trigger** on `orders` status → CANCELLED/REJECTED (counsel's hybrid, adopted) | structural floor for **every** writer — funnel, DEFINER fns, raw UPDATE, future sinners; per-row GUC save/restore so it is B3-flip-safe | **non-throwing** (per-row `BEGIN/EXCEPTION` swallow → detected by L-D); `ON CONFLICT DO NOTHING` |
| **L-D** `app_reconcile_refund_due()` reconciler | DEFINER fn, per-row exception isolation, called by the timeout-sweep worker every tick after the sweep; also surfaces `mismatch`-class terminal orders | inserts what the other layers missed within ≤1 tick; persistent failures → **DRIFT + Sentry + operator-visible alert**, never a freeze |

The sweep fn itself is **no longer modified at all** (v1's M-1 refund-fold is dropped). Sweep liveness is fully decoupled from money-ledger writes. Invariant restated two-tier: *transactional where a healthy layer covers the path; bounded-lag (≤1 reconciler tick) + surfaced-alert floor everywhere, for every writer.* Unrecorded debt can no longer be silent, and no recording failure can halt cancellation for anyone.

---

## 1. Breaker findings — dispositions

### C1 — fleet-wide sweep wedge → **FIX (redesigned; proposal §3.2–§3.3)**
The sweep is untouched; no throwing INSERT exists in any batch statement. Obligation for sweep-cancelled orders is recorded by L-C in the same tx when healthy (trigger fires under the sweep's UPDATE; a trigger failure is swallowed per-row and never aborts the batch) and by L-D within one tick otherwise. B3-flip safety: L-C/L-D are `SECURITY DEFINER` with per-row `app.current_tenant` save/restore satisfying the dual policy's GUC arm (`1790000000083:73-81`) — no dependence on member context or BYPASSRLS. New proofs P6 (sweep cancels AND refund_due appears), **P6b (poison-row liveness: forced insert-failure → sweep still cancels everything, alert fires)**, P14 (raw-UPDATE writer → trigger records), P15 (reconciler backstop + alert). The v1 B3-flip outage vector is gone because no batch path performs a throwing member-context insert.

### C2 — settlement backfill double-pay → **FIX (redesigned; proposal §4.1–§4.2)**
Cron **never** auto-creates pre-existing-backlog obligations. Catch-up scan is lower-bounded by a **literal deploy watermark** baked into the fn body (M-2): only post-fix rows self-heal (those cannot have been reconciled out-of-band pre-fix). Historical (pre-watermark) rows move only through an **operator-gated** flow: (1) read-only magnitude pre-count per courier×location pair (count + sum), (2) explicit operator invocation of a separate `app_backfill_historical_settlements()` (never called by cron), (3) rows land `pending` with `settlement_items.backfilled = true` (additive column) so operator AND courier see "N caught-up deliveries from before <fix-date>", (4) `/pay` remains human-gated and unchanged. No path auto-pays historical cash. Counsel's §4 legibility advice (pre-count heads-up, courier note) adopted wholesale. Proofs P9b (pre-watermark row NOT auto-swept), P13 (backfill flow: pre-count matches, flagged pending rows, no auto-pay path).

### H1 / DEP-1 — customer-cancel 500 (context-free fold) → **FIX (proposal §3.3.2)**
DEP-1 is now a specified precondition, not a hand-wave: the customer cancel fix must (a) verify order ownership via the session token, (b) resolve `location_id` and establish tenant context with `set_config('app.current_tenant', $loc, true)` on the tx — the **exact precedent already live at `apps/api/src/routes/payments-webhook.ts:41`** (DEFINER resolver → GUC → dual-policy GUC arm), (c) call `updateOrderStatus` inside that tx. The fold's insert then passes RLS under FORCE, pre- and post-B3. Proof **P4b**: crypto-paid customer cancel under a FORCE-RLS non-bypass role → 200 + `refund_due` row. Even if the fold still failed, L-C/L-D + alert mean no silent loss and no permanent wedge (ESC-2 shape).

### M2 — raw-UPDATE writers un-backstopped → **FIX (structural, via L-C + L-D)**
Every writer — present or future, app or DEFINER fn — is covered transactionally by the trigger and temporally by the reconciler with an alarm. The grep/eslint ban on raw `UPDATE orders SET status` is retained but **honestly relabeled advisory speed-bump**, no longer the only guard. Proof P14.

### M3 — mismatch (over/under-payment) leak → **PARTIAL FIX + DEFER-FLAG (proposal §3.6, §5)**
Fixed now: L-D also detects terminal non-fulfilled orders carrying `mismatch` events and raises an operator alert — received-crypto-on-dead-order is **surfaced, never silent** (proof P16). Deferred (MISSING, flagged): automatic `refund_due` for mismatch — because the obligation *amount* is ambiguous (overpay = amount received ≠ payment amount; underpay = partial receipt) and disposition policy is its own money-red-line decision; auto-obligating the wrong number is worse than alert-plus-human. All "LC6 closed" language rewritten to the scoped claim: **closed for `status='paid'` payments; mismatch-class funds surfaced-not-auto-obligated; full disposition = follow-up item `M3-mismatch-disposition`, owner: conductor backlog, precondition for calling crypto refunds "done".**

### M4 — ratchet alias/hoist-evadable → **FIX (strengthened) + ACCEPT-RISK residual (proposal §2.5)**
Strengthened to a mechanically checkable shape: expected values must live in **data-only vector files** (literals only, zero imports — a lint can verify "no import statements + only literal initializers" reliably); composition test files may import **only** the module under test + the vector file. Alias/hoist/laundering evasions all require an import that the rule rejects — there is no call-shape analysis to evade. **ACCEPT-RISK (residual):** vectors generated offline by running the implementation and pasting outputs is not mechanically detectable by any static gate. Owner: reviewer discipline — mandatory derivation comment per vector (bc/spreadsheet arithmetic shown) + ledger row; documented in the test header. This residual is inherent to any oracle-independence property and is accepted explicitly.

### M5 — parity-matrix demotion thins hotfix-window coverage → **FIX (proposal §2.5.3, §7)**
Demotion moved from commit 1 to the Option-B commit. Commit 1 *adds* (never removes): independent-constant vectors + property test + a **route-level server composition integration matrix (≥4 vectors: inclusive/exclusive × zero/boundary rates)** so server composition has combinatorial coverage before `composeOrderTotal` exists. The 432-combo parity matrix keeps running unchanged until Option B replaces the mirror itself.

### M6 — 409 covers only active-binding half of the strand → **FIX (proposal §3.5)**
Guard widened: owner PATCH → DELIVERED is refused with 409 when **(a)** an active binding exists (`ASSIGNMENT_ACTIVE`, as v1) **or (b)** the order is `IN_DELIVERY` without a `delivered` assignment (`USE_DELIVER_FLOW`) — the drained-binding strand the breaker found. Never-dispatched orders with zero assignments remain PATCH-able (nothing to strand, no courier cash in play — phone/manual flow preserved). Owner-proxy `/deliver` (`owner/dashboard.ts:447`) is the sanctioned completion for both refused cases. Proof P7 extended to both arms.

### M7 — receipt renders `sub + tax ≠ total` incoherence → **FIX (proposal §2.7)**
FE presentation is now in-scope: for inclusive venues the tax line renders as informational "includes VAT (r%) — X", never as an addend; `estimateOrderTotal` gains an additive `chargedTax` field so the FE cannot mis-render structurally; i18n keys (al/en) enumerated; E2E proof **P7b** asserts the label and `total === subtotal + fee` on a rendered receipt.

### L1 — settlement fns' no-GUC cross-tenant writes post-B3 → **DEFER-FLAG (MISSING)**
Not folded into M-2. Why: the B3/NOBYPASSRLS flip is its own gated launch-blocker council (memory: `launch-blocker-councils`), and bundling cross-tenant RLS rework into a money-fix migration inflates the red-line blast radius of both. Flagged as an explicit **B3-flip precondition checklist item**: "`app_generate_settlements` + sweep history-insert need a post-flip RLS strategy; the per-row GUC save/restore pattern shipped in L-C/L-D is the template to copy." Recorded in proposal §5 register (DEP-2).

### L2 — CREATE OR REPLACE mechanics → **FIX (proposal §5 mechanics checklist)**
Added verbatim: exact return-signature match required (`app_sweep_timeout_orders() RETURNS TABLE(id uuid, location_id uuid)` untouched anyway in v2; `app_generate_settlements(timestamptz,timestamptz) RETURNS void`), re-emit the `REVOKE ALL FROM PUBLIC` + `GRANT EXECUTE TO dowiz_app` block per the 078 pattern, staging DB first per Ship Discipline.

### L3 — indefinite deferral corner → **ACCEPT-RISK + mitigation (proposal §4.3)**
Accepted as deferral-not-loss (unique on `assignment_id` guarantees no double-settle; item eventually lands). Mitigation added: a backlog drift metric — oldest unsettled `delivered + cash_collected` item age, alert threshold (default 7 days) — plus the owner-UI "unsettled backlog" note. Owner: platform operator (settlements surface).

### Breaker's held attack (tax math sound) → recorded; no action. The four-quadrant table is carried into the P1 vector derivation comments.

---

## 2. Counsel ETHICAL-STOPs — dispositions

### ESC-1 — restitution decision record + VAT trace → **DESIGN REVISED + needs-human-decision (the decision itself)**
Encoded in proposal §2.6 as a **closure requirement**, not advice:
1. LC1 remediation is **not closed** until an operator **DECISION RECORD** exists — `{decision: refund | partial | notify-only | documented-no-action-with-cause, owner, date, rationale}` — filed under `docs/decisions/` (or the ops record of record).
2. **Precondition to that decision (blocking it, not the hotfix):** the **VAT trace** — establish whether `orders.tax_total` / `orders.total` feed any VAT filing, owner report, or export surface (owner analytics export, any accounting handoff). If venues over-remitted VAT to the Albanian tax authority off inflated figures, LC1 is a **compliance obligation with the state in the loop, not a discretionary goodwill call** — the record must say which it is.
3. The decision packet must carry: where the money sits (venue revenue vs over-remitted VAT vs platform), the refundable (crypto/card) vs practically-unreachable (cash) split, and the three-actor note (fault = platform, gain = venue, harm = customer).
The **decision itself is needs-human-decision** by design — the operator decides; the design only refuses to let silence default to keep-the-money. The forward hotfix ships regardless (counsel's own scoping).

### ESC-2 — fail-closed must fail to a human, never a silent freeze → **DESIGN REVISED (fixed structurally)**
No path in v2 can freeze silently or permanently:
- batch paths are never fail-closed (C1 fix);
- a per-order fold failure aborts **that order's** cancel only, and **must** emit Sentry + a DRIFT counter + an operator-visible alert (surfacing is part of the fold's contract, not optional);
- an **operator force-terminal escape hatch** exists: a conscious operator can force the cancel/reject with the fold SAVEPOINT-swallowed, which writes an audit row and fires the same friction-alert; L-D keeps retrying the recording and keeps alarming until it lands ("тертя-не-вирок, людина-завжди-завершує");
- armed-at-flip: the crypto flag-flip checklist gains the item "ESC-2 escape hatch + alert path proven on staging" (proof P15 covers the alert arm).

### Counsel sequencing advice (live worker harm before dark fix) → **ADOPTED (proposal §7)**
Rollout reordered: **1) LC1 hotfix (live customer harm) → 2) Settlement M-2 (live courier harm) → 3) LC6 layers (dark until flip) → 4) Option B consolidation + ratchet.** Nothing couples settlement behind LC6; the ethical ordering costs nothing technically.

### Counsel §3 "name the §2.6-vs-§4.1 seam" → **ADOPTED** — one-line doctrine added to proposal §4.2: *auto-mutating money-out: never; auto-surfacing an existing obligation into a human-gated pending queue: allowed — and post-C2, pre-watermark history does not even auto-surface; it is operator-pulled.*

### Counsel §5 trigger steel-man → **ADOPTED** (hybrid, as directed): app-fold primary + minimal DEFINER trigger as structural floor, **not deferred** to a hypothetical third bypass. The trigger is deliberately non-throwing so it can never recreate C1; its misses are caught by L-D's alarm, which is the deterministic detector.

---

## 3. Scoreboard

| ID | v1 severity | Disposition | Where |
|---|---|---|---|
| C1 sweep wedge | CRITICAL | **FIXED** (sweep untouched; trigger+reconciler; no batch fail-closed) | §3.2–§3.3, P6/P6b/P14/P15 |
| C2 backfill double-pay | CRITICAL | **FIXED** (watermark + operator-gated flagged backfill; never auto-pay) | §4.1–§4.2, P9b/P13 |
| H1 / DEP-1 customer-cancel | HIGH | **FIXED** (tenant-GUC precondition, webhook precedent) | §3.3.2, P4b |
| M2 raw-writer residual | MED | **FIXED** (structural, trigger+reconciler) | §3.2, P14 |
| M3 mismatch leak | MED | **PARTIAL FIX + DEFER-FLAG** (surfaced-not-auto-obligated; scoped claim; follow-up owner) | §3.6, P16 |
| M4 ratchet evasion | MED | **FIXED + ACCEPT-RISK residual** (literal vector files + import lint; offline-generation residual owned by review) | §2.5, P12 |
| M5 coverage thinning | MED | **FIXED** (demotion moved to Option-B; server matrix added to commit 1) | §2.5.3, P3b |
| M6 half-closed strand | MED | **FIXED** (409 both arms) | §3.5, P7 |
| M7 receipt incoherence | MED | **FIXED** (FE presentation spec + chargedTax) | §2.7, P7b |
| L1 post-B3 settlement RLS | LOW | **DEFER-FLAG** (B3-flip checklist item DEP-2; pattern template shipped) | §5 |
| L2 migration mechanics | LOW | **FIXED** (checklist) | §5 |
| L3 indefinite deferral | LOW | **ACCEPT-RISK + mitigation** (backlog age alert, owner: operator) | §4.3 |
| ESC-1 restitution | STOP | **design revised; decision = needs-human-decision** (record + VAT-trace precondition) | §2.6 |
| ESC-2 wedge→human | STOP | **design revised (fixed)**; flip-gate item | §3.2, P15 |

Open items after this round: **M3-mismatch-disposition** (flagged follow-up), **DEP-2/L1** (B3-flip checklist), **ESC-1 operator decision** (human), ADR (`docs/adr/ADR-audit-fix-money.md`) to be synced to v2 before implementation — flagged for the conductor.

---

## 4. Updated red→green proof matrix (deltas vs v1)

Unchanged: P1, P2, P5, P8 (rescoped post-watermark), P10, P11.
Modified: P3 (+P3b route-level server matrix ≥4 vectors), P6 (sweep unmodified — cancel succeeds AND refund_due appears via trigger), P7 (both 409 arms), P12 (vector-file zero-import lint, red on bad fixture).
New: **P4b** customer crypto-cancel under FORCE-RLS non-bypass role → 200 + refund_due; **P6b** poison-row → sweep still cancels fleet-wide, alert fires; **P7b** inclusive receipt label + total E2E; **P9b** pre-watermark row NOT auto-swept by cron; **P13** operator backfill flow (pre-count → flagged pending → no auto-pay); **P14** raw `UPDATE orders SET status` writer → trigger records refund_due; **P15** reconciler inserts missed obligation ≤1 tick + persistent-failure alert; **P16** mismatch-on-terminal order → operator alert row.

*Not self-certified: conductor re-runs the breaker against proposal v2.*
