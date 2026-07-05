# Resolution — Stage-21 Cash Reconciliation (B1)

Dispositions for every Breaker finding and every Counsel item against
`docs/design/stage21-reconciliation/proposal.md`. Design-time only — **NO production code**. Each row was
re-verified against the live tree before disposition; corrections to my own proposal are called out.

Legend: **FIX** (change the design now) · **ACCEPT-RISK(owner)** · **DEFER-FLAG** · **NEEDS-HUMAN** (Counsel /
STOP-ETHICS — not resolved unilaterally).

---

## Resolution round 1

### Source re-verification (what I confirmed before disposing)

| Claim | Live source | Verdict |
|-------|-------------|---------|
| Rename breaks the payout trigger | `prevent_payout_mutation()` body references `OLD/NEW.total_earned` **inside** `IF OLD.status IN ('approved','paid')` (`1780421100052:8-10`) — PL/pgSQL lazy-evals, so it throws on UPDATEs to **already-approved/paid** rows (dispute/reopen of an approved payout), not literally every UPDATE. | Breaker **substantively right**; magnitude refined. |
| Rename breaks the cron | `settlement-cron.ts:103-104` raw SQL `total_earned = total_earned + $2` — breaks **immediately**, unconditionally. | **Confirmed.** |
| Rename breaks the backup smoke check | `backup/smoke-checks.ts:148-152` `cp.total_earned`. | **Confirmed.** |
| `release`+`settle` mutual exclusion is structural | Migration `028` has only `CHECK(type IN …)` + `UNIQUE(order_id,type)`; mutual exclusion lived only in a read-time `NOT EXISTS` under READ COMMITTED. | Breaker **right**; my §4 claim was false. |
| `courier_payouts` is ENABLE-only / FORCE is the fix | **It is already `FORCE`** — `1780421100051:11` (`ALTER TABLE courier_payouts FORCE ROW LEVEL SECURITY`). My §5.3/ADR-mig-2 premise is **doubly wrong**: already FORCE, and FORCE is inert against a BYPASSRLS writer. | Breaker **right**, and my proposal was factually stale. |
| Hold amount bundles the delivery fee | `orders.ts:499`: `const total = subtotal + deliveryFee + taxTotal - discountTotal;` → `o.total` → `completeDelivery(... total ...)` → hold amount. | **Confirmed** (Counsel §5 answer below). |
| Ledger FK cascades from orders | `028:15` `order_id … REFERENCES orders(id) ON DELETE CASCADE`. | **Confirmed.** |
| Couriers can't read the cash ledger | `028:27` policy is member-only `app_member_location_ids()`; couriers are in `courier_locations`. But couriers **can** read their generated `courier_payouts` via `courier/settlements.ts:25,59` (sets `app.current_tenant` GUC). | **Confirmed** with nuance (a courier-readable *payout* path already exists; the *cash_ledger* is the deferred piece). |

---

### CRITICAL-1 — rename breaks `prevent_payout_mutation` + cron + smoke + readers → **FIX (surface-only rename; lower blast radius)**

The honesty win Counsel cares about lives at the **surface** (the field the owner/courier reads), not the
physical column. Adopt the conductor's lower-blast path:

- **Keep the physical column `courier_payouts.total_earned`.** No `RENAME COLUMN`, no migration on the 🔴
  column. The trigger, the cron, the smoke check, the immutability semantics — all **untouched**.
- **Rename only the read surface** to `collectedTotal` (+ honest i18n / UI label) so no surface frames it as
  owed-to-courier. This passes **DoD-5** at near-zero blast radius.
- The physical column keeps a `COMMENT` documenting the truth (`COMMENT ON COLUMN … IS 'integer minor units —
  COD cash the courier COLLECTED and owes the owner; surfaced as collectedTotal; NOT money owed to the
  courier'`) — schema honesty without a rename.

**Full reader list of physical `total_earned` (enumerated — all KEPT as-is under surface-only):**

1. `packages/db/migrations/1780421100043:12` — column definition + `CHECK(total_earned >= 0)`.
2. `packages/db/migrations/1780421100052:9-10` — `prevent_payout_mutation()` trigger body.
3. `apps/api/src/workers/settlement-cron.ts:103-104` — writer (`UPDATE … total_earned = total_earned + $2`).
4. `apps/api/src/routes/owner/settlements.ts:29` (SELECT), `:57` (DTO `totalEarned` → **rename to
   `collectedTotal`**), `:146` (event payload `totalEarned` → rename to `collectedTotal`).
5. `apps/api/src/routes/courier/settlements.ts:29,63` (SELECT → returns raw row; the row key `total_earned`
   surfaces to the courier → **map to `collectedTotal` in the response object**).
6. `apps/api/src/workers/backup/smoke-checks.ts:148-152` — `checkPayoutSums` (untouched; keeps working).
7. `packages/shared-types/src/contracts/owner/settlements.ts` — DTO type field (**rename `totalEarned →
   collectedTotal`**, matching the route).
8. `packages/ui/src/lib/i18n-catalog.ts` — "Payout History" / payout labels (**re-word to "Cash collected /
   to reconcile"**, never "owed to / paid to courier").
9. `apps/web/src/pages/courier/EarningsPage.tsx` — UI label (**re-word**; "Earnings" framing for a
   debt figure is the exact DoD-5 violation — see LATENT-STOP-1).
10. Tests touching the DTO: `apps/api/tests/phase5/integrity.test.ts`,
    `e2e/tests/flow-regulatory-settlements.spec.ts`, `e2e/tests/courier/full-coverage.spec.ts` — update the
    expected DTO field name.

**Net:** the rename is now a **DTO+i18n+UI surface change** (items 4,5,7,8,9,10) + a column COMMENT. The
physical column and every DB-level dependent (1,2,3,6) are unchanged → CRITICAL-1, and the LOW smoke-check
break, both disappear. RK-4 (physical-rename-on-a-🔴-table risk) is **withdrawn** — there is no physical
rename. The old A1↔A2 fork collapses: A1 (surface-only) is now the chosen path, and the "schema name stays a
lie" objection is bought off by the column COMMENT.

---

### CRITICAL-2 — `release`+`settle` coexistence / refund-after-settle / partial refund → **FIX (structural residual-guard trigger + hold-row serialization; courier-ledger = obligation-only)**

Make netting integrity **structural at write time**, not a read-time `NOT EXISTS`.

**Why not the partial-unique / EXCLUDE option (conductor option a):** a legitimate **partial refund** leaves an
order carrying BOTH a partial `release` (the refunded part) AND a `settle` of the remainder (worked example 3
below). "At most one contra per order" would forbid that. **Rejected.**

**Chosen — residual-guard trigger (structural) + hold-row lock (race-safe):**

1. **Residual-guard `BEFORE INSERT` trigger** on `courier_cash_ledger` for `type IN ('release','settle')`:
   `RAISE` if `NEW.amount > hold_amount(order) − COALESCE(SUM(existing release+settle for order), 0)`. This
   makes **net-can-never-go-negative** a DB invariant — over-reversal (the MED full-hold over-reverse),
   double-settle, and `release`+`settle` summing past the hold are all **structurally impossible at write
   time**. Replaces the false §4 `UNIQUE(order_id,type)` mutual-exclusion claim and the read-time `NOT
   EXISTS`. (This is the structural upgrade the MED RK-5 finding demanded.)
2. **Hold-row serialization:** the contra-writing path first `SELECT … FOR UPDATE` the `hold` row for that
   `order_id`. All contras for one order serialize through that lock, so the **race (scenario A)** between a
   settle-tx and a refund-tx cannot interleave a stale residual read — the loser re-evaluates against the
   committed residual. No global SERIALIZABLE needed (boring & proven).
3. **Contra amount = ACTUAL amount moved** (fixes the MED over-reverse): `release.amount =
   refunded_amount` (partial-aware), never a fixed full-hold. The assignment cash reversal becomes a
   **decrement of the refunded amount**, not a full `cash_amount=NULL` wipe, when the refund is partial.
4. **Courier ledger models COURIER OBLIGATION only.** This is what makes refund-after-settle correct:
   - **Refund before settle** (residual > 0): write `release(min(refund, residual))` → courier obligation
     drops. Owner till untouched (cash never reached owner).
   - **Refund after settle** (residual == 0, order already cash-dropped): the courier's obligation is
     **already 0**; a `release` would exceed residual → the trigger would raise. So the refund path does
     **NOT** write a courier-ledger row. Instead it records an **owner-refund fact** (the owner paid the
     customer from their own till) — see HIGH-4 for where that fact lands. Courier net stays 0 (**no phantom
     credit**); the owner's money-out is **recorded, not silently eaten** (closes Breaker scenario B both
     branches).
   - **Partial refund after partial settle:** refund up to the residual writes a `release`; any excess beyond
     the residual is owner-till money → owner-refund fact. The trigger caps the courier-ledger portion.

**Re-done §2 worked example — all orderings prove net-zero, no phantom credit:**

Orders A=3000, B=1500, C=2000; all delivered `paid_full` → obligation 6500.

*Ordering 1 — refund B (full) BEFORE settle:*

| Step | Event | Ledger | Courier obligation | Owner till |
|------|-------|--------|--------------------|-----------|
| 1-3 | hold A/B/C | hold 3000/1500/2000 | 6500 | — |
| 4 | refund B (residual 1500>0) | release B 1500 (=refund) | 5000 | unaffected (cash returned by courier) |
| 5 | shift close, confirm 5000 | settle A 3000, settle C 2000 | **0** ✅ | +5000 received |

Per-order net: A 0, B 0 (1500−1500), C 0.

*Ordering 2 — refund B (full) AFTER settle (the adversarial case):*

| Step | Event | Ledger | Courier obligation | Owner till |
|------|-------|--------|--------------------|-----------|
| 1-3 | hold A/B/C | holds | 6500 | — |
| 4 | shift close, confirm 6500 | settle A/B/C | **0** | +6500 received |
| 5 | refund B (residual 0) | **no courier-ledger row** (residual 0); owner-refund fact 1500 | **0** ✅ (no phantom credit) | −1500 (recorded owner refund) |

The owner's −1500 is a **recorded** fact, not silent loss; the courier net is 0, not −1500.

*Ordering 3 — PARTIAL refund of B (600 of 1500), before settle:*

| Step | Event | Ledger | Courier obligation |
|------|-------|--------|--------------------|
| 1-3 | hold A/B/C | holds | 6500 |
| 4 | refund 600 of B (≤ residual 1500) | release B **600** (not 1500) | 5900 |
| 5 | shift close, confirm 5900 | settle A 3000, settle B **900** (≤ residual 900), settle C 2000 | **0** ✅ |

Per-order net B: 1500 − 600 − 900 = 0. Order B legitimately carries a `release` **and** a `settle` — which is
exactly why the partial-unique option was rejected. The MED over-reverse (release 3000 on a 1000 refund) is now
impossible: the trigger caps `release ≤ residual` and the amount is the actual refund.

---

### HIGH-3 — FORCE-RLS placebo / self-DoS → **FIX (drop the false claim; bind to B3; make the writers NOBYPASSRLS-ready now)**

Two corrections + a real fix:

1. **Drop the headline claim.** Remove "FORCE closes the owner/BYPASSRLS gap." It is false: `FORCE` only
   subjects the table **owner** to RLS; it is inert against a separate **BYPASSRLS** login role
   (`1780421100065:8,21,35`). **And** `courier_payouts` is **already FORCE** (`1780421100051:11`) — so
   §5.3/ADR-mig-2 ("bring it to FORCE, it's ENABLE-only") was factually stale. Strike that migration item.
2. **Bind to B3 as a dependency** (mirror of B4's R11 cross-finding): the real closure of the money-RLS gap on
   `courier_payouts` / `courier_cash_ledger` is the **B3 NOBYPASSRLS writer-role** work. Record this as
   **DEPENDENCY: B3** in the ADR; Stage-21 does not claim to close it.
3. **Make the writers NOBYPASSRLS-ready NOW** so they don't self-DoS the day B3 lands:
   - `settlement-cron.ts` and `owner/settlements.ts` currently set **no** tenant GUC. Set
     `set_config('app.current_tenant', <locationId>, true)` at the start of each query block (as
     `courier/settlements.ts:25,59` already does).
   - Align the `courier_payouts` policy to tolerate an unset GUC without raising: use
     `current_setting('app.current_tenant', true)` (missing-ok) **or** migrate it to the canon
     `app_member_location_ids()` form. (Today `1780421100043:23` is `current_setting('app.current_tenant')::uuid`
     with no `true` → raises under FORCE+NOBYPASSRLS when unset, which is the self-DoS the Breaker named.)
   - The cron also needs INSERT/UPDATE, which the SELECT-only operational role can't grant — note this as part
     of the B3 role-grant work (the cron must run under a write-capable, tenant-GUC-setting role).
4. **Keep FORCE only as honestly-labeled defense-in-depth**, not the fix. The DoD adds: cron/owner-settlements
   return rows (don't self-DoS) under a set GUC.

---

### HIGH-4 — two-ledger divergence (refund vs baked `total_earned`/`settlement_items` snapshot) → **FIX (single source of truth = cash ledger; refund corrects the snapshot in-tx while pending, records owner-refund + flags when immutable; clean end-state = derive)**

Counsel's "two sources of truth for one money fact" is real. Decision:

- **MVP single source of truth = the cash ledger** (`hold − release − settle` = courier obligation). The
  `total_earned` / `settlement_items` snapshot is a **derived reporting** artifact.
- **The refund must keep the snapshot coherent in the same tx:** if a `settlement_item` exists for the
  refunded order **and the payout is still `pending`**, the refund tx contras/removes that item and decrements
  `total_earned` by the refunded amount (allowed while pending). This is the refund-after-settle owner-refund
  fact's landing place for the **snapshot** side.
- **If the payout is already `approved`/`paid`** (immutability trigger `1780421100052:8`), the snapshot is
  intentionally frozen and **cannot** be silently corrected. The refund then records the owner-refund as an
  **explicit fact** (owner-refund record + `settlement_audit_log` row + the reconciliation `status='discrepancy'`
  hook) and **flags the payout for owner review** — never leaves a stale figure the owner acts on without a
  marker. (This is the honest residual: an already-paid period's report stays historically as-paid; the
  correction is a forward-recorded fact, not a back-edit.)
- **Clean end-state (named, not built):** drop the stored `total_earned` snapshot entirely and **derive** the
  report from the ledger (`hold − release − settle` + an owner-refund view). One source of truth, no
  divergence possible, no immutability tension. Tracked as the post-MVP simplification.

---

### HIGH-5 — all-or-nothing strands the whole shift on a 1-unit miscount → **FIX (partial reconcile up to the confirmed amount; flag only the delta; NO auto-deduct)**

Replace "MVP = full-reconcile or none" with a bounded clearing path (the clearing is **not** deferred; only the
deeper who-eats-the-delta workflow is):

- Owner confirms cash received. The server **settles holds up to `confirmed_total`** (oldest-first), writing
  `settle` rows whose amounts the residual-guard trigger validates; the last partially-covered order gets a
  **partial settle** of the remaining confirmed amount.
- The **residual = computed − confirmed** is recorded as `courier_cash_reconciliations.status='discrepancy'`
  with the delta, and the courier's standing obligation is bounded to **exactly that delta**, not the whole
  shift.
- **NO auto-deduction** (NG-2 / `NO-AUTO-DEDUCT` preserved): the delta is a recorded fact for owner review,
  never a machine deduction from the courier.

*Worked example (Breaker's 6499-vs-6500):* settle A 3000, settle C 2000, partial settle B 1499 (≤ residual
1500). Confirmed 6499 cleared; B residual = **1** flagged discrepancy. Courier owes **1**, not 6500. The
off-by-one no longer inverts into full-shift phantom debt. The deeper discrepancy-**resolution** (who absorbs
the 1) stays **DEFER-FLAG** to its own Council (RK-1 narrowed to *resolution*, not *clearing*).

---

### MED — `prevent_ledger_mutation` vs `ON DELETE CASCADE` → **FIX (trigger blocks UPDATE only; deletion governed by the order's own erase)**

Make the new immutability trigger **`BEFORE UPDATE` only** — no `DELETE` clause. Rationale:

- There is **no app path** that DELETEs a `courier_cash_ledger` row directly; the only DELETE is the FK cascade
  from an `orders` hard-erase (GDPR), which **should** remove the order's ledger crumb too.
- Blocking UPDATE alone gives true content-immutability (you cannot alter a recorded cash event); deletion is
  bounded by the order's own heavily-guarded, audited erase governance. The cascade (GDPR hard-erase, test
  teardown) is **preserved** — no contradiction with the FK.
- DoD adds: a cascade DELETE of an order with ledger rows **succeeds**; a direct `UPDATE` on a ledger row
  **raises**.

---

### MED — courier can't read own cash ledger → **DEFER-FLAG, bound to LATENT-STOP-2 (no courier-visible debt ships without a courier-visible own-ledger)**

- MVP reconciliation is **owner-only**; no courier-facing "you owe X" view ships. Couriers already read their
  generated **payout report** (`courier/settlements.ts`, via `app.current_tenant` GUC) — a courier-readable
  settlement surface exists; only the `courier_cash_ledger` two-context read policy is deferred.
- **DEFER-FLAG** the courier-own cash-ledger read policy (RK-3). It is **gated by Counsel's pre-registered
  LATENT-STOP-2**: the moment any courier-visible debt/obligation view ships, the courier-visible own-ledger
  read policy **must** ship with it, or the STOP fires. Recorded so it is not re-litigated.

---

### MED — RK-5 coherence is a CI test, not structural → **FIX (now structural for over-reversal; precise term for the residual coherence test)**

- The **residual-guard trigger** (CRITICAL-2) now structurally enforces the load-bearing half: over-reversal /
  net-negative is impossible at write time, regardless of whether a future refund author remembers a test.
- The HIGH-4 same-tx snapshot correction enforces the cash-ledger↔snapshot move in code, not convention.
- The remaining CI coherence test is **re-specified with a defined term:** per courier, assert
  `SUM(hold) − SUM(release) − SUM(settle)` (cash ledger) equals
  `SUM(settlement_items.amount) − SUM(owner-refund corrections)` for the period. The old "un-`settle`d
  delivered assignments" term was ill-defined (`courier_assignments` has **no** `settle` concept — `settle` is
  the cash-ledger's). Fixed.

---

### LOW — currency hard-coded `'ALL'`, no GROUP BY → **ACCEPT-RISK(owner) under single-currency invariant + flag**

- Record the **single-currency-per-deployment invariant** explicitly: today `owner/settlements.ts:147`
  publishes `currency:'ALL'` and the cron sums `cash_amount` across assignments with no `GROUP BY
  currency_code` (`settlement-cron.ts:61,95`). Safe **only** while one currency.
- The `total_earned`/`collectedTotal` figure has **no currency dimension** — flag: before any multi-currency
  location, the reconciliation and the cron must `GROUP BY currency_code` and the figure must carry its
  currency. **ACCEPT-RISK** for MVP (single-currency); **owner**-owned flag for multi-currency.

---

### LOW — rename breaks backup smoke check → **FIX (resolved by CRITICAL-1 surface-only)**

The physical column stays `total_earned`, so `checkPayoutSums` (`smoke-checks.ts:148-152`) is **untouched**.
The separate observation — that even when working it compares two stale snapshots and can't see refund
divergence — is addressed by HIGH-4's same-tx snapshot correction (and fully by the derive end-state).

---

### RK-2 + Counsel §4/§5 (delivery-fee) → **NEEDS-HUMAN (STOP-ETHICS) — NOT resolved unilaterally**

Pre-staged for the human; I verified the factual half so the decision is informed.

**Q1 (Counsel §4 — the one decision that discharges RK-2):**
> At launch, is the courier ever a **non-owner hired worker**, or only the owner / a family member?
- Owner/family only → RK-2 discharged; debt-only model is honest for one's own till; keep rename + DoD-5;
  LATENT-STOPs stay dormant.
- Includes a hired courier → asymmetry is live; minimum-to-launch = rename + DoD-5 **PLUS** either (a)
  read-only `expected-pay-before-accept` (Counsel's steel-man sliver — light, non-scoring, does **not** touch
  any penalty/score column, so R-9 does not reach it), **or** (b) a **recorded** human acceptance that hired
  couriers launch with zero in-system earnings, earnings as the next named Council.

**Q2 (Counsel §5 — delivery-fee; FACTUAL ANSWER verified):**
> Does the `'hold'` / `collectedTotal` bundle the courier's own delivery fee?
- **Yes, structurally.** `orders.ts:499`: `total = subtotal + deliveryFee + taxTotal − discountTotal`; the hold
  amount is `o.total` (`courier/assignments.ts:319,356` → `completeDelivery(... total ...)` →
  `deliveryCompletion.ts:104-110` `hold = cashAmount = total`). The hold **includes the delivery fee**.
- **Whose money is the fee?** Under the current model there is **no earnings model** (NG-1), so the entire
  `delivery_fee` is treated as **owner revenue**; **no portion is designated courier pay anywhere in code**.
  The courier is therefore recorded as owing back **100%** of the COD including the delivery fee.
- **The sharp edge (for the human):** if the business intends the courier to **keep** the delivery fee (a
  common arrangement), the system currently records them owing it back in full, with their pay settled
  out-of-band by **no recorded mechanism** — i.e. the courier fronts the owner's cash **and** their own fee on
  an unrecorded promise (Counsel's "creditor-of-last-resort to their own employer"). This is **real and live**
  *iff* the fee is meant to be the courier's. The code today makes it owner revenue, so the question is a
  **business/labor intent** question, not a code question.

→ Both Q1 and Q2 go to **STOP-ETHICS / human**. RK-2 is **not closed** until they are answered. No active STOP
fires; Counsel's two **LATENT-STOPs** stay pre-registered (honesty floor; courier-visible-debt-needs-own-ledger;
remote-reconciliation-while-earnings-unmodeled).

---

### Counsel non-blocking items

- **`expected-pay-before-accept` sliver** — ruled on **separately** from the deferred engine (per Counsel §3):
  carried as its own **DEFER-FLAG / next-Council** item, distinct from NG-1's A3 rejection.
- **Two-ledgers-for-one-truth** — **named** as debt (HIGH-4): MVP keeps both with in-tx coherence; clean
  end-state derives the report from the ledger.

---

## Disposition summary

| Finding | Disposition |
|---------|-------------|
| CRITICAL-1 rename break | **FIX** — surface-only rename; physical column + trigger + cron + smoke untouched; reader list enumerated; RK-4 withdrawn |
| CRITICAL-2 release/settle coexistence, refund-after-settle, partial | **FIX** — residual-guard trigger + hold-row lock + obligation-only ledger + actual-amount contra; 3 orderings proven net-zero |
| HIGH-3 FORCE placebo / self-DoS | **FIX** — drop false claim (and the stale "ENABLE-only"); B3 dependency; set tenant GUC in cron + owner-settlements now; FORCE = defense-in-depth |
| HIGH-4 two-ledger divergence | **FIX** — ledger = SoT; refund corrects snapshot in-tx while pending, records owner-refund + flags when immutable; derive end-state named |
| HIGH-5 all-or-nothing miscount | **FIX** — partial reconcile up to confirmed; flag only the delta; no auto-deduct |
| MED ledger trigger vs cascade | **FIX** — UPDATE-only trigger; cascade preserved |
| MED courier can't read ledger | **DEFER-FLAG** bound to LATENT-STOP-2 |
| MED RK-5 CI-test-not-structural | **FIX** — trigger makes over-reversal structural; coherence test term redefined |
| MED full-hold over-reverse | **FIX** — folded into CRITICAL-2 (actual-amount contra) |
| LOW currency | **ACCEPT-RISK(owner)** under single-currency invariant + multi-currency flag |
| LOW smoke-check break | **FIX** — moot under surface-only rename |
| RK-2 (earnings/fairness) | **NEEDS-HUMAN** (Q1) |
| Counsel §5 delivery-fee | **NEEDS-HUMAN** (Q2) — factual answer verified and reported |

## Honest residuals (for the focused re-attack)

1. **The refund caller still does not exist** (RK-5). The residual-guard trigger now makes over-reversal
   *structurally* impossible, but the "record the owner-refund fact + correct the snapshot while pending +
   flag when immutable" obligations are a **contract** a future refund implementer must honor. The trigger
   catches the money-corruption half; the owner-refund **recording** half is still convention until that path
   ships with its own tests.
2. **Immutable already-paid snapshot** can't be back-corrected (HIGH-4) — a refund against an `approved`/`paid`
   period records a forward owner-refund fact and a review flag, but the historical report figure stays
   as-paid by design. The clean derive end-state is **named, not built**.
3. **Partial-settle ordering** (HIGH-5 oldest-first) is a policy choice; an owner who wants to choose *which*
   orders settle isn't served at MVP (acceptable — the math holds regardless of order).
4. **Multi-currency** remains unhandled (accepted under single-currency invariant).
5. **RK-2 / delivery-fee intent** is genuinely open and gates honest launch for a hired courier — human only.
6. **B3 dependency** is external: until the NOBYPASSRLS writer role + grants land, the money-RLS gap on
   `courier_payouts`/`courier_cash_ledger` is *not* closed by Stage-21; the GUC-readiness only prevents the
   self-DoS when it does.
