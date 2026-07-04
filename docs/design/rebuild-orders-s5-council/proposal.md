# S5-ORDERS/MONEY Port — Council Packet · PROPOSAL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Description input to the live Triadic Council
> (system-architect + system-breaker + counsel + human). **No S5 code is ported to Rust until this
> packet is council-APPROVED, every quirk-register row (§11) is dispositioned one by one, and the
> operator signs the 🔴 open questions (`open-questions.md`).** This is the **CROWN-JEWEL red-line
> surface** — order lifecycle + all money composition + channel attribution + the scariest cutover
> (orders are money + irreversible). Docs only; no product code.

- **Lane:** R3 (complete-rebuild) · **Surface:** S5 orders/money (REBUILD-MAP §3 Phase B, 5th
  strangler — `S3 catalog → S4 media → S5 orders 🔴`), incl. the `sales_channel`/channel-attribution rows.
- **Date:** 2026-07-04 · **Source commit:** `fix/audit-remediation` (working tree).
- **Census SSOT:** `inventory/10-api-realtime-jobs.md` — the orders/checkout/customer-order routes
  (`POST /orders`, `GET /orders/:id`, `PATCH /orders/:id/status`, `GET|POST /api/customer/orders/:orderId/{status,cancel,rating}`)
  + the pure money core already extracted on the Node side (`lib/order-pricing.ts`, `lib/money.ts`,
  `lib/order-persistence.ts`, `lib/orderStatusService.ts`, `lib/channel.ts`, `lib/orderAuthz.ts`).
- **Governing ADRs / prior councils (this surface inherits hard-won invariants — do not re-litigate):**
  - **ADR-audit-fix-money** (`docs/design/audit-fix-money/`) — **LC1** inclusive-tax double-charge fix
    (`chargedTax = price_includes_tax ? 0 : taxTotal`), **LC6** crypto refund black hole (the 4-layer
    L-A/L-B/L-C/L-D `refund_due` floor), settlement no-loss redesign, and migration drafts **085/086/087**
    (085 watermark **2026-07-10 HARD gate**).
  - **ADR-0005** (server is SoT of what is CHARGED; FE mirror is display-only + cash-422 backstop) ·
    **money-newtype council** (`Lek(i64)` minor-unit newtype) · **ADR-0016** (checkout communication /
    6-kind selector) · **ADR-0017** (crypto prepaid, dark) · **ADR-0013** (courier WS/read authz) ·
    **ADR-0004** (owner-token P-d live-membership re-read).
  - **S2-auth RESOLVE REV-3 / T-12** (`rebuild-auth-s2-council/` — customer-token **order-scope**
    FIX-IN-PORT; the seam `CustomerClaimsExt::require_order` + `service::customer_authorized_for_order`
    is **already built** in `rebuild/crates/api/src/auth/`; S5 **wires** it).
  - **S3-catalog RESOLVE REV-10** (tenancy-GUC contract: non-confusable `UserId`/`TenantId`; `with_user`
    for owner writes) and **REV-7** (per-surface **atomic** cutover posture — the whole route family
    flips together, never route-by-route).
- **Parity oracle:** the 174-spec Playwright net (load-bearing S5 specs: the order-lifecycle / checkout /
  cash-as-proof / preflight / tracking slices) **plus** the money invariant-cluster: the **zero-import
  hand-derived money vectors** (`audit-fix-money` §2.5, `order-total-composition.test.ts`) and the
  **byte-identical 10×10 `order_status.rs` matrix** (already ported + verified). No behavior change is
  real without a red→green test (Mandatory Proof Rule). Cutover DoD in §12.

---

## 1. Port objective and the load-bearing seam

S1 was read-only; S2 wrote **auth** tables; S3 wrote **owner catalog** through `with_user`
(`app.user_id`→memberships); S4 wrote **media metadata** + object storage. **S5 is the first Rust
surface that composes and persists MONEY and drives the ORDER STATE MACHINE** — the one surface where a
port defect is a *charge* defect (irreversible for crypto/cash) or a *stuck/forked order* defect.

There are **three** load-bearing seams, each an independent failure mode the port must hold
simultaneously:

1. **The money-composition seam** — the server-authoritative order total must be reproduced
   **byte-identically** in integer minor units (`Lek(i64)`), including the **BigInt half-up** tax
   arithmetic and the **LC1 inclusive-vs-exclusive** composition. A one-minor-unit drift or a
   re-introduced double-add is a live overcharge (§3, Q1 🔴).
2. **The state-machine seam** — the axum handlers wire onto the **already-ported, byte-identical**
   `order_status.rs` matrix, but the *rich* transition service (`updateOrderStatus`: status-guarded
   anti-race UPDATE, per-transition `*_at` stamps, the **R2-3 assignment-terminalize fold**, the **L-A
   `refund_due` fold**, `order_status_history` audit, ETA synthesis, WS publish) plus the **actor-gate**
   (`assertOwnerTargetAllowed`) must port with **every fold intact** — the machine says what is
   *possible*, the actor-gate says *who is allowed* (§4, Q2 🔴).
3. **The order-write tenancy seam** — **the current `POST /orders` create path seats NO tenant GUC at
   all** (`db.connect()` → `BEGIN` → INSERTs, no `set_config`; works only because the pool role is
   BYPASSRLS today). Post-B3-flip the `orders`/`order_items`/`idempotency_keys` INSERTs need
   `app.current_tenant = locationId` seated to satisfy `WITH CHECK`, or the whole checkout **silently
   matches 0 rows / raises** — the exact anonymizer-N1 / S3 / S4 raw-pool detonation, replayed on the
   money hot path (§8, Q3 🔴). Order writes seat the **`with_tenant` (`app.current_tenant`=locationId)**
   family — **not** `with_user` — because an order is created by an anonymous/customer principal against
   a *location*, exactly like the customer-cancel path already does
   (`customer/orders.ts:324` `set_config('app.current_tenant', $loc, true)`).

**The sharpest cutover fact (see §9, Q6 🔴):** during the strangler overlap **both Node and Rust accept
`POST /orders`**. Orders are money and irreversible. The ONLY thing preventing a double-order from a
cross-stack retry is that both stacks check the **same** `idempotency_keys (key, location_id)` unique
**and** compute a **byte-identical `request_hash`**. If the Rust `buildRequestHash` canonicalization
differs by one byte from Node's, a legitimate client retry that lands on the other stack either (a)
420s as `IDEMPOTENCY_KEY_REUSED` (false reuse) or (b) **creates a duplicate paid order**. The
request-hash canonicalization is therefore a **named, byte-fidelity cutover gate**, co-equal with the
money vectors.

## 2. Scope — what is S5, what is explicitly NOT

**In this packet (S5):**
1. **`POST /orders`** — the full create funnel: location/publish/venue gates → preflight (availability
   + velocity throttles + signals + OTP) → **idempotency** → in-tx MVCC price authority → **money
   composition** → delivery-fee ladder → customer upsert → persist (order + items + modifiers +
   idempotency key + track grant + transactional enqueues) → post-commit bus + customer-token mint →
   the **dark crypto prepaid fork** (ADR-0017, flags off).
2. **`PATCH /orders/:id/status`** — owner-driven transitions: membership-JOIN authz, the
   **`assertOwnerTargetAllowed`** actor-gate, the **honest-dispatch** IN_DELIVERY path, the **CC-1**
   DELIVERED/PICKED_UP strand guards (409 `ASSIGNMENT_ACTIVE` / `USE_DELIVER_FLOW`), and
   `updateOrderStatus`.
3. **`GET /orders/:id`** — the tri-principal read (owner membership-JOIN / courier live-binding verdict
   / customer order-scope), `softVerifyAuth`.
4. **`GET|POST /api/customer/orders/:orderId/{status,cancel,rating}`** — the customer order-scope
   surface (S2 **REV-3/T-12** FIX-IN-PORT wiring; the post-dispatch cancel with the **LC3 GUC dance**).
5. **The `updateOrderStatus` transition service** — the central mutator every path funnels through,
   with all folds (§4).
6. **Channel attribution** — the `x-channel` header → `orders.metadata.channel` write-only path
   (`lib/channel.ts`), and the **MessengerKind 3-kind-422 stale-enum** disposition (§7, Q5).

**NOT S5 (explicit boundary — each a separate slice):**
- **Payment webhook** (`payments-webhook.ts` — Plisio HMAC, L-B pay-after-cancel fold) — the
  unauthenticated money-in front door with its own HMAC/DEFINER threat model; **S8/payment-webhook
  slice**. S5 ports only the **dark create-side** crypto fork (which *creates* the `payments` row +
  hosted charge), never the inbound settle.
- **Courier dispatch / offer-handshake / completeDelivery / settlement** (`app_generate_settlements`,
  cash-as-proof, courier cash ledger, migration 085) — **S7 dispatch slice**. S5 *calls*
  `attemptHonestDispatch` and *reads* the binding guards, but the dispatch engine + the settlement
  money is S7. The **L-A `refund_due` fold** lives inside `updateOrderStatus`, so S5 ports it, but it
  is **inert** until crypto flips (matches zero `paid` rows today).
- **Realtime WS fan-out** (the order/dashboard/courier rooms) — **S6 WS slice**. S5 *publishes* to the
  bus (`messageBus.publish`) through the ported bus interface; the WS transport + fan-out authz is S6.
- **Owner dashboard order list / analytics** (`/owner/orders`, metadata passthrough reader) — rides the
  S3/owner surface; S5 only guarantees the fields it writes.
- **No schema change** — the DB is frozen. The `sales_channel` "first-class entity" named in
  REBUILD-MAP §6 is a **future migration**, not an S5 deliverable; S5 carries the **metadata-jsonb**
  channel attribution that actually shipped (no `sales_channel` table exists — grep-verified) (§7).

**Back-of-envelope (why boring wins, and where the real ceiling is).**
- **Scale:** target **N ≈ 10–50 active locations**, growth to low-hundreds. Orders are **low-hundreds/day
  system-wide**; lunch/dinner concentration (~3–5× average) → **peak ≈ 5–10 `POST /orders`/min**,
  bursty. Even a 10× growth headroom is ~1–2 creates/sec peak.
- **Per-create cost:** ONE operational connection held for a **bounded ≤4.5 s** write-tx (`SET LOCAL
  statement_timeout = 4500`, orders.ts:124 — carry verbatim as the pool-wedge fuse). The historical
  N-fan-out is already flattened (batch modifier-group query, orders.ts:470), so the hold **no longer
  scales with cart size**. At peak that is a handful of concurrent connections against the 20-conn
  operational pool — **negligible**.
- **The real ceiling is the CUTOVER connection budget, not order QPS.** During the S5 overlap the SAME
  Postgres/Supavisor pool is drawn by: **API operational (Rust ~20 + Node ~10 concurrently)** +
  **worker** (timeout-sweep, settlement-cron, notify-outbox, reconciler ticks) + **analytics/dashboard
  reads** (owner order list) + **migrations** (`release_command` transient). The sum during overlap
  must stay under the Supavisor/Postgres ceiling (the Phase-A Supavisor decision, REBUILD-MAP §Decision
  register) — **this is the scaling gate for the cutover window, and it is why the flip must be atomic
  and time-boxed, not an indefinite dual-run** (§9).
- **Conclusion:** the surface is **not** connection-bound at steady state; boring monolith-in-`api`
  (no new runtime) is correct. The engineering risk is entirely **correctness** (money bytes, state
  folds, idempotency, tenancy) and **cutover concurrency**, not throughput.

---

## 3. Concern 1 — Money composition contract (Q1 🔴)

**The full server-authoritative order total, integer minor units end-to-end (`orders.ts:490–535`,
`lib/order-pricing.ts`, `lib/money.ts`):**

```
# all values integer minor units (Lek(i64)); tax_rate is a percent config input (numeric/f64), NOT money
subtotal    = Σ_lines ( product.price + Σ modifier.price_delta ) × quantity     # computeOrderPricing, MVCC-snapshot price authority
taxTotal    = applyTax(subtotal, tax_rate, price_includes_tax, minor_unit)       # BigInt half-up; inclusive EXTRACTS, exclusive ADDS
chargedTax  = price_includes_tax ? 0 : taxTotal                                  # LC1 — inclusive tax is NEVER additive
deliveryFee = isPickup ? 0
              : subtotal ≥ free_delivery_threshold ? 0
              : resolveDeliveryFee(distance-tier | flat)                          # 422 NOT_DELIVERABLE / DELIVERY_NOT_CONFIGURED
discountTotal = 0                                                                # HARDCODED (orders.ts:533) — known gap
total       = subtotal + deliveryFee + chargedTax − discountTotal
assertNonNegative(total)
# gates: subtotal < min_order_value → 422 MIN_ORDER_NOT_MET (pickup AND delivery)
#        cash_pay_with < total      → 422 CASH_AMOUNT_TOO_LOW
```

**The invariant to pin forever (LC1):** `price_includes_tax=true ⇒ total === subtotal + deliveryFee −
discountTotal` **exactly, for every rate**. This has no oracle — it is the *definition* of inclusive
pricing; the property test needs no reference implementation.

**Port contract (each clause a red→green vector at DoD):**

1. **SSOT = server, unchanged (ADR-0005).** The Rust route computes the total in-tx from the
   MVCC-snapshot `products.price` / `modifiers.price_delta` read on the **same connection/snapshot** as
   the order INSERT (orders.ts:414–432 — a coherent snapshot, **not** a `FOR UPDATE` lock). The FE
   `estimateOrderTotal` mirror stays **display-only + the runtime cash-422 backstop**; it is *never* the
   charge authority. Port the pure `computeOrderPricing` (subtotal + modifier-group min/max validation +
   `DUPLICATE_MODIFIER`) and `resolveDeliveryFee` verbatim (identical 422 codes/messages).
2. **`applyTax` byte-parity — the sharpest arithmetic port (`lib/money.ts:1–22`).** The Node impl does
   **BigInt** with `SCALE = 1_000_000n`, `rateMicro = round(taxRate × 1e6)`, half-up rounding. Inclusive:
   `net = (sub·SCALE + denom/2)/denom; tax = sub − net`. Exclusive: `tax = (sub·rateMicro + SCALE/2)/SCALE`.
   The Rust port must reproduce this **bit-for-bit** using a **`i128` intermediate** (never `f64` — RED
   LINE): `sub·rateMicro` can reach ~`1e10 × 1e8 = 1e18`, inside `i64::MAX ≈ 9.2e18` but with no headroom
   at 100% rate on large carts — **`i128` for the intermediate is mandatory** to match unbounded BigInt
   and avoid a silent overflow-wrap that would *reduce* a charge (threat S5-T4). Short-circuits carry:
   `subtotal==0 || taxRate==0 ⇒ 0`. **The money vectors (`audit-fix-money` §2.5 zero-import,
   hand-derived) are the oracle** — the Rust test imports only the money module + the vector file.
3. **LC1 composition carried structurally.** `chargedTax = price_includes_tax ? 0 : taxTotal`, and
   `total` uses `chargedTax` (never `taxTotal`). `taxTotal` stays persisted to `orders.tax_total`
   (informational receipt line) unchanged. **A guardrail asserts the additive term in `total` is
   `chargedTax`, never `taxTotal`** — this is the exact composition bug the mirror-lock once certified;
   the port must not re-fork it. `estimateOrderTotal`'s `chargedTax` field (CC-3) is FE-mirror-only.
4. **`discountTotal` — CARRY the hardcoded 0 + FLAG (Q1a).** Promo/discount redemption **does not
   exist**; `discountTotal = 0` is a literal (orders.ts:533, mirrored in `estimateOrderTotal`). **Do NOT
   wire real redemption in the port** — it is a *feature*, not a port target, with its own council
   (schema, redemption ledger, abuse model — threat S5-T3). Port the `0`, keep the `− discountTotal` term
   in the formula (so the seam exists), and record an **accepted-risk row** naming the gap + owner. This
   is "schema-rich, runtime-minimal": the subtraction term is the seam; redemption stays unbuilt.
5. **Money newtype end-to-end (`Lek(i64)`).** `subtotal/deliveryFee/taxTotal/chargedTax/total/
   cash_pay_with/tip_amount` are minor-unit `i64` via the money-newtype council's type; `tax_rate` is
   `Option<f64>`/numeric (a rate, not money — the S3 `Q-TAX-RATE-FLOAT` precedent). `assertNonNegative`
   ports as a checked invariant on `total` (a negative total is unrepresentable / a hard 500-class bug,
   never a silent negative charge).
6. **Courier settlement money is OUT of S5** (S7) — but its *source*, `cash_amount` on the delivered
   assignment, is written by the deliver/dispatch flow, not by S5. S5 writes `cash_pay_with` (customer's
   declared cash) and `tip_amount`; the settlement of that cash to the courier is S7 + migration 085.

**Failure-first:** every 422 (`MIN_ORDER_NOT_MET`, `CASH_AMOUNT_TOO_LOW`, `PRODUCT_UNAVAILABLE`,
`PRODUCT_NOT_FOUND`, `MODIFIER_*`, `NOT_DELIVERABLE`, `DELIVERY_NOT_CONFIGURED`, `DUPLICATE_MODIFIER`)
ROLLBACKs before any write and returns the identical code/message. Transient PG classes
(`40001/40P01/57014/53300/08006/08003/08000`) → **503 retryable** (orders.ts:724), never a scary 500;
`db.connect()` failure → 503. Carry the full transient set verbatim.

## 4. Concern 2 — Order state-machine wiring (Q2 🔴)

**The domain matrix is DONE.** `rebuild/crates/domain/src/order_status.rs` ports the 10-value enum + the
10×10 transition table byte-identically (verified: `exhaustive_transition_table` = 100 ordered pairs;
`SameStatus`/`ScaffoldDisabled`/`IllegalTransition` error *classes* preserved; deliver-v2 offer-sweep
CANCELLED edges + IN_DELIVERY→READY revert + PICKED_UP terminal all present; `SCHEDULED` inert). **S5
does not re-derive the matrix — it wires handlers onto `assert_transition`.**

**What S5 must port (the rich mutator + the actor-gate — none of this is in `order_status.rs`):**

1. **`updateOrderStatus` (`lib/orderStatusService.ts`) — the central mutator, every fold intact:**
   - **State check then status-guarded UPDATE (anti-race):** `assert_transition(current, new)` (→ 400
     class), then `UPDATE … WHERE id=$ AND status=$currentStatus` — **0 rows → 409 `CONFLICT`** (a
     concurrent transition already moved it). This is the concurrency guard; carry the `WHERE status=`
     predicate on **every** transition (the CONFIRMED / DELIVERED dedicated branches + the
     `STATUS_AT_COLUMN` allowlist stamp path).
   - **Per-transition `*_at` stamps** from the fixed `STATUS_AT_COLUMN` allowlist (never user input —
     port as an enum→column match, the S3 `Q-DYNAMIC-SET` posture); `timeout_at = NULL` on transition.
   - **R2-3 assignment-terminalize fold** on `CANCELLED|REJECTED` or `IN_DELIVERY→READY`: the `WITH
     freed AS (UPDATE courier_assignments … RETURNING shift_id) UPDATE courier_shifts` — no order leaves
     to a terminal/downgrade with a live binding stranded. Idempotent; cash-safe (writes no ledger hold).
   - **L-A `refund_due` fold** on `CANCELLED|REJECTED`: SAVEPOINT-wrapped idempotent insert into
     `payment_events` for `paid` payments; **fail-closed per-order + fail-LOUD** (Sentry + DRIFT log +
     `ops.reconciliation_drift` bus); **ESC-2 `forceTerminal`** operator escape (SAVEPOINT-swallow +
     audit row). **Inert until crypto flips** (zero `paid` rows). Port the whole contract, including the
     GUC dance requirement (§8) so it is correct pre- and post-B3.
   - **`order_status_history` audit** (SAVEPOINT best-effort — a history failure never rolls back the
     applied status), **ETA-window synthesis** (SAVEPOINT best-effort, observe-don't-control), **WS
     publish** to `order`/`dashboard` rooms (claim-check: NO item-name/customer PII on the bus).
2. **The actor-gate is separate from the machine (`lib/orderAuthz.ts`).** The matrix *permits*
   CONFIRMED/PREPARING/READY→CANCELLED (SYSTEM dispatch-grace), but **an owner may not drive them** →
   `assertOwnerTargetAllowed` throws **403 `CANCEL_NOT_PERMITTED`**. Owner keeps PENDING→CANCELLED and
   IN_DELIVERY→CANCELLED. **Port this as a distinct authorization layer** — the machine says *possible*,
   the actor-gate says *allowed*. Missing it lets an owner drive a SYSTEM-only edge.
3. **Who may transition, per principal (the authorization matrix the port must encode):**
   | Principal | Route | Allowed transitions | Gate |
   |---|---|---|---|
   | **owner** | `PATCH /orders/:id/status` | PENDING→{CONFIRMED,REJECTED,CANCELLED}; CONFIRMED→PREPARING; PREPARING→READY; Ready→PICKED_UP; IN_DELIVERY→CANCELLED; IN_DELIVERY (via honest-dispatch) | membership-JOIN (ADR-0004 P-d) + `assertOwnerTargetAllowed` + CC-1 strand guard |
   | **customer** | `POST /customer/orders/:orderId/cancel` | IN_DELIVERY→CANCELLED only, within the 5-min post-dispatch window | order-scope (REV-3) + `FOR UPDATE OF o` + window + LC3 GUC dance |
   | **courier/system** | dispatch / deliver / offer-sweep / completeDelivery (**S7**) | the courier + SYSTEM edges (accept/pickup/deliver, grace-cancel, revert) | S7 — out of packet, but funnels through `updateOrderStatus` |
   | **system** | timeout-sweep worker | PENDING→CANCELLED past `timeout_at` | **S8 jobs** — funnels through the sweep fn |
4. **CC-1 DELIVERED/PICKED_UP strand guards (money-audit H1, carry verbatim):** on
   `PATCH → DELIVERED|PICKED_UP`, refuse **409 `ASSIGNMENT_ACTIVE`** (active binding) or **409
   `USE_DELIVER_FLOW`** (IN_DELIVERY with no `delivered` assignment) — never a silent 200-and-strand. The
   never-dispatched escape (zero assignments, never IN_DELIVERY) stays PATCH-able.
5. **Honest dispatch:** `newStatus==='IN_DELIVERY' && type==='delivery'` → `attemptHonestDispatch`
   **before** advancing (find a courier first; no courier → stay put, `{dispatched:false,
   reason:'no_courier'}`). The dispatch engine is S7; S5 preserves the *ordering* (dispatch-then-advance,
   never advance-then-orphan).

## 5. Concern 3 — Customer order-scope authz (Q3 🔴 — S2 REV-3/T-12 FIX-IN-PORT)

**The seam is already built; S5 wires it.** The S2 RESOLVE REV-3 shipped `CustomerClaimsExt::require_order
(order_id)` + `service::customer_authorized_for_order(claims, target)` (`claims.order_id ==
target_order_id`) in `rebuild/crates/api/src/auth/` — a type-state method a customer-scoped handler
**must** call, so `token(orderA)` **cannot** authorize `orderB`.

**The live Node bug this closes (breaker H3 / S2 T-12):** the customer JWT is minted per-order
`{role:'customer', orderId, locationId, sub=customerId}` (14-day tracking link, `?t=`, referer-leakable),
but the customer routes bind **`customer_id = sub`** (customer-*wide*), **never reading the token's
`orderId` claim** (`customer/orders.ts:50,237,284`). A token for order A can therefore read/cancel/rate
**every** order of that customer. (Note the asymmetry: `orders.ts:752` GET *does* bind the token's
`orderId` — the two read paths disagree on scope.)

**Port contract:**
1. **Bind BOTH `orderId` (token claim) AND `customer_id = sub` (belt-and-suspenders).** Every
   customer-scoped handler (`/status`, `/cancel`, `/rating`) calls `require_order(path_order_id)` (403 on
   mismatch) **and** keeps the `WHERE o.id = $orderId AND o.customer_id = $sub` predicate. The claim check
   is the primary authority; the `customer_id` predicate is defense-in-depth that holds independent of
   the token. **E2E delta (from REV-3): token for order A → cancel/read/rate order B must 403.**
2. **The order-write / cancel GUC seam (LC3, DEP-1).** The post-dispatch cancel path
   (`customer/orders.ts:308–341`) is a **tenant-table write by a customer principal**: it resolves
   `location_id` from the ownership-verified read, seats **`set_config('app.current_tenant', $loc, true)`**
   (the `with_tenant` family — §8), then calls `updateOrderStatus` inside that tx so the L-A `refund_due`
   fold passes FORCE-RLS pre- and post-B3 (precedent: `payments-webhook.ts:41`). Port the GUC dance
   verbatim — a context-free customer cancel **500s today** (the LC3 phantom-column bug is already fixed
   on Node; the port must carry the fix, not the pre-fix 500).
3. **`GET /orders/:id` tri-principal read (orders.ts:735–861) — carry all three scopes exactly:**
   owner = live membership-JOIN (`m.role='owner' AND m.status='active'`, the JOIN *is* the tenant
   boundary, no bare `WHERE id=$1`); courier = **`courierReadVerdict`** live-binding (ADR-0013, with the
   **503-on-UNAVAILABLE fail-closed-distinguishable** arm, never fail-open); customer = `orderId`-scoped
   (`user.orderId !== id → 404`); anonymous/unknown → **401** (no bare-UUID enumeration fallback).

## 6. Concern 4 — Idempotency (Q4)

**Order-create dedup (`orders.ts:394–412`, `insertOrderWithItems`):** the client mints
`idempotency_key` (a UUID, `.strict()`-validated); the create tx (a) looks up
`idempotency_keys WHERE key=$ AND location_id=$` (tenant-scoped, FX-5), (b) on hit with a **matching
`request_hash`** replays the existing order (200), (c) on hit with a **different `request_hash`** →
**422 `IDEMPOTENCY_KEY_REUSED`** (a reused key with a mutated cart is refused, never silently
re-priced), (d) on a race, the `idempotency_keys` unique surfaces as **409 `IDEMPOTENCY_CONFLICT`**
(`23505`). The key row is written **inside** the same tx as the order (order-persistence.ts:142) — so
the order and its dedup token commit atomically.

**Port contract:**
1. **`request_hash` byte-fidelity is a cutover gate, not a detail (§9).** `buildRequestHash`
   (`lib/order-canonical.ts`) canonicalizes `{locationId, type, items, pin, addressText, cashPayWith,
   currencyCode, menuVersion, customerId}`. The Rust port must produce the **identical hash** for the
   identical cart, or a cross-stack retry mis-fires (false 422 or a duplicate order). **DoD: a
   golden-vector test — a fixed cart → the exact Node hash string.** The customer identity in the hash is
   `request.user.sub` for a customer token, `'anonymous'` otherwise (the #8 security-hardening fix:
   reading `.userId` yielded `undefined` and silently degraded the fingerprint — port the `.sub` read).
2. **Retry-safety of money writes.** Because the money write and the idempotency-key write share one
   tx, a retried create either **replays the committed order** (identical total) or **conflicts** — it
   can **never** produce two orders or two charges for one key. This holds cross-stack **iff** (1) the
   key+hash are computed identically. The crypto prepaid fork (dark) uses `idempotency_key` as the
   provider charge idempotency key too (orders.ts:681) — so a replayed create does not double-charge the
   provider either.
3. **Preflight is NON-idempotent by design (carry) — the S4-confirm precedent.** `soft_confirm` (200)
   and `hard_block` (422) ROLLBACK **before** the idempotency check; the OTP session consume + velocity
   events are the only pre-commit side effects and are deliberately not keyed. A client that re-submits
   after a `soft_confirm` supplies `acknowledged_codes` and proceeds — the idempotency key first *binds*
   on the CLEAN pass. Port the ordering exactly (preflight → idempotency → price → persist).

## 7. Concern 5 — Channel attribution + the MessengerKind 3-kind-422 (Q5)

**Channel attribution — CARRY the metadata-jsonb path; there is NO `sales_channel` table (grep-verified).**
The QR/NFC/TMA channel work (`lib/channel.ts`) travels as the **`x-channel` header**, normalized against
a 13-value allowlist (never throws — malformed → `other`, missing → `web-direct`), and is folded
**write-only** into `orders.metadata.channel` (order-persistence.ts:103). It is **never read by pricing,
the state machine, dispatch, or any authz/RLS decision** — the owner dashboard's metadata passthrough is
the one expected reader.
- **Port contract:** reproduce `normalizeChannel` (case-insensitive, trimmed, ≤32 chars, allowlist or
  `other`, first-occurrence-on-array) and the write-only metadata fold. **The header, not a body field:**
  `CreateOrderInput` is `.strict()` and a red-line schema zone — carry the header sidestep.
- **`sales_channel` first-class entity is a FUTURE migration, not S5.** The DB is frozen; a new
  table/column would break the freeze. **Recommendation: CARRY metadata-jsonb; DEFER the `sales_channel`
  entity to a post-rebuild schema-evolution council** ("schema-rich, runtime-minimal": the attribution
  seam already exists in `metadata`; promoting it to a typed entity is a runtime we do not need yet).
  Q5a.

**The MessengerKind 3-kind-422 stale-enum (a live checkout break — FIX-IN-PORT candidate, Q5b 🔴):**
`CreateOrderInput.customer.messenger_kind` (`legacy.ts:48`) is a **stale 3-value enum**
`['telegram','whatsapp','viber']`, but the checkout communication selector (ADR-0016) offers **6 kinds**
`['phone','whatsapp','viber','telegram','signal','simplex']` (`apps/web/src/lib/messenger.ts:6`). A
customer who picks **phone / signal / simplex** hits a **`.strict()` Zod 422 at the order boundary** —
a real, current checkout failure (the "3-kind 422" flagged in memory). The DB CHECK is the true kind
gate (the receiver-contact comment confirms "DB CHECK is the kind gate"); the Zod enum is a narrower,
drifted copy.
- **Options:** **(a) CARRY the drift** (parity-pure — re-ship a live checkout break through a rewrite,
  weakest); **(b) FIX-IN-PORT: unify to ONE canonical `MessengerKind`** (the 6-kind set) in the Rust
  order-input contract, matching the checkout selector + the DB CHECK, with an E2E delta (a `signal`
  order that 422s today → 201). Recommendation **(b)** — this is a 🔴 correctness fix (a customer-facing
  order failure) the port naturally closes, exactly the fix-vs-carry rule's "FIX-IN-PORT for 🔴
  correctness with a documented E2E delta". Confirm the DB CHECK actually admits all 6 first (a Phase-0
  `ci-schema-drift` read) — else (b) would trade a 422 for a 500. Q5b 🔴.

## 8. Tenancy — the order-write GUC seam (Q3 🔴, the never-copy leak class)

**Two write families, spelled out because S5 is the first surface to seat the SERVICE/TENANT root on the
request path:**

- **Order writes seat `app.current_tenant = locationId` (the `with_tenant` family), NOT `with_user`.**
  An order is created by an **anonymous or customer** principal against a **location** — there is no owner
  membership to resolve. The tenant is the order's `location_id`, validated to exist (404) against
  `locations` before any write. This is the **courier/service root** (~102 policy sites), the same family
  the customer-cancel already seats (`customer/orders.ts:324`). S5 uses the Rust **`with_tenant(pool,
  TenantId, …)`** combinator (from S3 REV-10, the non-confusable type) — `db.rs::with_tenant` seating
  `app.current_tenant` is the **correct** root here (the opposite of S3, where it was the trap).
- **The create path seats NO GUC today (FIX-IN-PORT for B3-readiness).** `POST /orders` runs
  `db.connect() → BEGIN → INSERTs` with **no `set_config`** — it works only because the live pool role is
  BYPASSRLS. **Post-B3-flip the `orders`/`order_items`/`order_item_modifiers`/`idempotency_keys`/
  `velocity_events`/`customer_track_grants` INSERTs need `app.current_tenant` seated to satisfy
  `WITH CHECK`, or the entire checkout silently matches 0 rows / raises** (threat S5-T1, the
  anonymizer-N1 detonation on the money hot path). **The port routes the whole create tx through
  `with_tenant(locationId)`** — a FIX-IN-PORT divergence with a NOBYPASSRLS probe, not a verbatim
  context-free carry (inherits S3/S4 REV-5 probe scope).
- **`customer_id` upsert subtlety.** The customer upsert (`ON CONFLICT (location_id, phone)`) writes the
  `customers` tenant table — same `app.current_tenant=locationId` seat covers it. The resolved
  `customer_id` is then bound into the order.
- **Belt-AND-suspenders (carry verbatim).** Every order-scoped statement already carries an explicit
  predicate (`WHERE … location_id = $` / `customer_id = $` / `AND location_id = $2` on product/modifier
  reads). Carry these — they hold **independent of** which pool role is live (the sweep's identity-split
  root). The DoD extends the `rls-adversarial` "privileged pool queries have WHERE location_id" gate to
  the order-create statements.
- **The `updateOrderStatus` reads current status on a bare `WHERE id=$1`** (orderStatusService.ts:66) —
  **intentionally**: it is the *sanctioned mutator*, and **every caller seats the tenant context + does
  the ownership JOIN before calling it** (owner PATCH membership-JOIN; customer cancel GUC dance;
  dispatch/deliver seat their own). Port this contract: `updateOrderStatus` is called **only** inside a
  tenant-seated tx; a route may never call it context-free. The L-A/history/ETA folds inside it require
  the seat to pass FORCE-RLS.

## 9. Cutover concurrency — the scariest flip (Q6 🔴)

**Orders are money + irreversible; both Node and Rust accept `POST /orders` during the overlap.** The
failure classes and the controls:

1. **Order id / sequence collisions — NONE.** Order ids are UUIDs (`gen_random_uuid`), not a sequence;
   two stacks minting concurrently cannot collide. `idempotency_keys` and all child rows key on the UUID.
   No shared sequence, no collision vector. ✅
2. **Double-order from a cross-stack retry — the primary hazard.** A client whose first `POST /orders`
   lands on Node and whose retry (same `idempotency_key`) lands on Rust must **dedup**. Both stacks check
   the **same** `idempotency_keys (key, location_id)` unique, so the *second* create conflicts at the DB
   (409 `IDEMPOTENCY_CONFLICT`). **But the `request_hash` comparison (§6) fires FIRST** — if Rust's hash
   differs from Node's for the identical cart, the retry is misread as **key-reused-with-different-request
   (422)** instead of a clean replay. **Control: `request_hash` byte-fidelity is a NAMED cutover gate**
   (golden-vector test, both directions). This is co-equal with the money vectors — a hash drift is a
   silent duplicate-or-reject at the exact moment two stacks run.
3. **Double-charge — bounded.** Cash orders **do not charge at create** (cash is collected at delivery,
   S7); the crypto fork is **dark** (flags off). So there is **no synchronous charge at create** on either
   stack — the double-*charge* risk is bounded to the (dark) crypto path, where the provider charge is
   itself idempotency-keyed on `idempotency_key` (orders.ts:681). The live cutover hazard is the double
   **order** (2), not a double charge — but the packet names it because flipping crypto on **during**
   an overlap would open it (gate: **never flip crypto during the S5 overlap**; crypto flip is a
   post-cutover, single-stack act).
4. **The money ledger + state folds must be identical across stacks.** During overlap, an order created
   by Node may be transitioned by Rust (or vice versa). The `updateOrderStatus` folds (R2-3, L-A,
   history, stamps) must be **byte-equivalent** so a Node-created order transitioned by Rust records the
   same `refund_due` / assignment-terminalize / audit. **Control:** the **DB-level L-C trigger (migration
   086) is stack-agnostic** — it fires on the `orders.status` UPDATE regardless of which stack drove it,
   so it is the **cutover safety net** for the `refund_due` floor (a Node order cancelled by Rust, or an
   app-fold that a stack forgot, still gets the obligation). **Recommendation: land migration 086 BEFORE
   the S5 flip** so both stacks share the structural floor (§10).
5. **Atomic per-surface flip (S3 REV-7).** The **entire S5 route family flips together** behind the
   proxy — `POST /orders`, `PATCH /orders/:id/status`, `GET /orders/:id`, and the customer order routes
   share the state machine + money + idempotency table, so a split flip (create on Rust, cancel on Node)
   would run two divergent transition-service implementations against one order. Flip all-or-nothing; do
   not strangle route-by-route within S5.
6. **Rollback = instant flag-flip back to Node behind the proxy.** Because both stacks write the *same*
   tables through the *same* invariants, a rollback mid-overlap leaves committed orders valid on either
   stack (a Rust-created order is a normal row Node can read/transition, and vice versa) — **provided the
   request-hash + money + fold parity gates are green**. The rollback plan is a proxy flag, not a data
   migration.
7. **Connection-budget ceiling (§2 back-of-envelope).** The overlap doubles API pool draw on one
   Postgres/Supavisor. **Time-box the overlap** (canary → soak → flip → decommission Node), do not run an
   indefinite dual-accept. The scaling-gate: monitor combined operational-pool utilization; the flip is
   the *contract* to shed the Node pool.

**Cutover DoD gates specific to S5 (in addition to §12):** request-hash golden-vector parity (both
directions) · money-vector byte-parity · migration 086 landed (shared `refund_due` floor) · a
cross-stack idempotency probe (create on stack X, retry on stack Y, same key → one order) · crypto
stays dark throughout.

## 10. Migration-draft interaction (085/086/087) (Q7)

**Does S5 depend on the money migrations landing first? Mostly NO — with two named exceptions.**

- **085 (settlements-catchup, watermark 2026-07-10 HARD gate) — NOT an S5 dependency.** 085 rewrites
  `app_generate_settlements` (courier payout cron) — a **S7/settlement** concern the S5 order lifecycle
  never calls. **BUT the watermark literal `2026-07-10 00:00:00+00` interacts with *timing*:** erring
  EARLY (literal before the real apply) **double-pays** old rows; erring LATE is safe (rows defer to the
  operator backfill). If the *rebuild* work pushes the settlement apply past 2026-07-10, the operator
  must bump all three literal occurrences before apply (per the draft header N2). S5 does not gate on
  085, but the packet **flags** the watermark as a shared timing landmine the operator owns.
- **086 (refund_due trigger, M-1) — land it BEFORE the S5 cutover (cutover asset, not a build-blocker).**
  The L-C AFTER-UPDATE-OF-status trigger is **stack-agnostic** — it records `refund_due` no matter which
  stack (Node or Rust) drives the `orders.status` UPDATE, and covers writers the app-fold cannot see
  (raw UPDATEs, the sweep fn, future writers). Landing it before the flip gives **both** stacks the
  structural floor during overlap (§9.4). It is non-throwing by design (swallows per-row so it cannot
  wedge the fleet-wide sweep — do NOT "fix" it to the throwing template) and adds the **N5 partial unique
  `(payment_id) WHERE type='refund_due'`** that the L-A/L-C/L-D writers rely on for idempotency. **Inert
  until crypto flips** (no `paid` rows), so landing it is low-risk.
- **087 (reconciler, M-3) — worker-path, NOT request-path; land with the S8 jobs slice.** The L-D
  `app_reconcile_refund_due()` runs on the timeout-sweep tick — it is the bounded-lag alarm of last
  resort, not an S5 order-lifecycle dependency. S5 ports the L-A fold that 087 backstops, but S5 does not
  call 087.
- **Ordering summary:** S5 **code** builds independent of 085/086/087. For the **cutover**: land **086
  before the flip** (shared floor); 085/087 land on their own (settlement / jobs) schedules; the **085
  watermark is an operator timing gate** the packet surfaces. All three are `packages/db/migrations/`
  red-line, operator-placed-verbatim, staging-first, forward-only — **S5 does not author or apply them**;
  it consumes 086's floor and ports the app-side L-A fold.

## 11. Quirk register — carry-vs-fix (default = CARRY-VERBATIM)

**FIX-IN-PORT only for a 🔴 security/correctness issue or a build-correctness bug, each with an explicit
test/E2E delta.** Everything else CARRIES; shape-migration rows defer to post-Astro FE-lockstep.

| ID | Quirk (source) | Disposition |
|---|---|---|
| Q-TENANT-SEAT | `POST /orders` create tx seats **NO** tenant GUC (`orders.ts:108–117`, raw `db.connect()`+`BEGIN`, BYPASSRLS-masked) | **FIX-IN-PORT** → route the whole create tx through `with_tenant(app.current_tenant=locationId)`; 0-rows post-flip otherwise. NOBYPASSRLS probe (🔴 S5-T1) |
| Q-TENANT-FAMILY | order writes = `app.current_tenant`=locationId (service root), NOT `app.user_id` (owner root) | **CARRY the family** (correct root here — opposite of S3); use the S3 REV-10 non-confusable `TenantId` |
| Q-TAX-BIGINT | `applyTax` BigInt half-up, `SCALE=1e6`, inclusive-extract/exclusive-add (`money.ts:1–22`) | **CARRY exactly + FIX substrate:** Rust `i128` intermediate (never `f64`); byte-parity vs the zero-import money vectors (🔴 S5-T4 overflow) |
| Q-LC1-COMPOSE | `chargedTax = includesTax ? 0 : taxTotal`; `total` uses `chargedTax` not `taxTotal` (`orders.ts:532–534`) | **CARRY behavior + FIX-proof:** guardrail asserts the additive term is `chargedTax`; property test `inclusive ⇒ total = subtotal+fee` (🔴 S5-T4) |
| Q-DISCOUNT-ZERO | `discountTotal = 0` hardcoded; no redemption exists (`orders.ts:533`, mirror) | **CARRY the zero + FLAG** (Q1a) — keep the `− discountTotal` seam; redemption = own council; accepted-risk row + owner (🔴 S5-T3) |
| Q-PRICE-SNAPSHOT | price authority = in-tx MVCC snapshot, NOT `FOR UPDATE` (`orders.ts:414–432`) | **CARRY verbatim** — snapshot coherence between price read + INSERT; server authoritative (ADR-0005) |
| Q-CASH-422 | `cash_pay_with < total → 422 CASH_AMOUNT_TOO_LOW` (`orders.ts:537`) | **CARRY verbatim** — the runtime backstop that makes the FE mirror non-authoritative |
| Q-MIN-ORDER | `subtotal < min_order_value → 422 MIN_ORDER_NOT_MET` (pickup AND delivery) | **CARRY verbatim** |
| Q-TRANSIENT-503 | transient PG classes (`40001/40P01/57014/53300/08006/08003/08000`) → 503 retryable (`orders.ts:724`) | **CARRY verbatim** — graceful "try again", not a 500; incl. the `db.connect()` 503 + statement_timeout fuse |
| Q-STMT-TIMEOUT | `SET LOCAL statement_timeout = 4500` bounds the write-hold (`orders.ts:124`) | **CARRY verbatim** — the pool-wedge fuse; a stuck write self-aborts as 5xx, not a held connection |
| Q-IDEM-HASH | `request_hash` canonicalization (`order-canonical.ts`); customer id = `.sub` not `.userId` (#8 fix) | **CARRY exactly + GATE:** golden-vector byte-parity (cutover gate, §9); carry the `.sub` read (🔴 S5-T5) |
| Q-IDEM-REUSE | key hit + different hash → 422 `IDEMPOTENCY_KEY_REUSED`; race → 409 `IDEMPOTENCY_CONFLICT` (23505) | **CARRY verbatim** — the retry-safety of the money write |
| Q-STATE-MACHINE | 10×10 matrix + error classes (`order_status.rs`) | **DONE (ported + verified)** — S5 wires handlers; no re-derivation |
| Q-STATUS-GUARD | status-guarded UPDATE `WHERE status=$current` → 0 rows = 409 CONFLICT (`orderStatusService.ts:117–125`) | **CARRY verbatim** — the transition anti-race |
| Q-ACTOR-GATE | `assertOwnerTargetAllowed` blocks owner-driven SYSTEM-only CANCELLED edges → 403 (`orderAuthz.ts`) | **CARRY verbatim** — separate authz layer over the machine; machine=possible, gate=allowed |
| Q-CC1-STRAND | DELIVERED/PICKED_UP with active/undelivered binding → 409 ASSIGNMENT_ACTIVE / USE_DELIVER_FLOW (`orders.ts:929–955`) | **CARRY verbatim** — money-audit H1 anti-strand; the never-dispatched escape preserved |
| Q-R2-3-FOLD | assignment-terminalize + shift-free fold on CANCELLED/REJECTED/IN_DELIVERY→READY (`orderStatusService.ts:139`) | **CARRY verbatim** — no order terminalizes with a stranded binding; cash-safe (no ledger hold) |
| Q-LA-REFUND | L-A `refund_due` SAVEPOINT fold, fail-closed-per-order + fail-loud + ESC-2 forceTerminal (`orderStatusService.ts:165`) | **CARRY verbatim** — inert until crypto flips; requires the tenant GUC seat to pass FORCE-RLS (backstopped by 086 L-C) |
| Q-CUSTOMER-SCOPE | customer routes bind `customer_id=sub` only, ignore token `orderId` (`customer/orders.ts:50`) — cross-order read/cancel | **FIX-IN-PORT (S2 REV-3/T-12):** wire `require_order` + keep `customer_id` predicate; E2E token(A)→cancel(B)=403 (🔴 S5-T2) |
| Q-LC3-GUC | customer post-dispatch cancel seats `app.current_tenant=loc` then `updateOrderStatus` (`customer/orders.ts:324`) | **CARRY the fix verbatim** (DEP-1) — a context-free cancel 500s; the GUC dance is load-bearing pre/post-B3 |
| Q-COURIER-VERDICT | courier GET read via `courierReadVerdict` live-binding, 503-on-UNAVAILABLE fail-closed (`orders.ts:799`) | **CARRY verbatim** (ADR-0013) — never fail-open; binding-scope narrower than location |
| Q-CHANNEL-META | `x-channel` header → `orders.metadata.channel`, write-only, normalize-never-throw (`channel.ts`) | **CARRY verbatim** — no `sales_channel` table; DEFER the entity to a schema-evolution council (Q5a) |
| Q-MSGKIND-422 | `CreateOrderInput.messenger_kind` 3-value enum vs 6-kind checkout selector → phone/signal/simplex 422 (`legacy.ts:48` vs `messenger.ts:6`) | **FIX-IN-PORT (🔴 correctness, Q5b):** unify to the canonical 6-kind `MessengerKind`; E2E signal-order 422→201. Confirm DB CHECK admits 6 first |
| Q-OTP-DARK | OTP verify path (`orders.ts:163–353`), globally disabled (`OTP_ENABLED` off) | **CARRY dark** — port the branches inert (flag off); SMS is a `console.log` stub (REBUILD-MAP §8) |
| Q-CRYPTO-DARK | crypto prepaid fork creates `payments` row + hosted charge, swallows failure (`orders.ts:660–690`) | **CARRY dark** (ADR-0017, flags off) — a charge failure must never fail an already-committed order |
| Q-VENUE-GATE | `ENFORCE_VENUE_HOURS` closed-venue 409 (dark), `published_at` NOT_PUBLISHED 409, `delivery_paused` | **CARRY verbatim** — flag-guarded; refuse before any write |
| Q-CHILD-ENVELOPE | some 422s use bare `{error,code,details}` (MIN_ORDER, PHONE/IP_THROTTLE) not `sendError` | **CARRY** — post-Astro FE-lockstep (S2 Q4 / S3 Q3 posture); inline-fix candidate |
| Q-VELOCITY | phone (5/15min) + IP (20/15min) velocity throttles + fastify rate-limit (5/min) (`orders.ts:262–317`) | **CARRY verbatim** — anti-flood; keyed on real client IP (`clientIp`, #9 fix), not the Fly edge socket |

## 12. Cutover DoD (REBUILD-MAP §3, this surface)

Order/checkout E2E slice green (as-is specs — order-lifecycle, checkout, cash-as-proof, preflight,
tracking, cross-tenant-realtime order arms) · `openapi-diff` empty for the S5 namespace ·
invariant-cluster red→green:
- **Money byte-parity** — `applyTax` + full composition vs the zero-import hand-derived money vectors
  (inclusive/exclusive × zero/boundary rates); the **property test** `inclusive ⇒ total = subtotal+fee`;
  a route-level `POST /orders` composition matrix (≥4 vectors) → literal expected totals.
- **`i128` overflow guard** — a large-cart × 100%-rate vector produces the BigInt-identical tax (no wrap).
- **State-machine parity** — the 100-pair `order_status.rs` sweep (done) + a wired-handler probe: an
  illegal transition → 400 class; a concurrent double-transition → one 409 CONFLICT; the actor-gate
  (owner CONFIRMED→CANCELLED → 403); CC-1 strand guards (409 both arms) → then `/deliver` completes.
- **Tenancy** — a live NOBYPASSRLS probe asserts `app.current_tenant=locationId` seated on the create tx
  and every order-scoped write; `rls-adversarial` order-create WHERE-`location_id` gate green.
- **Customer order-scope (REV-3/T-12)** — token for order A → read/cancel/rate order B = **403**; the
  LC3 customer cancel under a FORCE-RLS non-bypass role → 200 + (dark) refund_due, not 500.
- **Idempotency** — replay (same key, same cart) → one order (200 replay); reused key + mutated cart →
  422; race → 409; **request-hash golden-vector byte-parity (both cutover directions)**.
- **Channel** — `normalizeChannel` allowlist/`other`/`web-direct` vectors; write-only metadata fold
  (never read by pricing/authz).
- **MessengerKind (if Q5b(b))** — a `signal` order that 422s today → 201; DB CHECK admits all 6.
- **Cutover-concurrency** — cross-stack idempotency probe (create on X, retry on Y → one order);
  migration 086 landed (shared `refund_due` floor); crypto dark throughout; combined pool-utilization
  monitored under the ceiling.

map-coverage zero-diff for the S5 namespaces · **council sign-off + rollback plan** (atomic proxy
flag-flip of the whole S5 family back to Node; time-boxed overlap). **No 🔴 S5 row builds before this
packet is APPROVED and the 🔴 questions (Q1/Q2/Q3/Q5b/Q6/Q7) are operator-signed.**

---

**council seats to run: breaker, counsel** (architect authored; human operator signs 🔴 Q1 money /
Q2 state-machine folds / Q3 order-write tenancy + customer-scope / Q5b MessengerKind / Q6 cutover
concurrency / Q7 migration-085-watermark timing).
**packet-status: 🟡 DRAFT.**
