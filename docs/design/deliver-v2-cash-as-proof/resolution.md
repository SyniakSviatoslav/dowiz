# Resolution — `deliver` v2 (Cash-as-Proof) · RESOLVE round

> ARCHITECT seat, Triadic Council. Per-finding disposition for every BREAKER finding and every
> COUNSEL binding condition. Each row: **FIXED** (design changed — see proposal/ADR §) ·
> **ACCEPT-RISK** (justification + owner) · **DEFER-FLAG** (MISSING, tracked) · **NEEDS-HUMAN**.
> Grounded re-verification done this round (file:line cited inline). Red lines (§E) intact.

## 0. The one root cause, named

Three of the CRITICALs (C-1, C-2, C-3) and one HIGH (H-1) all reduce to **two un-modeled DB facts**
the v1 proposal extended without touching:

1. `courier_assignments_order_uniq` is a **FULL** unique index on `order_id`
   (`1780421100041_courier-assignments.ts:23`, verified) — one assignment row per order **forever**.
2. The order machine has **no failure/return terminal out of `IN_DELIVERY`**
   (`order-machine.ts:23` — `IN_DELIVERY: ['DELIVERED']`, verified).

The fix set replaces (1) with a **partial-unique on active states only** + a **guarded
terminalize-then-insert** reassign, and (2) with two new edges (`IN_DELIVERY → CANCELLED`,
`IN_DELIVERY → READY`). Everything else hangs off these.

---

## 1. CRITICAL — all FIXED (no accept, no defer; red-line)

### C-1 · re-offer/reassign physically impossible → **FIXED**
**Was:** full unique `courier_assignments_order_uniq` (`…041:23`) + 6 plain INSERTs blocks the 2nd
assignment row → 0% redispatch for any order that ever had a row.
**Fix (proposal §5 DDL, ADR Migration):** drop the full unique; replace with a **partial unique on
ACTIVE states only**:
```
DROP INDEX courier_assignments_order_uniq;
CREATE UNIQUE INDEX courier_assignments_order_active_uniq
  ON courier_assignments (order_id)
  WHERE status IN ('offered','assigned','accepted','picked_up');
```
Terminal rows (`rejected`/`cancelled`/`offered_expired`/`delivered`) no longer count toward the
constraint, so a re-offer / redispatch INSERT after a decline succeeds. This alone unblocks the live
reject→`courier-dispatch.ts:84` redispatch bug. The §A loop (decline→re-offer, timeout→re-offer) now
has room for its second row. **Invariant restored:** single *active* binding per order (DB-enforced),
history rows accumulate.

### C-2 · owner-direct `IN_DELIVERY` trap → **FIXED (two layers)**
**Was:** courier `cancel` (`assignments.ts:428-446`) frees only the assignment + shift, never reverts
`orders.status`/`courier_id`; machine has no `IN_DELIVERY→{CANCELLED,READY}` (`order-machine.ts:23`).
Owner force-assign drives the order to `IN_DELIVERY` (`dashboard.ts:315-319`) with no handshake →
cancel leaves the order stuck `IN_DELIVERY` forever; customer sees "out for delivery" indefinitely.
**Fix:**
- **Edges added** (`order-machine.ts`): `IN_DELIVERY: ['DELIVERED', 'CANCELLED', 'READY']`. (proposal §5)
- **Layer 1 — flag-OFF (legacy owner-direct force path retained):** the courier `cancel` handler, in
  the **same tx**, reverts the order mirror, status-guarded:
  `UPDATE orders SET status='READY', courier_id=NULL WHERE id=$order AND status='IN_DELIVERY' AND courier_id=$me;`
  (no-op for queue-path orders that are still `CONFIRMED`/`READY`). Routed through `updateOrderStatus`
  so the revert carries audit + WS events. The order returns assignable; partial-unique (C-1) lets the
  owner re-offer.
- **Layer 2 — flag-ON (`COURIER_OFFER_HANDSHAKE_ENABLED`):** owner-direct creates an `'offered'` row;
  the **order stays `READY` until pickup** (R-2 unification). The order can never be `IN_DELIVERY`
  without a real `picked_up`-precursor `accepted` row → **H-4 closed structurally**.
**Invariant restored:** §A red-line — decline/cancel never leaves the customer order in a stuck
non-recoverable state.

### C-3 · decline ↔ reassign non-deterministic 500 → **FIXED**
**Was:** owner-reassign (`dashboard.ts:212-330`) is a fresh INSERT for courier B whose only guard is a
`busyCheck` on the *new* courier's id (`:246-251`); it never inspects the order's existing
`offered`/`assigned` row, and the status set excludes `'offered'` → two writers, one full-unique
constraint, outcome decided by commit order.
**Fix (proposal §6):** reassign/re-offer becomes a **status-guarded terminalize-then-insert**, single tx:
```
-- WINNER guard: terminalize the current ACTIVE row (rowcount=1 wins; rowcount=0 → 409 RACE_LOST, no-op)
UPDATE courier_assignments
   SET status='offered_expired', cancelled_at=now(), cancellation_reason='reassigned'
 WHERE order_id=$order AND status IN ('offered','assigned','accepted')   -- not picked_up
RETURNING id, shift_id;          -- rowcount is the authority
-- then (only the winner): revert order mirror if IN_DELIVERY (C-2), then INSERT the new 'offered' row
```
Because the prior row is moved to a **terminal** state first, `courier_assignments_order_active_uniq`
(C-1) is free for the new INSERT — no race on the constraint, a single rowcount authority. The
concurrent courier-decline targets the **same row**: first guarded UPDATE wins (rowcount=1), the loser
sees rowcount=0 → 409/idempotent. **Deterministic single winner** — claim in §6 corrected to reflect
the *guarded* path, not the old unguarded INSERT.

---

## 2. HIGH

### H-1 · no-cash tail mislabels as `DELIVERED` → **FIXED**
**Was:** delivered handler calls `updateOrderStatus(…,'DELIVERED')` regardless of `cash_collected`
(`assignments.ts:340`); no failure terminal.
**Fix (proposal §5/§7, ADR Decision 4a):** the no-cash tail terminal is **`CANCELLED`** (reuse the
existing terminal — **no `order_status` enum value add**, preserving A2). New edge `IN_DELIVERY→CANCELLED`.
Handler branches on the server-validated `payment_outcome`:
- `paid_full` (requires `cash_collected && cash_amount===total`) → assignment `'delivered'`, order
  `DELIVERED`, ledger `'hold'`, trace crumb.
- `refused_goods` / `refused_payment` / `customer_cancelled_on_door` → assignment `'cancelled'`
  (`cancellation_reason = payment_outcome`), order **`CANCELLED`**, **no** ledger hold, trace crumb
  (gps + payment_outcome), shift freed.
The customer sees **Cancelled**, never "Delivered" for refused food. Terminal, courier free → no trap.

### H-2 + counsel C3 · `paid_partial` silent debt → **FIXED (forbidden)**
**Was:** completion 422s on `cash_collected && cash_amount!==total` (`:324-327`); `'hold'` only for
exact total → partial cash is either a 422 stuck-`IN_DELIVERY` trap or unrecorded silent courier debt.
**Decision:** **forbid `paid_partial` as a delivered outcome** (counsel C3's "enum value present but
handler-rejected" — the leaner of the two offered branches; no partial-ledger machinery, no silent
debt). The handler accepts `payment_outcome ∈ {paid_full, refused_goods, refused_payment,
customer_cancelled_on_door}`. `paid_partial` (and `pending`) → **422 `PARTIAL_NOT_SUPPORTED`** before
any mutation. A customer short on cash is a **`refused_payment` no-cash tail** (order→CANCELLED, food
returns, dispute off-platform) — never a "delivered but short" state. **No courier ever holds
unrecorded cash.** Ledger rows: exactly one `'hold'` (= total) on `paid_full`, **zero** on every tail.

### H-3 · outcome = one boolean; `payment_outcome` never written; pocket-and-lie → **FIXED (crumb now collectable) + residual ACCEPT-RISK**
**Was:** body schema `{cash_collected, cash_amount?}` (`:276-279`) has no `payment_outcome` field; the
column is read (`orders.ts:46`) but **never written** by completion → refused/collected/pocketed are
byte-identical.
**Fix (proposal §6/§8):** the delivered body gains `payment_outcome: z.enum(['paid_full','refused_goods',
'refused_payment','customer_cancelled_on_door'])`; the handler **persists it** in the DELIVERED tx to
**both** `orders.payment_outcome` and `delivery_trace.payment_outcome`, server-authoritative
(`cash_collected ⟺ payment_outcome==='paid_full' ∧ cash_amount===total`, else 422 — server owns the
mapping, never a recomputed client value). Now refused vs collected is **recorded**.
**Residual (re-affirmed, tightened — owner: Product, R-3):** a courier who pockets the food and taps
`refused_goods` is still indistinguishable *in truth* from a genuine refusal — but no longer
indistinguishable in the *record*: we now persist `payment_outcome` + the immutable `delivery_trace`
crumb (delivered_at, gps proximity, total). The system **records what was claimed and where the
courier was**; it cannot **prove the claim false** — and by design (no verdict engine) it never tries.
The hole is bounded to "courier physically present at the door who lies about the door outcome on a
no-cash tail" — the cash-bond removes the lie's payoff for `paid_full` (lying = real cash debt), so the
residual lives **only** on the no-cash tail. **ACCEPT-RISK**: this is the §C accepted residual; the
crumb is the burden-of-proof artifact, not a gate.

### H-4 · owner-direct produces `DELIVERED` with no handshake → **FIXED structurally (flag-on) + ACCEPT-RISK (flag-off interim)**
**Was:** handshake is `COURIER_OFFER_HANDSHAKE_ENABLED` default-off; owner-direct (`dashboard.ts:302-320`)
INSERTs `'accepted'` + force-`IN_DELIVERY`; nothing requires `offered→accepted` before `delivered`.
**Fix:** **flag-ON** unifies both paths — owner-direct creates an `'offered'` row, order stays `READY`
until a real pickup, so an order **cannot** reach `IN_DELIVERY`/`DELIVERED` without an `accepted`
assignment that passed the handshake. The C-2 mirror-revert + partial-unique make the legacy
**flag-OFF** path no-trap in the interim. **ACCEPT-RISK (interim, owner: API):** until the flag flips,
the legacy owner-direct force path runs (degenerate "accepted-without-offer") — bounded, no-trap, and
the structural guarantee lands the moment the courier accept/decline UI ships and the flag turns on.
Guardrail added (§9): a test asserting **every `DELIVERED`/`IN_DELIVERY` order has an assignment row
that was `accepted`** — fails red if a force-path ever bypasses it once the flag is on.

---

## 3. MEDIUM

### M-1 · FORCE does not close cross-courier IDOR → **FIXED (honesty + guard stated)**
**Was:** policy is location-scoped only (`…041:28-29`); cross-courier (B acts on A's offer, same
location) is closed **solely** by inline `AND courier_id=$me`. FORCE closes the owner/BYPASSRLS bypass,
**not** the intra-location cross-courier vector — the ADR oversold FORCE.
**Fix (proposal §8, ADR Red lines):**
- **Two distinct guards, named separately:** (1) `FORCE ROW LEVEL SECURITY` + policy aligned to
  `app_member_location_ids()` closes the **owner/BYPASSRLS** bypass (the R-1 fix). (2) The
  **cross-courier-same-location** vector is closed **only** by the inline predicate
  `AND courier_id = $authenticatedCourier` on **every** offered mutation — this is app-code discipline,
  not the DB. The ADR prose corrected: FORCE ≠ cross-courier defense.
- **The actual guard for accept/decline/complete on `'offered'`:** `WHERE status='offered' AND
  courier_id=$me` (status-guarded **and** courier-scoped), rowcount-0 → 404 — the acting courier MUST
  be the offered courier. The existing accept path already routes through `acceptCourierAssignment`
  (`assignments.ts:141`, scoped to courier). The new decline keeps the identical predicate.
- **Guardrail:** the §9 lint extends to assert no `courier_assignments` mutation lacks an
  `AND courier_id = $<authed>` predicate (red→green test: courier B's decline of A's offer → 404).

### M-2 · `cash_amount` not int/nonneg → **FIXED**
**Was:** `cash_amount: z.number().optional()` (`:278`) — float/negative latent once the equality guard
is relaxed.
**Fix (proposal §5/§6):** schema → `cash_amount: z.number().int().nonnegative().optional()`; the
edge rejects float/negative with a **422** (graceful contract), not a DB-CHECK 500. Belt: the new
`delivery_trace.cash_amount integer CHECK (… >= 0)` + ledger `CHECK(amount>=0)` (`1790000000028:17`)
keep the integer-money invariant at the DB. Money stays integer minor units end-to-end.

### M-3 + counsel C2 · crumbs-passive unfalsifiable + customer can't see evidence → **FIXED (a) + FIXED (b)**
**(a) read-display vs read-decision (proposal §9):** the guardrail's authority is **behavioral**, not
purely static. Precise rule:
- **Allowed:** reading a signal row (`delivery_trace`/`order_sensor_events`/`customer_signals`) to
  **serialize it into an HTTP/WS response for a human to view** (display).
- **Banned:** any signal-row value flowing into a **control-flow branch that mutates order/assignment
  status or writes a ledger row**, or that **emits an automated alert/penalty** (the verdict-gate by
  another name — see counsel agent-health).
- **Deterministic gate:** a test asserting the delivered/transition handlers' **outcome is a pure
  function of the courier's tapped input + server-authoritative order columns, independent of any
  signal-row value** (mutate a signal row → outcome unchanged). The lint (signal columns in an
  `if`/`switch`/SQL `WHERE` of a state-mutating statement) is **advisory backup**; the test is the
  authority. Honest limit stated in the ADR: static read/display separation is undecidable in general
  → the behavioral test + code-review rule are the real gate.
**(b) counsel C2 — customer evidence (proposal new §C subsection):** give the **customer
read-access to their OWN immutable order snapshot** — items ordered, integer price, and **`delivered_at`
timestamp** — via their existing authenticated order read (their own data; **not** the courier
GPS/name crumbs). This recovers the "independent evidence nobody controls" kernel of the rejected
Option 1 (counsel §5 steel-man). Declared purpose + retention for the owner-only `delivery_trace`
crumbs (`gps_lat/lng`, `name_snapshot`, `price_snapshot`): **purpose = human dispute-adjudication
evidence**; **retention = 90 days, then GPS anonymized to NULL** (anonymize-not-delete red-line; the
non-PII delivery facts — total, delivered_at, distance — are retained). Customer order snapshot read
follows the existing §C 7-day window. Stated as a carried data-minimization constraint.

---

## 4. COUNSEL binding conditions

### C1 · reconciliation must not auto-deduct no-fault shortfalls → **FOLDED (carried constraint, NEEDS-HUMAN to accept)**
Added as an explicit **carried invariant** in the ADR (Red lines / Carried constraints) and proposal
§10 R-8: *a genuine no-fault shortfall (robbery / customer short-pay / counting error) is **never**
auto-deducted from a (min-wage) courier's pay; reconciliation surfaces it as **owner-reviewed friction,
not a verdict**.* deliver-v2 creates **no** deduction logic (no-op here, verified — the only ledger
write is the `'hold'`), but its "cash bond makes lying costly" justification leans on Stage-21
reconciliation honoring this. **NEEDS-HUMAN:** the Stage-21 reconciliation spec must record this
constraint before the bond is sold as the security primitive. Owner: Product + Stage-21 architect.

### C2 · accuser must see the evidence → **FOLDED** (see M-3(b) above). Owner: Product.

### C3 · specify or forbid `paid_partial` → **FOLDED** (see H-2 — forbidden, handler-rejected). Owner: API.

### "courier = embedded staff" assumption (counsel §6) → **ACCEPT-RISK (NEEDS-HUMAN verify)**
The entire ethics rests on courier = embedded/repeat/reputation-bound staff, **asserted never checked**.
Recorded as proposal §10 R-9 **ACCEPT-RISK, owner: Product** — before launch, verify the actual
employment status in target tenants; if gig/temp couriers without reputation stake are in scope, the
burden-of-proof + reputation levers degrade and the fairness story needs re-examination. **Agent-health
seam:** added to the §9 guardrail spirit + ADR — **no future courier-scoring/penalty layer at
reconciliation without its own Triadic Council** (closes the scoring-creep vector the counsel flagged).

### "cash bond" prose reframe → **FIXED**
Reframed throughout proposal (§1, §3 Option 2) + ADR (Decision 1) from "costly-to-fake cash **bond** /
surety" to **"collected-cash accountability (till-accountability)"** — it is cash the courier already
physically holds, reconciled at shift close like any cashier's till, **not** posted personal capital.
Removes the overselling that could seed a future courier-penalty framing.

---

## 5. LOW (carried)

- **L-1** (double-tap delivered → 404 on retry) → **DEFER-FLAG (cosmetic, owner: API).** Not a money
  bug (HOLD is `ON CONFLICT DO NOTHING`, no double-charge). Tracked for a UX idempotent-200 on a
  post-commit retry; out of scope for the red-line resolution.
- **L-2** (completion never reads `payment_method`; cash=proof mis-fires when card flips on) →
  **DEFER-FLAG (R-4, owner: Architecture).** Re-affirmed: the card-seam ADR must make completion read
  `payment_method` and not assume cash; burden-of-proof does **not** generalize to card (chargeback
  re-imports consumer protection). Not built now; the seam is named, not baked.

---

## Disposition summary

| Finding | Severity | Disposition |
|---|---|---|
| C-1 re-offer impossible | CRITICAL | **FIXED** (partial-unique on active states) |
| C-2 IN_DELIVERY trap | CRITICAL | **FIXED** (mirror-revert + new edges + flag-on unification) |
| C-3 reassign non-deterministic | CRITICAL | **FIXED** (guarded terminalize-then-insert) |
| H-1 no-cash tail → DELIVERED | HIGH | **FIXED** (tail → CANCELLED, new edge) |
| H-2/counsel-C3 paid_partial debt | HIGH | **FIXED** (forbidden, handler-rejected) |
| H-3 outcome boolean / pocket-and-lie | HIGH | **FIXED** (persist payment_outcome) + **ACCEPT-RISK** (R-3 residual) |
| H-4 owner-direct no handshake | HIGH | **FIXED** (flag-on structural) + **ACCEPT-RISK** (flag-off interim) |
| M-1 FORCE ≠ cross-courier IDOR | MED | **FIXED** (two guards named + predicate + guardrail) |
| M-2 cash_amount not int/nonneg | MED | **FIXED** (`.int().nonnegative()` + CHECK) |
| M-3a crumbs-passive unfalsifiable | MED | **FIXED** (behavioral test = authority; display allowed, branch banned) |
| M-3b/counsel-C2 customer can't see evidence | MED | **FIXED** (customer order snapshot read + purpose/retention) |
| L-1 double-tap 404 | LOW | **DEFER-FLAG** (cosmetic) |
| L-2 card seam / payment_method | LOW | **DEFER-FLAG** (R-4 card-seam ADR) |
| Counsel C1 reconciliation no-fault | binding | **FOLDED** (carried constraint, NEEDS-HUMAN @ Stage-21) |
| Counsel C2 accuser evidence | binding | **FOLDED** (= M-3b) |
| Counsel C3 paid_partial | binding | **FOLDED** (= H-2, forbidden) |
| Counsel: embedded-staff assumption | binding | **ACCEPT-RISK** (NEEDS-HUMAN verify, owner Product) |
| Counsel: cash-bond prose | advice | **FIXED** (reframed to till-accountability) |

**Red lines (§E) intact:** no verdict engine, human-tap authority, no-trap-states (now structurally
enforced), friction-not-verdict, crumbs passive, status-guarded transitions, claim-check, money
integer `CHECK(>=0)`, RLS FORCE, zero cookies, RS256, Zod `.strict()`, parameterized SQL.

**NEEDS-HUMAN before launch:** (1) Stage-21 reconciliation must encode counsel-C1 (no auto-deduct of
no-fault shortfall). (2) verify courier employment status (embedded-staff assumption). (3) flip
`COURIER_OFFER_HANDSHAKE_ENABLED` only when the accept/decline UI ships.

---

## RESOLVE round 2

> ARCHITECT seat, regression round. Per round-2 BREAKER finding (R2-1..R2-9) and COUNSEL hardening
> (C1/Q5 merge, Q4). Re-grounded against live source @ HEAD this round; file:line cited inline.
> The round-1 fixes were correct **where applied** — the misses are the **second completion path**
> (owner-proxy) and the **unaudited callers of the globally-widened machine**. Both reduce to one
> structural gap: *side-effects (HOLD/crumbs, assignment-terminalize) were attached per-handler
> instead of to the transition itself.* The round-2 fix-set **centralizes** them so they are
> structurally guaranteed on every path.

### 0. Root cause (round 2), named

The widened `IN_DELIVERY` edge and the cash-as-proof HOLD were both enforced **at one call-site**
(the courier handler), not at the **shared primitive**. Fix posture this round: move the two
invariants into **one shared completion function** (R2-1) and into **`updateOrderStatus` itself**
(R2-3), so no caller can reach `DELIVERED`/leave `IN_DELIVERY` without them.

### Audit result — every caller of the widened `IN_DELIVERY→{CANCELLED,READY}` map (grep verified)

| Caller | Edge taken | Terminalizes active assignment? | Round-2 verdict |
|---|---|---|---|
| `customer/orders.ts:300-318` self-cancel | IN_DELIVERY→CANCELLED (raw UPDATE) | **YES** (`:309-318`, `status IN ('assigned','accepted','picked_up')`) | SAFE — the reference pattern |
| `owner/signals.ts:234` no-show | IN_DELIVERY→CANCELLED via `updateOrderStatus` | **NO** | **STRANDS — R2-3** |
| `orders.ts:779` owner PATCH `/orders/:id/status` | IN_DELIVERY→CANCELLED (`StatusUpdateInput.status = OrderStatusEnum` ⊇ CANCELLED, `legacy.ts:97-98`) | **NO** | **STRANDS — R2-3 (2nd site)** |
| `owner/dashboard.ts:264-268` reassign revert | IN_DELIVERY→READY (raw UPDATE) | terminalizes OLD row (`:256`) but raw, no machine/WS | **R2-6** |
| `courier/assignments.ts` cancel (C-2 revert) | IN_DELIVERY→READY | frees shift + terminalizes, but **5-min-gated** | **R2-2** |
| `owner/dashboard.ts:585` `transitionOrder` | CONFIRMED/REJECTED only (`:195,:207`) — not in widened map | n/a | SAFE |
| `order-timeout-sweep.ts:68` | `WHERE status='PENDING'` only | n/a | SAFE |
| `lifecycle-handlers.ts:27` ORDER_CANCELLED consumer | resolves `location_alerts` only — **does NOT transition the order** (`:39-58`) | n/a | SAFE (confirms R2-5 is not a double-cancel) |
| `telegram-webhook.ts:282,412` | CONFIRMED/REJECTED only | n/a | SAFE |

**Shared invariant (round-2, structurally enforced):** *No order leaves `IN_DELIVERY` (to `CANCELLED`
or `READY`) without its active `courier_assignments` row terminalized — and its shift freed — in the
**same transaction**.* Enforced **centrally inside `updateOrderStatus`** (not per caller) + a guardrail
test. This closes R2-3 at every present and future call-site, not just the two found.

---

### HIGH (red-line)

#### R2-1 · owner-proxy deliver writes no HOLD / payment_outcome / trace → **FIXED (paths UNIFIED through one completion function)**
**Grounded:** `owner/dashboard.ts:408-481` (verified) — the owner-proxy `/deliver`: takes raw
`body?.cash_collected ?? true` + `body?.cash_amount` (`:413-414`, no Zod int/nonneg, no
`cash_amount===total` guard), sets assignment `'delivered'` (`:444-449`), calls
`updateOrderStatus(…,'DELIVERED')` (`:456`), and writes **no `delivery_trace`, no `payment_outcome`,
no `courier_cash_ledger` `'hold'`** even when `cashCollected=true`. The entire cash-as-proof primitive
is absent on this live path. The courier handler (`courier/assignments.ts:329-359`) writes all three.
**Fix — the exact shared completion path:** introduce **`apps/api/src/lib/deliveryCompletion.ts` →
`completeDelivery(client, args, { messageBus })`** (takes a `PoolClient`, runs inside the caller's tx,
**no `BEGIN`/`COMMIT`**). It is the **single** function that performs every completion side-effect:

```
completeDelivery(client, {
  orderId, locationId, courierId, assignmentId, shiftId,
  paymentOutcome,           // 'paid_full'|'refused_goods'|'refused_payment'|'customer_cancelled_on_door'
  cashAmount,               // integer minor units (already Zod .int().nonnegative())
  total,                    // server-authoritative, read FOR UPDATE from orders in the caller
  routeDistanceM, expectedDeliveryMin, gpsLat, gpsLng, nameSnapshot, priceSnapshot
}, { messageBus })
```
It (1) re-validates server-authoritatively (`paid_full ⟺ cashAmount===total`; `paid_partial`/`pending`
→ throw 422 `PARTIAL_NOT_SUPPORTED`/`CASH_AMOUNT_MISMATCH` **before any write**); (2) branches —
`paid_full` → assignment `'delivered'` + shift `'available'` + `updateOrderStatus(DELIVERED)` +
`delivery_trace` crumb (`payment_outcome`,gps,total,snapshots, `ON CONFLICT (order_id) DO NOTHING`) +
ledger `'hold'` (`ON CONFLICT (order_id,type) DO NOTHING`); no-cash tail → assignment `'cancelled'`
(reason=outcome) + shift `'available'` + `updateOrderStatus(CANCELLED)` + crumb, **no hold**; (3)
publishes the lifecycle event. **Both** `courier/assignments.ts` POST `/assignments/:id/delivered`
**and** `owner/dashboard.ts` POST `/:locationId/orders/:orderId/deliver` lose their inline completion
bodies and **call `completeDelivery`** — so the HOLD/`payment_outcome`/trace are written **structurally**
on every `delivered`, regardless of which path fires. The owner `/deliver` body gains the same
first-class `payment_outcome` field + `.int().nonnegative()` `cash_amount` as the courier body.
**Guardrail (red→green):** a test asserting *every order that reaches `DELIVERED` has a
`delivery_trace` row AND (when `paid_full`) a `courier_cash_ledger` `'hold'` row* — fails red against
the current owner-proxy path, green once both route through `completeDelivery`. 🔴 money red-line restored
on **both** paths. **Disposition: FIXED.** (proposal §5/§6/§7, ADR Decision 1 + Migration.)

#### R2-2 · C-2 revert gated behind the 5-min cancel window → **FIXED (distinct en-route abort, no time gate)**
**Grounded:** `courier/assignments.ts:423-426` (verified) returns **410 `CANCEL_WINDOW_EXPIRED`** when
`Date.now() − assigned_at > CANCEL_AFTER_DISPATCH_WINDOW_MS` (default 300000), and the C-2 mirror-revert
lives **after** that gate → a `picked_up` order en route >5 min that the courier cannot complete:
`/cancel` → 410 (revert unreached); reassign excludes `picked_up` → 409 → **stuck `IN_DELIVERY`**.
**Fix — the 5-min window is for the OFFER/accept regret, NOT an en-route failure; split the exits:**
- **`/cancel` (offer-regret) — window KEPT.** Semantics unchanged: a courier who *just accepted*
  reconsiders within `CANCEL_AFTER_DISPATCH_WINDOW_MS`. Order→`READY` (revert if owner-direct drove
  `IN_DELIVERY`) or untouched (queue path), assignment→`'cancelled'`, re-offerable. Time-gated because
  it is a grace-period **undo of the accept**.
- **`/abort` (en-route failure) — NO time gate (NEW courier action).** Guard:
  `WHERE id=$1 AND courier_id=$me AND status IN ('accepted','picked_up')` (status-guarded + courier-
  scoped, rowcount-0 → 404). It terminalizes the assignment `'cancelled'`
  (`cancellation_reason='courier_aborted_en_route'`), frees the shift, and transitions the order **via
  `updateOrderStatus`** (so the R2-3 central terminalize + history + WS apply):
  - assignment was `'accepted'` (food still at venue) → order→**`READY`** (re-offerable);
  - assignment was `'picked_up'` (food is with the failed courier) → order→**`CANCELLED`**
    (`comment='courier_aborted'`) — the honest terminal (food is NOT at venue; reverting to READY would
    lie); owner re-creates a fresh order if re-delivery is wanted.
  No 5-min gate → the realistic long-delivery stuck case is recoverable. **Transition + guard stated
  above.** **Disposition: FIXED.** (proposal §5/§7, ADR Decision 4 C-2.)

#### R2-3 · widened edge strands the no-show / owner-PATCH paths → **FIXED (invariant folded into `updateOrderStatus`)**
**Grounded:** `order-machine.ts:23` widened globally; `owner/signals.ts:232-238` calls
`updateOrderStatus(…,'CANCELLED')` with **no assignment touch** (verified) → post-widening it succeeds
on an `IN_DELIVERY` order and leaves the `'picked_up'` row **active** → `courier_one_active_assignment`
(partial-unique on active) **blocks that courier forever**. Audit (table above) found a **second**
unguarded site: `orders.ts:779` owner PATCH (CANCELLED ∈ `OrderStatusEnum`). **Fix — structural, central,
not per-caller:** inside **`updateOrderStatus` (`lib/orderStatusService.ts`)**, when
`currentStatus==='IN_DELIVERY' && newStatus IN ('CANCELLED','READY')`, in the **same tx** after the
guarded order UPDATE:
```
WITH freed AS (
  UPDATE courier_assignments
     SET status='cancelled', cancelled_at=now(),
         cancellation_reason = COALESCE($comment, 'order_'||lower($newStatus))
   WHERE order_id=$orderId AND status IN ('offered','assigned','accepted','picked_up')
  RETURNING shift_id )
UPDATE courier_shifts SET status='available' WHERE id IN (SELECT shift_id FROM freed);
```
Idempotent — a row already terminal (e.g. `completeDelivery` set it `'cancelled'`/`'delivered'` first,
or the no-cash tail) is a no-op; `DELIVERED` is **not** in `{CANCELLED,READY}` so the `'delivered'`
row is never reverted. This covers **signals.ts:234, orders.ts:779, the dashboard reassign revert
(once R2-6 routes through it), and the courier `/cancel`+`/abort`** — every present and future caller.
**Shared invariant:** *no order leaves `IN_DELIVERY` without its active assignment terminalized in the
same tx.* **Guardrail (red→green):** a test asserting *after any `IN_DELIVERY→{CANCELLED,READY}` there
is zero `courier_assignments` row for that order in an active status* — red against the current
no-show path, green after the fold. **Disposition: FIXED.** (proposal §5/§6/§9, ADR Decision 4 C-2.)

---

### MED

#### R2-4 · paid_partial debt moved to the doorstep → **FIXED (carried no-partial-handover rule made explicit)**
The ledger-level fix (forbid `paid_partial`) is honest **iff** the operational rule that makes
`refused_payment` a *true* record is stated and taught (counsel Q1). **Fix:** declare the explicit
carried courier rule in the ADR + proposal + courier-UX: **"No partial handover — full cash in hand
before the food changes hands; a customer short on cash gets no goods and the courier taps
`refused_payment` (→ order CANCELLED, food returns)."** Partial collection is **operationally
prevented** (the courier-app completion UI offers only `paid_full` / the no-cash tails — there is no
"partial amount" affordance), not merely enum-rejected. This is the carried invariant that makes
forbidding `paid_partial` a clean subtraction rather than relocating unrecorded cash to where the
software can't see it. **Disposition: FIXED (carried rule, owner: Product + courier-UX).**
(proposal §6/§7, ADR Decision 1.)

#### R2-5 · revert→READY still publishes ORDER_CANCELLED → **FIXED (broadcast matches resulting status)**
**Grounded:** `courier/assignments.ts:440-444` (verified) unconditionally publishes
`BUS_CHANNELS.ORDER_CANCELLED` at the cancel tail. With the C-2 revert the order is `READY`, so the
customer receives a contradictory "cancelled". **Fix:** the revert path no longer hand-publishes
`ORDER_CANCELLED`. The order transition routes through `updateOrderStatus`, which already publishes the
**correct** `ORDER_STATUS` event for the resulting status (`orderStatusService.ts` post-commit publish).
Broadcast rule, stated: **`/cancel`/`/abort` that reverts the order to `READY` emits a `READY`
`ORDER_STATUS` event; only an exit that actually terminalizes the order to `CANCELLED` (the no-cash
tail or picked-up `/abort`) emits `ORDER_CANCELLED`.** One event, matching the real state.
**Disposition: FIXED.** (proposal §6/§7.)

#### R2-6 · reassign revert is a raw UPDATE bypassing machine+WS → **FIXED (route through `updateOrderStatus`)**
**Grounded:** `owner/dashboard.ts:264-268` (verified) reverts the displaced order via raw
`UPDATE orders SET status='READY', courier_id=NULL` → no `order_status_history`, no WS event → that
order's customer is stranded on stale "out for delivery". **Fix:** replace with
`updateOrderStatus(client, old.order_id, locationId, 'READY', { messageBus, comment:'owner_reassigned' })`
followed by `UPDATE orders SET courier_id=NULL WHERE id=$old` (mirror of the assign path's
`courier_id` set at `:317-319`). The widened `IN_DELIVERY→READY` edge makes this a legal transition;
the R2-3 central fold makes the old assignment terminalize + shift-free happen inside it (the explicit
`:256` terminalize becomes redundant/idempotent). History + WS now fire. **Disposition: FIXED.**
(proposal §5/§6.)

#### R2-7 + counsel Q2b · 90-day GPS retention: no worker + 83-day overhang → **FIXED (a window aligned, b real worker)**
**Grounded:** `grep apps/api/src/workers` → **no** worker references `delivery_trace` (verified — the
existing retention crons cover `access_requests`, anonymizer-GDPR, acquisition only). The §8 "90 days →
NULL" was prose with no mechanism.
- **(a) Window aligned to purpose.** The crumbs' only declared consumer is dispute adjudication; the
  dispute window is **7 days** (§C). Set the GPS/`name_snapshot`/`price_snapshot` anonymize-to-NULL
  bound to **`DELIVERY_TRACE_GPS_RETENTION` = 14 days** (= 7-day dispute window + a **stated 7-day
  off-platform settlement buffer** for late-surfacing cash disputes) — *derived from* the purpose, not
  picked. The non-PII delivery facts (`total`, `delivered_at`, `route_distance_m`,
  `expected_delivery_min`, `payment_outcome`) are **retained** (anonymize-not-delete). ADR/proposal
  corrected from "90 days" → "14 days (7d dispute + 7d settlement buffer)".
- **(b) Real enforcing artifact.** New worker
  **`apps/api/src/workers/delivery-trace-retention.ts`** mirroring `access-request-retention.ts`
  (advisory-lock guard, `.catch`-wrapped `boss.schedule`, `assertDeliveryTraceSchedule()` boot-assert
  after `listen()`). Sweep (idempotent, anonymize-not-delete):
  ```
  UPDATE delivery_trace
     SET gps_lat=NULL, gps_lng=NULL, name_snapshot=NULL, price_snapshot=NULL
   WHERE delivered_at < now() - $1::interval        -- DELIVERY_TRACE_GPS_RETENTION
     AND (gps_lat IS NOT NULL OR name_snapshot IS NOT NULL);
  ```
  Cron `DELIVERY_TRACE_GPS_RETENTION_CRON` default `'0 3 * * *'`. **Guardrail:** boot-assert (prod
  `process.exit(1)` if the schedule is missing) — the same falsifiable mechanism as access-request.
  Converts the red-line from prose to an enforced control. **Disposition: FIXED.** (proposal §8/§9, ADR
  Carried constraints + Migration.)

#### R2-8 · M-1 guardrail phrasing mis-scopes owner paths → **FIXED (predicate precision)**
**Grounded:** owner reassign/pickup/deliver legitimately mutate assignment rows with no `courier_id=$me`
(`dashboard.ts:256,372,445`, owner authority, location-scoped). **Fix:** the §9 guardrail is scoped to
**courier-context** mutations only — *every `courier_assignments` mutation in a `routes/courier/*`
handler (the `request.user.sub`=courier surface) MUST carry `AND courier_id = $authenticatedCourier`;
owner-context handlers (`routes/owner/*`, location-scoped by RLS + `locationId` param) are explicitly
carved out.* Precision, not a hole. **Disposition: FIXED (guardrail re-scoped).** (proposal §9.)

#### R2-9 · the C2 route already leaks courier PII to the customer (pre-existing) → **ACCEPT-RISK (out of scope) + DEFER-FLAG**
**Grounded:** `customer/orders.ts:63-83` (verified) already decrypts+returns courier `full_name`
(first-char + `***`) / `phone` (masked) + live `courier_positions` GPS during an active order. This is
**pre-existing**, masked, and active-order-only (`courierActive` gate, `:87`), and is **outside this
change's surface** (the C2 fix adds only the customer's *own* snapshot, no new courier PII). The §8
minimization claim is scoped to the *new* crumbs (`delivery_trace.gps/name`), which stay owner-only.
**Disposition: ACCEPT-RISK (pre-existing, masked, bounded) + DEFER-FLAG** — tracked separately: revisit
whether masked courier name/phone/live-GPS to the customer is necessary vs an in-app relay. Owner:
Product/Privacy. Not introduced by v2; not blocking. **Honest note:** the §8 "minimization holds"
statement is true only for the *v2-added* crumbs, not for the route it piggybacks — stated plainly in
proposal §8.

---

### COUNSEL hardenings

#### C1/Q5 merged · materialize Stage-21 no-auto-deduct as a DURABLE artifact NOW → **FIXED (failing guardrail + ledger row, not prose)**
R-8 (no-auto-deduct of no-fault shortfall) + R-9 (embedded-staff) collapse into **ONE Stage-21
invariant** (counsel Q5): *reconciliation never auto-deducts a no-fault shortfall and never derives a
courier score/penalty from any crumb; every shortfall is **owner-reviewed friction**; no such layer
lands without its own Triadic Council.* Materialized **now** as a durable artifact a future Stage-21 PR
**trips** (not a design-doc sentence):
- **A failing pending-guardrail test** `apps/api/src/workers/__tests__/stage21-no-auto-deduct.invariant.test.ts`
  that asserts an ADR `docs/adr/ADR-stage21-reconciliation.md` exists **and** contains the markers
  `NO-AUTO-DEDUCT` **and** `NO-COURIER-SCORING`. It is **RED today** (the ADR does not yet exist) and
  goes green only when the Stage-21 author records the invariant — so the protection **cannot be
  silently forgotten**.
- **A code guardrail** (`tools/eslint-plugin-local`): any `INSERT INTO courier_cash_ledger` with a
  `type` other than `'hold'` (i.e. a `'deduction'`/`'penalty'`-typed write) — **and** any state/penalty
  write whose value derives from a `delivery_trace`/signal-row column — is **banned** unless the
  Stage-21 ADR marker is present. This is the **anti-scoring-creep guard** (merges the §9 agent-health
  seam) and the durable barrier the deduction-builder trips.
- **Regression-ledger row** added (`docs/regressions/REGRESSION-LEDGER.md`) naming both guardrails.
**Disposition: FIXED (durable artifact authored; the *mechanism* remains NEEDS-HUMAN @ Stage-21, but
the *guard* is now red-on-disk, not narrative).** Owner: Product + Stage-21 architect. (proposal §9/§10
R-8+R-9 merged, ADR Carried constraints.)

#### Q4 · the accused (customer) cannot see they were recorded as refuser → **FIXED (surface own outcome, humane render)**
Inversion of C2 (counsel Q4): a courier who pockets the food and taps `refused_goods` records the
*customer* as refuser, but the customer sees a flat "Cancelled" with no signal — undisputable.
**Fix:** the customer's authenticated order read (`customer/orders.ts:30-49` SELECT) surfaces the
customer's **own** `orders.payment_outcome` **and** `orders.cancellation_reason` (their own data,
RLS-scoped to `customer_id=$me`), rendered humanely (i18n) on their order page:
- `refused_payment` → "Cancelled — payment was not completed"
- `refused_goods` / `customer_cancelled_on_door` → "Cancelled — recorded as refused at the door"
- `courier_aborted` → "Cancelled — the delivery could not be completed"
The **customer-facing field = `payment_outcome` + `cancellation_reason` on the customer order
response** (humane-mapped client-side; raw enum never exposed). This lets the accused **see and
contest** the record (server stays authoritative; UI under-informs no longer). Extends C2.
**Disposition: FIXED.** Owner: Product/FE. (proposal §8 customer-evidence subsection.)

---

### Round-2 disposition summary

| Finding | Sev | Disposition |
|---|---|---|
| R2-1 owner-proxy deliver writes no HOLD/outcome/trace | HIGH 🔴 | **FIXED** — paths unified through `lib/deliveryCompletion.ts::completeDelivery` |
| R2-2 C-2 revert gated behind 5-min window | HIGH 🔴 | **FIXED** — distinct `/abort` en-route exit, no time gate |
| R2-3 widened edge strands no-show / owner-PATCH | HIGH 🔴 | **FIXED** — terminalize folded into `updateOrderStatus` (central invariant) |
| R2-4 paid_partial debt moved to doorstep | MED | **FIXED** — explicit carried no-partial-handover rule (UX+ADR) |
| R2-5 revert→READY publishes ORDER_CANCELLED | MED | **FIXED** — broadcast matches resulting status |
| R2-6 reassign revert raw-UPDATE bypasses machine+WS | MED | **FIXED** — route through `updateOrderStatus` |
| R2-7 + Q2b 90-day GPS retention, no worker, overhang | MED | **FIXED** — 14d (7d dispute+7d buffer) + real sweep worker |
| R2-8 M-1 guardrail mis-scopes owner paths | LOW | **FIXED** — guardrail scoped to courier-context only |
| R2-9 C2 route already leaks masked courier PII | LOW | **ACCEPT-RISK** (pre-existing, masked, bounded) + **DEFER-FLAG** |
| Counsel C1/Q5 merged no-auto-deduct + anti-scoring | binding | **FIXED** — durable failing guardrail + ledger row (mechanism NEEDS-HUMAN @ Stage-21) |
| Counsel Q4 accused can't see accusation | binding | **FIXED** — surface own `payment_outcome`+`cancellation_reason` |

**Every §E red line re-checked and intact** after round 2: no verdict engine, human-tap authority,
no-trap-states (now structurally enforced on BOTH completion paths + ALL exit paths), friction-not-
verdict, crumbs passive (+ real retention sweep), status-guarded transitions, claim-check (bus id-only),
money integer `CHECK(>=0)` + `.int().nonnegative()` (now on the owner path too), RLS FORCE, zero
cookies, RS256, Zod `.strict()`, parameterized SQL, anonymize-not-delete (now enforced by a worker).

**Genuinely-not-an-issue (no hand-waving):** R2-9 is real but **pre-existing and out of this change's
surface** — v2 adds no new courier PII to the customer; the only honesty correction is to scope the §8
minimization claim to the v2-added crumbs (done). `lifecycle-handlers.ts` consuming `ORDER_CANCELLED`
is **not** a hidden second cancel — it only resolves dwell alerts (`:39-58`), confirmed by read.

**NEEDS-HUMAN (unchanged + 1 sharpened):** (1) Stage-21 reconciliation must author
`ADR-stage21-reconciliation.md` with the `NO-AUTO-DEDUCT`+`NO-COURIER-SCORING` markers — **now a
red-on-disk guardrail, not a reminder**. (2) verify embedded-staff employment assumption (R-9, fused
into the Stage-21 invariant). (3) flip `COURIER_OFFER_HANDSHAKE_ENABLED` only when the accept/decline +
`/abort` courier UI ships.

---

## RESOLVE round 3

> ARCHITECT seat, convergence round. Per round-3 BREAKER finding (R3-1/2/3). Re-grounded against live
> source @ HEAD this round; file:line cited inline. The round-3 misses are **properties of the two NEW
> runtime artifacts round-2 introduced** — the cross-tenant retention sweep and the `/abort`
> `accepted`-branch — plus the LOW exhaustiveness caveat on the central fold. Red lines (§E) intact.

### 0. Root cause (round 3), named

Both open findings reduce to **the round-2 fixes assumed a shape the live table/machine does not have**:

1. The retention sweep was specified as *"mirror `access-request-retention.ts`"* — but that precedent
   runs against an **ops allow-all `USING(true)`** policy (`1790000000041:49-50`, verified), whereas
   `delivery_trace` is **tenant-scoped FORCE** `USING (location_id IN (SELECT app_member_location_ids()))`
   (`1790000000027:22-25`, verified). A plain operational-pool `UPDATE` sees **0 rows** → the red-line
   never fires, and the schedule-existence boot-assert cannot detect it.
2. The `/abort` `accepted`-branch was specified to drive the order `→READY`, assuming `accepted ⟺
   IN_DELIVERY` — but the A2 flag-ON model it adopts keeps the order at `CONFIRMED/PREPARING/READY` until
   pickup (`order-machine.ts:20` `CONFIRMED:['PREPARING','IN_DELIVERY']` — no `READY`; verified), so
   forcing `READY` throws `IllegalTransition`/`SameStatus` and rolls back the whole abort.

Fix posture: make each new artifact's authority **match the live shape** — a privileged
`SECURITY DEFINER` sweep that bypasses the tenant FORCE policy (the canonical cross-tenant mechanism),
and an order-side action **conditional on the order's actual status** (never a forced transition).

---

### HIGH (red-line)

#### R3-1 · 14-day GPS-anonymize sweep matches 0 rows under tenant-scoped FORCE; schedule-existence guardrail can't detect it → **FIXED (SECURITY DEFINER sweep + OUTCOME-based guardrail)**
**Grounded:** `delivery_trace` policy is `tenant_isolation USING (location_id IN (SELECT
app_member_location_ids()))` under `ENABLE + FORCE` (`1790000000027:22-25`, verified — **not**
`USING(true)`). The cited precedent `access_requests` uses `allow_ops_access_requests_all FOR ALL
USING(true)` (`1790000000041:49-50`, verified), so its operational-pool sweep passes RLS context-free;
`delivery_trace`'s does not. A worker on `createOperationalPool()` with no `set_config('app.user_id'…)`
→ `app_member_location_ids()` empty → the `UPDATE … WHERE delivered_at < now()-$1` matches **0 rows** →
GPS/`name_snapshot`/`price_snapshot` retained indefinitely while `assertDeliveryTraceSchedule()` (schedule
existence) stays GREEN. A silent false-green on a 🔴 PII red-line.

**Fix — the exact mechanism (mirrors the proven P6 `app_is_shadow_location` / `read_preview_menu`
posture, `1790000000070:33-34,59-63`):** the sweep does NOT run as a context-free operational `UPDATE`.
It runs through a **`SECURITY DEFINER` function owned by the privileged migration role** (the Supabase
`postgres`/admin owner that already owns the table and bypasses RLS — the **same** mechanism by which
`read_public_menu`/`app_is_shadow_location` cross tenant boundaries today). The definer's RLS-bypass is
what reaches **all** rows past the window, with **no per-tenant context loop and no reliance on an
operational-role `BYPASSRLS` attribute** (which project memory flags as an uncertain env artifact —
this design must not depend on it). The function is narrowly scoped (NULLs only the PII crumb columns,
returns a count, exfiltrates no rows), pins `search_path`, and is `REVOKE ALL … FROM PUBLIC` + grant-
mirrored — exactly the canon:

```sql
-- in the v2 migration, AFTER the delivery_trace gps/name/price columns are added (proposal §5):
CREATE OR REPLACE FUNCTION anonymize_stale_delivery_trace(p_window interval)
RETURNS integer
LANGUAGE plpgsql
VOLATILE                       -- it writes; NOT stable
SECURITY DEFINER               -- executes as the privileged owner → bypasses delivery_trace FORCE RLS
SET search_path = public       -- pinned (closes the SECURITY-DEFINER search_path class, memory ledger)
AS $$
DECLARE
  v_count integer;
BEGIN
  WITH anon AS (
    UPDATE delivery_trace
       SET gps_lat = NULL, gps_lng = NULL, name_snapshot = NULL, price_snapshot = NULL
     WHERE delivered_at < now() - p_window
       AND (gps_lat IS NOT NULL OR gps_lng IS NOT NULL
            OR name_snapshot IS NOT NULL OR price_snapshot IS NOT NULL)
    RETURNING 1)
  SELECT count(*)::int INTO v_count FROM anon;
  RETURN v_count;     -- the worker logs/asserts this; 0-with-overdue-rows-present is now detectable
END;
$$;

REVOKE ALL ON FUNCTION anonymize_stale_delivery_trace(interval) FROM PUBLIC;
-- mirror EXECUTE to whatever role already executes read_public_menu_all_locales (grant-mirror DO-block,
-- 1790000000070:114-129 pattern) so the operational worker pool can call it without naming a role.
```

The worker (`workers/delivery-trace-retention.ts`) changes from a raw `UPDATE` to:
`const { rows } = await client.query('SELECT anonymize_stale_delivery_trace($1::interval) AS n',
[env.DELIVERY_TRACE_GPS_RETENTION || '14 days']);` and logs `rows[0].n`. The advisory-lock guard,
`.catch`-wrapped `boss.schedule`, and post-`listen()` boot-assert are unchanged.

**Guardrail strengthened from schedule-existence → OUTCOME-based (the round-3 ask):** in addition to
`assertDeliveryTraceSchedule()` (kept — it still catches a *missing* cron), add a deterministic
**efficacy** test that fails when the sweep anonymizes nothing it should:
```
// delivery-trace-retention.efficacy.test.ts (integration, real PG)
// 1. seed a delivery_trace row across ≥2 tenants with delivered_at = now() - (window + 1 day),
//    gps_lat/lng + name_snapshot + price_snapshot NON-NULL;
// 2. invoke the sweep the way the worker does (operational pool, NO app.user_id/tenant set);
// 3. ASSERT the call RETURNs >= seeded-count  AND
//    ASSERT zero delivery_trace rows older than (window + grace) still have non-null gps_lat/lng/name/price
//    (SELECT count(*) WHERE delivered_at < now()-(window) AND gps_lat IS NOT NULL  ==> 0).
```
This is **red** against the round-2 raw context-free `UPDATE` (it anonymizes 0 cross-tenant rows under
FORCE) and **green** only once the sweep routes through the `SECURITY DEFINER` function. A silently-0-row
sweep is now RED, not GREEN — the red-line is enforced by *measured efficacy across tenants*, not the
presence of a cron row. **Disposition: FIXED.** (proposal §5/§8/§9, ADR Migration + Carried constraints.)

> §E note: a privileged cross-tenant `SECURITY DEFINER` PII-anonymization sweep is **consistent** with
> the RLS-FORCE red line, not a breach — it is the sanctioned, narrowly-scoped, audited cross-tenant
> mechanism (identical posture to the GDPR/anonymizer workers and `read_public_menu`). FORCE remains the
> isolation guarantee on every *normal* access path; this one privileged maintenance path is by design.

---

### MEDIUM

#### R3-2 · `/abort` from `accepted` forces `updateOrderStatus(…,'READY')` on a pre-pickup order → 400 throw rolls back the whole abort → **FIXED (order-side action CONDITIONAL on the order's actual status, never forced)**
**Grounded:** under flag-ON, accept does not advance the order (proposal §3 A2:137-138) → an `'accepted'`
assignment coexists with an order at `CONFIRMED`/`PREPARING`/`READY`. `assertTransition`
(`order-machine.ts:35-42`, verified): `CONFIRMED:['PREPARING','IN_DELIVERY']` → `READY` is
`IllegalTransitionError` (400); `READY→READY` is `SameStatusError` (400). Either throw propagates out of
`/abort` → `ROLLBACK` → the assignment is never terminalized, the shift never freed → the courier cannot
abort a pre-pickup accepted offer; retry loops on the same 400. Only the `PREPARING`→`READY` sub-case is
incidentally legal.

**Fix — terminalize the binding UNCONDITIONALLY first (so abort always frees it), then take an order-side
action that is GUARDED on the order's actual status (so it never forces an illegal/same-status
transition):**
```
BEGIN
-- 1. lock binding + order; rowcount-0 → 404 (not yours / wrong state). Capture BOTH statuses.
SELECT a.status AS asg_status, a.shift_id, o.status AS ord_status
  FROM courier_assignments a JOIN orders o ON o.id = a.order_id
 WHERE a.id = $1 AND a.courier_id = $me AND a.status IN ('accepted','picked_up')
 FOR UPDATE;                                   -- 0 rows → 404 ASSIGNMENT_NOT_ABORTABLE

-- 2. ALWAYS free the binding + shift (abort can never leave a dangling active assignment)
UPDATE courier_assignments
   SET status='cancelled', cancelled_at=now(), cancellation_reason='courier_aborted_en_route'
 WHERE id = $1;                                -- already row-locked, guaranteed rowcount=1
UPDATE courier_shifts SET status='available' WHERE id = $shift_id AND status='on_delivery';

-- 3. order-side action: CONDITIONAL on ord_status — NO forced transition
IF ord_status = 'IN_DELIVERY' THEN            -- flag-OFF owner-direct (force-drove IN_DELIVERY) or picked_up
   IF asg_status = 'picked_up' THEN
     updateOrderStatus(client, $order, $loc, 'CANCELLED', {comment:'courier_aborted'});  -- food is out → honest terminal
   ELSE                                        -- 'accepted' while IN_DELIVERY (legacy force) → food at venue
     updateOrderStatus(client, $order, $loc, 'READY',    {comment:'courier_aborted_pre_pickup'}); -- legal: IN_DELIVERY→READY widened
   END IF;
ELSE                                           -- flag-ON 'accepted': order is CONFIRMED/PREPARING/READY → never advanced
   UPDATE orders SET courier_id = NULL WHERE id = $order AND courier_id = $me;  -- clear stale binding; NO status transition
END IF;
COMMIT;
```
Key properties: (a) the assignment-terminalize + shift-free in step 2 happen **before and independent of**
any order transition, so abort **always** frees the assignment; (b) `updateOrderStatus` is called **only**
when the order is `IN_DELIVERY` — the **one** state from which `→CANCELLED`/`→READY` is a legal widened
edge — so it can **never** throw `IllegalTransition`/`SameStatus`; (c) the flag-ON pre-pickup case takes
the no-transition branch (the order was never moved → nothing to revert; it stays re-offerable in its
current status, binding cleared). Step 2's pre-terminalize is idempotent with the R2-3 central fold (the
fold's `WHERE status IN (active…)` no longer matches the now-`'cancelled'` row → no-op).

**Guardrail (red→green, flag-ON):** a test that — with `COURIER_OFFER_HANDSHAKE_ENABLED=on` — offers then
accepts an order (order stays `CONFIRMED`/`READY`), POSTs `/abort`, and asserts: **200** (not 400),
assignment `status='cancelled'`, order status **unchanged** (still `CONFIRMED`/`READY`, NOT forced to
`READY`, NOT `DELIVERED`), `orders.courier_id IS NULL`, shift `'available'`, and the order is re-offerable.
Red against the round-2 "force READY" spec (throws 400 → rollback), green after the conditional.
**Disposition: FIXED.** (proposal §5/§7, ADR Decision 4 C-2.)

---

### LOW

#### R3-3 · the central fold's exhaustiveness is carried by a duplicate raw-UPDATE at `customer/orders.ts:300-304` → **ACCEPT-RISK + DEFER-FLAG (consolidation follow-up + no-new-raw-UPDATE guardrail)**
**Grounded:** `customer/orders.ts:300-304` (verified) is a raw `UPDATE orders SET status='CANCELLED',
cancelled_at=now(), cancellation_reason=$1 WHERE id=$2`, guarded by `status==='IN_DELIVERY'` at `:286`,
**outside** `updateOrderStatus`. It carries its **own** terminalize block (`:309-318`, status set
`('assigned','accepted','picked_up')` — omits `'offered'`, harmless here: at `IN_DELIVERY` the binding is
always `'picked_up'`, never `'offered'`) and, critically, a cash-reversal coupling at `:307-318`:
`SET LOCAL app.settlement_reversal='true'` to bypass the cash-immutable check while it sets
`cash_collected=false, cash_amount=NULL`.

**Decision — ACCEPT-RISK + DEFER-FLAG (do NOT consolidate inside v2).** The disposition the breaker
offered, chosen deliberately:
- **Why not consolidate now:** the duplicate is not a pure audit/WS gap — it is welded to a **money-path
  cash-reversal** (`settlement_reversal` GUC + `cash_collected=false` on the binding). Routing it through
  `updateOrderStatus` would require the central R2-3 fold to **own cash-reversal semantics** (set
  `cash_collected=false`/`cash_amount=NULL` + emit the GUC) for *every* `IN_DELIVERY→CANCELLED` it
  terminalizes. That expands a 🔴 money/red-line primitive for a **LOW** cosmetic gain (history + WS delta
  on one customer self-cancel path), against ponytail/minimum-viable and the "don't roll money risk for a
  LOW" rule. The post-condition (zero active binding after `IN_DELIVERY→CANCELLED`) **holds today** via the
  duplicate, and the R2-3 no-strand guardrail is a post-condition test that passes through it.
- **Residual accepted, named:** a customer self-cancel of an `IN_DELIVERY` order is audit/realtime-
  inconsistent with every other cancel (no `order_status_history` row, only `ORDER_CANCEL_AFTER_DISPATCH`,
  not the standard `order.status` WS delta), and the two terminalize blocks now carry **different** status
  sets that must be kept in sync by hand. Bounded, pre-existing, customer-facing-only (no money or
  isolation impact). Owner: API.
- **Consolidation follow-up (DEFER-FLAG):** a dedicated change extends the central fold (or a shared
  `terminalizeBindingWithCashReversal` helper) to own the cash-reversal, then routes `customer/orders.ts`
  self-cancel through `updateOrderStatus` — reviewed as a **money-path change with its own proof + Triadic
  Council**, not folded into v2. Tracked as R-16.
- 🔴 **Guardrail (added now, red→green — prevents the class from growing):** a `tools/eslint-plugin-local`
  / grep-gate asserting **no NEW raw `UPDATE orders SET status … 'CANCELLED'` reaching from an
  `IN_DELIVERY`-guarded path outside `updateOrderStatus`** is introduced. The single existing site
  (`customer/orders.ts:300-304`) is the **named, frozen, grandfathered** exception (allow-listed by exact
  location); any new occurrence is RED. This caps the duplicate at exactly one and forces every future
  `IN_DELIVERY→CANCELLED` writer through the central fold.
**Disposition: ACCEPT-RISK (residual) + DEFER-FLAG (R-16 consolidation) + guardrail.** (proposal §9/§10
R-16, ADR Consequences.)

---

### Round-3 disposition summary

| Finding | Sev | Disposition |
|---|---|---|
| R3-1 GPS-anonymize sweep matches 0 rows under tenant FORCE; guardrail measures schedule-existence | HIGH 🔴 | **FIXED** — `SECURITY DEFINER anonymize_stale_delivery_trace(interval)` (pinned search_path, REVOKE/grant-mirror) + OUTCOME-based efficacy guardrail (zero non-null GPS past window across tenants) |
| R3-2 `/abort` from `accepted` forces illegal/same-status `READY` → 400 rollback, assignment never freed | MED | **FIXED** — terminalize binding unconditionally first; order-side action guarded on the order's actual status (no forced transition); flag-ON accept = no-transition branch |
| R3-3 central fold's exhaustiveness carried by a duplicate raw-UPDATE (cash-reversal-coupled) | LOW | **ACCEPT-RISK** (residual, pre-existing, customer-facing-only) + **DEFER-FLAG** (R-16 consolidation @ own Council) + **guardrail** (no NEW raw IN_DELIVERY→CANCELLED UPDATE; existing site frozen) |

**Every §E red line re-checked and intact** after round 3: no verdict engine, human-tap authority,
no-trap-states (R3-2 — abort always frees the binding, never throws), friction-not-verdict, crumbs passive,
status-guarded transitions, claim-check, money integer `CHECK(>=0)` (R3-3 — cash-reversal NOT widened for a
LOW), RLS FORCE (R3-1 — the privileged `SECURITY DEFINER` sweep is the sanctioned cross-tenant maintenance
mechanism, narrowly scoped, not a normal-path bypass), `SET search_path` pinned on the new DEFINER fn,
zero cookies, RS256, Zod `.strict()`, parameterized SQL, anonymize-not-delete (now enforced by a sweep that
*provably reaches the rows* + an efficacy guardrail).

**NEEDS-HUMAN (unchanged):** (1) Stage-21 `ADR-stage21-reconciliation.md` (`NO-AUTO-DEDUCT`+
`NO-COURIER-SCORING` — failing guardrail red-on-disk). (2) verify embedded-staff employment assumption.
(3) flip `COURIER_OFFER_HANDSHAKE_ENABLED` only when the accept/decline + `/abort` courier UI ships.

**converged: yes — 0 open CRITICAL / 0 open HIGH after this round** (R3-1 HIGH fixed; R3-2 MED fixed; R3-3
LOW accept-risk+defer-flag with a guardrail). All round-1 CRITICALs, round-2 HIGHs, and the round-3 HIGH
are closed at design level; the only carried items are the named ACCEPT-RISK residuals + the NEEDS-HUMAN
Stage-21 artifact (already a red-on-disk guard).

---

## RESOLVE round 4 (residuals)

> ARCHITECT seat — final residual disposition. The council has hit hard-exit (0 CRITICAL/HIGH). This round
> disposes the four MED/LOW residuals from BREAKER "RE-ATTACK round 4" so nothing is a loose end. **It does
> NOT reopen the loop** — all four are flag-gated / guardrail-precision / cosmetic, none touches a red line.
> Two fixes-in-spec changed the design (proposal §5/§7/§9 + ADR Decision 4 / Carried constraints / Migration
> updated this round); the prose correction on the DEFINER bypass is folded into proposal §5/§8 + ADR.

### R4-3 · [MED] flag-ON no-transition abort branch is signal-silent + not auto-re-offered → **FIX-IN-SPEC**
**Real coherence gap on the new `/abort` path.** The decline/reject path re-enqueues to
`courier_dispatch_queue` (`assignments.ts:193-197`, the only enqueue site) so the dispatch worker auto-re-offers,
and publishes a delta. The round-3 abort flag-ON `accepted` (pre-pickup) branch took only the no-transition
writes (`assignment='cancelled'`, `shift='available'`, `courier_id=NULL`) and — because it does **not** call
`updateOrderStatus` — emitted **no event** and did **not** re-enqueue → owner/customer go stale and the order
is not auto-re-offered (asymmetric with decline). **Fix (proposal §5 R2-2, §7 abort row; ADR Decision 4 R4-3):**
that branch must, in the same tx, **(a)** publish a binding-change broadcast over `MessageBus` — the id-only
`ORDER_STATUS` delta for the order's (unchanged) status so owner/customer realtime reconverges and the
"courier dropped" change is visible — **and (b)** `INSERT INTO courier_dispatch_queue (order_id, …)` exactly
as the decline path does, so the dispatch worker re-offers through the **same** mechanism. After this, abort
and decline **converge**: terminalized binding + freed shift + broadcast + back in the assignable pool.
"Re-offerable" now holds in the *mechanism* sense, not merely the *eligibility* sense. **Disposition: FIXED.**

### R4-1 · [MED] efficacy guardrail is non-discriminating unless run under a NOBYPASSRLS role → **FIX-IN-SPEC (precondition)**
The R3-1 outcome guardrail invokes the sweep "the way the worker does — operational pool, no `app.user_id`."
But the operational pool role carries `BYPASSRLS` today (migration 070's crux comment). Under `BYPASSRLS` the
round-2 **raw context-free `UPDATE`** (the regression the guardrail must catch) ALSO anonymizes every
cross-tenant row → the test is GREEN against **both** the broken raw UPDATE and the DEFINER fix; it proves
"rows got anonymized," not "the DEFINER routing reached them," and would not go RED on a revert to the raw
UPDATE. **Fix (proposal §9 R3-1 efficacy test; ADR Carried constraints R4-1):** the efficacy test's
operational caller **MUST run under a NOBYPASSRLS role** (the proven P6 `provision-rls.test.ts` pattern). Then
a context-free raw `UPDATE` genuinely sees **0 rows → RED**, and only the DEFINER fn (reaching rows via its
*owner's* privilege, not the caller's) → **GREEN**. The red→green claim is now guaranteed.
**Disposition: FIXED (guardrail precondition stated).**

### R4-2 · [LOW] `p_window` has no dispute-window floor → **FIX-IN-SPEC (clamp in the fn)**
A valid-but-too-small `DELIVERY_TRACE_GPS_RETENTION` (e.g. `'1 day'`) would silently over-anonymize GPS
*inside* the 7-day dispute window, destroying evidence early. **Fix (proposal §5 SQL; ADR Migration R4-2):**
the function clamps the effective window to `v_window := GREATEST(p_window, interval '7 days')` (the dispute
window) — chosen over a caller-side clamp so **every** caller is protected, not just the one worker. A mis-set
env can no longer anonymize inside the dispute window; a malformed value still throws safely on the `::interval`
cast. **Disposition: FIXED (floor in the fn).**

### R4-4 · [LOW] abort's legacy `IN_DELIVERY→READY` branch leaves `orders.courier_id` stale → **FIX-IN-SPEC (clear it)**
`updateOrderStatus` does not touch `orders.courier_id`, so the abort `accepted-while-IN_DELIVERY→READY`
(legacy flag-OFF) branch left a re-offerable READY order pointing at the departed courier. Not a trap (dispatch
keys off `courier_assignments` active rows, not `orders.courier_id`; PII is gated on `courierActive`), so
ACCEPT-RISK was available — but the fix is one statement and removes a stale-mirror drift class, so **chosen to
fix** (boring-correct over carrying cosmetic drift). **Fix (proposal §5 R2-2, §7 abort row; ADR Decision 4
R4-4):** that branch also runs `UPDATE orders SET courier_id=NULL`, converging with the flag-ON branch and the
original C-2 revert (`SET status='READY', courier_id=NULL`). **Disposition: FIXED.**

### Prose correction folded — the DEFINER sweep bypasses FORCE via the function-owner's privilege
**Stated explicitly now (proposal §5 comment + §8; ADR Carried constraints + Migration):** `SECURITY DEFINER`
**alone does NOT bypass `FORCE`** — FORCE exists precisely to subject the table OWNER to RLS. The sweep reaches
all-tenant rows **only because the function's OWNER role (the migration `postgres`/admin) carries
`BYPASSRLS`/superuser.** The earlier "executes as the privileged owner → bypasses FORCE" prose is imprecise; it
correctly decouples from the *operational* role's future NOBYPASSRLS, but it **depends on the function-owner
being superuser/`BYPASSRLS`**. 🔴 **Function-owner-is-privileged assumption made explicit:** migrations run as
a privileged (BYPASSRLS/superuser) owner (standard Supabase/Fly deploy); run by a NOBYPASSRLS non-superuser
role, the sweep silently anonymizes 0 rows — the exact R3-1 false-green. No longer a silent assumption.

### Round-4 disposition summary

| Finding | Sev | Disposition |
|---|---|---|
| R4-3 flag-ON no-transition abort branch signal-silent + not auto-re-offered | MED | **FIXED** — broadcast `ORDER_STATUS` delta + re-enqueue to `courier_dispatch_queue` (converge with decline) |
| R4-1 efficacy guardrail non-discriminating under BYPASSRLS | MED | **FIXED** — efficacy test's operational caller runs NOBYPASSRLS (provision-rls pattern) |
| R4-2 `p_window` has no dispute-window floor | LOW | **FIXED** — fn clamps `GREATEST(p_window, '7 days')` |
| R4-4 abort legacy `IN_DELIVERY→READY` leaves `orders.courier_id` stale | LOW | **FIXED** — branch also `SET courier_id=NULL` |
| DEFINER-bypass prose / function-owner-privilege assumption | correction | **FOLDED** — stated explicitly in proposal §5/§8 + ADR (owner-BYPASSRLS, not DEFINER-alone) |

**No CRITICAL/HIGH reopened.** Every §E red line re-checked and intact: no verdict engine, human-tap authority,
no-trap-states, friction-not-verdict, crumbs passive (+ retention sweep that provably reaches rows, now with a
dispute-window floor and a discriminating efficacy guardrail), status-guarded transitions, claim-check (the
R4-3 broadcast + re-enqueue payloads are id-only), money integer `CHECK(>=0)`, RLS FORCE (the DEFINER sweep's
owner-privilege bypass is the sanctioned cross-tenant maintenance path, assumption now explicit), pinned
`search_path`, zero cookies, RS256, Zod `.strict()`, parameterized SQL, anonymize-not-delete.

**converged: YES — council closed. 0 open CRITICAL / 0 open HIGH; all four round-4 residuals FIXED-IN-SPEC.**
NEEDS-HUMAN carried (unchanged): (1) Stage-21 `ADR-stage21-reconciliation.md` (`NO-AUTO-DEDUCT` +
`NO-COURIER-SCORING`, failing guardrail red-on-disk); (2) verify embedded-staff employment assumption;
(3) flip `COURIER_OFFER_HANDSHAKE_ENABLED` only when the accept/decline + `/abort` courier UI ships.

---

## RESOLVE round 5 (implementation drift) — design↔shipped-code reconciliation

> ARCHITECT seat. **Different axis from rounds 1–4.** Those hardened the *design* (now converged, 0
> CRIT/HIGH). This round disposes a **drift audit** (4 read-only lanes, Playwright/grep-grounded) of the
> SHIPPED code on `feat/mvp-sensor-seams` (L1–L5b + §A offer handshake) against the converged resolution.
> Findings are **implementation drift**, not design holes: several contradict items rounds 1–4 marked
> **FIXED** that did not ship that way, plus two guardrails the ADR/resolution assert were "materialized
> NOW" but were never written. **No production code in this round** — it re-dispositions; the fixes are the
> next gate (commit→staging→proof). Each row re-grounded against live source, file:line inline.

### 0. Root cause (round 5), named

Two distinct drift classes, both *between* design and code, not within the design:

1. **Per-path drift on the completion/cancel surface.** The same structural lesson rounds 1–2 learned for
   the *delivered* path (centralize the invariant so no caller can bypass it — R2-1/R2-3) was **not**
   applied to the two *exit* paths the audit re-checked: the legacy courier `/cancel`
   (`assignments.ts:441-457`) and the owner-reassign displaced-order revert (`owner/dashboard.ts:283-288`)
   each still hand-roll a raw, machine-bypassing exit. `/abort` (the new path) is correct; its older
   siblings were never brought onto the same rail.
2. **Specified-but-unbuilt durable artifacts.** Two guardrails the resolution names as the *authority* for
   a red line (M-3a behavioral signal-independence test; the C1/Q5 anti-scoring-creep ledger-`type` ban)
   are absent on disk. The design converted "intent→artifact"; the implementation did not author the
   artifact. The red lines hold *in practice today* (delivery_trace is read by nothing; no non-`'hold'`
   write exists) but their *deterministic gate* is missing — exactly the "honest by intent, not by rule"
   state the council said it had left.

Fix posture: **conform code to the converged design** (the design was right — D1/D2/D3/D4/D5 are
FIX-IN-CODE), and **ratify two deliberate code divergences that are actually better** (the 400-via-enum
forbidding of `paid_partial`; the courier-membership FORCE policy correction) by amending the design to
match, so the artifact stops over-claiming.

### HIGH (red-line)

#### D1 · `/cancel` re-creates the C-2 trap + emits a false `ORDER_CANCELLED` (flag-OFF, the launch default) → **FIX-IN-CODE (red-line; converge `/cancel` onto the `/abort` rail)**
**Grounded (confirmed this round):** `apps/api/src/routes/courier/assignments.ts:441-457` — the legacy
`/cancel` handler, inside the 5-min accept-regret window, updates **only** `courier_assignments.status='cancelled'`
(`:441-445`) + frees the shift (`:447-449`), then **unconditionally** `messageBus.publish(BUS_CHANNELS.ORDER_CANCELLED…)`
(`:453-457`). It **never** calls `updateOrderStatus`, so an order an owner-direct force-assign drove to
`IN_DELIVERY` (`owner/dashboard.ts`, flag-OFF) is left `IN_DELIVERY` with no active binding (the central
R2-3 fold runs only *inside* `updateOrderStatus`, which this path does not enter), **and** the customer
receives a "cancelled" for an order that is not cancelled. This is **exactly** the two things C-2 layer-1
(resolution §1 C-2: "the courier `cancel` handler, in the **same tx**, reverts the order mirror,
status-guarded … routed through `updateOrderStatus`") and R2-5 (resolution §R2-5: "the revert path no
longer hand-publishes `ORDER_CANCELLED`") were marked **FIXED** for — on the path they were specified for.
**Severity calibration (honest):** post-C-1 the order is *recoverable* by a manual owner reassign (terminal
rows no longer block the new INSERT), so this is a **soft trap (needs manual intervention) + a definitely-wrong
notification**, not the "stuck forever" of the pre-fix C-2 — but it still breaches the §A no-trap red-line's
intent (auto-revert to assignable) and ships a customer-facing lie.
**Fix (conform to the C-2/R2-2/R2-5 design already written for `/abort`):** `/cancel`, in the same tx,
takes the **conditional order-side action** `/abort` already implements (`assignments.ts:509-526`):
terminalize the binding (already done) + free shift, then `updateOrderStatus(client, order_id, loc,
'READY', {comment:'courier_cancelled_pre_pickup'})` **only when the order is `IN_DELIVERY`** (legal widened
edge; flag-ON queue-path order still `CONFIRMED`/`READY` → no-transition branch, clear `courier_id`), and
**delete the unconditional `ORDER_CANCELLED` publish** (let `updateOrderStatus` emit the correct `READY`
event, or — no-transition branch — the same `binding_changed` broadcast `/abort` uses). After this `/cancel`
and `/abort` differ only in the time-gate (accept-regret vs en-route), as the design intended.
**Guardrail (red→green):** flag-OFF test — owner-direct force-assign → courier `/cancel` within window →
assert order `READY` (NOT `IN_DELIVERY`), assert **no** `ORDER_CANCELLED` emitted, assignment `'cancelled'`,
order re-offerable. RED against the current handler, green after the converge. **Disposition: FIX-IN-CODE.**

### MED

#### D2 · owner-reassign displaced-order revert is still a raw `UPDATE` bypassing machine+history+WS → **FIX-IN-CODE (R2-6 was marked FIXED, not shipped)**
**Grounded (confirmed this round):** `owner/dashboard.ts:283-288` — the busy-courier displaced-order revert
is `UPDATE orders SET status='READY', courier_id=NULL WHERE id=$1`, a raw write — **no** `order_status_history`
row, **no** `order.status` WS delta → the displaced order's customer is stranded on stale "out for delivery".
This is precisely the construct R2-6 ("replace with `updateOrderStatus(… 'READY' …)` … History + WS now
fire") marked **FIXED**. The binding is hand-terminalized at `:274-282`, so there is **no stranded active
binding** (not a trap) — the gap is audit + realtime coherence only, hence MED not HIGH.
**Fix (conform to R2-6):** replace `:283-288` with `await updateOrderStatus(client, old.order_id, locationId,
'READY', { messageBus, comment:'owner_reassigned' })` (the widened `IN_DELIVERY→READY` edge makes it legal;
the central fold makes the explicit `:274-282` terminalize idempotent) followed by `UPDATE orders SET
courier_id=NULL WHERE id=$1`. **Guardrail:** assert a displaced `IN_DELIVERY` order reverted by reassign has
an `order_status_history` row + a `READY` WS delta. **Disposition: FIX-IN-CODE.**

#### D3 · owner `/deliver` body has no Zod schema → M-2 edge guard + `.strict()` + `payment_outcome` enum absent on one money path → **FIX-IN-CODE (R2-1/M-2 parity was specified, not shipped)**
**Grounded (confirmed this round):** `owner/dashboard.ts:445` registers the route with only `config:{rateLimit}`
and reads `request.body as any` / `body?.cash_amount` raw — **no** `schema` block. The resolution (R2-1,
this file: "*The owner `/deliver` body gains the same first-class `payment_outcome` field + `.int().nonnegative()`
`cash_amount` as the courier body*"; round-2 summary "*`.int().nonnegative()` (now on the owner path too)*")
required parity. Consequences on the un-validated path: a negative `cash_amount` trips the DB `CHECK` as an
ungraceful **500** (not a 422/400); a `paid_partial` sent here is not `'paid_full'` → coerced to the no-cash
tail → **silent `CANCELLED` write** (a mutation), not a reject-before-write. The cash-as-proof *ledger
invariants* still hold (both paths route through `completeDelivery`, idempotent `'hold'`); the gap is the
**edge contract**.
**Fix (conform):** attach the courier `/delivered` body schema to the owner `/deliver` route verbatim —
`z.object({ payment_outcome: z.enum(['paid_full','refused_goods','refused_payment','customer_cancelled_on_door']).optional(),
cash_collected: z.boolean().optional(), cash_amount: z.number().int().nonnegative().optional() }).strict()`.
This also closes the silent-cancel divergence: `.strict()` + the enum reject a `paid_partial`/float/negative
at the edge **before** the handler coerces it. **Guardrail:** owner `/deliver` with `cash_amount:-5` → 400
(not 500); with `payment_outcome:'paid_partial'` → 400 (not a silent CANCELLED). **Disposition: FIX-IN-CODE.**

#### D4 · C1/Q5 anti-scoring-creep ledger-`type` ban never written → **FIX-IN-CODE (fold into the SHIPPED `guardrail-deliver-v2.mjs`, not a new eslint rule)**
**Grounded:** the resolution (C1/Q5) + ADR (Carried constraints) assert an `eslint-plugin-local` rule banning
any `INSERT INTO courier_cash_ledger` with `type` ≠ `'hold'` **and** any state/penalty write derived from a
`delivery_trace`/signal-row column — "materialized NOW." It is **absent** (`tools/eslint-plugin-local/src/index.js`
has no such rule). What *did* ship is `scripts/guardrail-deliver-v2.mjs` (wired into `verify:all`) — a
grep-gate that bans ledger/trace INSERTs **outside `completeDelivery`** (R2-1 parity), a *different*
invariant: it does not discriminate on `type` and has no anti-scoring check.
**Decision — FIX-IN-CODE, but in the cheaper shipped artifact:** extend the existing, already-wired
`guardrail-deliver-v2.mjs` with two new grep assertions — (a) no `courier_cash_ledger` insert with a `type`
literal other than `'hold'`; (b) no penalty/score/deduct write whose value references a `delivery_trace`/
`order_sensor_events`/`customer_signals` column — rather than standing up a new eslint plugin rule (YAGNI /
ponytail: reuse the wired gate). **Honest scope note:** the *harm* this guards is downstream Stage-21 (deliver-v2
writes only `'hold'`), and the record is already gated by the red-on-disk `stage21-no-auto-deduct.invariant.test.ts`
— so this is **defense-in-depth, lower urgency than D1–D3**, but it is the "durable barrier the deduction-builder
trips" the council promised, so it ships, not defers. **Disposition: FIX-IN-CODE (extend existing gate).**

#### D5 · M-3a behavioral signal-independence test (named the AUTHORITY) never written → **FIX-IN-CODE**
**Grounded:** resolution M-3a names a behavioral test "*the authority*" for "never build the verdict engine":
mutate a `delivery_trace`/signal row → assert the delivered/transition outcome is **unchanged** (outcome is a
pure function of courier-tapped input + server-authoritative order columns). Neither it nor its advisory-lint
backup exists. The red line **holds today** (no decision path reads a signal row — grep-verified across rounds),
so this is FIX-IN-CODE at MED, not a live breach.
**Fix:** author the test in the deliver-v2 suite — seed a completed order, mutate its `delivery_trace.gps_lat`
(and an `order_sensor_events` row), re-run/inspect the transition outcome, assert byte-identical. Pair it with
the (advisory) signal-column-in-state-branch grep as backup. **Disposition: FIX-IN-CODE.**

### Ratified divergences (code diverged and is BETTER — amend the design, don't change the code)

#### D-R1 · `paid_partial` forbidding is `400`-via-enum-omission, not the named `422 PARTIAL_NOT_SUPPORTED` → **RATIFY + AMEND ADR/resolution**
The ADR/resolution named a `422 PARTIAL_NOT_SUPPORTED`. Shipped: the courier body Zod enum simply omits
`paid_partial`/`pending`, so `.strict()` rejects with a **400** before the handler. This is the **leaner,
single-source-of-truth** mechanism (the enum *is* the allowed set; no custom error code to keep in sync), and
once D3 lands it is **uniform across both money paths**. **Decision: ratify the enum-omission→400 contract;
amend the ADR/resolution to drop the `422 PARTIAL_NOT_SUPPORTED` name.** `CASH_AMOUNT_MISMATCH` (422) remains
the one explicit completion error. No code change beyond D3.

#### D-R2 · `courier_assignments` FORCE policy admits TWO contexts (`current_tenant` OR member-locations), not `app_member_location_ids()` alone → **RATIFY + AMEND ADR**
**Grounded:** migration `1790000000073:36-49` documents that couriers are not org *members* (they live in
`courier_locations`, not memberships), so an `app_member_location_ids()`-only policy under FORCE would break
**all** courier access. The two-context policy (`location_id = current_setting('app.current_tenant')` for the
courier session **OR** `location_id = ANY(app_member_location_ids())` for the owner) is a **correct grounding
fix**, not a weakening — and the cross-courier IDOR it could be feared to open is closed independently by the
M-1 predicate (`AND courier_id=$me` on the locking SELECT of every courier mutation), which the audit verified
PASS (`assignments.ts:144/193/254/323/425/491/563`). **Decision: ratify; amend the ADR Migration/Red-lines
prose from "align to `app_member_location_ids()`" to the two-context policy + restate that isolation for the
new surface rests on the M-1 predicate, not the policy.**

#### D-R3 · `/abort` flag-ON re-offer broadcast is a custom `binding_changed` event, not an `order.status` delta → **RATIFY + DEFER-FLAG (FE must handle `binding_changed`)**
The R4-3 fix asked the no-transition branch to "publish a binding-change `ORDER_STATUS` delta." Shipped: a
`{ type:'binding_changed', orderId }` event on `orderChannel` (`assignments.ts:539`) — id-only (claim-check
clean) and on the right channel, but a distinct event type. Functionally satisfies R4-3 (owner/customer get a
realtime nudge + the order is re-enqueued). **Decision: ratify the event shape; DEFER-FLAG that the owner +
customer FE must subscribe to/handle `binding_changed` (re-fetch the order) for reconvergence** — tracked as an
FE task, not a backend change. Only relevant once `COURIER_OFFER_HANDSHAKE_ENABLED` flips.

### LOW / housekeeping (carried)

- **completion-parity test exercises the `completeDelivery` *primitive*, not the owner-proxy *HTTP handler***
  → **FIX (fold into D3's proof):** the owner-path discrimination currently rests on the static grep-gate +
  call-site grep. Strengthen `deliver-completion.test.ts` (or D3's new test) to POST the owner `/deliver`
  route and assert the `'hold'` + crumb, so parity is *behaviorally* proven on both paths.
- **regression-ledger row 26 under-names guardrails** (omits the R3-3 no-new-raw-cancel gate + the D4 ban) →
  **housekeeping:** add rows when D1–D5 land.

### Round-5 disposition summary

| Finding | Sev | Disposition |
|---|---|---|
| D1 `/cancel` re-creates C-2 trap + false `ORDER_CANCELLED` (flag-OFF default) | HIGH 🔴 | **FIX-IN-CODE** — converge `/cancel` onto the `/abort` rail (conditional `updateOrderStatus` revert + drop unconditional publish) + flag-OFF guardrail |
| D2 owner-reassign displaced revert raw `UPDATE` bypasses machine+WS | MED | **FIX-IN-CODE** — route through `updateOrderStatus` + clear `courier_id` (conform to R2-6) |
| D3 owner `/deliver` body unvalidated (no `.int().nonnegative()`/`.strict()`/enum) | MED | **FIX-IN-CODE** — attach courier body schema (also closes silent-CANCELLED-on-`paid_partial`) |
| D4 anti-scoring-creep ledger-`type` ban absent | MED | **FIX-IN-CODE** — extend the shipped `guardrail-deliver-v2.mjs` (not a new eslint rule) |
| D5 M-3a behavioral signal-independence authority test absent | MED | **FIX-IN-CODE** — author it + advisory grep backup |
| D-R1 `paid_partial` forbidding is 400-via-enum, not `422 PARTIAL_NOT_SUPPORTED` | — | **RATIFY + AMEND** — enum-omission is leaner; drop the 422 name |
| D-R2 `courier_assignments` FORCE two-context policy | — | **RATIFY + AMEND** — grounding fix (couriers ≠ members); IDOR closed by M-1 predicate |
| D-R3 `/abort` `binding_changed` event vs `order.status` delta | — | **RATIFY + DEFER-FLAG** — FE must handle `binding_changed` (flag-ON only) |
| parity test hits primitive not owner HTTP path; ledger row under-named | LOW | **FIX (fold into D3 proof) + housekeeping** |

**Red lines (§E) re-checked against SHIPPED code this round:** no verdict engine (holds in practice — no
signal-row decision path; D5 restores the *deterministic* gate the council promised), human-tap authority
(intact), **no state is a trap** (D1 is the one breach — soft trap + false-cancel on `/cancel`; FIX-IN-CODE
red-line), friction-not-verdict (intact), crumbs passive (intact; retention sweep verified faithful — R2-7/
R3-1/R4-1/R4-2 0 findings), status-guarded transitions (intact; D2 routes the one raw revert back onto the
machine), money integer `CHECK(>=0)` (DB holds; D3 restores the graceful edge contract), RLS FORCE (intact;
D-R2 ratified as a grounding fix, IDOR closed by M-1), claim-check (intact — `binding_changed` is id-only),
RS256 / zero cookies / Zod `.strict()` (D3 restores `.strict()` on the owner path) / parameterized SQL
(intact), anonymize-not-delete (verified faithful).

**Faithful-as-shipped (no drift — attacked, held):** the retention sweep (DEFINER fn + `GREATEST` floor +
NOBYPASSRLS efficacy test), the `order-machine` `IN_DELIVERY` widen + the central R2-3 terminalize fold,
`/abort` (R3-2/R4-3/R4-4 conditional-not-forced, terminalize-first, broadcast+re-enqueue+`courier_id`-clear),
C-1 partial-unique swap, C-3 guarded terminalize-then-insert, M-1 courier-IDOR locking-SELECT guards, the
money core (single `completeDelivery` primitive, both paths unified, one idempotent `'hold'`, no-cash tail →
CANCELLED, `payment_outcome` persisted to both tables, server-authoritative total), and the Stage-21
pending-guardrail (correctly RED-on-disk).

**NEEDS-DO (code — the next gate, NOT this round):** D1 (red-line first) → D2 → D3 → D4 → D5, each with its
red→green guardrail; then the two ADR amendments (D-R1/D-R2) + the D-R3 FE flag; then commit → staging deploy
→ Playwright + unit/integration proof (Mandatory Proof Rule). **NEEDS-HUMAN (unchanged from round 4):**
Stage-21 `ADR-stage21-reconciliation.md`; embedded-staff verification; `COURIER_OFFER_HANDSHAKE_ENABLED`
flip-readiness.

**converged: drift round closed at design level — 1 HIGH (D1) + 4 MED (D2–D5) FIX-IN-CODE, 3 ratified
divergences amended, 0 design changes. The design stays converged (rounds 1–4); the implementation owes
5 conformance fixes + 2 guardrails before the spine matches the artifact it was built from.**
</content>
</invoke>
