# Breaker findings — S5 MONEY batch → Rust port (R2b)

Target: `docs/design/s5-money-batch-rust-port/proposal.md` + `docs/adr/ADR-s5-money-batch-rust-port.md`.
Method: read-only source verification (assignments.rs, orders/pg.rs, deliveryCompletion.ts, payments-webhook.ts,
settlements.ts, settlement-cron.ts, migrations 043/045/046/051/052/078/083). No fixes proposed — cause + violated
invariant only. Round: R1 (initial attack).

Verdict headline: the proposal spends its "money red-line RESOLVE" on Flag-A with a **misdiagnosed mechanism** and a
**reconcile path that cannot fire**, while the actually-live concurrency defect on the same two rows (a lock-ordering
deadlock) and a cross-tenant settlement-generation self-poison go un-enumerated.

---

## CRITICAL

### C1 — B-CONSIST · Flag-A Fix-1's `409 ORDER_RACED_TERMINAL` cash-reconcile branch is UNREACHABLE for the customer_cancel race → courier-collected cash is silently unreconciled
The proposal (§7 Fix-1b, ADR Decision 2) makes the whole RESOLVE ask hinge on: raced deliver → `apply_transition`
returns `false` → emit `409` with a cash-reconcile signal ("loser reconciles cash explicitly"). **That branch has no
reachable code path for the customer_cancel scenario it is designed for.**

- Every `→CANCELLED` funnels through `apply_transition` (Q-ORDER-FUNNEL), whose terminalize fold in the SAME committed
  tx flips the assignment out of `picked_up`: `UPDATE courier_assignments SET status='cancelled' … WHERE order_id=$1
  AND status IN ('offered','assigned','accepted','picked_up')` (`pg.rs:800-813`).
- So the instant the order is observably `CANCELLED`, its assignment is already `cancelled`. The courier deliver's
  gate is `… WHERE ca.id=$1 AND ca.courier_id=$2 AND ca.status='picked_up' FOR UPDATE` (`assignments.rs:1091`) →
  0 rows → `DeliveredOutcome::NotFound` → **404**, returning at `assignments.rs:1100` long before the
  `apply_transition` bool at `1163-1164` is ever evaluated.

**Break scenario (cash path, crypto-independent, LIVE):** customer_cancel commits first (order CANCELLED + refund_due
recorded + assignment `cancelled`); courier who has physically collected cash taps "delivered" → gets a bare **404**,
never the promised `409 ORDER_RACED_TERMINAL`, never any reconcile signal. Result: business owes the customer a refund
AND the courier holds cash with **no `courier_cash_ledger` 'hold'** row — the exact "phantom / unaccounted cash"
outcome the fix claims to prevent, arriving through a different door. Fix-1's `bool=false` case (order terminal WHILE
assignment still `picked_up`) is not reachable given the funnel, so the branch is effectively dead code.
- **Violated invariant:** courier cash accountability — every collected cash amount has a ledger hold or an explicit
  reconcile record; "money stays coherent, courier cash becomes an explicit reconcile item" (proposal §7) is not
  delivered.

---

## HIGH

### H1 — B-FAIL/B-SCALE · Lock-ordering deadlock: courier deliver locks `ca→o`; customer_cancel locks `o→ca` (AB-BA). Proposal's "same discipline as assign/pickup" claim is false
Verified orderings on the same order row-pair:
- courier deliver (`assignments.rs:1091`): single JOIN statement `FROM courier_assignments ca JOIN orders o …
  FOR UPDATE` (no `OF` → locks BOTH); scan is driven by the filtered `ca` → **ca locked, then o**.
- customer_cancel (`pg.rs:674` then `786`/`803`): `SELECT … FROM orders … FOR UPDATE` (o) → `apply_transition` →
  `UPDATE courier_assignments …` (ca) → **o locked, then ca**.
- owner-proxy deliver (`dashboard.ts:470→483`) and assign/pickup (`dashboard.ts:230→255`, `389→400`) all lock
  **o→ca**. So the proposal's §7 assertion that the courier deliver uses "the same discipline assign-courier/pickup
  already use" is factually wrong — courier deliver is the ONE path that locks `ca` first.

**Break scenario:** during the pickup→deliver window (exactly when customer_cancel is designed to fire) a courier
deliver and a customer_cancel hit the same order: T1(deliver) holds `ca`, waits `o`; T2(cancel) holds `o`, waits `ca`
→ Postgres `40P01` → one tx aborts → mapped to **503 transient** (proposal §7 TRANSIENT_PG). Fix-1(a) adds a
redundant order lock but does NOT reorder, so the deadlock persists. Two downstream harms:
1. If cancel wins the deadlock, deliver's 503-retry hits the `ca.status='picked_up'` gate now `cancelled` → **404**
   (feeds directly into C1: no reconcile).
2. The DoD "concurrency probe" (§9 / ADR Verification) asserts "exactly one of {DELIVERED+hold,
   CANCELLED+refund_due+409}" — but one arm will frequently observe `40P01`→503, not the asserted 409. A green run is
   therefore non-representative / flaky, so the probe does not actually prove the invariant it gates.
- **Violated invariant:** NO_AUTO_DEGRADE (a money mutation returning a spurious 503 on a benign, self-inflicted lock
  collision) + the DoD's own verification soundness.

### H2 — B-CONSIST · Fix-1 diagnosis is wrong (order row is ALREADY locked); the real footgun (unconditional post-transition writes) is left partially gated
The proposal/reliability-gate premise is "deliver reads `SELECT status FROM orders` **without a row lock** → TOCTOU
window." Not true: the order row is locked from `assignments.rs:1091` (JOIN `FOR UPDATE`, no `OF`, locks `orders`);
the later `SELECT status::text FROM orders WHERE id=$1` (`1157`, no `FOR UPDATE`) reads an already-locked row. **Fix-1(a)
"take FOR UPDATE at the top to close the window" is a no-op — the window is already closed.** The genuine defect is
that the money/terminalize writes after the `if assert_transition(...).is_ok() { apply_transition }` block
(`1163-1164`) run **unconditionally**: assignment terminalize (`1128`), shift free (`1143`), `payment_outcome`
(`1175`), `delivery_trace` (paid_full), `courier_cash_ledger` 'hold'. Today this is safe only by *emergent
interaction* (JOIN locks o + funnel terminalizes ca + `picked_up` JOIN guard), not by local logic. Fix-1(b) enumerates
gating only "cash-hold / delivery_trace paid_full / payment_outcome" — it does **not** gate the assignment terminalize
(`1128`) or shift-free (`1143`), which execute before the bool is even known.
- **Break scenario:** if the `bool=false` case ever becomes reachable (e.g. a future non-`apply_transition` writer of
  `orders.status='CANCELLED'`, or a refactor of `1091` to `FOR UPDATE OF ca`), Fix-1 as specified still writes
  `courier_assignments.status='delivered'` + frees the shift on a CANCELLED order, and a retry hits
  `ca.status='picked_up'`→404 — so even the "fixed" path yields an assignment/order-status incoherence and a
  non-idempotent 409. The RESOLVE is asked to ratify a fix whose stated mechanism (a) is inert and whose gate scope (b)
  is incomplete.
- **Violated invariant:** assignment↔order-status coherence; a fix must be justified by the actual failure mechanism.

### H3 — B-CONSIST/B-DATA · Settlement `regenerate` + daily cron self-poison the ENTIRE all-tenant sweep via the payout-immutability trigger; not "exactly-once by construction"
`app_generate_settlements` (one SECURITY DEFINER tx over ALL locations, `mig 078:161-192`) upserts the payout keeping
prior status (`ON CONFLICT … DO UPDATE SET status = courier_payouts.status`, `078:170`) but then bumps
**unconditionally**: `UPDATE courier_payouts SET deliveries_count = …+…, total_earned = total_earned + v_added_total
WHERE id=v_payout.id` (`078:189`) with **no status guard**. The immutability trigger `prevent_payout_mutation`
(`mig 052:6-16`) raises `payout immutable after approval` whenever `OLD.status IN ('approved','paid')` and
`total_earned` changes → the whole function tx aborts.

**Break scenario (reachable, normal ops — worse under `SETTLEMENT_PERIOD='weekly'`):** owner approves courier X's
period-P payout; courier X delivers one more cash order in period P (approve route has no period-close guard,
`settlements.ts:121-126`); next daily cron / manual regenerate re-selects P, finds the new `NOT EXISTS(settlement_items)`
assignment, inserts the item, then bumps `total_earned` on the `approved` payout → trigger RAISE → **entire sweep for
every location/courier aborts** (`settlement-cron.ts:42-45` re-throws; manual route returns 500). It keeps failing on
every subsequent run while that one dangling assignment exists → couriers at *other* tenants get no settlements
generated. The proposal §6 calls settlements "exactly-once by construction" and R3 dismisses regenerate as a benign
"low-frequency manual op"; neither the design nor the DoD has a settlement-concurrency probe.
- **Violated invariant:** tenant blast-radius isolation ("each route owns one tx; single-payout blast radius, never a
  batch", §7) — one tenant's normal approve-then-deliver DoSes global settlement generation.

---

## MEDIUM

### M1 — B-SCALE/B-OPS · R7 `statement_timeout` precedent is mis-cited; the affected row-lock paths have NO Node timeout to "port at parity"
Proposal §2/§7/R7 says the Rust row-lock txs must set a per-tx `statement_timeout` "equivalent (Node uses 4500ms,
`orders.ts:124`)" as "Node parity." But `SET LOCAL statement_timeout = 4500` is only on the **order-CREATE** path
(`orders.ts:124`). The owner-proxy deliver/assign/pickup (`dashboard.ts:227/386/467`) run `BEGIN` with **no**
statement_timeout; grep of `dashboard.ts` finds none. So on the exact paths R7 worries about, "parity with Node" = no
timeout. The mitigation must EXCEED parity, contradicting the parity contract framing.
- **Break/number:** Rust operational pool = 20 (`db.rs:105`). A wedged `FOR UPDATE` on deliver (lock wait, no
  timeout — PG default `statement_timeout=0`) pins 1/20 unbounded; ~20 concurrent wedged money mutations exhaust the
  pool → new `POST /orders` gets 503. This is the same pool-wedge mode the create path explicitly guards and the
  deliver path does not.
- **Violated invariant:** bounded row-lock hold / pool non-exhaustion.

### M2 — B-FAIL · NO_AUTO_DEGRADE is contradicted by the settlements list catch-all `{payouts:[]}` — a byte-parity port preserves a silent money-read degrade
`GET /settlements` wraps the query in `try { … } catch { return { payouts: [] } }` (`settlements.ts:46-71`): an RLS
500, a `decryptPII` throw, or a pool timeout all return an **empty list**, indistinguishable from "no settlements."
The parity contract (port at byte-parity) preserves this. §7 asserts S5 "must never degrade silently."
- **Break scenario:** under NOBYPASSRLS a missing tenant seat throws on `courier_payouts` (H-adjacent, see M4); the owner
  sees zero payouts, assumes nothing to pay, and couriers go unpaid — no error surfaced, health shows green.
- **Violated invariant:** NO_AUTO_DEGRADE for money reads (empty-vs-error is a payout-visibility integrity issue).
  Parity-vs-invariant conflict the proposal does not flag.

### M3 — B-SEC · `GET /settlements/:id` ships the courier `full_name_encrypted` ciphertext in the response body
`SELECT p.*, c.full_name_encrypted FROM courier_payouts p JOIN couriers c …` and `return { payout:
payoutRes.rows[0], items }` (`settlements.ts:82-106`) — `payout.full_name_encrypted` is emitted as a raw ciphertext
field, while the list route decrypts+masks (`charAt(0)+'***'`, `settlements.ts:51-55`). A byte-parity port reproduces
the ciphertext egress, directly contradicting §7 ("never leak ciphertext") and §8 ("mask parity, widen NOTHING").
Also `SELECT p.*` returns internal `approved_by_owner_id` and makes the Rust typed-row port body-drift-fragile (a new
`courier_payouts` column auto-appears in Node `p.*` but not in the typed Rust struct → silent body divergence).
- **Violated invariant:** PII data-minimization / no ciphertext egress.

### M4 — B-DATA/B-CONSIST · #77 cast taxonomy (§5) mis-covers the money-minor WRITE binds and the text-vs-enum columns on the webhook/settlement paths
Verified column types: `payment_events.amount_minor` / `settlement_items.amount` / `courier_payouts.total_earned` are
`integer` (**int4**, and `amount_minor` is NULLABLE) — `mig 083:56`, `045:10`, `043:12`. `payments.status`,
`orders.payment_status`, `payment_events.type` are **text+CHECK**, NOT enums (`mig 083:30,20,55`), whereas
`orders.status`/`orders.payment_outcome` ARE enums (cast `::order_status`/`::payment_outcome`).
- §5 lists `amount_minor`/`total_earned` under `::bigint` **reads** but the webhook BINDS `amount_minor` on the
  insert-select (`payments-webhook.ts:46-54`, `$3`) — binding a Rust `i64`/`Lek` into an int4 column is the exact
  sqlx encode-mismatch (#77) class on the WRITE side, uncovered by the read-side rule; and nullable→must be `Option`.
- §5's "enum binds cast `::enumtype`" invites over-casting `payment_status`/`payments.status`/`payment_events.type`
  (text) to nonexistent enums, or under-casting `orders.status` — both #77 landmines the taxonomy lumps together
  without per-column disambiguation.
- **Break scenario:** the `#[ignore]` live-PG probe (or a real crypto event) 500s on the amount_minor int4 bind /
  a mistaken `::payment_event_type` cast. Crypto is dark (flag off) so latent, but it is exactly the cutover bug class
  the ratchet exists to catch — the proposal's own cast guidance does not name it.
- **Violated invariant:** integer-money bind/decode parity (ledger #77).

### M5 — B-CONSIST · Webhook out-of-order `failed`→`completed` leaves `payments.status='paid'` but `orders.payment_status='failed'` — §6's "monotonic" claim is half-true
Guards: completed flips payments `WHERE status NOT IN ('refunded','paid')` (`payments-webhook.ts:60-62`) — admits
`'failed'` → paid+captured. But the orders flip is `WHERE payment_status IN ('pending','authorized')`
(`payments-webhook.ts:66-68`) — a prior `failed` event set `orders.payment_status='failed'`, not in the set → skipped.
- **Break scenario:** Plisio sends `pending→expired(failed)`, then a late `completed` (or webhooks arrive
  out-of-order). Result: money arrived (`payments.status='paid'`) but the order reads `payment_status='failed'` → the
  held prepaid order is never offered to fulfillment, and the refund_due fold requires `o.status IN
  ('CANCELLED','REJECTED')` (`payments-webhook.ts:82-83`) so no obligation is recorded → **funds in limbo**. §6 claims
  webhook writes are "monotonic"; they are monotonic per-table but the two tables diverge. Crypto dark → MED not
  CRITICAL, and the port preserves it verbatim.
- **Violated invariant:** payment-state coherence across `payments` and `orders`.

### M6 — B-SEC/B-OPS · `POST /settlements/regenerate` ignores the path `:locationId` and triggers an all-tenant sweep; `referenceDate` unvalidated
`worker.handleGenerate(new Date(referenceDate))` runs for ALL locations regardless of `:locationId`
(`settlements.ts:308-316`, comment "Technically processes all locations"); body `referenceDate` is `z.string()` →
`new Date(x)` yields `Invalid Date` on garbage, propagated into period-boundary math. An owner of one tenant triggers
a cross-tenant settlement recompute (noisy-neighbor / cross-tenant WRITE trigger), rate-limited only 5/5min. Proposal
R3 treats regenerate as a benign local op and does not note the all-tenant fan-out or the missing date validation.
- **Violated invariant:** per-tenant scoping of a money-mutation trigger; validate-at-edge.

---

## LOW

### L1 — B-CONSIST · Owner-proxy deliver enum omits `delivered_prepaid` (4 values, `dashboard.ts:456`) while the courier path supports 5 (`assignments.rs:1102-1105`). The "single completion primitive, no fork" claim (§7) already has a value-set fork at the edge; a crypto-prepaid order cannot be owner-proxy-delivered. Parity trap for the port (match the narrower owner enum). Crypto dark.

### L2 — B-CONSIST · `POST /api/orders/:id/messages` has no idempotency key (`order-messages.ts:32`); a client network-retry inserts a duplicate message. Not money, but §6 enumerates idempotency for every other write and omits messages.

### L3 — B-SEC · reveal-contact is detective-only. Audit-before-return (R9) is ALREADY satisfied in Node — the `customer_contact_reveals` insert is inside the `withTenant` tx and commits before the plaintext is returned (`reveal-contact.ts:33-55,69-74`) — so R9 as a "risk to fix" is largely redundant. The real gap: the only preventive control is `rateLimit: 10/min` (`reveal-contact.ts:17`) ≈ 600 full name+phone reveals/hour; a valid-but-revoked ≤24h owner token (ADR-0004) can bulk-harvest customer PII, and the audit records but does not prevent it. §8 calls the audit "load-bearing" — it is a detective, not a preventive, control; the proposal has no enumeration/harvest rate model.

---

## Regression note (for RE-ATTACK)
No prior breaker round on this proposal (R1). On revision, re-check specifically: (i) whether Fix-1 restructures the
courier deliver to lock `o` before `ca` (H1) or the fix stays a redundant lock; (ii) whether the raced-cash reconcile
path is reachable given the terminalize fold flips `ca` out of `picked_up` (C1); (iii) whether the DoD concurrency
probe asserts a specific status per arm (405/409/503) rather than "not both", so a `40P01`→503 can't pass as green;
(iv) whether regenerate/cron guards the `total_earned` bump against `status IN ('approved','paid')` (H3).
