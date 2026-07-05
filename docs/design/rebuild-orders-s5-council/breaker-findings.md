# S5-ORDERS/MONEY Port — Council Packet · BREAKER FINDINGS (Round 1)

> **System Breaker DeliveryOS.** Adversarial attack on the S5 crown-jewel packet
> (`proposal.md` / `open-questions.md` / `threat-model.md`). Every finding is demonstrable
> (concrete input/scenario or back-of-envelope number) with `file:line` evidence. **No fixes** —
> the finding states HOW it breaks and WHICH invariant/claim is violated; the architect fixes.
> Read-only verification against the live tree + migrations. Precedent: S3/S4 = 2 CRIT each.

**Severity counts: 1 CRITICAL · 2 HIGH · 3 MEDIUM · 2 LOW.**

The single CRIT is not a runtime bug — it is a **false premise under a 🔴 operator-signed,
port-blocking decision (Q3)**: the packet has the create-path RLS predicate backwards, so the
proposed fix and its named DoD probe defend the wrong GUC while the real B3 failure mode is
inverted and unguarded.

---

## CRITICAL

### C1 · B-SEC / B-OPS — Q3 create-side RLS premise is FALSE; the anonymous-INSERT policies gate on `app_current_user() IS NULL`, not on `app.current_tenant`

**Packet claim (§8, Q3, Q-TENANT-SEAT, threat S5-T1):** "Post-B3-flip the `orders`/`order_items`/
`order_item_modifiers`/`idempotency_keys`/`velocity_events`/`customer_track_grants` INSERTs **need
`app.current_tenant` seated** to satisfy `WITH CHECK`, or the entire checkout silently matches 0
rows / raises" → therefore route the create tx through `with_tenant(app.current_tenant=locationId)`,
verified by a NOBYPASSRLS probe that "asserts the GUC is seated and the INSERT affects the row."

**Ground truth — every create-path write policy keys on the principal being anonymous, NOT on tenant match:**
- `anonymous_insert ON orders … WITH CHECK (app_current_user() IS NULL)` — `1780315000000_customer-rls.ts:6-7`
- `anonymous_insert ON customers … WITH CHECK (app_current_user() IS NULL)` — `:16-17`
- `anonymous_insert ON idempotency_keys … WITH CHECK (app_current_user() IS NULL)` — `:21-22`
- `anonymous_update ON customers … USING (app_current_user() IS NULL)` (upsert `ON CONFLICT DO UPDATE`) — `1780338981782_customer-anonymous-update.ts:6-7`
- `anonymous_select ON orders … USING (app_current_user() IS NULL)` (parent-visibility for `order_items` INSERT) — `1780338981783_anonymous_orders.ts:5-6`
- `anonymous_insert ON velocity_events / order_item_modifiers / customer_track_grants … WITH CHECK (app_current_user() IS NULL)` — `1790000000077_rls-nobypassrls-phase1-policies.ts:18,20,22`
- `app_current_user()` = `NULLIF(current_setting('app.user_id', true), '')::uuid` — `1780310071220_core-identity.ts:70-72`

`order_items`' `anonymous_insert WITH CHECK (EXISTS (SELECT 1 FROM orders o WHERE o.id = order_items.order_id))`
(`1780315000000_customer-rls.ts:11-13`) resolves the parent through the `anonymous_select ON orders`
policy above — which itself requires `app_current_user() IS NULL`.

**Break / number:** Permissive policies OR-combine. The live create tx opens `db.connect() → BEGIN →
SET LOCAL statement_timeout` with **no `set_config` at all** (`orders.ts:108-124`) — it sets neither
`app.user_id` nor `app.current_tenant`. Post-B3 (NOBYPASSRLS), each create INSERT is admitted iff
ANY permissive `WITH CHECK` passes. `tenant_isolation` (`1780310074262_orders.ts:83-84`,
`location_id IN app_member_location_ids()`) fails for a non-member, **but `anonymous_insert`
(`app_current_user() IS NULL`) PASSES** because a context-free create never sets `app.user_id`.
**The context-free create does NOT 0-row post-B3 — it commits.** The claimed "total silent checkout
outage" (S5-T1, the stated reason to reject Q3(b)) does not exist for the INSERT path.

Three consequences that make this CRITICAL for a red-line sign-off:
1. **Wrong-GUC fix.** Seating `app.current_tenant=locationId` is **inert** for every create-path
   INSERT/UPDATE/SELECT policy — none reads `app.current_tenant`. The Q3 "FIX-IN-PORT" defends a
   predicate that is never consulted on this path.
2. **Non-discriminating DoD probe → false assurance.** The specified NOBYPASSRLS probe ("assert
   `app.current_tenant` seated + INSERT affects the row") passes **whether or not the seat is
   load-bearing** — the INSERT affects the row regardless, via the anon arm. The gate cannot fail on
   the very thing it certifies, so it green-lights the port while proving nothing.
3. **The real B3 failure mode is INVERTED and unguarded.** Because the anon policies gate on
   `app_current_user() IS NULL`, the way to actually 0-row the create under NOBYPASSRLS is to **seat
   `app.user_id`** (an authenticated-customer create, or a `with_tenant`/`with_user` confusion of the
   S3 REV-10 combinators). Then `app_current_user()` is non-NULL → `anonymous_insert` fails →
   `tenant_isolation` requires membership the customer lacks → **INSERT raises/0-rows.** The packet
   frames the hazard as "forgot to seat the tenant GUC"; the actual hazard is "accidentally seated the
   user GUC," which the packet neither warns about nor tests.

**Invariant violated:** the Q3 🔴 decision (declared *port-blocking* in the decision-ordering note)
is presented to the operator on a factually incorrect RLS predicate, with a DoD gate that provides
false B3-readiness assurance. B-SEC (tenancy correctness) + B-OPS (a scaling/flip gate that does not
actually latch the invariant it claims).

---

## HIGH

### H1 · B-CONSIST — request-hash byte-fidelity cannot hold across Node↔Rust: the hash embeds JS-`JSON.stringify`-serialized f64 lat/lng, and a single golden vector cannot cover the float domain

**Packet claim (§6.1, §9.2, DoD §12, threat S5-T5/T9):** request-hash byte-parity is a NAMED 🔴
cutover gate; the control is "a golden-vector test — a fixed cart → the exact Node hash string (both
directions)."

**Ground truth:** `buildRequestHash` canonicalizes with `JSON.stringify` over a body that includes
`pin: { lat: latRounded5, lng: lngRounded5 }` where `latRounded5 = Math.round(pin.lat*100000)/100000`
— **an f64** (`order-canonical.ts:37-38,40-50`), plus `items[].quantity` (a JS number) and
`cash_pay_with`. `pin.lat/lng` are free client input, Zod-bounded to `[-90,90]`/`[-180,180]`
(`legacy.ts:53-54`).

**Break (guaranteed for a class of inputs):** V8's `JSON.stringify` and Rust `serde_json`/ryu do NOT
agree byte-for-byte on f64:
- **Integer-valued coordinate:** `pin.lat = 42.0` → `Math.round(4200000)/100000 = 42` → JS emits
  `{"lat":42}`. A Rust f64 `42.0` serializes as `{"lat":42.0}` (ryu always emits a fractional part).
  **`42` vs `42.0` → different SHA-256.**
- **Negative zero:** a pin just below 0 (e.g. `lat = -0.000001`) → `Math.round(-0.1) = -0`, `-0/1e5 =
  -0` → JS `JSON.stringify(-0)` emits `0`; Rust f64 `-0.0` emits `-0.0`. **`0` vs `-0.0`.**
- **`quantity`/number typing unpinned:** if the Rust canonical models `quantity`/`cash_pay_with` as
  f64, `2` (JS) vs `2.0` (Rust) diverges on **every** order; the packet's "carry the exact inputs"
  never pins the wire numeric types.

**Scenario:** during the Q6 overlap a client's first `POST /orders` lands on Node, the retry (same
`idempotency_key`) lands on Rust. If the cart's pin has an integer-valued coordinate, Rust computes a
different `request_hash` → the `request_hash !== requestHash` branch fires (`orders.ts:402`) → **422
`IDEMPOTENCY_KEY_REUSED`** on a legitimate retry (customer blocked), or a duplicate order if the miss
lands the other way. The DoD's **single** golden vector (one fixed cart) provably cannot exercise the
integer/-0/typing corners where the divergence lives — the gate is green while the hazard is open.

**Note (packet self-inconsistency, folds into severity):** §1/§9.2 call this a "duplicate **PAID**
order," but §9.3 states cash does not charge at create and crypto is dark → **no charge moves at
create during the overlap.** The true worst live outcome is a duplicate *unpaid* order or a false
422, not a paid duplicate. Real, but the "PAID" framing overstates it → HIGH, not CRIT.

**Invariant violated:** Q4→Q6 "byte-identical `request_hash`" — the sole cross-stack double-order
guard — is not achievable via `JSON.stringify(f64)` parity, and its named control does not cover the
failure domain.

### H2 · B-CONSIST / B-DATA — Q5b scope misses the co-located `receiver` drift: the "deliver to someone else" flow 400s on `CreateOrderInput.strict()`, so ADR-0016 is not actually closed

**Packet claim (§7, Q5b):** the only checkout-communication drift is `customer.messenger_kind`
(3-value enum vs 6-kind selector); fix = "unify to ONE canonical 6-kind `MessengerKind`," DB CHECK
confirmed to admit 6.

**Ground truth — a second, broader drift from the SAME ADR-0016 / migration 074 lives one field over:**
- The web sends a top-level `receiver: { name, messenger_kind, handle }` when "deliver to someone
  else" is chosen — `CheckoutPage.tsx:332-334`.
- The route reads it: `receiverName/receiverMessengerKind/receiverHandle = (input as any).receiver?.*`
  — `orders.ts:592-594`; persisted to `orders.receiver_*` — `order-persistence.ts:88-109`.
- The DB is ready: `receiver_messenger_kind text CHECK (… IN (6 kinds))` — `1790000000074:33-36`.
- **But the request is parsed by `CreateOrderInput.parse(request.body)` (`orders.ts:93`), and
  `CreateOrderInput` is `.strict()` with NO `receiver` key** (`legacy.ts:40-77`, `.strict()` at :72).

**Break:** `.strict()` **throws** on unknown keys (it does not strip). So every non-same-receiver
order sends `receiver` → `CreateOrderInput.parse` throws `Unrecognized key(s): 'receiver'` → the
"deliver to someone else" order **fails at the boundary**, and `(input as any).receiver` at
`orders.ts:592-594` can never be populated (the write path for `receiver_*` is effectively dead; the
mig-074 columns are unreachable). Q5b(b) as scoped fixes only `messenger_kind`; a port that faithfully
mirrors `CreateOrderInput`'s strict shape **re-ships this break** — the packet declares the ADR-0016
checkout-communication surface handled while a core part of it stays broken.

**Invariant violated:** the fix-vs-carry rule ("FIX-IN-PORT for 🔴 correctness with a documented E2E
delta") is applied to `messenger_kind` but not to the strictly-worse, same-ADR `receiver` object — a
customer-facing order failure the port would carry silently.

---

## MEDIUM

### M1 · B-SCALE / B-ANTIPATTERN — the "i128 overflow guard" (Q1 / DoD §12) is unfalsifiable; its supporting numbers are wrong by ~100×, so the gate cannot distinguish an i64 port from an i128 port

**Packet claim (§3.2, Q1, threat S5-T4, DoD §12):** "`sub·rateMicro` can reach ~`1e10 × 1e8 = 1e18`
… with no headroom at 100% rate on large carts — **`i128` for the intermediate is mandatory** to …
avoid a silent overflow-wrap that would reduce a charge"; DoD names "a large-cart × 100%-rate vector
produces the BigInt-identical tax (no wrap)."

**Ground truth (schema-bounded):**
- Money columns are **`integer` (int4, max 2,147,483,647 ≈ 2.147e9)**, not 1e10:
  `orders.subtotal/total integer` — `1780310074262_orders.ts:32-33`; `delivery_fee/discount_total/
  tax_total integer` — `1780338982013_money_breakdown.ts:6-8`.
- `taxRate` is a **fraction**, not a percent: `applyTax` computes `tax = sub·rateMicro/SCALE`,
  `SCALE=1e6`, `rateMicro = round(taxRate·1e6)` (`money.ts:10-21`); at 100% (`taxRate=1.0`)
  `rateMicro = 1e6`, not `1e8`. (`tax_rate numeric` — `1780338982014_location_commerce.ts:8`;
  called `applyTax(subtotal, Number(location.tax_rate), …)` — `orders.ts:531`.)

**Number:** the largest **persistable** subtotal is int4-max 2.147e9. Exclusive intermediate at 100%
= `2.147e9 × 1e6 = 2.147e15`, which is **~4,300× below `i64::MAX` (9.22e18)**. Even a nonsensical
`tax_rate=100` (10,000%, no DB CHECK bounds it) gives `2.147e9 × 1e8 = 2.147e17`, still ~43× below
i64. To wrap i64 you would need `subtotal > 9.2e12` at 100% — ~4,285× larger than the int4 column can
hold (any such subtotal 22003-errors on INSERT before it matters). **No schema-valid order can wrap an
i64 intermediate.** Therefore the named "overflow guard" DoD vector produces identical output for an
i64 and an i128 implementation — it **cannot fail on the S5-T4 hazard it certifies.** The i128 rec is
harmless (over-conservative belt), but a 🔴 red-line gate that cannot be red is verification theater,
and it papers over the actual Q1 risk (rounding/serialization parity, which IS achievable and IS the
thing to gate).

### M2 · B-CONSIST — the idempotency §6 contract omits the "hit + matching hash but missing order row → DELETE key + recreate" branch (`orders.ts:406-411`)

**Ground truth:** on an idempotency-key hit with a **matching** `request_hash`, the code reads the
existing order; **if that order row is absent** (`rowCount 0`) it `DELETE`s the key and **falls
through to create a brand-new order** — `orders.ts:406-411`. The packet's §6 enumerates only
(a) hit+match→replay, (b) hit+mismatch→422, (c) race→409 — it never names this delete-and-recreate
fold.

**Scenario:** an order hard-deleted (GDPR anonymizer / cleanup) after its key committed leaves a
dangling key; the next retry recreates. During the Q6 overlap, if the Rust port implements only the
three enumerated branches, a retry on the other stack diverges (422 or replay-of-nothing vs
recreate). Low probability, but it is an unenumerated money-write branch on the idempotency path the
packet declares fully specified. **Invariant:** the "single-writer, fully-carried idempotency
contract" is incomplete.

### M3 · B-CONSIST / B-DATA — packet's Q1/Q9 "duplicate PAID order" framing contradicts its own §9.3 (no charge at create); the crown-jewel severity is mis-stated

**Ground truth:** cash does not charge at create (collected at delivery, S7) and the crypto fork is
dark (`orders.ts:660-690`, flags off; "never flip crypto during the overlap"). Confirmed. Yet §1 and
§9.2 repeatedly name the primary cutover hazard "duplicate **paid** order," while §9.3 proves "there
is **no synchronous charge at create** on either stack." The two framings are inconsistent; a council
sizing the Q6 risk off §1/§9.2 will over-scope the mitigation and mis-rank it against the money
vectors. The *actual* worst live cutover outcome is a duplicate **unpaid** order or a false 422 (see
H1). **Invariant:** honest severity calibration on the surface the whole atomic-flip posture exists
for.

---

## LOW

### L1 · B-STATE — `updateOrderStatus` publishes `ORDER_CONFIRMED` / `ORDER_REJECTED` lifecycle bus events (`orderStatusService.ts:286-290`) that the §4.1 fold list does not enumerate

The packet's §4.1 mutator fold list names status-guarded UPDATE, `*_at` stamps, R2-3, L-A, history,
ETA, and "WS publish to order/dashboard rooms" — but not the separate
`messageBus.publish(BUS_CHANNELS.ORDER_CONFIRMED/ORDER_REJECTED)` notification-fan-out triggers at
`orderStatusService.ts:286-290`. A port that ports the enumerated list literally drops the
notify-outbox trigger on confirm/reject (notifications silently stop). Minor, but it is a fold, not
just "a WS publish."

### L2 · B-STATE — the actor-gate covers only CANCELLED edges; a **pickup** order can be driven `READY→IN_DELIVERY` bypassing honest-dispatch → orphaned IN_DELIVERY

`assertOwnerTargetAllowed` blocks only `to==='CANCELLED' && from∈{CONFIRMED,PREPARING,READY}`
(`orderAuthz.ts:11,19-27`). The machine permits `READY→IN_DELIVERY` (`order_status.rs:60` /
`order-machine.ts:28`), and honest-dispatch guards IN_DELIVERY only for `type==='delivery'` (§4.5).
For a **pickup** order (`type='pickup'`), honest-dispatch is skipped and the actor-gate does not
block, so an owner PATCH can move a pickup order into IN_DELIVERY with no courier — an orphaned
binding-less IN_DELIVERY the R2-3 fold only cleans on the *reverse* edge. Owner-foot-gun, low impact,
but the packet's §4.3 matrix implies IN_DELIVERY is reachable "via honest-dispatch" only; the code
does not enforce that for pickup.

---

## Confirmed sound (regression baseline for RE-ATTACK)

Attacked and **held** — recorded so a later revision cannot silently regress them:

- **State matrix byte-parity (Q2).** `order_status.rs` (10 values, `can_transition`, `assert_transition`,
  `is_terminal`, deliver-v2 CANCELLED/READY-revert/PICKED_UP edges) is a faithful, exhaustive
  (100-pair) port of `order-machine.ts` — no drift. `order_status.rs:51-95` vs `order-machine.ts:18-56`.
- **S2 customer-order-scope seam exists (Q3 customer side).** `CustomerClaimsExt::require_order`
  (`extractors.rs:122-123`) + `service::customer_authorized_for_order (claims.order_id ==
  target_order_id)` (`service.rs:124-125`) with cross-order-403 tests (`extractors.rs:333-339`,
  `service.rs:327-329`). The live Node bug is real: `customer/orders.ts:29,50,230,237,271,286` bind
  `o.id = path.orderId AND o.customer_id = sub` (token `sub`), never the token's `orderId` claim,
  while `GET /orders/:id` DOES (`orders.ts:752`) — the asymmetry the packet describes is accurate.
- **Q5b messenger DB-CHECK precondition holds.** `customers/couriers/orders/receiver` CHECKs admit all
  6 kinds (`1790000000074:12,19,23,27,35`); the Zod enum is the narrower drifted copy (`legacy.ts:48`
  vs `messenger.ts:8`). Q5b(b) will not trade a 422 for a 500 on `customer_messenger_kind`. (But see
  **H2** — the co-located `receiver` object is a second, unhandled drift.)
- **Migration 086 is genuinely stack-agnostic (Q6.4/Q7).** `AFTER UPDATE OF status … WHEN NEW.status
  IN ('CANCELLED','REJECTED')` fires at the DB regardless of which stack drives the UPDATE, non-throwing
  by design, N5 partial-unique `(payment_id) WHERE type='refund_due'`, per-row GUC save/restore for
  FORCE-RLS — `1790000000086:70-104`. The "land 086 before the flip" recommendation is sound; inert
  until crypto (no `paid` rows).
- **`applyTax` is byte-portable.** The BigInt half-up arithmetic (`money.ts:10-21`) is deterministic
  integer truncation-toward-zero on non-negative operands; an i128 (or, per M1, even i64) port
  reproduces it bit-for-bit against the zero-import money vectors. The port risk is the overflow-gate
  theater (M1), not the arithmetic.
- **Lek(i64) newtype** rejects negative wire values at the deserialize boundary and has no `From<f64>`
  (`money.rs:128-136,46-57`); the 2^53 JS-consumer caveat is documented (module doc). No new hole.
</content>
</invoke>
