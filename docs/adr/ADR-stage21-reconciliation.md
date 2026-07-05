# ADR — Stage-21 Courier Cash Reconciliation (B1: money model + shift-close)

- **Status:** **COUNCIL-CONVERGED (R4 breaker ack: CONVERGED, 2026-07-02) — operator ratification required
  before build (money red-line).** Resolution round 1 + Revision 2 (re-attack) + Revision 3 (convergence turn) +
  **Revision 4 (idempotency single-branch)** applied
  (`docs/design/stage21-reconciliation/resolution.md`, `.../reattack-resolution.md`);
  counsel: SATISFIED-WITH-CONDITIONS (C1–C4 folded into DoD/NEEDS-HUMAN, rev.3/4).
  Non-blocking build-time note (breaker R4, MED): amnesty re-run + an in-flight deliveryCompletion tx
  spanning the amnesty MVCC snapshot (created_at ≤ cutoff, commit after snapshot) would amnesty a live
  obligation on the RE-RUN — implementer must either forbid re-runs after the first successful run or
  take the re-run's hold-set from the ORIGINAL run's recorded snapshot, never a fresh scan.
  Design-time, NO production code — awaiting human sign-off (STOP-DESIGN-B).
- **Date:** 2026-06-29 (rev.1 post-breaker/counsel) · **rev.2 2026-07-02 (post re-attack)** · **rev.3
  2026-07-02 (convergence turn — 3 under-specifications pinned + counsel binding conditions folded)** ·
  **rev.4 2026-07-02 (idempotency single-branch — amount-derived server key deleted)**
- **Red-line:** 🔴 MONEY · launch-blocker B1
- **Proposal:** `docs/design/stage21-reconciliation/proposal.md`
- **Extends / bound by:** `ADR-deliver-v2-cash-as-proof.md` (the `'hold'` primitive; carried invariants
  R-8/R-9; the anti-scoring-creep guardrail), `docs/finance/settlements.md`.
- **Materializes:** the deliver-v2 *"NEEDS-HUMAN before launch"* item — the merged R-8+R-9 Stage-21
  invariant. Authoring this file with the two markers below turns the red-on-disk guardrail
  `apps/api/tests/stage21-no-auto-deduct.invariant.test.ts` GREEN.

---

## Revision 2 — 2026-07-02 (post re-attack)

A second Breaker pass, verified against the live tree, invalidated four load-bearing premises of Rev.1 and
surfaced four new money-integrity gaps. Rev.2 resolves all eight. **Summary of what changed vs Rev.1:**

- **[C1] Contra multiplicity was structurally impossible.** `1790000000028:19 UNIQUE(order_id,type)` caps the
  ledger at **one** `settle` per order, so partial settlement across two shift-closes collides and
  `ON CONFLICT DO NOTHING` would *silently drop* the second settle → a permanently open hold. Rev.1's
  residual-guard math assumed many contra rows can accumulate per order — false under that UNIQUE.
  **Rev.2:** contras (`settle`/`release`) move to a **new `courier_cash_contras` table** with per-event
  idempotency keys. `courier_cash_ledger` stays **hold-only and literally untouched** (its `UNIQUE(order_id,type)`
  and the `deliveryCompletion` hold insert are unchanged — the "cash spine untouched" constraint forbids the
  in-place `DROP UNIQUE`, since the hold insert's `ON CONFLICT (order_id,type)` requires that exact index).
- **[C2] The owner-refund fact had no storage.** `courier_cash_reconciliations CHECK status IN
  ('reconciled','discrepancy')` cannot hold a refund-after-settle write-off. **Rev.2:** a dedicated
  **`courier_cash_owner_refunds`** table, written by the (future) refund path in the refund tx, surfaced to the
  owner as a "refunds to review" list + a payout review flag.
- **[H1] Stale ground truth.** The `total_earned` writer is **`app_generate_settlements()` SECURITY DEFINER**
  (`1790000000078:160-197`, write at `:189`), NOT `settlement-cron.ts:95-105` (those lines do not exist — the
  file is 51 lines and delegates via `SELECT app_generate_settlements($1,$2)` at `:44`). **Rev.2:** every
  citation re-grounded; the DoD "cron survives" now targets the DEFINER fn.
- **[H2] The NOBYPASSRLS "fix" was inert.** Setting `app.current_tenant` in the cron caller does nothing — the
  money write executes inside the DEFINER fn owned by a bypass role. **Rev.2:** the DEFINER-fn-as-gateway *is*
  the B3 closure for the system-sweep path; the GUC fix is re-scoped to the **owner reconciliation path** only.
- **[H3-me] Dropped courier surface.** `courier/me.ts:218` serves `total_earned AS amount` to the courier as
  earnings — the exact inversion, missing from Rev.1's reader list. **Rev.2:** added.
- **[H4] Rollout amnesty.** Every `paid_full` delivery since mig 028 wrote a `hold` and zero settles → a naïve
  flag-flip surfaces **Σ(all lifetime holds)** as the courier's open obligation, including already-paid-out
  periods. **Rev.2:** an explicit, auditable **opening-balance amnesty** (one `settle` contra per outstanding
  hold as-of flag-enable) — replaces Rev.1's "no backfill".
- **[H5] Surplus race.** `confirmed_total > Σ(visible holds)` (a delivery lands mid-count) must be a recorded
  **unmatched-cash fact**, never a `RAISE`. **Rev.2:** specced symmetric to the shortfall path.
- **[M1] First-reconcile N+1.** **Rev.2:** amnesty removes the lifetime-holds scan; the settle path is
  set-based (`INSERT … SELECT` with a windowed cumulative sum) + a per-tx batch cap with idempotent
  continuation.

---

## Revision 3 — 2026-07-02 (convergence turn)

Round-2 Breaker verdict: architecture **sound**, all 8 Rev.2 findings **closed**, **not yet converged** on
exactly three under-specifications. Counsel opinion: **SATISFIED-WITH-CONDITIONS**. Rev.3 pins all three and
folds in the counsel's binding conditions; no premise changed, only tightened.

- **[U1 · HIGH] Deterministic `reconciliation_id` for the OWNER path was not mandated.** Rev.2 said "owner
  idempotency-key header OR pg-boss job id" — but the owner shift-close is a **synchronous HTTP tap; no
  pg-boss job id exists there**, and the header was optional. A lost-response retry with a fresh uuid
  **over-settles** while the residual-guard stays silent (it fires per-order, and the retry targets *different*
  residual orders). Failure demo: confirm 900/1500 settles orders 1–3; a retry under a fresh id sees residual
  600 on orders 4–5 → settles → **1500 settled for 900 physically received**. **Rev.3 MANDATE (§3):** the owner
  path derives a **server-side deterministic id** `reconciliation_id = kind='shift_close' :: uuid_v5(namespace,
  location_id ∥ courier_id ∥ shift_id-or-confirmed-window ∥ confirmed_total)` **OR** requires an
  `Idempotency-Key` header → **HTTP 422 on absence** (no silent fresh-uuid). Uniqueness: one
  `courier_cash_reconciliations` row per derived id (unique). Replay: same id → **return the stored
  reconciliation result verbatim, write zero new contras** (the settle `INSERT` `ON CONFLICT (order_id,
  reconciliation_id) WHERE type='settle' DO NOTHING` no-ops; the reconciliation-row insert
  `ON CONFLICT (id) DO NOTHING RETURNING` falls back to a `SELECT` of the stored row).
- **[U2 · MED-HIGH] Amnesty as-of cutoff was unbounded.** `deliveryCompletion` is **not** flag-gated, so holds
  keep arriving *during* the amnesty run — a naïve "settle every outstanding hold" would amnesty live
  obligations created after the sweep started. **Rev.3 (§7):** capture `amnesty_cutoff_ts` **once** at amnesty
  start, store it on the `kind='opening_balance'` reconciliation row; amnesty settles **only holds with
  `order created_at <= amnesty_cutoff_ts`**. Holds after the cutoff are **live obligations** for the first real
  shift-close, never amnestied.
- **[U3 · LOW-MED] Batch/cap continuation arithmetic was implicit.** For a capped shift-close settle that
  resumes across batches, the remaining cap must be recomputed from what THIS reconciliation already wrote —
  not from the original `confirmed_total`. **Rev.3 (§4/§7):** each continuation batch recomputes, inside the tx,
  `remaining_cap = confirmed_total − Σ(contra.amount WHERE reconciliation_id = THIS id AND type='settle')`; the
  windowed cumulative sum is capped at `remaining_cap`. Convergence when `remaining_cap = 0` or no residual
  holds remain.

**Counsel binding conditions folded (opinion SATISFIED-WITH-CONDITIONS):**
- **[C-counsel-1] opening_balance settles must stay semantically distinct** in any owner UI (keyed off
  `reconciliation.kind`) — **never** aggregated into "cash received from courier". Added to DoD.
- **[C-counsel-2] `unmatched_cash` needs an owner-resolution affordance** (analog of the refunds' `resolved_at`)
  — added as a **NAMED** item to the deferred RK-1 discrepancy-resolution council, not left silent.
- **[C-counsel-3] Historical credit-side blindness (RK-2 re-arm trigger):** amnesty forgives only the **debt
  side the system can see** (Σ visible holds). For a **hired** courier the **credit side** — a possible
  out-of-band *under*payment of wages for the same epoch — is **invisible** to the system; amnesty cannot
  reason about it. Recorded as part of the **RK-2 re-arm** trigger for the first non-owner courier.

---

## Revision 4 — 2026-07-02 (idempotency single-branch)

Round-3 Breaker ack: architecture **sound**, U2/U3 pinned correctly, but the U1 idempotency contract carried a
**residual [CRITICAL]** in its *first* branch. Rev.3 offered two mandated ways to derive `reconciliation_id`;
the server-side amount-derived branch is itself a C1-class defect and is **deleted** in Rev.4. No new premise —
one branch removed, one filter re-keyed.

- **[U1 · CRITICAL] Delete the server-side amount-derived `reconciliation_id` branch.** Rev.3's
  `uuid_v5(location ∥ courier ∥ shift ∥ confirmed_total)` server key **collides on two legitimate same-amount
  closes in one shift** — common with round cash amounts — and the settle's `ON CONFLICT (order_id,
  reconciliation_id) WHERE type='settle' DO NOTHING` then **silently drops the second settle**: the exact **C1
  failure class** the whole redesign exists to kill, reintroduced on the key-derivation side. **Rev.4 MANDATE
  (§3):** the **client-generated `Idempotency-Key` header is the ONLY contract.** The key identifies a
  shift-close **GESTURE** — the client generates a fresh UUID for each deliberate close and **reuses the same
  key only on a transport retry** of that one gesture. **HTTP 422 when the header is absent** (no server-derived
  fallback of any shape). **Replay** (same key) returns the stored reconciliation result **verbatim** and writes
  **ZERO** new contras — *including when the retry carries a different `confirmed_total`*: the **stored result
  wins** and the owner sees the stored figure echoed back (a changed figure is a **new gesture** and demands a
  **new key**). Two legitimate same-amount closes therefore carry two distinct keys → two reconciliations, both
  settle; any amount-derived server key would have merged them and dropped the second settle via
  `ON CONFLICT DO NOTHING`.
- **[U2 · MED] Re-key the amnesty cutoff filter onto the HOLD row's own `created_at`.** Rev.3 filtered on the
  **order's** `created_at`, which mis-classifies a **late delivery of an old order**: the order predates the
  cutoff but its hold is written by `deliveryCompletion` **after** the sweep — it must remain a **live
  obligation**, not be amnestied. **Rev.4 MANDATE (§7):** filter on **`courier_cash_ledger.created_at <=
  amnesty_cutoff_ts`** (the hold's own write time). The amnesty runs as a **single `INSERT … SELECT` per
  courier** (one MVCC snapshot); a re-run reuses the same opening-balance reconciliation id **and** the same
  stored `amnesty_cutoff_ts`, so late-arriving holds (`created_at > cutoff`) are never amnestied and the re-run
  is a true no-op.
- **[U3 · LOW-MED] CLOSED — no change.** Its capped-continuation safety **inherits the now-single-branch key
  stability**: the deterministic id it relied on is the client `Idempotency-Key`, which is stable across a
  gesture's retries by construction (no server re-derivation that could shift under a changed figure).

---

## Context

Verified ground truth (file:line — **re-grounded in Rev.2**):

- `courier_payouts.total_earned` (`packages/db/migrations/1780421100043:12`) is computed **inside the
  SECURITY DEFINER fn `app_generate_settlements()`** (`packages/db/migrations/1790000000078:160-197`; the
  write is `total_earned = total_earned + v_added_total` at `:189`) as `SUM(courier_assignments.cash_amount)`
  over delivered + `cash_collected` assignments — **the COD cash the courier COLLECTED**, no commission/wage
  deduction. `settlement-cron.ts` (51 lines) merely **delegates**: `SELECT app_generate_settlements($1,$2)`
  at `:44`. There is **no earnings model** anywhere in the codebase. *(Rev.1 wrongly cited
  `settlement-cron.ts:95-105/:103` as the writer — corrected here per re-attack H1.)*
- Every surface frames this as **money the owner owes the courier**: `owner/settlements.ts:29,57`
  (`totalEarned`); **`courier/me.ts:218`** (`total_earned AS amount`, served to the courier as earnings —
  added in Rev.2); i18n "Payout History"; `docs/finance/settlements.md:24` "paid: Owner marks the payout as
  transferred to the courier". It is the **opposite** — cash the courier holds and **owes the owner**. Paying
  it out = owner double-loss.
- `deliveryCompletion.ts:118-121` appends one `courier_cash_ledger` `'hold'` row at DELIVERED when
  `paid_full`, via `INSERT … VALUES (…, 'hold', $4) ON CONFLICT (order_id, type) DO NOTHING` — **this insert
  and its `UNIQUE(order_id,type)` dependency are the untouchable cash spine.** The CHECK allows
  `('hold','release','settle')` (`1790000000028:16`) but `'release'`/`'settle'` are **never written** → a
  hold is permanent; **no shift-close clears it**. The `UNIQUE(order_id,type)` (`028:19`) also caps the table
  at one contra per type per order (the C1 defect).
- No post-delivery refund path exists; the pre-delivery cancel (`customer/orders.ts:307-326`) runs before any
  hold is written, so the latent orphan-debt risk is the *absence* of a reversal, not a buggy one.

## Decision

1. **Till-accountability only; NO earnings model.** The courier collects the owner's cash and owes it back;
   wages are paid out of band and are **not modeled here** (segment-correct for a 1–5-person cash shop). This
   finishes the deliver-v2 *till-accountability* primitive; it introduces no new money concept.

2. **Honest naming — SURFACE-ONLY (RESOLVED CRITICAL-1 rev.1; reader list corrected rev.2).** **No
   `RENAME COLUMN`.** A physical rename breaks `prevent_payout_mutation` (references `OLD/NEW.total_earned`),
   the DEFINER writer `app_generate_settlements()` (`1790000000078:189`), and `checkPayoutSums`. Keep
   `total_earned` physical (+ a truth-telling `COMMENT`) and rename only the read surface → `collectedTotal`
   at every reader. **Reader list (rev.2 — adds `courier/me.ts`):** `owner/settlements.ts:57,146`;
   `courier/settlements.ts` (map row key); **`courier/me.ts:218`** (`total_earned AS amount` → honest key;
   and the adjacent `today/week/month` `SUM(cash_amount) AS amount` summary block `:227-234` reframed off
   "earnings"); `packages/shared-types/.../owner/settlements.ts`; `i18n-catalog.ts`; `EarningsPage.tsx`. The
   physical column + the DEFINER fn + `checkPayoutSums` stay untouched (DoD-5 at near-zero blast radius).

3. **Release the hold via a SEPARATE append-only contra table; netting enforced STRUCTURALLY (REDESIGNED —
   RESOLVED CRITICAL-1/CRITICAL-2 rev.2).**
   - **`courier_cash_ledger` stays HOLD-ONLY and literally untouched** — its `UNIQUE(order_id,type)` and the
     `deliveryCompletion` hold insert (`ON CONFLICT (order_id,type)`) are unchanged (the cash-spine-untouched
     constraint forbids the in-place `DROP UNIQUE`).
   - **New `courier_cash_contras` table** holds `settle` and `release` rows (integer, `amount >= 0`):
     - `'settle'` — owner-confirmed cash drop; `reconciliation_id NOT NULL`; **partial allowed**.
     - `'release'` — refund/cancel contra; `reversal_id NOT NULL`; `amount = ACTUAL refunded amount`
       (partial-aware).
   - **Precise idempotency invariant (what uniquely identifies a contra row):**
     - a **settle** is uniquely `(order_id, reconciliation_id)` — partial unique index
       `WHERE type='settle'`. **Multiple settle rows per order** (one per reconciliation event) → partial
       settlement **across successive shift-closes is now structurally possible** (the C1 fix).
     - a **release** is uniquely `(order_id, reversal_id)` — partial unique index `WHERE type='release'`.
       Multiple partial releases per order (one per refund event) allowed.
   - **Double-settle guard — SINGLE-BRANCH idempotency contract (rev.4 U1; rev.3's server-key branch DELETED).**
     The owner shift-close is a **synchronous HTTP tap; there is NO pg-boss job id on that path.** The
     **client-generated `Idempotency-Key` header is the ONLY contract** — the rev.3 server-side amount-derived
     `uuid_v5(loc ∥ courier ∥ shift ∥ confirmed_total)` branch is **struck**: it collides on two legitimate
     same-amount closes in one shift (common with round cash) and would silently drop the second settle via
     `ON CONFLICT DO NOTHING` — the exact **C1 failure class**.
     - **Key = the shift-close GESTURE.** The client mints a fresh UUID per **deliberate** close and reuses it
       **only on a transport retry** of that gesture. `reconciliation_id` = that key.
     - **HTTP 422 on absence** (no server-derived fallback of any shape).
     - **Uniqueness:** one `courier_cash_reconciliations` row per key.
     - **Replay semantics:** same key → **return the stored reconciliation result verbatim, write ZERO new
       contras — including when the retry carries a *different* `confirmed_total`: the STORED result wins and the
       owner sees the stored figure echoed back** (a changed figure is a *new gesture* → *new key*). The
       reconciliation-row insert is `ON CONFLICT (id) DO NOTHING` and, on conflict, `SELECT`s and returns the
       stored row; the settle `INSERT … ON CONFLICT (order_id, reconciliation_id) WHERE type='settle' DO NOTHING`
       no-ops every already-written contra.
     - **Two same-amount closes → two distinct keys → two reconciliations, both settle** (no merge). (Same
       idempotency shape for `release` via `reversal_id`, which retains its natural job/reversal id.)
   - A **`BEFORE INSERT` residual-guard trigger on `courier_cash_contras`** raises if
     `NEW.amount > hold_amount(order) − COALESCE(Σ existing contras for order, 0)`, reading the hold from
     `courier_cash_ledger`; the contra path `SELECT … FOR UPDATE`s the hold row so concurrent settle∥refund
     **serialize** (kills the TOCTOU race). Net-negative / over-reversal / double-settle are **structurally
     impossible** at write time. Neither `settle` nor `release` is a penalty type
     (`guardrail-deliver-v2.mjs:73`) — ship without weakening the gate.
   - **Courier obligation** = `Σ(holds) − Σ(contras)` (join of the two tables). Nets to zero on reconciliation.

4. **Reconciliation authority = owner-confirmed cash drop, PARTIAL-reconcile + SURPLUS-safe (RESOLVED HIGH-5
   rev.1 + surplus race rev.2).** Owner taps "cash received from courier C"; the server settles holds **up to
   `confirmed_total`** (oldest-first, last order partial) in one idempotent tx.
   - **Shortfall** (`confirmed_total < Σ visible residuals`): settle the confirmed amount; record only the
     **delta** as `status='discrepancy'`. The courier's standing obligation is bounded to the delta, **never**
     the whole shift; no auto-deduct (NG-2).
   - **Surplus / race** (`confirmed_total > Σ visible residuals` — a delivery completes during the count):
     settle **all** visible holds fully; record the excess `confirmed_total − Σ visible holds` as
     **`unmatched_cash`** on the reconciliation row (a fact for owner review — most likely a hold not yet
     written or a next-period order). **Never `RAISE`, never a negative obligation, never an auto-credit.**
   - **Capped-settle continuation arithmetic (rev.3 U3).** The settle is set-based with a per-tx batch cap
     (M1/§7). When a large shift-close spans batches, each continuation batch recomputes, **inside the tx**,
     `remaining_cap = confirmed_total − Σ(courier_cash_contras.amount WHERE reconciliation_id = THIS id AND
     type='settle')` — resuming from what THIS reconciliation already wrote, **never** re-basing on the raw
     `confirmed_total`. The windowed cumulative sum is capped at `remaining_cap`; the run converges when
     `remaining_cap = 0` or no residual holds remain. Because `reconciliation_id` = the client
     `Idempotency-Key` (stable across a gesture's retries by construction — rev.4 U1), a mid-continuation retry
     is idempotent (already-written contras `ON CONFLICT` no-op; `remaining_cap` recomputes to the correct
     residual).
   Server computes; owner confirms the figure, never sets it. No time-based auto-release.

5. **Refund reversal is same-tx, obligation-aware; the owner-refund fact is STORED (RESOLVED CRITICAL-2 +
   HIGH-4 rev.1; storage designed rev.2 C2).** If residual > 0: append a `'release'` contra
   (`amount = min(refund, residual)`) + decrement the assignment cash, and **while the payout is `pending`**
   also contra the `settlement_item` + decrement `total_earned` (snapshot coherent in-tx, via the owner-path
   DEFINER fn — see §8/H2). If residual == 0 (already settled) or the payout is `approved`/`paid` (immutable):
   write **no contra row**; instead record an **owner-refund fact** in the new
   **`courier_cash_owner_refunds`** table + flag the payout for owner review. Net 0, no phantom credit, owner
   loss **recorded**, never silently eaten.

6. **Money-RLS = B3 dependency; NOBYPASSRLS-readiness re-specced for the DEFINER writer (RESOLVED HIGH-3 rev.1,
   CORRECTED HIGH-2 rev.2).**
   - `courier_payouts` is **already `FORCE`** (`1780421100051:11`). FORCE is inert against the live
     **BYPASSRLS** writer; the real closure is the **B3 NOBYPASSRLS** work (**DEPENDENCY: B3**).
   - **System-sweep path (cron):** the total_earned write runs **inside** `app_generate_settlements()`
     (SECURITY DEFINER, owner role). Setting `app.current_tenant` in `settlement-cron.ts` **does nothing** for
     that write (it bypasses RLS as the fn owner). The **DEFINER-fn-as-gateway is itself the B3 closure**
     (per the `1790000000078` header): post-B3, `dowiz_app` (NOBYPASSRLS) cannot touch `courier_payouts` /
     `settlement_items` directly — only via this audited fn (search_path pinned `:161`, `REVOKE ALL FROM
     PUBLIC` + `GRANT EXECUTE TO dowiz_app` `:196-197`). NOBYPASSRLS-readiness for this path = the fn's owner is
     a dedicated non-login RLS-bypass role, not the app login role. **Strike** Rev.1's "set GUC in the cron
     closes it."
   - **Owner reconciliation path:** the `settle`/contra writes and the pending-payout snapshot correction run
     as `dowiz_app` with the owner's **member** identity. Post-B3 they ARE subject to FORCE-RLS; the owner is a
     member, so set `app.current_tenant = location_id` in the owner path and make the `courier_payouts` policy
     missing-GUC-tolerant. The pending-payout `total_earned` decrement (§5) touches a member-keyed money table
     → route it through a small **owner-path DEFINER fn** (B3-consistent), not a raw member UPDATE.

7. **Rollout amnesty for tenants with history (RESOLVED HIGH-4 rev.2 — replaces "no backfill").** Every
   `paid_full` delivery since mig 028 wrote a `hold` with zero settles. Before the flag flips on for a tenant,
   a one-time, **auditable opening-balance amnesty** runs: for each outstanding hold (no contra), insert a
   `settle` contra referencing a synthetic `courier_cash_reconciliations` row of `kind='opening_balance'`
   (amount = full residual). This treats all pre-reconciliation obligation as already-settled out-of-band —
   **recorded and auditable**, so the first real shift-close starts from zero, not from Σ(lifetime holds).
   Order: schema/triggers/indexes migrate → amnesty backfill → flag flip. Idempotent (`ON CONFLICT` on the
   opening-balance reconciliation per courier+location).
   - **As-of cutoff (rev.4 U2 — MANDATED; re-keyed onto the HOLD's own `created_at`).** `deliveryCompletion` is
     NOT flag-gated, so holds keep arriving **during** the amnesty run. Capture `amnesty_cutoff_ts` **once** at
     amnesty start and **store it on the `kind='opening_balance'` reconciliation row**; amnesty settles **only
     holds whose `courier_cash_ledger.created_at <= amnesty_cutoff_ts`** — the **hold row's own write time, NOT
     the order's `created_at`.** *(Rev.3 keyed on the order's `created_at`; that mis-classifies a **late delivery
     of an old order** — the order predates the cutoff but its hold is written after the sweep, so it must stay a
     **live obligation**.)* The sweep is a **single `INSERT … SELECT` per courier (one MVCC snapshot)**; any hold
     with `created_at > amnesty_cutoff_ts` is a **live obligation** for the first real shift-close — **never
     amnestied**. A re-run reuses the **same opening-balance reconciliation id AND the same stored
     `amnesty_cutoff_ts`**, so it re-derives the identical hold set and `ON CONFLICT` no-ops — a **true no-op**.

8. **Hardened append-only.**
   - `prevent_ledger_mutation` / `prevent_contra_mutation` are **`BEFORE UPDATE` only** (content-immutability);
     **no `DELETE` clause** so the `orders ON DELETE CASCADE` (GDPR hard-erase) still works on both tables.
   - See §6 for the NOBYPASSRLS / B3 posture.

9. **Flag-gated runtime.** `COURIER_CASH_RECONCILIATION_ENABLED` (default OFF) gates the shift-close runtime;
   schema + lib + amnesty land inert (amnesty runs at flag-enable time per tenant). The naming rename ships
   unflagged (pure honesty fix).

## Carried invariants (the council's binding conditions — materialized markers)

> **NO-AUTO-DEDUCT** — Stage-21 reconciliation NEVER auto-deducts a no-fault shortfall (robbery / short-pay /
> miscount) from a courier. A shortfall is recorded as a fact (`courier_cash_reconciliations.status =
> 'discrepancy'`, computed-vs-confirmed delta) for **owner review** — owner-reviewed friction, never a
> machine deduction. A **surplus** is recorded as `unmatched_cash` for owner review, never an auto-credit.
> The discrepancy-resolution layer does not land without its own Triadic Council.

> **NO-COURIER-SCORING** — No crumb-derived courier score or penalty. No `'deduction'`/`'penalty'`/`'fine'`/
> `'score'` ledger type; no penalty derived from `delivery_trace` / `order_sensor_events` /
> `customer_signals`. The anti-scoring-creep guardrail (`guardrail-deliver-v2.mjs:73`) stays in force,
> unweakened. Any scoring/penalty engine requires its own Triadic Council.

## Consequences

**Positive:** the ledger nets to zero on reconciliation across *all* orderings and *partial settlement over
successive shift-closes*; the inversion is removed at every layer including the courier's own surface; the
owner is never double-charged and refund-after-settle is a **recorded** owner loss, not a silent one;
additive/forward-only; the red-line cash spine (`deliveryCompletion` hold-in-tx + `courier_cash_ledger`
UNIQUE) is **literally untouched**; the anti-scoring-creep gate is preserved, not bent; existing tenants get
an auditable amnesty rather than a phantom lifetime debt.

**Negative / accepted:** earnings/wage model deliberately absent (NG-1; fairness + delivery-fee =
NEEDS-HUMAN, RK-2 — and, per counsel C-counsel-3, amnesty forgives only the **debt side the system sees**;
for a hired courier the **credit side** — possible out-of-band wage *under*payment for the same epoch — is
invisible and is part of the RK-2 re-arm trigger); discrepancy *resolution* deferred (RK-1 — clearing is
in-scope, absorption of the delta is not; the deferred RK-1 council now carries a **NAMED** item: an
owner-resolution affordance for `unmatched_cash` via `unmatched_cash_resolved_at`, analog of the refunds'
`resolved_at` — C-counsel-2); courier cash-ledger/contra view deferred, bound to LATENT-STOP-2 (RK-3); the `release` primitive +
`courier_cash_owner_refunds` writer are built ahead of any refund caller (RK-5 — over-reversal is structurally
blocked, but the owner-refund-*recording* obligation is a contract until the caller ships with tests);
two tables for the ledger (holds vs contras) is a slight elegance cost, chosen because the cash-spine-untouched
constraint forbids the single-table `DROP UNIQUE`; multi-currency unhandled under the single-currency
invariant (RK-6); money-RLS closure depends on **B3** (external).

**NEEDS-HUMAN before launch (STOP-ETHICS — `resolution.md` §RK-2; DISCHARGED owner/family-only per
`ethical-decisions.md`, re-armed for the first hired courier):**
1. Is the launch courier ever a **non-owner hired worker**, or only owner/family? *(Answered 2026-06-29:
   owner/family only at launch → RK-2 discharged with a recorded SEGMENT CONSTRAINT; re-arms before any hired
   courier.)* **Re-arm trigger now also carries the counsel's credit-side observation (C-counsel-3):** for a
   hired courier the amnesty (and the whole till-accountability model) sees only the **debt side** (cash owed
   back); a symmetric **out-of-band wage underpayment for the same epoch is invisible** to the system, so the
   re-arm review must weigh the credit side a machine cannot.
2. The hold/`collectedTotal` **bundles `delivery_fee`** (verified `orders.ts:499`), treated 100% as owner
   revenue with no courier-pay portion in code — if the business intends the courier to keep the fee, the
   ledger records them owing their own pay back, settled out-of-band by no mechanism. Honest, or the asymmetry
   at its sharpest? *(Factual half verified; intent is a business/labor call — still open for a hired courier.)*
Plus: approve flipping `COURIER_CASH_RECONCILIATION_ENABLED` (and running the per-tenant amnesty) only when the
owner shift-close UI ships.

## Migration (forward-only, additive, integer) — design-time, not built here

1. **Naming truth (surface-only — no physical RENAME).** `COMMENT ON COLUMN courier_payouts.total_earned IS
   '… surfaced as collectedTotal; NOT owed to the courier'`. Rename is a DTO+i18n+UI surface change including
   **`courier/me.ts:218`** (rev.2). The DEFINER fn `app_generate_settlements()` and `checkPayoutSums` are
   untouched.
2. **`courier_cash_ledger` UNTOUCHED** — hold-only; `UNIQUE(order_id,type)`, the FK cascade, and the
   `deliveryCompletion` hold insert stay exactly as-is. Add only a `prevent_ledger_mutation()` **BEFORE UPDATE
   only** trigger (content-immutability; no DELETE clause → cascade survives).
3. **New `courier_cash_contras`** `(id, order_id uuid REFERENCES orders(id) ON DELETE CASCADE, courier_id,
   location_id, type text CHECK(type IN ('settle','release')), amount integer CHECK(>=0), reconciliation_id
   uuid NULL REFERENCES courier_cash_reconciliations(id), reversal_id uuid NULL, created_at)` with a CHECK that
   exactly one ref matches the type (`settle`→reconciliation_id, `release`→reversal_id). Indexes:
   `UNIQUE (order_id, reconciliation_id) WHERE type='settle'`, `UNIQUE (order_id, reversal_id) WHERE
   type='release'`. `ENABLE + FORCE RLS`, tenant policy, grant-mirror, `prevent_contra_mutation()` BEFORE
   UPDATE only, and the **residual-guard BEFORE INSERT** trigger (reads the hold from `courier_cash_ledger`).
4. **New `courier_cash_reconciliations`** `(id, courier_id, location_id, owner_id, confirmed_total integer
   CHECK(>=0), order_count int, unmatched_cash integer NOT NULL DEFAULT 0 CHECK(>=0),
   unmatched_cash_resolved_at timestamptz NULL, amnesty_cutoff_ts timestamptz NULL, kind text CHECK(kind IN
   ('shift_close','opening_balance')) DEFAULT 'shift_close', status text CHECK(status IN
   ('reconciled','discrepancy')), created_at)` — `ENABLE + FORCE RLS`, tenant policy, grant-mirror. (`kind`
   and `unmatched_cash` are rev.2 additions; `amnesty_cutoff_ts` is the rev.3 U2 as-of cutoff (set on
   `opening_balance` rows only); `unmatched_cash_resolved_at` is the rev.3 C-counsel-2 owner-resolution
   affordance — the BEFORE-UPDATE guard permits the owner to set it, analog of the refunds' `resolved_at`.)
   The **`id` = the client-generated `Idempotency-Key`** on the owner path (rev.4 U1 — the rev.3 server-derived
   `uuid_v5(location∥courier∥shift∥confirmed_total)` branch is **deleted**; header required → HTTP 422 on
   absence); the row insert is `ON CONFLICT (id) DO NOTHING` so a replay returns the stored row and writes no new
   contras — even when the retry's `confirmed_total` differs (stored figure wins).
5. **New `courier_cash_owner_refunds`** `(id, order_id uuid REFERENCES orders(id) ON DELETE CASCADE,
   courier_id, location_id, amount integer CHECK(>=0), against_payout_id uuid NULL, reconciliation_id uuid
   NULL, refunded_at timestamptz, reason text, resolved_at timestamptz NULL, created_at)` — `ENABLE + FORCE
   RLS`, tenant policy, grant-mirror, BEFORE UPDATE guard limited to the immutable columns (owner may set
   `resolved_at`). This is the C2 storage for the refund-after-settle / immutable-payout owner loss.
6. **Owner-path DEFINER fn** for the pending-payout snapshot correction (`total_earned -= refund` while
   `status='pending'`) — B3-consistent (SECURITY DEFINER, pinned search_path, `GRANT EXECUTE TO dowiz_app`),
   so the owner path never raw-UPDATEs a member-keyed money table under NOBYPASSRLS.
7. **`courier_payouts` RLS:** no FORCE migration (already FORCE). Make the policy missing-GUC-tolerant
   (`current_setting('app.current_tenant', true)`) and set the GUC in the **owner reconciliation path** (not
   the cron). The system-sweep path stays closed by the DEFINER fn (B3).
8. **Opening-balance amnesty (rev.2 H4 + rev.4 U2 cutoff):** a one-time, per-tenant, idempotent backfill —
   capture `amnesty_cutoff_ts = now()` once, store it on the `kind='opening_balance'` reconciliation row, and in
   a **single `INSERT … SELECT` per courier (one MVCC snapshot)** insert a `settle` for each outstanding hold
   with no contra **AND `courier_cash_ledger.created_at <= amnesty_cutoff_ts`** (the hold's own write time — NOT
   the order's `created_at`, so a late delivery of an old order stays live). Holds with `created_at > cutoff`
   stay live. Runs at flag-enable, after §3-§7 land. Auditable; not "no backfill".
9. No `order_status` enum churn; no physical rename. `courier_cash_ledger`'s CHECK still nominally allows
   `release`/`settle` but they are never written there (contras live in `courier_cash_contras`) — harmless;
   left as-is to keep the spine migration untouched.

## DoD (red → green) — hardened post-re-attack

- `courier_payouts` UPDATE (approve/pay/dispute/reopen) **survives** the rename · **`app_generate_settlements()`
  runs end-to-end** and still writes `total_earned` (the DEFINER fn, not a phantom cron line — H1) · worked
  example nets zero in **all orderings** (before/after-settle, partial) ·
- **Partial settlement across TWO reconciliations nets zero** (order B: settle 900 in recon-1, settle 600 in
  recon-2 → two contra rows, no collision, no dropped settle) — the C1 regression · a retried reconciliation
  (same idempotency key) writes **no** duplicate settle (idempotent) ·
- **OWNER-path over-settle blocked (U1, single-branch rev.4):** an owner shift-close request with **no
  `Idempotency-Key` header → HTTP 422** (no server-derived fallback of any shape); the demo case (confirm
  900/1500, then a lost-response retry under the **same** key) settles **exactly 900 once** — the retry returns
  the stored result and writes **zero** new contras, **even if the retry carries a different `confirmed_total`**
  (stored figure echoed back) · **two same-amount closes in one shift under two DISTINCT keys → two
  reconciliations, BOTH settle** (no `ON CONFLICT` merge — the exact case an amount-derived server key would
  have silently dropped, C1 class) ·
- **capped-settle continuation (U3):** a shift-close larger than the batch cap resumes with
  `remaining_cap = confirmed_total − Σ(this-id settles)` recomputed per batch, converges to exactly
  `confirmed_total`, and a mid-continuation retry double-writes nothing ·
- release+settle past the hold + refund-after-full-settle **RAISE** (residual-guard) · partial refund reverses
  only the refunded amount ·
- **refund-after-settle records a `courier_cash_owner_refunds` row** (C2) and surfaces it to the owner; net 0,
  no phantom credit ·
- **surplus race** (`confirmed_total > Σ holds`) records `unmatched_cash`, does **not** `RAISE`, no negative
  obligation (H5) ·
- **amnesty**: on a tenant with N lifetime holds and zero settles, flag-enable → obligation = 0 (not Σ holds),
  with N auditable opening-balance settle rows (H4) · **first real reconciliation is bounded** (set-based +
  batch cap), not O(lifetime holds) (M1) ·
- **amnesty as-of cutoff (U2, rev.4 hold-keyed):** a hold whose **`courier_cash_ledger.created_at >
  amnesty_cutoff_ts`** (a delivery completing mid-amnesty) is **NOT** amnestied — it survives as a live
  obligation into the first real shift-close; a **late delivery of an OLD order** (order `created_at` pre-cutoff,
  hold written post-cutoff) also stays live (filter is the hold's own write time, not the order's) · a re-run of
  the amnesty is a **true no-op** (same opening-balance id + same stored `amnesty_cutoff_ts`) ·
- **opening_balance settles stay distinct in the owner UI (C-counsel-1):** any owner surface keys off
  `reconciliation.kind` and **never aggregates opening_balance into "cash received from courier"** ·
- owner reconciliation path + amnesty **don't self-DoS** under a set GUC; the DEFINER writer is unaffected by
  caller GUC (H2) · cascade delete still works on both ledger tables, direct UPDATE raises ·
- **no courier-facing surface** (`courier/me.ts` payouts list + summary, `EarningsPage`) frames
  `collectedTotal`/collected cash as "earned/owed to courier" (H3-me, DoD-5) ·
- miscount strands only the **delta** · `stage21-no-auto-deduct.invariant.test.ts` GREEN · penalty-typed writes
  still banned · two-ledger coherence holds.

Full checklist: `docs/design/stage21-reconciliation/proposal.md` §DoD + `.../reattack-resolution.md`.
