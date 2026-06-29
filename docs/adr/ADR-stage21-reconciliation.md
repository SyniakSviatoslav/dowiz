# ADR — Stage-21 Courier Cash Reconciliation (B1: money model + shift-close)

- **Status:** DRAFT — **Resolution round 1 applied** (`docs/design/stage21-reconciliation/resolution.md`);
  design-time, NO production code — awaiting re-attack + human sign-off
- **Date:** 2026-06-29 (rev. post-breaker/counsel)
- **Red-line:** 🔴 MONEY · launch-blocker B1
- **Proposal:** `docs/design/stage21-reconciliation/proposal.md`
- **Extends / bound by:** `ADR-deliver-v2-cash-as-proof.md` (the `'hold'` primitive; carried invariants
  R-8/R-9; the anti-scoring-creep guardrail), `docs/finance/settlements.md`.
- **Materializes:** the deliver-v2 *"NEEDS-HUMAN before launch"* item — the merged R-8+R-9 Stage-21
  invariant. Authoring this file with the two markers below turns the red-on-disk guardrail
  `apps/api/tests/stage21-no-auto-deduct.invariant.test.ts` GREEN.

## Context

Verified ground truth (file:line):

- `courier_payouts.total_earned` (`packages/db/migrations/1780421100043:12`) is computed by
  `settlement-cron.ts:95-105` as `SUM(courier_assignments.cash_amount)` over delivered + `cash_collected`
  assignments — **the COD cash the courier COLLECTED**, no commission/wage deduction. There is **no earnings
  model** anywhere in the codebase.
- Every surface (`owner/settlements.ts:29,57` → `totalEarned`; i18n "Payout History";
  `docs/finance/settlements.md:24` "paid: Owner marks the payout as transferred to the courier") frames this
  as **money the owner owes the courier**. It is the **opposite** — cash the courier holds and **owes the
  owner**. Paying it out = owner double-loss.
- `deliveryCompletion.ts:104-110` appends one `courier_cash_ledger` `'hold'` row at DELIVERED when
  `paid_full`. The CHECK allows `('hold','release','settle')` (`1790000000028:16`) but `'release'`/`'settle'`
  are **never written** → a hold is permanent; **no shift-close clears it**.
- No post-delivery refund path exists; the pre-delivery cancel (`customer/orders.ts:307-326`) runs before any
  hold is written, so the latent orphan-debt risk is the *absence* of a reversal, not a buggy one.

## Decision

1. **Till-accountability only; NO earnings model.** The courier collects the owner's cash and owes it back;
   wages are paid out of band and are **not modeled here** (segment-correct for a 1–5-person cash shop). This
   finishes the deliver-v2 *till-accountability* primitive; it introduces no new money concept.

2. **Honest naming — SURFACE-ONLY (RESOLVED CRITICAL-1).** **No `RENAME COLUMN`.** A physical rename breaks
   `prevent_payout_mutation` (references `OLD/NEW.total_earned`), the cron writer (`settlement-cron.ts:103`),
   and `checkPayoutSums`. Keep `total_earned` physical (+ a truth-telling `COMMENT`) and rename only the read
   surface → `collectedTotal` (DTO `owner/settlements.ts:57,146` + `courier/settlements.ts`; shared-types;
   i18n; `EarningsPage.tsx`) so no surface reads "owed to / paid to courier". Passes DoD-5 at near-zero blast
   radius. (Full reader list in `resolution.md`.)

3. **Release the hold via append-only contra rows; netting enforced STRUCTURALLY (RESOLVED CRITICAL-2).**
   - `'release'` — contra for a **refund/cancel**; `amount = ACTUAL refunded amount` (partial-aware).
   - `'settle'` — contra for an **owner-confirmed cash drop**; may be **partial**.
   - The cash ledger models **courier obligation only**. A `BEFORE INSERT` **residual-guard trigger** raises
     if a contra's `amount > hold − Σ(prior release+settle)`, making net-negative / over-reversal /
     double-contra **structurally impossible** at write time; the contra path `SELECT … FOR UPDATE`s the hold
     row so concurrent settle∥refund **serialize** (kills the TOCTOU race). This **replaces** the false
     "`NOT EXISTS` + `UNIQUE(order_id,type)` mutual-exclusion" claim. The two types are **not** the penalty
     types the anti-scoring-creep guardrail bans (`guardrail-deliver-v2.mjs:73`) — ship without weakening it.

4. **Reconciliation authority = owner-confirmed cash drop, PARTIAL-reconcile (RESOLVED HIGH-5).** Owner taps
   "cash received from courier C"; the server settles holds **up to `confirmed_total`** (oldest-first, last
   order partial) in one idempotent tx. A shortfall records only the **delta** as `status='discrepancy'` —
   the courier's standing obligation is bounded to the delta, **never** the whole shift; no auto-deduct
   (NG-2). Server computes; owner confirms the figure, never sets it. No time-based auto-release.

5. **Refund reversal is same-tx, obligation-aware (RESOLVED CRITICAL-2/HIGH-4).** If residual > 0: append a
   `'release'` (amount = refunded, ≤ residual) + decrement the assignment cash, and **while the payout is
   `pending`** also contra the `settlement_item` + decrement `total_earned` (snapshot stays coherent in-tx).
   If residual == 0 (already settled) or the payout is `approved`/`paid` (immutable): write **no** courier
   row; record an **owner-refund fact** + flag for owner review. Net 0, no phantom credit, owner loss
   recorded.

6. **Harden append-only; money-RLS = B3 dependency, GUC-ready now (RESOLVED MED-cascade/HIGH-3).**
   - `prevent_ledger_mutation` is **`BEFORE UPDATE` only** (content-immutability); **no `DELETE` clause** so
     the `orders ON DELETE CASCADE` (GDPR hard-erase) still works.
   - `courier_payouts` is **already `FORCE`** (`1780421100051:11`) — the stale "ENABLE-only, add FORCE" claim
     is **struck**. FORCE is inert against the live **BYPASSRLS** writer; the real closure is the **B3
     NOBYPASSRLS** work (**DEPENDENCY: B3**). Now: set `app.current_tenant` in `settlement-cron.ts` +
     `owner/settlements.ts` and make the policy missing-GUC-tolerant so they don't self-DoS when B3 lands.
     FORCE here = honestly-labeled defense-in-depth, not the headline fix.

7. **Flag-gated runtime.** `COURIER_CASH_RECONCILIATION_ENABLED` (default OFF) gates the shift-close runtime;
   schema + lib land inert. The naming rename ships unflagged (pure honesty fix).

## Carried invariants (the council's binding conditions — materialized markers)

> **NO-AUTO-DEDUCT** — Stage-21 reconciliation NEVER auto-deducts a no-fault shortfall (robbery / short-pay /
> miscount) from a courier. A shortfall is recorded as a fact (`courier_cash_reconciliations.status =
> 'discrepancy'`, computed-vs-confirmed delta) for **owner review** — owner-reviewed friction, never a
> machine deduction. The discrepancy-resolution layer does not land without its own Triadic Council.

> **NO-COURIER-SCORING** — No crumb-derived courier score or penalty. No `'deduction'`/`'penalty'`/`'fine'`/
> `'score'` ledger type; no penalty derived from `delivery_trace` / `order_sensor_events` /
> `customer_signals`. The anti-scoring-creep guardrail (`guardrail-deliver-v2.mjs:73`) stays in force,
> unweakened. Any scoring/penalty engine requires its own Triadic Council.

## Consequences

**Positive:** the ledger nets to zero on reconciliation; the inversion is removed at every layer; the owner
is never double-charged; additive/forward-only; reuses the existing `'hold'` primitive and the CHECK-reserved
contra types; zero new pools/queues/workers; the anti-scoring-creep gate is preserved, not bent.

**Negative / accepted:** earnings/wage model deliberately absent (NG-1; fairness + delivery-fee =
NEEDS-HUMAN, RK-2); discrepancy *resolution* deferred (RK-1 — clearing is now in-scope); courier cash-ledger
view deferred, bound to LATENT-STOP-2 (RK-3); the `release` primitive is built ahead of any refund caller
(RK-5 — over-reversal is now structurally blocked, but the owner-refund-recording obligation is a contract
until the caller ships); multi-currency unhandled under the single-currency invariant (RK-6); money-RLS
closure depends on **B3** (external).

**NEEDS-HUMAN before launch (STOP-ETHICS — `resolution.md` §RK-2):**
1. Is the launch courier ever a **non-owner hired worker**, or only owner/family? (discharges RK-2)
2. The hold/`collectedTotal` **bundles `delivery_fee`** (verified `orders.ts:499`), treated 100% as owner
   revenue with no courier-pay portion in code — if the business intends the courier to keep the fee, the
   ledger records them owing their own pay back, settled out-of-band by no mechanism. Honest, or the
   asymmetry at its sharpest?
Plus: approve flipping `COURIER_CASH_RECONCILIATION_ENABLED` only when the owner shift-close UI ships.

## Migration (forward-only, additive, integer) — design-time, not built here; **surface-only rename, no physical RENAME**

1. **No `RENAME COLUMN`.** `COMMENT ON COLUMN courier_payouts.total_earned IS '… surfaced as collectedTotal;
   NOT owed to the courier'`. The rename is a DTO+i18n+UI surface change (see proposal §5.1 reader list).
2. `prevent_ledger_mutation()` **BEFORE UPDATE only** on `courier_cash_ledger` → RAISE (content-immutability;
   no DELETE clause so the `orders` cascade survives). Contra rows are INSERTs, unaffected.
3. **Residual-guard** `BEFORE INSERT` trigger on `courier_cash_ledger` for `release`/`settle`: RAISE if
   `amount > hold − Σ(prior contra)`. Net-negative structurally impossible.
4. **No** `courier_payouts FORCE` migration (already FORCE, `1780421100051:11`). Instead: set
   `app.current_tenant` in the cron + `owner/settlements` and make the policy missing-GUC-tolerant
   (NOBYPASSRLS-ready for B3).
5. New `courier_cash_reconciliations (id, courier_id, location_id, owner_id, confirmed_total integer
   CHECK(>=0), order_count int, status text CHECK(status IN ('reconciled','discrepancy')), created_at)` —
   `ENABLE + FORCE RLS`, tenant policy, grant-mirror; nullable `reconciliation_id` FK on `courier_cash_ledger`
   for `settle` rows.
6. No `order_status` enum churn; no backfill; no physical rename.

## DoD (red → green) — hardened post-resolution

`courier_payouts` UPDATE (approve/pay/dispute/reopen + cron) **survives** · worked example nets zero in **all
three orderings** (before/after-settle, partial) · release+settle past the hold + refund-after-full-settle
**RAISE** (residual-guard) · partial refund reverses only the refunded amount · cron + owner-settlements
**don't self-DoS** under a set GUC · **cascade delete still works**, direct ledger UPDATE raises ·
miscount strands only the **delta** · `stage21-no-auto-deduct.invariant.test.ts` GREEN · penalty-typed writes
still banned · no surface frames `collectedTotal` as owed-to-courier · two-ledger coherence holds. Full
checklist: `docs/design/stage21-reconciliation/proposal.md` §DoD.
