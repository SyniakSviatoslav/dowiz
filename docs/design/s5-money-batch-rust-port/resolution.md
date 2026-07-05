# Resolution — S5 MONEY batch → Rust port (R2b), RESOLVE rounds R1 + R2

Status: RESOLVED (design-time; **R2 applied 2026-07-05** — the R1 fix survived Ламача R2 largely intact on intent but
had **five concrete gaps in the mechanism**, all validated against source and fixed below; see `## RESOLVE round R2`).
Author: System Architect.
Inputs: `proposal.md` (this dir), R1 `breaker-findings.md` (C1, H1–H3, M1–M6, L1–L3), R1 `counsel-opinion.md`
(ETHICAL-STOP-1/2, epistemic notes); **R2 Ламач inline (N1–N5), R2 Counsel (STOP-1/2 SATISFIED; OPEN-1/OPEN-2; §7b M5
forward-gate).**
Method: every finding re-verified against live source before adjudication (no disposition on the breaker's word alone).
Disposition vocabulary: **FIX** (proposal.md updated with concrete design) · **ACCEPT-RISK** (justified, owner named) ·
**DEFER-FLAG** (MISSING — owner + trigger to close) · **REVISE** (STOP → design changed) · **HUMAN** (needs a
recorded human/operator decision).

Headline: the breaker is **right on the load-bearing mechanism** — my Flag-A diagnosis was wrong. The order row is
already locked (H2); the promised `409 ORDER_RACED_TERMINAL` cannot fire (C1); and the real live defect on those two
rows is an **AB-BA lock-ordering deadlock** (H1) I mis-described as "the same discipline as assign/pickup." The fix is
**re-architected**, not patched: (1) unify lock ordering **o→ca** everywhere, (2) distinguish *raced-terminal* from
*not-found* with a status-free second read, (3) write a **durable** cash-truth record + owner alert in the same tx
(satisfies STOP-1), (4) gate **all** post-transition writes structurally by branching on the locked order status —
not by capturing a bool after the fact. Two consequences the proposal denied: this batch is **no longer
zero-migration** (STOP-1 durability + H3 self-poison both need operator-placed forward-only migrations), and
**promotions ships as a Node keep-set, not a lying stub** (STOP-2).

---

## Verification ledger (source-confirmed before adjudication)

| finding | claim | verified at | verdict |
|---|---|---|---|
| C1 | 409 branch unreachable — terminalize fold flips `ca` out of `picked_up`, gate 404s first | `assignments.rs:1091,1097-1099`; `pg.rs:800-813` | **TRUE** |
| H1 | AB-BA: deliver `ca→o`, cancel/owner-proxy `o→ca` | `assignments.rs:1091` (JOIN `FOR UPDATE`, no `OF`); `pg.rs:674,803`; `dashboard.ts:470,483` | **TRUE** |
| H2 | order already locked (Fix-1a no-op); terminalize(1127)+shift-free(1142) run before the bool | `assignments.rs:1091,1127,1142,1156-1170`; `pg.rs:774,796,878` | **TRUE** |
| H3 | settlement-gen self-poisons all-tenant sweep via immutability trigger | mig 078:160-197 (unguarded bump L189); mig 052:6-16 (RAISE on `approved`/`paid`) | **TRUE** |
| M1 | R7 `statement_timeout` mis-cited — deliver/assign/pickup have none in Node | `dashboard.ts:447-536` (no `SET LOCAL statement_timeout`); create-only at `orders.ts:124` | **TRUE** |
| M2 | `GET /settlements` catch-all `{payouts:[]}` = silent money-read degrade | `settlements.ts:69-71` | **TRUE** |
| M3 | `GET /settlements/:id` ships `full_name_encrypted` ciphertext + `p.*` | `settlements.ts:83,106` | **TRUE** |
| M4 | write-side bind of int4 money / text-vs-enum cols uncovered by §5's read rule | webhook `$3=amount_minor`,`$2=type` (mig 083:56,55 = int4/text) | **TRUE** |
| M5 | webhook out-of-order `failed→completed` diverges `payments.status=paid` vs `orders.payment_status=failed` | `payments-webhook.ts:60-62,66-68,82-83` | **TRUE** |
| M6 | regenerate ignores `:locationId` (all-tenant sweep); `referenceDate` unvalidated | `settlements.ts:314` (`z.string()`, comment "processes all locations") | **TRUE** |
| L1 | owner-proxy deliver enum 4 values (no `delivered_prepaid`) vs courier 5 | `dashboard.ts:454`; `assignments.rs:1101-1105` | **TRUE** |
| L2 | `POST .../messages` no idempotency key | `order-messages.ts:32` | **TRUE (accepted)** |
| L3 | reveal-contact audit-before-return already correct; gap is preventive control | `reveal-contact.ts:33-55,69-74`; STOP absent | **TRUE (R9 downgraded)** |

Every breaker claim reproduced against HEAD. No finding dismissed.

---

## CRITICAL

### C1 + ETHICAL-STOP-1 (adjudicated together) — **REVISE + FIX** → owner: **council + operator (money red-line)**

**Accepted in full.** The proposal's `409 ORDER_RACED_TERMINAL` is dead code for the customer_cancel race, and
Fix-1(b)'s "write NO 'hold'" was the *mirror* injustice Counsel named: cash physically in the courier's hand → order
raced CANCELLED → deliver returns a bare **404** → **no ledger row at all** → courier personally liable for cash the
system never saw. Both the phantom-paid narrative (original defect) and the erased-cash narrative (my fix) punish the
courier; the durable truth is "collected on a raced-terminal order → reconcile," and it must survive the courier
closing the app.

**New design (replaces §7 Fix-1 entirely; lands in the shared Rust completion primitive both the live courier deliver
and the new owner-proxy deliver call — the fork is closed for real, not aspirationally).**

1. **Reorder to o→ca (this is the H1 fix — see below).** Resolve `order_id` from the assignment with a *non-locking*
   read, then `SELECT … FROM orders WHERE id=$order_id FOR UPDATE` (lock the order first), then lock the assignment.
   The proposal's Fix-1(a) "take FOR UPDATE at the top" is a **no-op** (H2 — the row is already locked by the JOIN);
   the load-bearing move is *reordering*, not *adding*.

2. **Distinguish raced-terminal from not-found (the C1 fix).** After locking, read the assignment *without a status
   gate*:
   - assignment row absent / wrong courier → genuine **404** `ASSIGNMENT_NOT_FOUND_OR_NOT_PICKED_UP` (parity with today).
   - assignment present **and** (`order.status` terminal **or** `ca.status` terminal) → **raced-terminal**.
   - assignment present, `order.status=IN_DELIVERY`, `ca.status='picked_up'` → normal completion (byte-identical happy
     path).

3. **Durable cash-truth on raced-terminal (the STOP-1 fix).** When the courier reported cash collected
   (`outcome.is_paid_full()` with a concrete `cash_amount`) and the order raced terminal:
   - write **one durable row in the same tx** — a `courier_cash_ledger` entry of a **new type `'reconcile'`**
     (`INSERT … (courier_id, location_id, order_id, 'reconcile', cash_amount) ON CONFLICT (order_id,type) DO NOTHING`).
     This is settlement-safe: `app_generate_settlements` selects only `courier_assignments.status='delivered'`
     (mig 078:166,176) — a `'reconcile'` ledger row on a `cancelled` assignment is **never** swept into a payout.
   - **owner alert** (the §21 alert-friction mechanism, PII-free / claim-check intact): publish
     `COURIER_CASH_RECONCILE_DUE {orderId, courierId, locationId, amount, reason:'raced_terminal'}` **post-commit**
     (durability lives in the ledger row, notification is best-effort — the alert failing never erases the truth; this
     is precisely what "ephemeral-409-only doesn't satisfy" demanded).
   - return a **distinct `409 ORDER_RACED_TERMINAL`** carrying `{amount, reconcileId}` so the courier UI renders a
     *human instruction*, not a red error (Counsel advice #1 — coupled FE requirement, owner: FE/S6).
   - No `delivery_trace` paid_full, no `payment_outcome='paid_full'`, no assignment→`delivered`. The order stays
     CANCELLED with its `refund_due` (customer protected); the cash becomes an explicit, durable reconcile obligation
     (courier protected). Narrative is **true**, not false-delivered and not erased.

**Migration consequence (honest):** `courier_cash_ledger.type` CHECK is `('hold','release','settle')` with
`UNIQUE(order_id,type)` (mig 028:16,19). Adding `'reconcile'` is a forward-only, atomic
`ALTER TABLE … DROP CONSTRAINT … ADD CONSTRAINT … CHECK (type IN ('hold','release','settle','reconcile'))` — **red-line,
operator-placed** (drafted **M-A** below). The batch is **no longer zero-migration**; the non-goal is amended.

**Reachability check (does the reconcile write re-open H1?):** No. In the raced case the cancel tx has already
committed and released its locks, so the deliver tx acquires the order lock uncontended, reads the `cancelled`
assignment, and writes the `'reconcile'` row + order-scoped alert — all under the order lock, in o→ca order. The
ledger insert touches neither the `orders` nor the `courier_assignments` row-lock graph in a way that inverts
ordering. No new AB-BA. (Verified against the lock graph, not asserted.)

---

## HIGH

### H1 — Lock-ordering deadlock — **FIX** → owner: **port author (under R1 council gate)**

**Accepted.** My §7 claim ("the same discipline assign-courier/pickup already use") is false — courier deliver is the
one path that locks `ca` first. Fix: the shared completion primitive locks **o→ca**, matching `customer_cancel`
(`pg.rs:674→803`) and owner-proxy deliver (`dashboard.ts:470→483`).

Design (two statements, boring, deadlock-free — no clever single-JOIN lock):

```sql
-- (1) resolve order_id WITHOUT locking ca (plain read); absent → genuine 404
SELECT order_id FROM courier_assignments WHERE id = $1 AND courier_id = $2;
-- (2) lock the ORDER first (o), read the money-authoritative fields
SELECT status::text, total::bigint, payment_status, payment_method::text
  FROM orders WHERE id = $order_id FOR UPDATE;
-- (3) lock the ASSIGNMENT (ca) second, status-free (feeds the C1 raced-terminal branch)
SELECT id, shift_id, status FROM courier_assignments WHERE id = $1 AND courier_id = $2 FOR UPDATE;
```

Consequence for the DoD probe: with o→ca unified, the two-task race resolves **deterministically** to
{DELIVERED+hold} or {CANCELLED+refund_due+409+reconcile} — the `40P01→503` arm the breaker flagged (which made the
"not both" assertion non-representative) **no longer occurs**. The concurrency probe is therefore rewritten to assert
a **specific status per arm** (see H1(2)/regression-note iii): deliver-wins → 200+`hold`; cancel-wins → deliver gets
**409 `ORDER_RACED_TERMINAL`** + a `'reconcile'` row + a `refund_due` row; **never** a `503` and **never** both
money outcomes. A `503`/`40P01` observed in the probe is now a **red** (regression), not an accepted arm.

### H2 — Fix diagnosis wrong; writes gated incompletely — **FIX** → owner: **port author**

**Accepted.** Epistemic note #1 is upheld: "add a lock" is inert; the substance is (b) gating. The re-architecture
gates **all** post-transition writes **structurally**: the assignment terminalize, shift-free, `payment_outcome`,
`delivery_trace`, and `courier_cash_ledger 'hold'` writes now live **inside the happy branch**, entered only after the
locked order is confirmed `IN_DELIVERY` and `ca` is `picked_up`. There is no longer any write that runs before the
outcome is known. `apply_transition`'s returned bool is retained as a **defense-in-depth assert** (under the order
lock it must be `true`; `false` ⇒ an impossible interleave ⇒ abort the tx as a logged 500, never proceed) — the
opposite of today's discard at `1164`. This also removes H2's forward-hazard: a future non-funnel writer of
`orders.status='CANCELLED'` can no longer produce a delivered-assignment-on-cancelled-order, because the branch keys
on the *locked, freshly-read* order status, not on emergent JOIN-guard interaction.

### H3 — Settlement regenerate/cron self-poisons the all-tenant sweep — **DEFER-FLAG (operator-placed migration draft)** → owner: **operator + settlement-worker owner**

**Accepted; this is a pre-existing production defect the port inherits, not one it introduces** — the daily
`SettlementCronWorker` triggers `app_generate_settlements` regardless of this batch. But my proposal (§6
"exactly-once by construction", R3 "benign low-frequency op") mis-characterized it. Corrections:

- §6 is **scoped**: the settlement *lifecycle transitions* (approve/pay/dispute/reopen) are exactly-once by
  construction (status-guarded UPDATEs, `settlements.ts:124/180/224/275`). The settlement *generation* sweep is **not**
  — it is the self-poisoning path.
- Root fix (**M-B**, drafted below, red-line operator-placed): guard the `total_earned`/`deliveries_count` bump so it
  never mutates an `approved`/`paid` payout — either `UPDATE … WHERE id=v_payout.id AND status='pending'`, or skip the
  courier/period when `v_payout.status <> 'pending'` (a period-close guard). Restores the "single-payout blast radius"
  invariant.
- Port-side, **until M-B lands**: route 13 (`regenerate`) is **kept Node-served / not exposed on Rust** (strangler
  non-zero keep-set), OR mounted **fail-loud** (503 `SETTLEMENT_GEN_NOT_PORTED`) — never a benign Rust re-trigger of a
  poisoned sweep, and never inlined settlement math (Counsel advice #4). Cron ownership stays with the settlement
  worker; the fix is theirs to place with M-B.
- A **settlement-generation concurrency probe** is added to the DoD (approve payout P → deliver one more cash order in
  P → run generate → assert **no RAISE, other tenants unaffected**). Red today, green after M-B.

---

## MEDIUM

### M1 — `statement_timeout` mis-cited; parity = no timeout — **FIX (conscious parity DEVIATION)** → owner: **port author**

**Accepted.** Node deliver/assign/pickup run `BEGIN` with **no** `statement_timeout` (`dashboard.ts:447-536`); only
create sets `4500ms` (`orders.ts:124`). So "port at parity" = no bound. R7 is re-framed: the Rust row-lock money txs
set a per-tx `statement_timeout` (~4500ms) as a **deliberate deviation from Node parity** — justification: pool
non-exhaustion (a wedged `FOR UPDATE` pinning 1/20 → 20 wedged = create gets 503) outranks byte-parity on an
operational bound the client never observes. Logged in the **conscious-deviation ledger** (proposal §7a).

### M2 — settlements-list catch-all `{payouts:[]}` — **FIX (conscious parity DEVIATION)** → owner: **port author**

**Accepted; parity-vs-invariant conflict I failed to flag.** Porting `try{…}catch{return {payouts:[]}}`
(`settlements.ts:69-71`) byte-for-byte would preserve a **silent money-read degrade** (an RLS-seat miss / decrypt
throw / pool timeout indistinguishable from "no payouts" → couriers unpaid, health green) — the exact NO_AUTO_DEGRADE
violation §7 forbids for money. Deviation: the Rust `GET /settlements` does **not** swallow to empty; a query failure
surfaces the real class (503 transient / 500 internal, correlation-id logged). "No payouts" (0 rows) and "couldn't
read payouts" (error) become distinct. Deviation is deliberate: money-read integrity > byte-parity. Logged in §7a.

### M3 — ciphertext egress on `GET /settlements/:id` — **FIX (conscious parity/security DEVIATION)** → owner: **port author**

**Accepted.** `SELECT p.*, c.full_name_encrypted … return {payout: rows[0]}` (`settlements.ts:83,106`) emits raw
`full_name_encrypted` ciphertext **and** internal `approved_by_owner_id`, contradicting §7/§8. A byte-parity port
would reproduce the leak *and* be body-drift-fragile (a new `courier_payouts` column auto-appears in `p.*`). Deviation:
the Rust `:id` route uses a **typed, explicit-column SELECT** (no `p.*`), **decrypts+masks** the courier name to
`charAt(0)+'***'` (identical to the list route), and **never** emits ciphertext or `approved_by_owner_id`. This is a
**conscious body-shape divergence from Node** — the one place the batch breaks byte-parity **on purpose**. Security /
data-minimization > byte-parity, and the parity probe for this route asserts the *corrected* shape (ciphertext-absent),
documented as an intentional exception, not a probe failure. Logged in §7a. (This is the security>parity DEVIATION the
RESOLVE task asked to be sent as an explicit deliberate departure.)

### M4 — #77 cast taxonomy mis-covers write-side binds + text-vs-enum columns — **FIX** → owner: **port author**

**Accepted.** §5's read-only rule is replaced by a **per-column cast table** distinguishing three families (verified
column types: mig 083:56,55,30,20; 045:10; 043:12):

| column(s) | pg type | bind (write) | read | note |
|---|---|---|---|---|
| `payment_events.amount_minor` | int4, **NULLABLE** | `i32` / `Option<i32>` (or explicit `$n::int4`) | `Option<i64>` cast `::bigint` | webhook **binds** it (`$3`) — encode-side #77; must be `Option` |
| `settlement_items.amount`, `courier_payouts.total_earned`, `deliveries_count` | int4 | `i32`/`Option<i32>` | `::bigint`→`i64` | write-side binds on regenerate/generate |
| `payments.status`, `orders.payment_status`, `payment_events.type` | **text + CHECK** | bind as `&str`/`String`, **NO** enum cast | read as `String`, no cast | over-casting these to a nonexistent enum is itself a #77 landmine |
| `orders.status`, `orders.payment_outcome`, `orders.payment_method` | true enum | cast `::order_status` / `::payment_outcome` / `::payment_method` on bind | cast `::text` on read | already the live-path convention (assignments.rs:1089,1175) |

The `#[ignore]` live-PG suite gains **write-side (bind) cases** on `amount_minor` (int4 + nullable) and the text-vs-enum
columns — not just read-side decode. This closes the WRITE half of ledger #77 the read rule missed.

### M5 — webhook out-of-order `failed→completed` divergence — **DEFER-FLAG (S8/payments)** → owner: **payments/S8 owner**

**Accepted; latent (crypto dark).** A late `completed` after a `failed` flips `payments.status='paid'`
(`WHERE status NOT IN ('refunded','paid')` admits `'failed'`, line 60-62) but **not** `orders.payment_status`
(`WHERE payment_status IN ('pending','authorized')` skips `'failed'`, line 66-68), and the refund_due fold needs
`o.status IN ('CANCELLED','REJECTED')` (line 82-83) → funds in limbo. §6's "monotonic" claim is **per-table, not
cross-table** — corrected in the proposal. Disposition: the port **preserves the current behavior verbatim** (parity —
crypto is OFF, widening this now expands scope onto a dark surface) but records it as a **known coherence gap** with a
noted fix for when crypto lights (S8): widen the orders/refund_due arm to also re-drive from `'failed'`
(`payment_status IN ('pending','authorized','failed')`), so a genuine late `completed` reconciles the order too. This
is a **conscious carry** (§7b), not a silent one — the parity probe asserts today's (divergent) behavior so the port
doesn't accidentally "fix" it into a different divergence, and S8 owns the real fix. Not fixed in this parity port.

### M6 — regenerate all-tenant + unvalidated `referenceDate` — **FIX (edge) + DEFER-FLAG (scope)** → owner: **port author (edge) / operator (scope, with M-B)**

**Accepted (couples to H3).** Two parts:
- **Edge validation — FIX now (no migration):** `referenceDate` becomes `z.string().datetime()` (reject non-ISO →
  400 at the edge), and the parsed date is asserted valid before entering period math (never propagate `Invalid Date`).
  This is a **conscious parity deviation** (Node accepts garbage `z.string()`) justified by validate-at-edge — logged
  §7a.
- **All-tenant scope — DEFER-FLAG:** the per-location scoping (a `p_location_id`-parameterized generate variant, so an
  owner's regenerate cannot fan out cross-tenant) rides **M-B** (same DEFINER function surface). Until then route 13 is
  Node-kept / fail-loud per H3. Owner: operator + settlement-worker owner.

---

## LOW

### L1 — owner-proxy deliver enum omits `delivered_prepaid` — **ACCEPT-RISK** → owner: **port author**

The owner-proxy edge enum is 4 values (`dashboard.ts:454`), courier 5 (`assignments.rs:1101-1105`); a crypto-prepaid
order cannot be owner-proxy-delivered. Accepted: match the **narrower owner enum** for parity (crypto dark → inert).
The "single completion primitive, no fork" claim is softened in §7 to "single primitive, **value-set narrowed at the
owner edge** (crypto dark)" — the primitive is shared; the *edge validation* differs, honestly. Revisit when crypto
lights (with M5/S8).

### L2 — `POST .../messages` no idempotency key — **DEFER-FLAG** → owner: **port author (post-port)**

Accepted: not money; a client network-retry duplicates a message. §6 is corrected to acknowledge messages are the one
write without an idempotency guard. Defer a client-supplied `Idempotency-Key` (or a `(order_id, sender, hash, minute)`
dedup) to a post-port ticket — not a red-line, not gating the batch. Owner: port author.

### L3 — reveal-contact detective-only / harvest rate — **ACCEPT (R9 downgraded) + DEFER-FLAG (preventive)** → owner: **security (post-port)**

Accepted, and Counsel's epistemic note upheld: audit-before-return is **already** correct in Node
(`reveal-contact.ts:33-55,69-74` — INSERT inside `withTenant`, commits before plaintext), so **R9 is downgraded from
"risk to fix" to "invariant to preserve"** (the port must keep audit-in-tx-before-return; widen nothing). The genuine
gap — the only preventive control is `rateLimit:10/min` ≈ 600 reveals/hr, and a valid-but-revoked ≤24h owner token
(ADR-0004) can bulk-harvest PII — is a **detective-not-preventive** weakness the parity port correctly carries
forward. Defer a post-port ticket (mandatory `reason` + harvest-anomaly alert + tighter rate) — **not** touched in
this byte-parity port (a behavior change against the parity contract). Owner: security. (This is also Counsel advice
#2 — same ticket.)

---

## ETHICAL-STOPs

### 🔴 STOP-1 — Cash-in-hand must not be *erased* — **REVISE (design changed)** → recorded human/money decision: **operator ratifies the reconcile obligation as a money red-line**

Resolved by the C1 re-architecture above: raced-terminal cash is written as a **durable** `courier_cash_ledger
'reconcile'` row **in the same tx** (source of truth) + a **post-commit owner alert** (notification) + a **distinct
409** the courier UI renders as a human instruction. The durable truth **does not depend on the ephemeral 409** — if
the courier loses the response, closes the app, or the phone dies, the ledger row and the owner's reconcile queue still
carry it. Narrative is true ("collected on raced-terminal → reconcile"), neither false-delivered nor erased. STOP-1 is
**lifted** on this design. **Recorded decision for the operator:** ratify "whoever commits order-status first wins;
the loser's physical cash becomes a durable, owner-visible reconcile obligation" as the money rule, and place migration
**M-A**. This is the one money-red-line human sign-off the round routes to a person.
**[SUPERSEDED BY R2 N3 — see `## RESOLVE round R2`]:** "owner-visible" is refined to **"durable + auditable +
courier-instructed-live NOW"** — the ledger is audit-only (mig 028) and the alert is a no-op seam (S6), so the
owner-proactive surface (alert transport + reconcile-queue read) is a **gated S6/owner-FE follow-up** the operator also
ratifies (two-phase surfacing). The durable row still ships in this batch.

### 🟠 STOP-2 (conditional) — promotions façade must not *assert* emptiness — **REVISE (no lying stub)** → owner: **System Architect (R4 already accepts the keep-set)**

Resolved by **not mounting the affirmative-empty stub at all.** Counsel's simplest STOP-lifting path is taken:
promotions stays on the **Node keep-set** (strangler permits a non-zero keep-set; R4 already accepts it), with a
front-door **hard-guard** that promotions/S3 pricing routes **physically cannot be flipped to Rust** until the real
port. If any Rust mount exists for routing completeness it is **fail-loud** (`503 PROMOTIONS_NOT_PORTED`), **never**
`{promotions:[]}` — a mis-flip fails **loudly**, not by silently hiding a tenant's live promotions (which would be a
data-hiding dark-pattern on a pricing-affecting surface, and a NO_AUTO_DEGRADE violation). Option C1 (POTEMKIN) is
**withdrawn**; the decision becomes **C1′ (Node-keep + fail-loud guard, no affirmative-empty)**. Un-lightability is
provable (front-door guard) → STOP-2 lifted. Proposal §3/§4 updated.

---

## Counsel non-blocking advice — dispositions

1. **409 as human instruction (dignity, coupled to STOP-1)** — ADOPTED as a coupled FE requirement: the raced-terminal
   409 renders as a calm action state ("Order was cancelled while you delivered. You collected X. [Return to customer]
   / [Hand in for reconcile]"), not a red ERROR. Owner: FE/S6. Tracked with the C1 design.
2. **reveal `reason` mandatory (forward-looking)** — ADOPTED as the L3 post-port ticket (same ticket). Not in this
   parity port. Owner: security/product.
3. **Option B tripwire by race-count** — ADOPTED into the ADR: Option B (settlement/money-state consolidation) is
   recorded as a **planned post-cutover consolidation with a race-count tripwire** — "on the 3rd cross-surface money
   race, consolidate under its own ADR" — not "maybe if a second consumer appears." Fix-1 is the patch; Option B is the
   vaccine; sequence is deliberate. Owner: System Architect (revisit trigger recorded).
4. **Route 13 stays thin** — ADOPTED (H3 disposition: enqueue-or-Node-keep, never inline settlement math).
5. **Bless settlements duplication** — ACCEPTED (parity > DRY on the money line is a virtue here).

## Counsel open question (§5) — physical-world witness — **HUMAN (operator/product)**

Recorded, not resolved here: the batch optimizes **DB-state coherence** (no phantom delivered+paid); it cannot
adjudicate **what physically happened at the door** — both witnesses (courier + customer) stand there and the system
picks one machine narrative. The raced-cash case (customer cancelled; courier arrives with food + cash) is left
physically unscripted. The reconcile obligation (STOP-1) at least stops the DB from *lying*, but a *dignified* long-run
design would let the two people **jointly attest** (handed-over / cash-taken) rather than receive a verdict from a DB
that saw neither. Flagged to operator/product as a **forward-looking framing** (not a requirement of this port): keep
"parity/coherence" from masquerading as "the DB knows what happened." Owner: product. No design change in this batch.

## Epistemic notes — carried into the proposal

1. **"Fix = lock" is false.** The row is already locked (H2). The load-bearing fix is **reorder (o→ca)** + **gate all
   writes structurally** + **durable reconcile** — not "add a lock." The proposal's mechanism framing is corrected so
   no implementer under-invests in gating believing a lock closed it.
2. **"Parity = safety" is false.** Parity preserves known behavior — good invariants **and** latent weaknesses. The
   proposal now carries an explicit split: **§7a conscious DEVIATIONS from parity** (M1 timeout, M2 no-empty-degrade,
   M3 no-ciphertext, M6 date-validation — each justified security/reliability > byte-parity) and **§7b conscious
   CARRIES** (M5 webhook out-of-order → S8, L2 message idempotency, L3 reveal preventive-control → post-port). "Preserve
   an invariant" (mandatory) is now distinguished, in writing, from "preserve an incidental weakness" (separate ticket).

---

## Migration drafts required (both red-line — operator-placed, forward-only, atomic; NOT written by the port)

The batch is **no longer zero-migration.** Two operator-placed drafts, each with its own boot-guard + `#[ignore]`
live-PG proof, staged on staging-DB before boot:

- **M-A (STOP-1 durability + R2 OPEN-2 semantics):** `ALTER TABLE courier_cash_ledger DROP CONSTRAINT <type_check>, ADD
  CONSTRAINT … CHECK (type IN ('hold','release','settle','reconcile'))`. Forward-only, additive to the CHECK domain, no
  data backfill. Enables the durable reconcile row. **Carries an obligation-sum guardrail comment (R2 OPEN-2):**
  `'reconcile'` is owner-mediated audit, EXCLUDED from any future Σhold cash-cycle obligation; the future settlement
  integration must scope hold-sum to `type='hold'` (never `type <> x`). Mirror the comment onto mig 028. Owner: operator
  (placement) + settlement-worker owner (semantics).
- **M-B (H3 self-poison + M6 scope + R2 N4 spillover) — REDESIGNED:** `CREATE OR REPLACE FUNCTION
  app_generate_settlements(...)` with (i) the `total_earned`/`deliveries_count` bump status-guarded (never mutate an
  `approved`/`paid` payout); (ii) an optional `p_location_id` for per-location scoping (M6); and (iii) — **the R2 N4
  addition, non-negotiable** — a **spillover destination** so a late in-period earning whose natural-period payout is
  immutable is routed to a **mutable (`pending`) payout** (recommended: supplemental payout via a `generation_seq`
  discriminator on the payout unique key; alternative: carry-forward to the current pending payout), preserving
  `total_earned == Σitems` per payout — **never a bare guard that silently underpays**. Owner: settlement-worker owner
  (destination design) + operator (placement).

Neither is inlined by the route port. Both are drafts for the operator's `packages/db/migrations/` placement (085–089
precedent). Their absence **blocks** the raced-reconcile path (M-A) and the regenerate/cron safety (M-B) — so until
placed, the courier deliver reconcile branch and route 13 are gated (fail-loud / Node-kept) rather than shipped
half-working.

---

## RESOLVE round R2 (Ламач N1–N5 + Counsel OPEN-1/OPEN-2 + §7b M5 forward-gate)

Headline: R1's re-architecture was **right on intent, wrong/incomplete on five mechanisms**. Every N re-verified against
HEAD before adjudication; **none dismissed**, and N1 turned out **broader than filed** (accept + the offer/assign paths
also lock `ca→o`, not just pickup/cancel/abort). Two findings (N3, N4) sharpened once the source was read: the
`courier_cash_ledger` is **audit-only** (mig 028:3-7 — nothing reads or sums it), and the `payout_sums` invariant
(`smoke-checks.ts:178-182`) makes a bare M-B guard a provable underpayment.

### Verification ledger R2 (source-confirmed before adjudication)

| finding | claim | verified at | verdict |
|---|---|---|---|
| N1 | reorder done only for deliver; pickup/cancel/abort (**and accept**) still `ca→o`; concept-ledger "o→ca everywhere" false; partial reorder ADDS a deliver-vs-cancel deadlock | `assignments.rs:934/969` (accept ca-lock→`advance_order`), `1050-1067` (pickup), `1226-1244`+`1364` (cancel→`release_binding_and_reoffer`), `1264-1277` (abort); `pg.rs:674→686` (customer_cancel o→ca), `pg.rs:576` (owner_order_action o→ca) | **TRUE (broader — accept too)** |
| N2 | raced-terminal predicate too broad — DELIVERED is terminal; a successful-deliver retry → phantom `'reconcile'` (UNIQUE(order_id,type) doesn't block a different type) + false alert + misleading 409; today = clean 404 | `assignments.rs:1091` (`status='picked_up'` gate → 404 on replay today); mig 028:19 (`UNIQUE(order_id,type)`) | **TRUE** |
| N3 | STOP-1 "owner-visible" unenforced at cutover — alert is a no-op seam, no owner reader of `type='reconcile'` | `pg.rs:876` (`let _lifecycle = fx.lifecycle_event;` no-op); mig 028:3-7 (audit-only, `'release'/'settle'` reserved-never-written, nothing sums it) | **TRUE (sharper — audit-only)** |
| N4 | M-B silently underpays: (a) guard-bump → item lands, no bump → `Σitems>total_earned`; (b) skip → never-settled | mig 078:178 (`NOT EXISTS settlement_item` re-selects forever), 078:181 (item INSERT), 078:189 (unguarded bump); `smoke-checks.ts:178-182` (`total_earned==Σitems` invariant) | **TRUE** |
| N5 | reconcile not observably-idempotent — `ON CONFLICT DO NOTHING` → None reconcileId + duplicate alert on retry | proposal §7 move 3 (`DO NOTHING`, no RETURNING/SELECT-on-conflict); webhook §6 `rowcount=1` pattern is the correct model | **TRUE** |

### N1 [HIGH] — Global lock-order unification NOT done — **FIX (redesign)** → owner: **port author (under R1 council gate)**

**Accepted, and it is worse than filed.** The complete set of courier paths that lock BOTH rows and today lock
`ca→o`: **accept** (`assignments.rs:934,969`), **pickup** (`1050`), **deliver** (`1088`), **cancel** (`1226`), **abort**
(`1264`). The `o→ca` reference side is `customer_cancel` (`pg.rs:674`) and `owner_order_action`/mark-no-show (`pg.rs:576`,
via `apply_transition`'s UPDATE-orders-then-terminalize-ca). Reordering **only deliver** (R1) (i) leaves the AB-BA for
accept/pickup/cancel/abort vs the owner/customer paths, and (ii) **creates a NEW pair** deliver(`o→ca`) vs
cancel/abort/pickup/accept(`ca→o`) — before R1 all courier paths were `ca→o`, mutually consistent. The fix is a
**discipline: always lock `orders` before `courier_assignments`, on every path** — reorder all five courier paths to
resolve `order_id` via a non-locking read, `SELECT … FROM orders … FOR UPDATE` first, then the assignment. `proposal.md`
§7 move 1 now carries the complete path map and the DoD becomes a **deadlock MATRIX** {accept,pickup,deliver,cancel,abort}
× {customer_cancel,owner_order_action} asserting **never 40P01/503** — the deliver-only reorder would go RED on the
deliver×{cancel,abort} cells, which is precisely the regression the matrix catches. The concept-ledger claim is corrected
from an aspiration to a proven discipline.

### N2 [HIGH] — Raced-terminal predicate too broad — **FIX** → owner: **port author**

**Accepted.** `DELIVERED` (order) and `'delivered'` (ca) are terminal, so R1's "terminal OR ca-terminal → raced-terminal"
turns a **network retry of a successful cash-deliver** into a phantom `'reconcile'` row (the prior success wrote a
`'hold'` — a different `type`, so `UNIQUE(order_id,type)` does NOT block the reconcile), a **false owner alert**, and a
**misleading 409** for an order the courier genuinely delivered — where today the `status='picked_up'` gate returns a
clean idempotent 404. Fix (`proposal.md` §7 move 2): a **narrow, state-specific** predicate — `ca.status='delivered'` →
idempotent **200 echo** (write nothing / alert nothing — a conscious deviation §7a, strictly safer than today's 404, and
consistent with N5); raced-terminal fires **only** on a NON-delivered terminal (`order ∈ {CANCELLED,REJECTED}` or
`ca ∈ {cancelled,rejected}`); happy path unchanged; else 404. DELIVERED-by-self can never be misread as a race.

### N3 [MED] — STOP-1 "owner-visible" unenforced at cutover — **FIX (honest downgrade + prod-flip gate) + OPEN-2 semantics** → owner: **S6 / owner-FE + settlement-worker owner + operator**

**Accepted, and Counsel R2 (OPEN-1) names the same surface.** Source: `courier_cash_ledger` is **audit-only**
(mig 028:3-7 — `'release'/'settle'` reserved-but-never-written; owner cash figures come from
`courier_assignments.cash_amount` + `settlement_items`; **nothing sums the ledger**), and the alert transport is a
**no-op seam until S6** (`pg.rs:876`). So the durable `'reconcile'` row is **real, auditable, and money-conserving** (on a
raced CASH order there is no `refund_due` — the fold at `pg.rs:817-826` is inert for cash — so the reconcile row is the
*sole* record of the cash the courier holds), and the courier is **instructed live** by the 409 body, but the OWNER is
**not proactively surfaced**. Dispositions:
- **Ship the durable row now** (strictly better than the R1-inherited erasure; do not block the truth on the surface).
- **Honestly downgrade the STOP-1 language** from "owner-visible" to "durable + auditable + courier-instructed-live";
  the **owner-proactive surface** (real alert transport + an owner **reconcile-queue read** of `type='reconcile'` with no
  close row) is a **gated S6/owner-FE deliverable that MUST land before the S5-money prod-flip is declared
  STOP-1-complete** (`proposal.md` §9 gate, R16). The operator's STOP-1 sign-off now ratifies this **two-phase surfacing**.
- **OPEN-2 (obligation-sum semantics — Counsel R2):** `'reconcile'` is an **owner-mediated audit obligation, NOT part of
  the hold/release/settle cash-cycle**; any FUTURE integration that sums the ledger into a courier obligation
  (Σholds − Σcontras — none exists today) **MUST scope the hold-sum to `type='hold'`**, never `type <> x`, else
  `'reconcile'` becomes an **uncloseable** debit against the courier. Carried as a guardrail note in M-A + a mig-028
  comment update; close-trigger owned by the settlement-worker owner when release/settle are wired (R17).

### N4 [MED] — M-B drafted = silent underpayment — **FIX (redesign M-B with a spillover destination)** → owner: **settlement-worker owner + operator**

**Accepted.** A **bare** status-guard on the bump is REJECTED — both shapes silently underpay (verified against
mig 078:178,181,189 + `smoke-checks.ts:178-182`): guard-the-bump → item lands, bump skipped → `Σitems > total_earned`
(smoke-check RED + underpayment); skip-the-period → the assignment stays `NOT EXISTS settlement_item` and is **re-selected
forever for the same immutable period, never settled**. The current **loud `prevent_payout_mutation` RAISE** (mig 052) is
**honester than either silent underpayment** and is the **interim** posture. M-B is redesigned (`proposal.md` §5) with a
**non-negotiable requirement**: a late in-period earning whose natural-period payout is `approved`/`paid` MUST be routed
to a **mutable (`pending`) payout**, never silent-skipped, preserving `total_earned == Σitems` **per payout**. Recommended
destination: **supplemental payout** (period-honest — a `generation_seq`/`supplemental` discriminator folded into M-B's
unique key opens a fresh `pending` P′ for the same period); lighter alternative: **carry-forward** to the courier's
current `pending` payout (no unique-key change, sacrifices period-attribution). DoD probe added: late delivery is
**eventually settled** (courier paid) with `payout_sums` green per payout — a variant that leaves it unsettled or breaks
`payout_sums` is RED. Route 13 stays Node-kept/fail-loud and cron stragglers stay the loud RAISE until M-B's destination
is designed + placed (R3).

### N5 [LOW] — reconcile observable-idempotency — **FIX** → owner: **port author**

**Accepted.** The reconcile INSERT gains `RETURNING id` and **gates the alert + 409-body on `rowcount=1`** (the webhook
§6 discipline): a genuinely new row → publish the alert, return `409{amount, reconcileId}`; a conflict (retry) → 0 rows →
`SELECT id … WHERE order_id=$ AND type='reconcile'` → return the **same reconcileId** (never `None`), **no second alert**.
`proposal.md` §7 move 3 + §9 replay-idempotency probe.

### Counsel R2 — dispositions

- **STOP-1 / STOP-2 SATISFIED** — acknowledged. STOP-1's remaining seam (owner-visibility) is handled by N3 as an honest
  downgrade + a prod-flip gate, not a re-open.
- **OPEN-1 (owner-close surface)** = N3 disposition (gated S6/owner-FE reconcile-queue read + alert transport).
- **OPEN-2 (Stage-21 obligation-sum must consciously treat `'reconcile'`)** = N3/OPEN-2 disposition — exclude from any
  future Σhold; scope hold-sum to `type='hold'`; guardrail in M-A. (No live sum exists today — the ledger is audit-only.)
- **§7b M5 forward-gate** — ADOPTED: the S8 M5-fix (re-drive orders/refund_due from `'failed'`) is a **HARD GATE on
  lighting crypto** — `CRYPTO_ENABLED`/`PAYMENTS_PREPAID` must not flip until M5 is fixed + probed, else a real webhook
  race strands funds in limbo. Recorded on the §7b M5 row + R11.

### R2 epistemic note — carried

**"A partial application of a discipline can be worse than none."** R1 unified lock ordering for deliver only; that both
left the old AB-BA and manufactured a new one (N1). The lesson for the implementer: lock-ordering is a **global**
property — it is proven over the whole set of paths that touch the shared rows, never per-path. The DoD is therefore a
matrix over paths, not a single-pair probe.

---

## RESOLVE outcome

- **R2 — HIGH:** N1 FIX (global o→ca on all 5 courier paths + matrix probe), N2 FIX (narrow predicate + delivered-replay
  echo). **R2 — MED:** N3 FIX (honest downgrade + S6 owner-surface gate) + OPEN-2 (obligation-sum semantics), N4 FIX
  (M-B spillover, reject silent-skip). **R2 — LOW:** N5 FIX (observable idempotency). **Counsel R2:** STOP-1/2 SATISFIED;
  OPEN-1/OPEN-2 folded into N3; M5 crypto forward-gate ADOPTED.
- **CRITICAL:** C1 resolved (REVISE+FIX, migration M-A, operator sign-off).
- **HIGH:** H1 FIX (reorder), H2 FIX (structural gating), H3 DEFER-FLAG (migration M-B, operator/worker owner).
- **MEDIUM:** M1/M2/M3/M6-edge FIX as conscious deviations; M4 FIX (cast table + write-side probes); M5 DEFER-FLAG (S8);
  M6-scope DEFER-FLAG (M-B).
- **LOW:** L1 ACCEPT; L2 DEFER-FLAG; L3 ACCEPT (R9 downgraded) + DEFER-FLAG (preventive).
- **STOPs:** STOP-1 REVISE (lifted on durable-reconcile; operator ratifies); STOP-2 REVISE (no lying stub; lifted).
- **Open question:** HUMAN (product; framing, no batch change).

No finding marked "resolved" without either a concrete design in `proposal.md` or a named owner + close-trigger. Two
new red-line migrations acknowledged; two conscious-departure ledgers added; the Flag-A fix re-architected from a patch
into a lock-reorder + structural-gate + durable-reconcile design. `proposal.md` updated accordingly (§ non-goals, §3,
§4, §5, §6, §7, §7a, §7b, §8, §9, §10, concept ledger).
