# S5-ORDERS/MONEY Port — Council Packet · THREAT MODEL

> **STATUS: 🟡 DRAFT — NOT APPROVED.** Adversarial input to the S5 council. Assets, trust boundaries, and
> the failure modes the Rust port must not silently introduce — on the one surface where a defect is a
> *charge* defect (irreversible for crypto/cash) or a *forked/stuck order* defect. Read alongside
> `proposal.md`. Docs only; no code.

- **Method:** STRIDE-lite over the S5 order/money surface + fold-in of the money invariants (ADR-audit-fix-money
  LC1/LC6, ADR-0005 server-SoT), the S2 REV-3/T-12 customer order-scope, the S3 REV-10 tenancy-GUC contract,
  and the cutover-concurrency threat class unique to a strangler where **both stacks accept money-writes**.
- **Scope note:** the B3 (NOBYPASSRLS) flip and the `app_member_location_ids()` search_path pin are
  **B3-council fixes**; recorded here because they change what S5 must hold, but their *fix* lives in that
  council. The **payment webhook** (Plisio HMAC / L-B), **courier dispatch/settlement** (migration 085), and
  **WS fan-out** are OUT of S5 (proposal §2) — their threats are owned by the S7/S8 slices. S5 ports the
  **dark create-side** crypto fork and the **app-side L-A fold** only.

---

## 1. Assets

| ID | Asset | Where it lives | Why it matters |
|---|---|---|---|
| O1 | The **charged total** (`orders.total`, `subtotal`, `tax_total`, `delivery_fee`, `discount_total`) | `orders` (tenant, RLS) — integer minor units | The money actually owed; a one-minor-unit drift or a re-added inclusive tax is a **live overcharge** at scale (LC1: 16.7% of an inclusive cart at r=0.2) |
| O2 | The **order state** + its transition history | `orders.status`, `order_status_history`, `*_at` stamps | The single source of fulfillment truth; a bad transition strands a courier binding, fabricates a delivery, or loses a refund obligation |
| O3 | The **idempotency ledger** (`idempotency_keys` + `request_hash`) | `idempotency_keys` (tenant, RLS) | The ONLY guard against a duplicate paid order from a retry — cross-stack during cutover (Q6) |
| O4 | The **customer identity + contact** (`customers.phone`, `messenger_*`, `receiver_*`) | `customers`/`orders` (tenant, RLS) · never in the JWT | PII; the customer token is minted per-order and must never widen to customer-scope (T-12) |
| O5 | The **refund obligation** (`payment_events type='refund_due'`) | `payment_events` (tenant, FORCE RLS) | A cancelled/rejected order with a `paid` payment owes a refund; a lost obligation = 100% of principal kept (LC6). **Dark until crypto flips** |
| O6 | The **customer cash declaration + tip** (`cash_pay_with`, `tip_amount`) | `orders` (tenant, RLS) | Feeds the cash-422 backstop + courier settlement (S7); the settlement of that cash is S7 + mig 085 |
| O7 | The **channel attribution** (`orders.metadata.channel`) | `orders.metadata` jsonb (tenant, RLS) | Write-only analytics; must NEVER influence pricing/state/authz (a spoofed header must be inert) |
| O8 | The **tenant boundary on order writes** (`app.current_tenant`=locationId GUC) | txn-scoped GUC + RLS policies | The service-root seat that makes an order write land in the right tenant under FORCE RLS |

## 2. Trust boundaries

- **TB-1 anonymous/customer → order create (`POST /orders`)** — largely **unauthenticated** (public
  storefront checkout); the request body IS the input. Gates before any write: `locationId` validated
  against `locations` (404), `published_at` (409 NOT_PUBLISHED), `ENFORCE_VENUE_HOURS` (dark 409),
  preflight (availability + velocity throttles + signals + OTP), and the fastify rate-limit (5/min/phone-or-IP).
  **The tenant of the write is the body's `locationId`** — legitimate (the customer orders *at* that
  location), but it means the write's tenant is client-chosen-among-published, so the GUC seat + the
  `WITH CHECK` are what bind the row to that tenant.
- **TB-2 request → tenant GUC seat (order-write plane)** — the `with_tenant` combinator turns `locationId`
  into `app.current_tenant` inside one tx. **Currently seated on ZERO of the create-path writes** (raw
  pool, BYPASSRLS-masked — proposal §8). A context-free create dissolves the RLS arm the instant B3 flips.
- **TB-3 customer token → order scope** — a per-order JWT `{orderId, locationId, sub=customerId}` (14-day
  `?t=` tracking link, referer-leakable). The authority is the **minted `orderId` tuple**, NOT
  customer-wide. The Node routes bind `customer_id=sub` only (T-12 drift); the port must bind the `orderId`
  claim (S2 REV-3 seam) so `token(A)` cannot touch order B.
- **TB-4 owner → order (`PATCH`/`GET`)** — ADR-0004 P-d live `status='active'` membership-JOIN (the JOIN
  IS the tenant boundary; no bare `WHERE id=$1`). Plus the actor-gate (`assertOwnerTargetAllowed`) — an
  owner may not drive SYSTEM-only CANCELLED edges.
- **TB-5 courier → order (`GET`)** — ADR-0013 live-binding `courierReadVerdict` (binding-scope, narrower
  than location); 503-on-UNAVAILABLE fail-closed-distinguishable, never fail-open.
- **TB-6 stack → stack (cutover)** — the **novel** boundary: during the overlap a Node-created order may be
  transitioned by Rust and vice versa. The trust each stack places in the other's writes is mediated
  **only** by the shared DB invariants (the same tables, the same `idempotency_keys` unique, the
  stack-agnostic mig-086 trigger). A divergent money/fold implementation across stacks breaks it.

## 3. Port-specific failure scenarios (Rust)

| # | Scenario | Trigger in the port | Mitigation to prove red→green |
|---|---|---|---|
| **S5-T1** | **Wrong/absent GUC on the create write** — checkout INSERTs match 0 rows post-B3 (total silent checkout outage) | Carrying the raw-pool `db.connect()`+`BEGIN` context-free (proposal §8, Q3) | Route the create tx through `with_tenant(app.current_tenant=locationId)`; NOBYPASSRLS probe asserts the GUC is seated and the INSERT affects the row; `rls-adversarial` order-create WHERE-`location_id` gate (inherits S3/S4 REV-5) |
| **S5-T2** | **Cross-order / cross-customer access** — `token(orderA)` reads/cancels/rates order B (T-12) | Binding `customer_id=sub` only, ignoring the token `orderId` claim (`customer/orders.ts:50`) | Wire `CustomerClaimsExt::require_order(path_id)` (403 on mismatch) + keep the `customer_id` predicate (belt); E2E: token(A) → cancel/read/rate(B) = 403 |
| **S5-T3** | **Promo/discount abuse** — a wired-but-unhardened redemption is drained, or a client supplies a negative `discountTotal` | Wiring redemption in the port (Q1b) without an abuse model; or trusting a client discount field | **CARRY `discountTotal=0`** (Q1a) — no redemption exists to abuse; the `− discountTotal` seam is a literal `0`, never client-supplied. Redemption = own council with its own abuse threat. Accepted-risk row + owner |
| **S5-T4** | **Negative-money / overflow / rounding drift** — a total that under/over-charges, or an `i64` wrap that *reduces* a charge | `f64` money arithmetic; an `i64` intermediate in `applyTax` (`sub·rateMicro` reaches ~1e18 at 100% × large cart); dropping half-up | RED LINE: integer-only, `i128` intermediate in the tax math; `assertNonNegative(total)`; byte-parity vs the zero-import hand-derived money vectors + the `inclusive ⇒ total=subtotal+fee` property + a large-cart×100%-rate overflow vector |
| **S5-T5** | **Idempotency bypass / duplicate paid order** — a retry produces two orders/charges | `request_hash` canonicalization drifting from Node (cross-stack, Q6); or reading `.userId` (undefined) instead of `.sub` for the customer id in the hash (#8) | Carry the exact `buildRequestHash` inputs + the `.sub` read; **golden-vector byte-parity (both cutover directions)**; replay→200-one-order, reused-key+mutated-cart→422, race→409 |
| **S5-T6** | **State-machine bypass / illegal transition** — an order jumps to DELIVERED with no attestation, or an owner drives a SYSTEM-only cancel | Skipping `assert_transition`, the status-guarded `WHERE status=$current`, the actor-gate, or the CC-1 strand guards | The 100-pair matrix (done) + wired-handler probes: illegal→400 class; concurrent double-transition→one 409 CONFLICT; owner CONFIRMED→CANCELLED→403; DELIVERED with active/undelivered binding→409 (both arms) then `/deliver` completes |
| **S5-T7** | **Stranded courier binding / lost refund obligation** — a terminal transition leaves a live binding, or a cancelled paid order records no `refund_due` | Dropping the R2-3 fold or the L-A fold on the `updateOrderStatus` port | Carry both folds verbatim; the L-A fold under a tenant-seated tx (S5-T1) passes FORCE-RLS; backstopped by the stack-agnostic mig-086 L-C trigger (land before flip). Inert until crypto flips but proven then |
| **S5-T8** | **Channel spoofing** — a forged/oversized `x-channel` header influences pricing, routing, or authz, or crashes the create | Reading `orders.metadata.channel` in a pricing/state/authz decision; or throwing on a malformed header | Carry `normalizeChannel` (never throws — malformed→`other`, missing→`web-direct`, ≤32 chars, allowlist); assert the channel is **write-only** (a guardrail that no pricing/state/authz path reads `metadata.channel`) |
| **S5-T9** | **Cutover double-order** — the same intent creates two orders because two stacks accept it (TB-6) | A cross-stack retry where the request-hash differs, or a non-atomic route-by-route flip within S5 | **Atomic per-surface flip** (whole S5 family together, S3 REV-7); shared `idempotency_keys` unique; request-hash byte-parity (S5-T5); a **cross-stack idempotency probe** (create on X, retry on Y → one order); crypto dark throughout the overlap |
| **S5-T10** | **MessengerKind 422 → lost order** — a customer picking phone/signal/simplex is 422'd at the order boundary | Carrying the stale 3-value `CreateOrderInput.messenger_kind` enum (`legacy.ts:48`) against the 6-kind checkout selector | Q5b: unify to the canonical 6-kind `MessengerKind` (matching the checkout + the DB CHECK), confirmed by a Phase-0 `ci-schema-drift` read that the CHECK admits all 6; E2E signal-order 422→201 |
| **S5-T11** | **Price/total tampering** — a client submits its own total/subtotal and the server honors it | Trusting any client-supplied money field instead of the in-tx MVCC recompute | Server-authoritative (ADR-0005): the total is computed from the snapshotted `products.price`/`modifiers.price_delta`; the ONLY client money input is `cash_pay_with` (validated `≥ total`) + `tip_amount` (validated `≥0` int). Guardrail: no request money field feeds `total` |
| **S5-T12** | **Order-flood / velocity bypass** — an attacker rotates phones/IPs to flood a location | Dropping the phone (5/15min) + IP (20/15min) velocity throttles or the fastify rate-limit; keying on the Fly edge socket instead of the real client IP | Carry all three throttles verbatim (`velocity_events`), keyed on `clientIp(request)` (the #9 real-IP fix), not `request.ip`; the bounded write-tx `statement_timeout=4500` fuse caps a single wedged write |

## 4. What the B3 RLS flip changes for S5

- **Today (BYPASSRLS):** RLS is bypassed; the explicit `WHERE location_id`/`customer_id` predicates + the
  server-authoritative recompute are the only live boundaries. The **context-free create "works" despite
  seating no GUC** — the danger is invisible, exactly the S3 anonymizer-N1 / S4 raw-pool masking, now on
  the money hot path.
- **Post-flip (NOBYPASSRLS):** RLS is authoritative on every order-write. `orders`/`order_items`/
  `idempotency_keys`/`velocity_events`/`customers`/`customer_track_grants` INSERTs need
  `app.current_tenant` for `WITH CHECK`; the `updateOrderStatus` folds (`order_status_history`,
  `payment_events` L-A) need it to pass FORCE RLS (the dual-policy GUC arm). **The context-free create
  matches 0 rows (S5-T1).** The B3-council fixes named (not fixed) here: the `app_member_location_ids()`
  search_path pin (the owner GET/PATCH JOIN reads it transitively) and the GUC-always-seated invariant.
  **The mig-086 L-C trigger's per-row GUC save/restore dance is the template** for how a DEFINER surface
  writes `payment_events` under FORCE RLS pre- and post-B3.
- **S5's rule:** every order write is correct **independent of which pool role is live** (belt = explicit
  predicate; suspenders = seated GUC); the money recompute + state folds are byte-identical across stacks
  so the B3 flip and the Node→Rust flip are two orthogonal, independently-reversible events.

## 5. Residual risks (summary for the human)

- **The `discountTotal=0` carry (S5-T3 / Q1a)** — re-shipping a known money gap (no promo redemption)
  through a deliberate rewrite. Defensible as "unbuilt feature, not a defect," but must be an **explicit
  accepted-risk with a named owner**, not a silent omission. **The most likely counsel flag.** Owner:
  operator (product decision) + S5 lead (the seam).
- **Cross-stack double-order (S5-T9 / Q6)** — the money-irreversible failure the atomic-flip posture
  exists to prevent. Bounded by the shared `idempotency_keys` unique **iff** the request-hash is
  byte-identical (S5-T5). **The most likely breaker escalation** — the council should have the breaker
  attack the request-hash canonicalization + a route-by-route flip. Owner: architect + operator.
- **The 2026-07-10 settlement watermark (Q7)** — not an S5-order dependency, but a shared timing landmine:
  a settlement apply that slips past the literal DOUBLE-PAYS old courier rows unless the operator bumps all
  three occurrences. Surfaced so the rebuild schedule cannot silently trip it. Owner: operator.
- **The customer token is a referer-leakable per-order bearer (TB-3)** — the REV-3 order-scope wiring
  bounds a leaked token to ONE order (read/cancel/rate), not the whole customer. The residual (the
  httpOnly-vs-localStorage question) is an S2 fast-follow, not re-opened here. Owner: S2 lead.
- **Order writes seat a client-chosen (among published) tenant (TB-1)** — legitimate (the customer orders
  at that location), and the `WITH CHECK` binds the row to it; but it is worth the council pricing that the
  order-write tenant is not derived from an authenticated principal the way owner/courier writes are — it is
  derived from a validated body field. Accepted as a *current, correct* property (public checkout);
  documented so the port doesn't accidentally "harden" it into a broken flow. Owner: S5 lead.

**None of O1–O8's failure modes is *introduced* by the rewrite** — each (money composition, state folds,
tenancy seat, idempotency, customer-scope, channel) is a **current** property the port must carry
**visibly** (matrix row + test). The rewrite's *new* risk is entirely the **cutover concurrency** (S5-T9,
TB-6) — two stacks writing money to one ledger — which no prior single-stack packet faced at this stakes.
**Breaker-escalation candidate: the cross-stack request-hash drift (S5-T5/T9).** **Counsel-flag candidate:
the `discountTotal=0` carry (S5-T3)** — CARRYing a known money gap through a deliberate rewrite is
acceptable only as an explicit, owned accepted-risk, never by silence.
