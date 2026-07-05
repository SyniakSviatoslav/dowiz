# Re-attack Resolution — Stage-21 Cash Reconciliation (B1)

Round-2 Breaker re-attack against `ADR-stage21-reconciliation.md` (Rev.1) + `proposal.md`, verified against
the live tree. Design-time only — **NO production code, NO migrations built here.** Each finding →
disposition (**FIX** / **ACCEPT-RISK** / **DEFER-FLAG** / **NEEDS-HUMAN**) with the precise resolution.

Legend anchors (live tree, re-verified this round):
`packages/db/migrations/1790000000028_courier-cash-ledger.ts:19` (`UNIQUE(order_id,type)`),
`1790000000078_phase2-sweep-fns.ts:160-197` (`app_generate_settlements()` SECURITY DEFINER; `total_earned`
write `:189`), `apps/api/src/workers/settlement-cron.ts:44` (delegates via `SELECT app_generate_settlements`),
`apps/api/src/routes/courier/me.ts:218` (`total_earned AS amount` → courier),
`apps/api/src/lib/deliveryCompletion.ts:120-121` (`hold` insert `ON CONFLICT (order_id,type)`).

---

## C1 [CRITICAL] — Contra multiplicity structurally impossible under `UNIQUE(order_id,type)` → **FIX (separate contra table; keep spine untouched)**

**Verified:** `028:19 UNIQUE(order_id,type)` allows at most one `settle` per order. Partial settlement across
two shift-closes needs a second `settle` on the same order → collides; `ON CONFLICT DO NOTHING` would silently
drop it → permanent open hold. Rev.1's residual-guard math assumed many contra rows accumulate per order —
false.

**Why not the task's "drop the UNIQUE in-place" option:** `deliveryCompletion.ts:121` writes the hold with
`ON CONFLICT (order_id, type) DO NOTHING`, which requires that exact `UNIQUE(order_id,type)` index for
inference. Dropping it forces a change to the hold insert — violating the **"cash spine (deliveryCompletion
hold-in-tx) untouched"** holding constraint. Postgres ON CONFLICT inference needs the exact index columns, so
no in-place index reshape keeps that insert byte-identical while allowing multiple settles. The two constraints
are only jointly satisfiable by moving contras out of the ledger table.

**Chosen invariant (precise):**
- `courier_cash_ledger` stays **hold-only and literally untouched** (UNIQUE, FK cascade, hold insert
  unchanged); it now means one hold per order.
- New **`courier_cash_contras`** holds `settle`/`release`. A **settle** row is uniquely identified by
  `(order_id, reconciliation_id)` (partial unique `WHERE type='settle'`); a **release** by
  `(order_id, reversal_id)` (partial unique `WHERE type='release'`). **Multiple settle rows per order** →
  partial settlement across successive shift-closes is now structural.
- **Double-settle guard on retried jobs:** `reconciliation_id` is minted deterministically from an idempotency
  key (owner idempotency-key header or pg-boss job id); a retry reuses the same id → partial-unique
  `ON CONFLICT (order_id, reconciliation_id) WHERE type='settle' DO NOTHING` no-ops the retry (same for
  `release` via `reversal_id`).
- Residual-guard `BEFORE INSERT` trigger on `courier_cash_contras`: raise if
  `NEW.amount > hold_amount − Σ existing contras`; contra path `SELECT … FOR UPDATE`s the hold row →
  serialization. Net-negative / over-reversal / double-settle structurally impossible.

**ADR:** Decision §3, Migration §2-§3.

---

## C2 [CRITICAL] — Owner-refund fact has no storage → **FIX (dedicated `courier_cash_owner_refunds` table)**

**Verified:** `courier_cash_reconciliations CHECK status IN ('reconciled','discrepancy')` cannot hold a
refund-after-settle write-off; Rev.1 named an "owner-refund fact" with no home.

**Resolution:** new **`courier_cash_owner_refunds`** `(id, order_id, courier_id, location_id, amount,
against_payout_id NULL, reconciliation_id NULL, refunded_at, reason, resolved_at NULL, created_at)`,
`ENABLE + FORCE RLS` + tenant policy + grant-mirror + BEFORE-UPDATE immutability guard (owner may set only
`resolved_at`). **Writer:** the (future) refund path, **in the refund tx**, when residual == 0 (already
settled) or the payout is `approved`/`paid` (immutable) — i.e. exactly when no `release` contra can be written.
**Surfaces to the owner:** an "owner refunds to review" list (unresolved rows) in owner/settlements + a payout
review flag; owner acknowledges → `resolved_at`. Net 0, courier not credited, owner loss recorded not eaten.

**ADR:** Decision §5, Migration §5.

---

## H1 [HIGH] — Stale ground truth: `total_earned` writer is the DEFINER fn, not the cron file → **FIX (re-ground all citations)**

**Verified:** `settlement-cron.ts` is 51 lines and delegates via `SELECT app_generate_settlements($1,$2)`
(`:44`); the `total_earned = total_earned + v_added_total` write lives in `app_generate_settlements()`
SECURITY DEFINER at `1790000000078:189` (fn `:160-197`). Rev.1's `settlement-cron.ts:95-105/:103` citations
were fabricated line numbers.

**Resolution:** every reference re-grounded to the DEFINER fn (Context bullet 1; Decision §2, §6; Migration §1).
The DoD "cron survives the change" now targets **`app_generate_settlements()` runs end-to-end and still writes
`total_earned`** — trivially true under surface-only rename (the fn body is untouched).

**ADR:** Context, Decision §2/§6, DoD.

---

## H2 [HIGH] — NOBYPASSRLS fix inert (write runs inside DEFINER fn on a bypass role) → **FIX (re-spec: DEFINER-fn-as-gateway is the closure; GUC re-scoped to owner path)**

**Verified:** `app_generate_settlements()` is SECURITY DEFINER → executes as the fn owner (bypass role);
setting `app.current_tenant` in the cron caller has zero effect on the write inside it.

**Resolution — what NOBYPASSRLS-readiness actually means:**
- **System-sweep (cron) path:** the DEFINER fn **is** the B3 closure (per the `1790000000078` header):
  post-B3, `dowiz_app` (NOBYPASSRLS) cannot touch `courier_payouts`/`settlement_items` directly — only via
  this audited fn (search_path pinned `:161`; `REVOKE ALL FROM PUBLIC` + `GRANT EXECUTE TO dowiz_app`
  `:196-197`). Readiness requirement: the fn owner is a dedicated non-login RLS-bypass role, not the app login
  role. **Strike** Rev.1's "set GUC in `settlement-cron.ts` closes it."
- **Owner reconciliation path:** settle/contra writes + the pending-payout snapshot correction run as
  `dowiz_app` with the owner's **member** identity → post-B3 they ARE RLS-subject. Set
  `app.current_tenant = location_id` in the owner path (owner is a member → `app_member_location_ids()` passes)
  and make the `courier_payouts` policy missing-GUC-tolerant. The `total_earned -= refund` correction (member-
  keyed money table) routes through a small **owner-path DEFINER fn** (B3-consistent), not a raw member UPDATE.

**ADR:** Decision §6, Migration §6-§7.

---

## H3-me [HIGH] — `courier/me.ts:218` serves `total_earned AS amount` to the courier as earnings, dropped from the reader list → **FIX (add it)**

**Verified:** `courier/me.ts:218` `SELECT id, total_earned AS amount, …` in the courier payouts list; the
adjacent summary block `:227-234` presents `SUM(cash_amount)` today/week/month as courier "earnings" — the same
inversion on the courier's own surface.

**Resolution:** add `courier/me.ts:218` to the surface-only rename reader list (map to the honest key), and
reframe the `:227-234` summary off "earnings" language (cash collected / to reconcile). DoD-5 assertion now
covers the courier surfaces (`courier/me.ts` + `EarningsPage`), not only the owner DTO.

**ADR:** Context bullet 2, Decision §2, Migration §1, DoD.

---

## H4 [HIGH] — Flag-flip on tenants with history surfaces Σ(lifetime holds) as open obligation → **FIX (auditable opening-balance amnesty — replaces "no backfill")**

**Verified:** every `paid_full` delivery since mig 028 wrote a `hold`, zero settles ever written. A naïve
flag-flip computes obligation = Σ(all holds), including periods already paid out.

**Resolution:** before the flag flips per tenant, a one-time **opening-balance amnesty**: for each outstanding
hold with no contra, insert a `settle` contra referencing a synthetic `courier_cash_reconciliations` row of
`kind='opening_balance'` (amount = full residual). Pre-reconciliation obligation is treated as already-settled
out-of-band — **recorded and auditable**, not silently zeroed and not an epoch-skip. Order: schema/triggers/
indexes → amnesty → flag flip. Idempotent (`ON CONFLICT` on the opening-balance reconciliation per
courier+location). Rev.1's "No backfill" (proposal §5.6) is **struck**.

**ADR:** Decision §7, Migration §4 (`kind` column), §8.

---

## H5 [HIGH] — `confirmed_total > Σ(visible holds)` race (delivery lands mid-count) → **FIX (record unmatched-cash fact, never RAISE)**

**Verified:** Rev.1 only specced the shortfall side; a surplus (a hold written after the owner started
counting, or a confirmed amount exceeding visible residuals) had no defined behavior.

**Resolution:** settle **all** visible holds fully; record the excess `confirmed_total − Σ visible holds` as
**`unmatched_cash`** (new column) on the reconciliation row — a fact for owner review (most likely a
not-yet-written hold or a next-period order). **Never `RAISE`, never a negative obligation, never an
auto-credit.** Symmetric to the shortfall/discrepancy path; both honor NO-AUTO-DEDUCT (extended to
no-auto-credit).

**ADR:** Decision §4, Migration §4, carried invariant NO-AUTO-DEDUCT, DoD.

---

## M1 [MED] — First reconciliation is O(lifetime holds) N+1 in one tx → **FIX (amnesty removes the scan; set-based + batch cap)**

**Verified:** Rev.1's per-order `FOR UPDATE` + INSERT loop over all outstanding holds is N+1 and unbounded on
first run.

**Resolution:** (1) the H4 amnesty means the first real reconciliation only sees the current shift's holds, not
lifetime; (2) the settle path is **set-based** — a single `INSERT … SELECT` over a windowed cumulative sum of
residuals oldest-first, capped at `confirmed_total`, avoiding the per-row loop; (3) a per-tx **batch cap**
(e.g. 500 orders) with idempotent continuation for any pathological large set. Residual-guard still fires
per-row (BEFORE INSERT), which is fine; hold rows are `SELECT … FOR UPDATE`d as a batch for race-safety.

**ADR:** Decision §7 (amnesty), DoD ("first real reconciliation is bounded").

---

## Disposition summary

| Finding | Disposition |
|---------|-------------|
| C1 contra multiplicity vs `UNIQUE(order_id,type)` | **FIX** — separate `courier_cash_contras` table; spine untouched; precise per-event idempotency keys |
| C2 owner-refund fact has no storage | **FIX** — new `courier_cash_owner_refunds` table; refund-tx writer; owner review surface |
| H1 stale writer citation | **FIX** — re-grounded to `app_generate_settlements()` DEFINER fn (`078:189`) |
| H2 NOBYPASSRLS placebo | **FIX** — DEFINER-fn-as-gateway = closure for cron; GUC re-scoped to owner path + owner-path DEFINER fn |
| H3-me courier surface dropped | **FIX** — added `courier/me.ts:218` (+ summary) to reader list & DoD-5 |
| H4 rollout amnesty | **FIX** — auditable opening-balance settle-per-hold; replaces "no backfill" |
| H5 surplus race | **FIX** — `unmatched_cash` fact, no RAISE, no auto-credit |
| M1 first-reconcile N+1 | **FIX** — amnesty + set-based settle + batch cap/continuation |

## Remaining NEEDS-HUMAN

1. **RK-2 earnings/wage asymmetry** — NEEDS-HUMAN. *Discharged for launch* (owner/family-only segment, operator
   2026-06-29, `ethical-decisions.md`) with a recorded SEGMENT CONSTRAINT that **re-arms before the first
   hired/non-owner courier** (then requires at minimum read-only expected-pay-before-accept). Not
   architect-closable.
2. **Delivery-fee intent (Counsel §5)** — NEEDS-HUMAN. Factual half verified (the hold bundles `delivery_fee`,
   treated 100% owner revenue, no courier-pay portion in code, `orders.ts:499`); whether recording the courier
   as owing back money that includes their own fee is honest is a business/labor call — open for a hired
   courier.
3. **Flag-flip + per-tenant amnesty approval** — human gate: flip `COURIER_CASH_RECONCILIATION_ENABLED` and run
   the opening-balance amnesty only when the owner shift-close UI ships.
4. **LATENT-STOP-2** (courier-visible own cash-ledger/contra view) — stays pre-registered; fires if any
   courier-visible debt view ships without a courier-visible own-record.

## Honest residuals (design-time, carried)

- The **refund caller still does not exist**: residual-guard blocks over-reversal structurally, but the
  `courier_cash_owner_refunds` write + pending-snapshot correction are a **contract** the future refund path
  must honor with tests.
- Already-paid snapshots can't be back-corrected — a refund against an `approved`/`paid` period records a
  forward owner-refund fact + review flag; the historical figure stays as-paid by design.
- Multi-currency remains deferred under the single-currency invariant.
- **B3 NOBYPASSRLS** is an external dependency; Stage-21 makes the owner path GUC-ready and relies on the
  DEFINER fn for the system-sweep path, but does not itself land B3.

---

## Round-3 convergence turn — 2026-07-02

Round-2 Breaker verdict: architecture **sound**, 8/8 Rev.2 findings **closed**, **not yet converged** on
exactly three under-specifications. Counsel: **SATISFIED-WITH-CONDITIONS**. All three pinned below → **FIX**;
counsel's binding conditions folded. ADR status → **COUNCIL-CONVERGED (pending breaker ack R3) — operator
ratification required before build (money red-line)**.

### U1 [HIGH] — Deterministic `reconciliation_id` for the OWNER path not mandated → **FIX (mandate the idempotency contract)**

**Verified:** the owner shift-close is a **synchronous HTTP tap — no pg-boss job id exists on that path**, and
Rev.2's "owner idempotency-key header OR pg-boss job id" left the header optional. **Failure demo:** owner
confirms 900 of a 1500 shift → settles orders 1–3 under a fresh uuid; the HTTP response is lost; the owner
re-taps and the server mints a **new** uuid → the settle now sees residual 600 on orders 4–5 and settles them
→ **1500 settled for 900 physically received**. The residual-guard stays **silent** because it fires per-order
and the retry targets *different* residual orders.

**Resolution — MANDATED (no optional path):** the owner path derives `reconciliation_id` one of two ways:
- **server-side deterministic** — `uuid_v5(stage21_namespace, location_id ∥ courier_id ∥
  shift_id-or-confirmed-window ∥ confirmed_total)`; a re-tap of the same confirmed figure for the same shift
  resolves to the **same** id; **OR**
- a **required `Idempotency-Key` header → HTTP 422 on absence** (no silent fresh-uuid fallback).

**Uniqueness:** one `courier_cash_reconciliations` row per id. **Replay:** same id → **return the stored
reconciliation result verbatim, write ZERO new contras** — the reconciliation-row insert is
`ON CONFLICT (id) DO NOTHING` + `SELECT` of the stored row on conflict; the settle
`INSERT … ON CONFLICT (order_id, reconciliation_id) WHERE type='settle' DO NOTHING` no-ops. `release` keeps its
natural reversal id. **ADR:** Decision §3, §4, Migration §4, DoD.

### U2 [MED-HIGH] — Amnesty as-of cutoff unbounded (holds arrive during the run) → **FIX (created_at <= amnesty_cutoff_ts)**

**Verified:** `deliveryCompletion` is **not** flag-gated → holds keep landing while the amnesty sweep runs; a
naïve "settle every outstanding hold" would amnesty live obligations created after the sweep started.

**Resolution:** capture `amnesty_cutoff_ts` **once** at amnesty start, **store it on the
`kind='opening_balance'` reconciliation row**; amnesty settles **only holds with `order created_at <=
amnesty_cutoff_ts`**. Holds after the cutoff are **live obligations** for the first real shift-close — never
amnestied. Makes the amnesty deterministic and re-runnable (same opening-balance id → same hold set →
`ON CONFLICT` no-op). **ADR:** Decision §7, Migration §4 (`amnesty_cutoff_ts` col), §8.

### U3 [LOW-MED] — Batch/cap continuation arithmetic implicit → **FIX (recompute remaining_cap per batch inside tx)**

**Verified:** for a capped shift-close settle resuming across batches, resuming from the raw `confirmed_total`
would re-settle already-written amounts.

**Resolution:** each continuation batch recomputes, **inside the tx**,
`remaining_cap = confirmed_total − Σ(courier_cash_contras.amount WHERE reconciliation_id = THIS id AND
type='settle')`; the windowed cumulative sum is capped at `remaining_cap`; converges when `remaining_cap = 0`
or no residual holds remain. Deterministic id (U1) makes a mid-continuation retry idempotent. **ADR:**
Decision §4, DoD.

### Counsel binding conditions folded (SATISFIED-WITH-CONDITIONS)

- **C-counsel-1** — opening_balance settles must stay **semantically distinct** in any owner UI (keyed off
  `reconciliation.kind`), **never** aggregated into "cash received from courier". → **DoD** item added.
- **C-counsel-2** — `unmatched_cash` needs an **owner-resolution affordance** (analog of the refunds'
  `resolved_at`). → added `unmatched_cash_resolved_at` column + registered as a **NAMED** item on the deferred
  **RK-1** discrepancy-resolution council (not silent). ADR Migration §4, Consequences.
- **C-counsel-3** — **historical credit-side blindness:** amnesty forgives only the **debt side the system can
  see** (Σ visible holds); for a **hired** courier the **credit side** (possible out-of-band wage
  *under*payment for the same epoch) is invisible. → recorded as part of the **RK-2 re-arm** trigger for the
  first non-owner courier. ADR NEEDS-HUMAN §1, Consequences.

### Convergence disposition

| Item | Disposition |
|------|-------------|
| U1 owner-path deterministic `reconciliation_id` | **FIX** — mandated: deterministic `uuid_v5` OR required `Idempotency-Key` (422 on absence); replay returns stored result, zero new contras |
| U2 amnesty as-of cutoff | **FIX** — `amnesty_cutoff_ts` captured once, stored on opening-balance row; `created_at <= cutoff` only |
| U3 capped-settle continuation | **FIX** — `remaining_cap = confirmed_total − Σ(this-id settles)` recomputed per batch in-tx |
| C-counsel-1 opening_balance UI distinct | **FIX** — DoD assertion (key off `kind`, never aggregate) |
| C-counsel-2 unmatched_cash resolution | **FIX** — `unmatched_cash_resolved_at` + NAMED RK-1 council item |
| C-counsel-3 credit-side blindness | **RECORDED** — RK-2 re-arm trigger (NEEDS-HUMAN, not architect-closable) |

---

## Round-4 resolution — 2026-07-02 (idempotency single-branch)

Round-3 Breaker ack: architecture **sound**, U2/U3 pinned correctly, but the U1 idempotency contract carried a
**residual [CRITICAL]** in its *first* (server-key) branch, and the U2 cutoff was keyed on the wrong timestamp.
Both re-keyed below → **FIX**; U3 **CLOSED**, no change. ADR status → **COUNCIL-CONVERGED (R4, pending final
breaker ack) — operator ratification required before build (money red-line)**.

### U1 [CRITICAL] — Server-side amount-derived `reconciliation_id` branch collides on same-amount closes → **FIX (delete the branch; client `Idempotency-Key` is the ONLY contract)**

**Verified residual:** Rev.3 mandated `reconciliation_id` via **either** a server-side
`uuid_v5(location ∥ courier ∥ shift ∥ confirmed_total)` **or** a required `Idempotency-Key` header. The
server-key branch is itself a **C1-class defect**: **two legitimate same-amount closes in one shift** (common
with round cash amounts) derive the **same** id → the settle's `ON CONFLICT (order_id, reconciliation_id) WHERE
type='settle' DO NOTHING` **silently drops the second settle** → a permanently open hold. The whole redesign
exists to kill exactly that ON-CONFLICT-DO-NOTHING silent drop; the amount-derived key reintroduced it on the
key-derivation side.

**Resolution — MANDATED (single branch, no server fallback):**
- The **client-generated `Idempotency-Key` header is the ONLY contract.** The rev.3 server-derived branch is
  **struck** from Decision §3, §4 and Migration §4.
- The key identifies a shift-close **GESTURE**: the client mints a fresh UUID for each **deliberate** close and
  reuses it **only on a transport retry** of that gesture. `reconciliation_id` = that key.
- **HTTP 422** when the header is absent (no server-derived fallback of any shape).
- **Replay** (same key) returns the stored reconciliation result **verbatim** and writes **ZERO** new contras —
  **including when the retry carries a different `confirmed_total`**: the **stored result wins**, and the owner
  sees the stored figure echoed back (a changed figure is a **new gesture** → **new key**).
- **Two same-amount closes → two distinct keys → two reconciliations, both settle** (no merge, no dropped
  settle).

**DoD update:** the over-settle assertion now covers **both** directions — (a) two same-amount closes with
distinct keys → two reconciliations, both settle; (b) same key → replay, zero new contras (even under a differing
`confirmed_total`); (c) absent header → HTTP 422.

**ADR:** Decision §3, §4, Migration §4, DoD; Revision-4 changelog.

### U2 [MED] — Amnesty cutoff keyed on the order's `created_at`, not the hold's → **FIX (filter on `courier_cash_ledger.created_at`)**

**Verified residual:** Rev.3's cutoff filtered `order created_at <= amnesty_cutoff_ts`. A **late delivery of an
old order** (order predates the cutoff, but `deliveryCompletion` writes its hold **after** the sweep) would be
mis-classified as pre-cutoff and **amnestied away** — silently forgiving a live obligation.

**Resolution:** the cutoff filter keys on the **HOLD row's own write time** — `courier_cash_ledger.created_at <=
amnesty_cutoff_ts`. The amnesty runs as a **single `INSERT … SELECT` per courier (one MVCC snapshot)**; a
re-run reuses the **same opening-balance reconciliation id AND the same stored `amnesty_cutoff_ts`**, so
late-arriving holds (`created_at > cutoff`) are never amnestied and the re-run is a **true no-op**. Holds after
the cutoff are live obligations for the first real shift-close.

**ADR:** Decision §7, Migration §8, DoD.

### U3 [LOW-MED] — Capped-settle continuation → **CLOSED (no change)**

No change needed. Its safety **inherits the now-single-branch key stability**: the deterministic id the
continuation arithmetic relies on is the client `Idempotency-Key`, stable across a gesture's retries by
construction (no server re-derivation that could shift under a changed figure). The `remaining_cap` recompute
per batch is unchanged.

### Round-4 disposition

| Item | Disposition |
|------|-------------|
| U1 server-key branch collides on same-amount closes | **FIX** — delete server branch; client `Idempotency-Key` only (422 on absence); replay stored-wins even on differing figure; two keys → two settles |
| U2 amnesty cutoff on order vs hold `created_at` | **FIX** — key on `courier_cash_ledger.created_at`; single `INSERT…SELECT`/courier; late-old-order stays live; re-run true no-op |
| U3 capped-settle continuation | **CLOSED** — no change; inherits single-branch key stability |
