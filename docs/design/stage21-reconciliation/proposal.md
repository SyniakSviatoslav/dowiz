# Design Proposal — Stage-21 Cash Reconciliation (B1: courier money model + shift-close)

- **Status:** DRAFT — **Resolution round 1 applied** (see `resolution.md`); design-time, NO production code
- **Date:** 2026-06-29 (rev. post-breaker/counsel)
- **Author:** System Architect (DeliveryOS)
- **Red-line:** 🔴 MONEY · launch-blocker B1
- **Companion ADR:** `docs/adr/ADR-stage21-reconciliation.md` (DRAFT)
- **Extends / bound by:** `ADR-deliver-v2-cash-as-proof.md` (R-8/R-9 carried invariants, the `'hold'`
  primitive, the anti-scoring-creep guardrail), `docs/finance/settlements.md`.

---

## 1. Problem + non-goals

### The defect (B1)

The courier cash model is **semantically inverted** and **structurally incomplete**. Three concrete,
verified facts:

1. **Inverted naming (owner double-loss trap).** `courier_payouts.total_earned`
   (`1780421100043:12`) is computed by `settlement-cron.ts:95-105` as `SUM(courier_assignments.cash_amount)`
   over delivered + `cash_collected` assignments — i.e. **the full COD cash the courier physically
   COLLECTED from customers**, with zero commission/wage/fee deduction (grep confirms there is no earnings
   model anywhere). Yet the column is `total_earned`, the owner API (`owner/settlements.ts:29,57`) returns it
   as `totalEarned`, the i18n label reads **"Payout History"**, and `docs/finance/settlements.md:24` states
   *"**paid**: Owner marks the payout as **transferred to the courier**."* Every surface frames this as
   *money the owner owes the courier*. **It is the opposite: cash the courier already holds and owes back to
   the owner.** An owner who "pays" `total_earned` pays out cash the courier is already sitting on — a
   **double loss** equal to the COD total.

2. **The hold is never released (unbounded phantom debt).** `deliveryCompletion.ts:104-110` appends exactly
   one `courier_cash_ledger` row `type='hold'` (amount = order total) at DELIVERED when `paid_full`. The
   table CHECK allows `('hold','release','settle')` (`1790000000028:16`) but **`'release'`/`'settle'` are
   never written** — reserved (the migration comment says so explicitly). There is **no shift-close that
   clears a hold** when the courier hands the cash to the owner. The ledger therefore only ever grows; a
   courier's apparent "owed" balance never nets to zero even after they have physically returned every lek.

3. **No post-delivery refund reversal exists (latent orphan debt).** The audit cited
   `customer/orders.ts:315-326` as a hold-orphaning path — **corrected on inspection**: that path is the
   *pre-delivery* cancel (`order.status === 'IN_DELIVERY'`, assignment in
   `('assigned','accepted','picked_up')`, line 294/324) and runs *before* any `'hold'` is written (a hold is
   only written at DELIVERED), so it orphans nothing. The real gap is the **absence** of any post-delivery
   refund/cancel path: were one to ship, today it would leave the `'hold'` standing — the courier would show
   as owing cash they have already refunded to the customer. The design must close this *before* a
   post-delivery refund path lands.

### Why this is the launch-blocker

In a 1–5-person cash shop the courier collects COD and the owner is the cash custodian. If the product says
"the owner owes the courier 6 500" when the truth is "the courier owes the owner 6 500", the first real
shift either double-pays the courier or leaves a permanent fake debt on the books. Both are money-integrity
failures on a 🔴 table. No amount of UI polish ships over an inverted ledger.

### Non-goals (explicit)

- **NG-1 — No earnings/wage model.** Courier compensation (commission / per-delivery / hourly) is **out of
  scope** and paid **out of band**. (Decision §4, Option A. The "no earnings model" the audit flagged is the
  *correct* shape for this segment — see the fork.) Adding wages is a separate Council.
- **NG-2 — No auto-deduction, ever** (deliver-v2 R-8 / `NO-AUTO-DEDUCT`). A counting shortfall / robbery /
  short-pay is **owner-reviewed friction**, never a machine deduction from a (often minimum-wage) courier.
- **NG-3 — No courier scoring/penalty engine** (deliver-v2 R-9 / `NO-COURIER-SCORING`). No crumb-derived
  score, no `'deduction'`/`'penalty'`/`'fine'` ledger type. (The anti-scoring-creep guardrail already bans
  exactly those type strings — `guardrail-deliver-v2.mjs:73` — and we keep it.)
- **NG-4 — No new pool / queue / worker.** Reconciliation runs in the owner's request transaction. "Boring &
  proven."
- **NG-5 — Card / non-cash settlement** stays the unbuilt §D seam (deliver-v2 §8). `payment_method` enum is
  `('cash')` only.

---

## 2. Back-of-envelope

### Scale / connection budget

- Topology: **5 locations × ~3 couriers × ~30 COD deliveries/courier/day ≈ 450 orders/day** (early-stage
  target; 10× headroom = 4 500/day still trivial).
- Ledger row growth: **1 `'hold'` per delivered+cash order** + **1 contra** (`'settle'` at shift-close, or
  `'release'` on the rare refund). Worst case ≈ **2 rows / order ≈ 900 rows/day ≈ 330 k rows/year**. A
  two-column-indexed integer table; Postgres does not notice this.
- Reconciliation frequency: **owner tap, ~1–3 / courier / day** (shift close / cash drop). Synchronous on the
  **existing API pool** inside one request tx. **No new pool, no pg-boss queue, no worker** (settlement-cron
  already exists; we add no liveness surface). Connection budget delta = **0**.
- Refund-reversal: rides **inside the refund tx** that already exists (when that path ships). Delta = 0.

### Worked example (the required money proof)

> **RESOLVED:** the single-ordering example below is superseded by the **three-ordering proof** in
> `resolution.md` (CRITICAL-2): refund-before-settle, refund-**after**-settle, and **partial** refund — each
> proven net-zero with no phantom credit and the owner-refund recorded. The table below is the
> refund-before-settle case only.

Courier C, one shift, three COD orders **A=3000, B=1500, C=2000** (integer minor units). Refund B
post-delivery, then hand the remaining cash to the owner at shift close.

| Step | Event | Ledger row appended | `courier_cash_ledger` net for C | Courier owes owner |
|------|-------|---------------------|---------------------------------|--------------------|
| 1 | Deliver A, `paid_full` | `hold A 3000` | +3000 | **3000** |
| 2 | Deliver B, `paid_full` | `hold B 1500` | +4500 | **4500** |
| 3 | Deliver C, `paid_full` | `hold C 2000` | +6500 | **6500** |
| 4 | **Refund B** (post-delivery) | `release B 1500` (contra) **+** assignment-B cash reversal (same tx) | 6500 − 1500 = **5000** | **5000** (cash physically returned to customer; courier no longer holds it) |
| 5 | **Shift close**, owner confirms cash received | `settle A 3000`, `settle C 2000` (contra, one per outstanding held order) | 5000 − 5000 = **0** | **0** ✅ |

**Net invariant:** `SUM(hold) − SUM(release) − SUM(settle) = 0` per courier once reconciled. The owner is
**never charged** anything — the owner *receives* the 5000 of collected cash and the ledger records the
hand-over. No money flows owner→courier in this model.

**Parallel settlement-cron figure stays consistent:** `total_earned` for the period =
`SUM(cash_amount)` over delivered + `cash_collected` = A 3000 + C 2000 = **5000** (B excluded because the
refund set `cash_collected=false`). After the naming fix this field is surfaced as **"Cash collected /
to reconcile: 5000"**, not "owed to courier 5000". The two ledgers agree: *5000 collected = 5000 reconciled =
0 still owed.*

---

## 3. Options (≥2, with the concept each applies)

### Fork A — the semantic correction (naming / earnings)

**Concept:** honest naming; *the name is the defect* on a 🔴 money table.

> **RESOLVED (round 1): A1 surface-only chosen.** The Breaker proved A2's physical `RENAME COLUMN` breaks the
> `prevent_payout_mutation` trigger, the cron writer (`settlement-cron.ts:103`), the backup smoke check, and
> every reader. The honesty win lives at the **surface** the owner/courier reads, not the physical column. We
> keep `total_earned` physical (+ a truth-telling column `COMMENT`) and rename only the DTO field →
> `collectedTotal` + i18n + UI label. Passes DoD-5 at near-zero blast radius. RK-4 (physical-rename risk) is
> **withdrawn**. Reader list enumerated in `resolution.md` (CRITICAL-1).

| Option | What | Tradeoff | Reversibility |
|--------|------|----------|---------------|
| **A1 — surface-only rename (CHOSEN)** | Keep physical `total_earned` + truth `COMMENT`; rename DTO field → `collectedTotal`, i18n + `EarningsPage` label ("Cash collected / to reconcile"). | Cheapest, **zero migration on the 🔴 column**, no trigger/cron/smoke break. The column-name objection is bought off by the `COMMENT`. | Trivial. |
| **A2 — physical `RENAME COLUMN` (REJECTED)** | `total_earned → collected_total`. | Breaks `prevent_payout_mutation` on approved/paid UPDATEs, breaks `settlement-cron.ts:103` immediately, breaks `checkPayoutSums`. Large blast radius on a 🔴 table for a surface win. | Rejected. |
| **A3 — add an earnings model** | New `commission`/`pay_rate` columns; system computes courier *net pay* = wages − cash-owed and a real owner→courier settlement. | Introduces genuine owner→courier money logic + invites the scoring/penalty framing R-9 forbids. **Over-engineering** for a segment where the courier is often the owner or paid off-platform. Violates NG-1 / ponytail. | Heavy; hard to walk back. |

### Fork B — what triggers the hold release (reconciliation authority)

**Concept:** human-authority (deliver-v2 red line) + owner is the cash custodian.

| Option | Trigger | Tradeoff |
|--------|---------|----------|
| **R1 — owner-confirmed cash drop** | Owner taps "cash received from courier C" → server computes outstanding holds, writes `settle` contra rows, nets to 0. | Server-authoritative integer; matches "owner authoritative for money"; single tap; no courier UI needed for MVP. Owner could mis-count → shortfall (handled as friction, NG-2). |
| **R2 — courier-declared + owner-confirm (two-phase)** | Courier declares the drop, owner confirms. | More accountable, but needs courier UI now and a pending state. Defer; the ledger shape supports adding it later. |
| **R3 — auto-release on settlement-cron (time-based)** | Cron clears holds after the period. | **REJECT.** Auto-clears a real debt with no human confirming cash physically moved — exactly the auto-verdict deliver-v2 forbids. Violates human-authority red line. |

---

## 4. Decision + rationale (→ ADR-stage21-reconciliation)

- **A1 surface-only rename** (DTO+i18n+UI; physical `total_earned` kept with a truth `COMMENT`) +
  **R1 (owner-confirmed cash drop)**, with **partial reconcile** (HIGH-5) so a miscount strands only the delta.
- **NG-1 holds — till-accountability only, no earnings model.** The courier collects the owner's cash and
  owes it back; wages are out-of-band. This is the deliver-v2 *till-accountability* primitive
  (`ADR-deliver-v2-cash-as-proof.md:37-40`), finished — not a new money concept.

**Rationale (truth-of-engineering):**

- The inversion is a **naming lie on a 🔴 table**. A physical `RENAME COLUMN` (A2) would tell the truth at the
  schema layer but breaks the `prevent_payout_mutation` trigger, the cron writer, and the smoke check
  (Breaker CRITICAL-1) — too much blast radius for a surface win. **Resolution:** keep the physical column,
  buy off the "schema name stays a lie" objection with a truth-telling column `COMMENT`, and rename every
  **surface** the owner/courier actually reads. DoD-5 (no surface frames it as owed-to-courier) is satisfied
  by the surface change; the schema-name residual is documented, not lived.
- A3 would build real owner→courier money flow and is the precise over-engineering the segment doesn't need;
  it also reopens the R-9 scoring framing. Rejected by YAGNI + the carried invariants.
- R1 keeps the owner as the single money authority (server-authoritative integer), needs no courier UI for
  MVP, and the contra-row shape leaves R2 a pure additive upgrade. R3 is rejected on the human-authority red
  line.

**The release mechanism = append-only contra rows, netting enforced STRUCTURALLY (RESOLVED — CRITICAL-2):**

- `'hold'` — cash collected, courier owes (existing; unchanged). The cash ledger models **courier obligation
  only**.
- `'release'` — contra for a **refund/cancel** of a held order; `amount = the ACTUAL refunded amount`
  (partial-aware), never a fixed full-hold.
- `'settle'` — contra for an owner-confirmed **cash-drop reconciliation**; may be a **partial** settle (HIGH-5).
- **Net can never go negative — DB-enforced, not read-time.** A `BEFORE INSERT` **residual-guard trigger**
  raises if a contra's `amount > hold − Σ(existing release+settle for the order)`. The contra path first
  `SELECT … FOR UPDATE` the order's `hold` row so concurrent settle∥refund **serialize** (kills the TOCTOU
  race). This **replaces** the false "`NOT EXISTS` + `UNIQUE(order_id,type)` make them mutually exclusive"
  claim — `UNIQUE(order_id,type)` never prevented a `release` and a `settle` coexisting.
- **Refund-after-settle is correct by the obligation model:** if the order is already fully settled (residual
  0) the courier owes nothing; the refund writes **no courier-ledger row** (a `release` would exceed residual
  → trigger raises) and instead records an **owner-refund fact** (owner paid the customer from their till) —
  no phantom credit, owner loss recorded not silent. Partial-unique / EXCLUDE was **rejected** because a
  partial refund legitimately leaves an order with both a partial `release` and a `settle` of the remainder
  (worked example 3 in `resolution.md`).

**Gate fit (verified, load-bearing):** the anti-scoring-creep guardrail bans only *penalty-flavored* ledger
types — `guardrail-deliver-v2.mjs:73` matches `'(deduction|penalty|fine|score|chargeback|adjustment|debit)'`.
It does **not** match `'release'`/`'settle'`. So the netting contra types are *already the sanctioned
mechanism* and ship **without weakening the gate**. We keep the penalty ban exactly as-is (NG-3).

---

## 5. Data / migrations (forward-only, atomic, RLS FORCE, integer)

All additive / forward-only; pre-launch so revertible by inverse migration until launch. No `order_status`
enum churn.

1. **Naming truth (A1 surface-only — RESOLVED).** **No `RENAME COLUMN`.** Keep physical `total_earned`; add
   `COMMENT ON COLUMN courier_payouts.total_earned IS 'integer minor units — COD cash the courier COLLECTED
   and owes the owner; surfaced as collectedTotal; NOT money owed to the courier';`. The rename is a
   **DTO+i18n+UI surface change** (`owner/settlements.ts:57,146` `totalEarned→collectedTotal`;
   `courier/settlements.ts` map row key; `shared-types/.../owner/settlements.ts`; `i18n-catalog.ts`;
   `EarningsPage.tsx`). Trigger, cron writer, and `checkPayoutSums` are **untouched**.
2. **`courier_cash_ledger` harden (append-only made real — RESOLVED MED cascade).** Table already has
   `ENABLE + FORCE RLS` (`1790000000028:24-25`), `UNIQUE(order_id,type)`, `amount integer CHECK(>=0)`. Add a
   `prevent_ledger_mutation()` **`BEFORE UPDATE` trigger only** → `RAISE` (true content-immutability). **No
   `DELETE` clause** — the only DELETE is the `ON DELETE CASCADE` from `orders` (GDPR hard-erase), which must
   succeed and *should* remove the order's ledger crumb. Contra rows are INSERTs — unaffected.
3. **Residual-guard trigger (RESOLVED CRITICAL-2).** `BEFORE INSERT` on `courier_cash_ledger` for
   `type IN ('release','settle')`: `RAISE` if `NEW.amount > hold − Σ(prior release+settle for the order)`.
   Net-negative becomes structurally impossible. Pair with `SELECT … FOR UPDATE` on the hold row in the contra
   path for race-safety.
4. **`courier_payouts` RLS — DEPENDENCY on B3, GUC-ready now (RESOLVED HIGH-3).** `courier_payouts` is
   **already `FORCE`** (`1780421100051:11`) — the earlier "ENABLE-only, add FORCE" item was stale and is
   **struck**. FORCE is inert against the live **BYPASSRLS** writer; the real closure is the **B3
   NOBYPASSRLS** work (recorded as a dependency). Now: set `set_config('app.current_tenant', <locationId>,
   true)` in `settlement-cron.ts` and `owner/settlements.ts` (as `courier/settlements.ts` already does) and
   make the policy missing-GUC-tolerant (`current_setting('app.current_tenant', true)`) so they don't self-DoS
   when B3 lands.
5. **Reconciliation audit (who/when/how-much).** New small append-only table
   `courier_cash_reconciliations (id, courier_id, location_id, owner_id, confirmed_total integer
   CHECK(>=0), order_count int, created_at, status text CHECK(status IN ('reconciled','discrepancy')))` with
   `ENABLE + FORCE RLS`, tenant policy, grant-mirror. `settle` rows reference its id (additive nullable
   `reconciliation_id` column on the ledger). One reconciliation = one row = one tap = one audit fact. The
   discrepancy row records only the **delta** (HIGH-5 partial reconcile), never the whole shift.
6. **No backfill, no physical rename, no enum add.** All changes additive; the naming fix is surface-only (§5.1).

---

## 6. Consistency + idempotency

- **Hold (unchanged):** `INSERT … ON CONFLICT (order_id, 'hold') DO NOTHING`.
- **Settle (reconciliation — partial-reconcile, RESOLVED HIGH-5):** in one tx — compute each order's
  **residual** (`hold − Σ prior contra`); settle orders up to `confirmed_total` (oldest-first), the last
  partially-covered order getting a **partial** `settle` of the remaining confirmed amount; every contra is
  validated by the residual-guard trigger. Re-running is idempotent (each `settle` is `ON CONFLICT
  (order_id,'settle') DO NOTHING`, and a re-confirm sees zero residual). If `confirmed_total <
  SUM(residuals)` the **delta** is recorded `status='discrepancy'` — the courier's standing obligation is
  bounded to that delta, **never** the whole shift; **no auto-deduct** (NG-2). Server computes; owner confirms
  the figure, never sets it.
- **Release (refund reversal — obligation-aware, RESOLVED CRITICAL-2/HIGH-4):** **same tx as the refund.** If
  the order's residual > 0: `INSERT 'release'` with `amount = min(refundAmount, residual)` + decrement the
  assignment cash by that amount (`SET LOCAL app.settlement_reversal='true'`), and — **while the payout is
  `pending`** — contra the `settlement_item` + decrement `total_earned` by the same amount (keeps the snapshot
  coherent in-tx). If residual == 0 (already settled) or the payout is `approved`/`paid` (immutable): write
  **no courier-ledger row**; record an **owner-refund fact** (owner paid the customer) + flag the payout for
  owner review — never leave a stale figure unmarked. Atomic; no cross-service call.
- **Two-ledger coherence guardrail (term fixed, RESOLVED MED RK-5):** a test asserts, per courier and period,
  `SUM(hold) − SUM(release) − SUM(settle)` (cash ledger) equals
  `SUM(settlement_items.amount) − SUM(owner-refund corrections)`. The old "un-`settle`d delivered assignments"
  term was ill-defined — `courier_assignments` has **no** `settle` concept (`settle` is the cash ledger's).
  The structural half (over-reversal / net-negative) is now enforced by the residual-guard trigger at write
  time, not by this test.

---

## 7. Failures + degradation (every path: behavior on failure, zero cascade)

- **Reconciliation tx fails mid-way** → `ROLLBACK`; no partial `settle` rows; owner retries; idempotent
  re-run is safe. Degraded state = holds simply remain outstanding (the honest pre-reconciliation truth),
  never a half-netted ledger.
- **Refund commits but reversal fails** → impossible by construction: the `release` contra (or owner-refund
  fact) + assignment/snapshot reversal are **inside the refund tx**; if any fails the refund itself rolls
  back. No orphan debt, no phantom credit (residual-guard trigger caps the contra).
- **Refund AFTER the cash drop (residual 0)** → no courier-ledger row written (the trigger would raise);
  records an **owner-refund fact** so the owner's money-out is captured, courier net stays 0. No silent owner
  loss (RESOLVED Breaker scenario B).
- **Owner counts less cash than computed (shortfall / robbery / miscount)** → per **NG-2 (NO-AUTO-DEDUCT)**:
  **partial-reconcile up to the confirmed amount** (HIGH-5) — settle what was confirmed, record only the
  **delta** as `status='discrepancy'` for **owner review**; do **not** touch the courier, do **not** score. A
  1-unit miscount strands 1 unit, never the whole shift. The deeper discrepancy-**resolution** workflow (who
  absorbs the delta) is **DEFER-FLAG to its own Council** (R-8); the **clearing** is not deferred. No machine
  ever moves money against a courier.
- **No external calls anywhere** in this surface (all Postgres) → no timeout/circuit-breaker/fallback needed;
  the only "failure mode" is tx rollback, which is safe and idempotent-retryable.
- **settlement-cron unchanged** and independent; a reconciliation failure cannot cascade into payout
  generation (different tx, different trigger).

---

## 8. Security + tenant isolation

- Integer minor units end-to-end; `CHECK(amount >= 0)`; Zod `.strict()` `.int().nonnegative()` at the edge.
- `courier_cash_ledger` + new `courier_cash_reconciliations` = `ENABLE + FORCE RLS`, tenant-scoped, grant-
  mirror (the `read_public_menu` canon). **`courier_payouts` is already `FORCE`** (`1780421100051:11`) — the
  real money-RLS closure is the **B3 NOBYPASSRLS writer** (dependency); FORCE here is honestly-labeled
  defense-in-depth, **not** the fix (RESOLVED HIGH-3). The cron + `owner/settlements` are made
  **NOBYPASSRLS-ready now** by setting `app.current_tenant` so they don't self-DoS when B3 lands.
- **Writer = owner** (`requireRole(['owner'])` + `requireLocationAccess`), server-authoritative; the owner is
  the cash custodian, so the existing member-location ledger policy
  (`location_id IN (SELECT app_member_location_ids())`) suffices for the **write** path.
- **Courier-facing cash-ledger view = DEFER-FLAG, bound to LATENT-STOP-2.** Couriers already read their
  generated **payout report** (`courier/settlements.ts`, via `app.current_tenant`); only the
  `courier_cash_ledger` own-read is deferred (its policy needs the deliver-v2 **two-context** form —
  `current_tenant` for the courier OR `app_member_location_ids()` for the owner; couriers live in
  `courier_locations`, so a member-only policy denies them under FORCE). The deferral is gated: the moment any
  courier-visible debt/obligation view ships, the courier-visible own-ledger read **must** ship with it or
  Counsel's pre-registered **LATENT-STOP-2** fires.
- No PII in `MessageBus` (claim-check): a reconciliation event carries `{reconciliationId, courierId,
  locationId}` only — `settlement_audit_log` already follows this (`settlements.md:27-29`).
- No cookies, RS256 JWT, parameterized SQL, `crypto.randomUUID()` — inherited posture, unchanged.

---

## 9. Operability

- **Health:** request-path only; **no worker liveness** to monitor (degraded-vs-down is the existing
  settlement-cron's concern, untouched).
- **Observability (<1 min):** every reconciliation appends a `settlement_audit_log` row
  (`action='reconciled'`, actor='owner', metadata = computed-total / confirmed-total / order-count) + the
  `courier_cash_reconciliations` row — fully auditable from existing tables.
- **Rollback / scaling-gate:** **`COURIER_CASH_RECONCILIATION_ENABLED` (default OFF)** gates the runtime;
  schema + lib land **inert** ("schema rich, runtime minimal"). The **surface-only** naming rename (§5.1) +
  the GUC-readiness writes (§5.4) ship unflagged (no behavior change to existing flows). Flip the runtime only
  when the owner shift-close UI ships.
- **Forward-only migrations**, revertible pre-launch by inverse migration.

---

## 10. Open / accepted risks (owner)

| # | Risk | Disposition | Owner |
|---|------|-------------|-------|
| RK-1 | **Discrepancy *resolution*** (who absorbs the delta after a partial reconcile). | **DEFER-FLAG** → own Council (R-8). NARROWED: the **clearing** is now in-scope (HIGH-5 partial reconcile bounds debt to the delta); only the resolution workflow defers. Never auto-deduct. | Human / Council |
| RK-2 | **Earnings/wage model** deliberately absent (NG-1) + **delivery-fee** in the hold (Counsel §5). | **NEEDS-HUMAN / STOP-ETHICS** — two questions pre-staged in `resolution.md`; the delivery-fee fact is verified (hold bundles `delivery_fee`, treated 100% owner revenue, no courier-pay portion). Not closed until answered. | Human |
| RK-3 | **Courier-facing cash-ledger view** needs two-context RLS. | **DEFER-FLAG**, bound to **LATENT-STOP-2** (no courier-visible debt ships without a courier-visible own-ledger). Couriers already read their generated payout report. | Owner |
| RK-4 | ~~A2 physical rename on a 🔴 table~~. | **WITHDRAWN** — A1 surface-only rename chosen; there is no physical rename (RESOLVED CRITICAL-1). | — |
| RK-5 | **Post-delivery refund path does not yet exist;** the `release` primitive is built ahead of its caller. | **ACCEPT** (schema-rich/runtime-minimal). The residual-guard trigger now makes over-reversal **structurally** impossible at write time; the owner-refund-recording obligation remains a contract until the refund caller ships with tests. | Owner |
| RK-6 | **Multi-currency** — `collectedTotal`/cron sums have no currency dimension (`currency:'ALL'` hard-coded, no `GROUP BY`). | **ACCEPT-RISK** under the single-currency invariant; flag a `GROUP BY currency_code` requirement before any multi-currency location (RESOLVED LOW). | Owner |

---

## DoD — red → green (the money proof, hardened post-resolution)

1. **Worked example nets zero in ALL orderings.** Tests for refund-before-settle, refund-**after**-settle
   (no courier-ledger row; owner-refund recorded; net 0), and **partial** refund (release = refunded amount;
   order carries both a partial release and a settle; net 0). Each asserts `SUM(hold) − SUM(release) −
   SUM(settle) = 0` per courier and no negative net.
2. **`courier_payouts` UPDATE SURVIVES the change.** A test runs approve→pay→dispute→reopen and the cron
   `total_earned += …` UPDATE; all succeed (surface-only rename did not touch the column/trigger/cron).
3. **release+settle cannot coexist past the hold (constraint RED).** A test inserting a `release` + `settle`
   that sum past the hold (or a refund-after-full-settle `release`) **raises** via the residual-guard trigger.
4. **Refund-after-settle nets zero.** Settle first, then refund → asserts **no** new courier-ledger row, an
   owner-refund fact recorded, courier net 0 (no phantom credit, no silent owner loss).
5. **Partial refund reverses only the refunded amount.** Refund 1000 of a 3000 hold → `release` = 1000, net =
   2000 (not −2000).
6. **Cron + owner-settlements don't self-DoS under a set GUC.** With `app.current_tenant` set and a
   NOBYPASSRLS-simulated role, the list/detail/approve handlers and the cron return/write rows (no swallowed
   empty, no 500).
7. **Cascade delete still works.** Deleting an `orders` row with ledger rows **succeeds** (UPDATE-only
   immutability trigger); a direct `UPDATE` on a ledger row **raises**.
8. **No auto-deduct / no scoring.** `apps/api/tests/stage21-no-auto-deduct.invariant.test.ts` GREEN;
   `guardrail-deliver-v2.mjs` still bans penalty-typed ledger writes (unchanged).
9. **Miscount strands only the delta.** confirmed 6499 vs computed 6500 → settles 6499, leaves a 1-unit
   discrepancy; courier obligation = 1, not 6500.
10. **The label is honest (DoD-5, load-bearing).** No surface (DTO / i18n / `EarningsPage`) frames
    `collectedTotal` as "owed to / paid to courier"; an assertion confirms "cash collected / to reconcile".
11. **Two-ledger coherence** holds (§6 guardrail, term-corrected).
