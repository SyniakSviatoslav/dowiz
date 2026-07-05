# ADR 0007: Atomic `stock_remaining` Decrement at CONFIRM (+ flag-guarded restock, claim-first idempotency)

**Status:** PROPOSED — **stock RUNTIME DEFERRED to a named follow-up (Breaker R3-C1).** Only the inert
`products.stock_remaining` column ships in the sensor-seam batch (NULL=unlimited, zero runtime). The
decrement-at-confirm (§2) + restock trigger (§3) below are the **pre-designed spec for the "Stock-runtime
(decrement + restock) follow-up"**, NOT shipped in this batch. The idempotency `state` lifecycle (§4) is
independent of stock and DOES ship now. Implements brief §3.2.
**Version:** v4 — stock decrement/restock RUNTIME de-scoped from the sensor batch into a focused follow-up after
the area leaked THREE rounds running (C1 → R2-C1 → R3-C1, each a unit-leak via a different context). R3-C1
verified: the v3 trigger's `UPDATE products` hits 0 rows under `products` FORCE-RLS in the customer-cancel's
empty-`app.user_id` context → restocks nothing, consumes the flag. The follow-up's pre-designed fix
(SECURITY DEFINER restock fn, row-derived location, anti-cheat-green DoD vs the REAL handler) is folded into §3.
v3 — restock made UNBYPASSABLE via a DB trigger (Breaker R2-C1: a raw customer-cancel UPDATE
bypassed the service-method restock); claim-first idempotency txn-semantics pinned (Breaker R2-H2).
v2 — re-architected after Breaker C1/H1/M4 (decrement moved from create → CONFIRM; restock
compensation; claim-first idempotency; sorted multi-row lock order). v1 (decrement-at-create) is **withdrawn**.
**Supersedes:** nothing · **Extends:** the order lifecycle (`apps/api/src/routes/orders.ts`,
`apps/api/src/lib/orderStatusService.ts:89-117`) and the guarded-UPDATE anti-race idiom.
**Companion design:** `docs/design/mvp-sensor-seams/proposal.md` §2.1 · **Resolution:** `…/resolution.md` C1/H1/M4.
**Red-line:** 🔴 money / order correctness. Requires a red→green race+lifecycle test as DoD.

## Context

§2.3 of the brief adds an optional **daily stock cap** per product: `products.stock_remaining int`
(`NULL` = unlimited; a non-NULL int = a daily cap). §3.2 requires the decrement to be **atomic**, status-
guarded, idempotent, on the server-authoritative snapshot, with **no oversell on the last unit** AND **no
unit leak on any terminal path**.

**Grounded reality (verified — the v1 design ignored the lifecycle after COMMIT):**
- An order is INSERTed as **`status='PENDING'`** (`orders.ts:609`), NOT confirmed. Confirmation is a
  **separate, later** transition (`orderStatusService.ts:89-94`, owner tap or auto-confirm timer).
- The dominant terminal path for an unconfirmed order is the per-minute sweep:
  `UPDATE orders SET status='CANCELLED' … WHERE status='PENDING' AND timeout_at < now()`
  (`order-timeout-sweep.ts:67-71`) — a **bare flip, no restock**. Zero restock logic exists anywhere.
- **Therefore decrement-at-create (v1) leaked a unit on every never-confirmed order** → the whole daily cap
  exhaustible by orders nobody ever confirms (Breaker C1). v1 is withdrawn.
- The idempotency key is read at `orders.ts:364` but INSERTed at `:655` (near COMMIT) → two concurrent
  same-key txns both see 0 rows and both proceed (Breaker H1). `idempotency_keys` PK is `(location_id, key)`
  (`1790000000029_idempotency-composite-pk.ts:11`).
- `products.stock_remaining` does **not** exist yet (`1780310072731_menu.ts:18-31`). Money is `integer`.

## Decision

### 1. Columns — split across the seam boundary (v4)

**Ships NOW (inert seam, sensor batch):** the `stock_remaining` column only. NULL=unlimited → zero behavioural
change until an owner sets a number → cold-start safe, irreversible-part-now per "schema full, runtime later".
```sql
ALTER TABLE products
  ADD COLUMN IF NOT EXISTS stock_remaining integer;
ALTER TABLE products
  ADD CONSTRAINT products_stock_remaining_nonneg
  CHECK (stock_remaining IS NULL OR stock_remaining >= 0);
```

**Ships with the FOLLOW-UP (the runtime):** the `orders.stock_committed` flag (the idempotency guard for
decrement/restock — true on decrement, false on restock; a restock only fires `WHERE stock_committed = true`).
```sql
ALTER TABLE orders
  ADD COLUMN IF NOT EXISTS stock_committed boolean NOT NULL DEFAULT false;
```
Because no runtime in the sensor batch reads `stock_remaining`, the column is provably inert there; the flag is
meaningless without the decrement/restock code, so it lands with that code.

### 2. Decrement at **CONFIRM**, not at create — one guarded `UPDATE … RETURNING` per DISTINCT product

**Stock is a kitchen-commitment resource; the commitment is the confirm, not the customer's tap.** A PENDING
order reserves nothing. The decrement rides the CONFIRMED transition (`orderStatusService.ts:89-94`), in the
**same guarded UPDATE txn** that sets `confirmed_at = now()` and `status='CONFIRMED'`. Auto-confirm is the
identical transition fired by a timer — the decrement composes with it for free (brief §4 premise intact).

Iterate the order's distinct products **`ORDER BY product_id`** (deterministic lock order — Breaker M4: two
overlapping multi-item orders can never acquire row locks in opposing order → no 40P01 deadlock). Aggregate
quantity across duplicate line-items so the predicate sees true demand:

```sql
UPDATE products
   SET stock_remaining = stock_remaining - $qty
 WHERE id = $pid
   AND location_id = $loc
   AND (stock_remaining IS NULL OR stock_remaining >= $qty)
 RETURNING id, stock_remaining;
```

- **0 rows** → insufficient stock → ROLLBACK the confirm → the order stays PENDING and the confirm returns
  `422 { code:'OUT_OF_STOCK', error:'Product <name> is out of stock' }` (humane cause-hint — Counsel #4, NOT a
  bare 422). The owner/auto-confirm sees the line is unsellable; nothing is decremented.
- **NULL row** → `IS NULL` branch matches; `NULL - qty = NULL` → unlimited stays unlimited (no-op).
- All decrements + the `status='CONFIRMED'` + `stock_committed = true` share **one** guarded UPDATE txn → no
  partial decrement can survive; any 0-row line rolls the whole confirm back.

### 3. Restock made UNBYPASSABLE — a DB trigger on `orders`, not a service method (Breaker R2-C1)

**v2 wired restock into `orderStatusService.updateOrderStatus` and assumed every `CONFIRMED→terminal`
transition flows through it. Source disproves that assumption.** A grep of every `UPDATE orders SET status`
writer (verified) is:

| Writer (file:line) | Status it sets | Post-confirmed terminal? | v2 restock fires? |
|---|---|---|---|
| `orderStatusService.ts:91/101/113` (the guarded path) | CONFIRMED/DELIVERED/other | n/a (the home) | yes — but only here |
| `order-timeout-sweep.ts:68` (bulk sweep) | CANCELLED **`WHERE status='PENDING'`** | no — never confirmed (flag false) | n/a (correct no-op) |
| `owner/dashboard.ts:260` (reassign) | `'READY'` | no (not terminal) | n/a |
| `owner/signals.ts:230` (no_show) | sets `status_notes`, **not `status`** | no | n/a |
| **`customer/orders.ts:289-293`** (post-dispatch cancel) | **CANCELLED via raw `client.query`** | **YES — IN_DELIVERY ⇒ post-CONFIRMED ⇒ `stock_committed=true`** | **NO — bypasses `updateOrderStatus` → LEAK** |

The customer post-dispatch cancel (`POST /orders/:orderId/cancel`, gated on `IN_DELIVERY` + a 5-min window,
`customer/orders.ts:275,283`) flips status with a **raw UPDATE** and never calls `updateOrderStatus`. A
service-method restock is **by construction blind** to it: 5 ordinary customer cancels burn 5 units — the
identical permanent leak C1 describes, on a shipped customer-facing feature. **A service method is only as
strong as every code path calling it; a DB trigger cannot be bypassed by any `UPDATE orders SET status`
regardless of which writer issues it.** v3 moves restock from the service layer INTO the database:

> **R3-C1 (CRITICAL, verified) — why the v3 trigger as written STILL leaks, and the v4 fix.** A plpgsql trigger
> with no `SECURITY DEFINER` runs **SECURITY INVOKER** — under the caller's role AND its `app.*` settings. The
> customer-cancel handler (`customer/orders.ts:255-319`) runs on a **raw `db.connect()`** connection setting
> ONLY `app.settlement_reversal` — **never** `app.user_id`. `products` is **FORCE RLS** (`menu.ts:43`) with the
> sole writable policy `USING (location_id IN (SELECT app_member_location_ids()))`, and
> `app_member_location_ids()` returns the **empty set** when `app.user_id` is unset (`core-identity.ts:75-79`).
> So the v3 (INVOKER) `UPDATE products` matches **0 rows** in the cancel context → restocks nothing while
> flipping `stock_committed=false` (consuming the guard). The v4 fix makes the function **`SECURITY DEFINER`**
> (runs as the table owner, bypasses RLS) with the location derived from the **order row** (`NEW.location_id`),
> taking NO caller input — so it cannot be steered cross-tenant (see the abuse-safety proof below the DDL).

```sql
-- Restock is a property of the orders row's status transition, enforced in the DB so NO
-- code path (service method, raw UPDATE, future writer) can bypass it. Fires on the row
-- transition to a terminal NON-FULFILLED state while a unit is still committed.
-- SECURITY DEFINER (v4, R3-C1): the restock UPDATE must land regardless of the caller's RLS
-- context (the customer-cancel raw handler sets no app.user_id; products is FORCE-RLS, so an
-- INVOKER UPDATE would hit 0 rows there). DEFINER runs as the table owner and bypasses RLS.
CREATE OR REPLACE FUNCTION orders_restock_on_terminal() RETURNS trigger
  LANGUAGE plpgsql
  SECURITY DEFINER
  SET search_path = pg_catalog, public   -- pin search_path (DEFINER hardening; no caller-controlled resolution)
AS $$
BEGIN
  -- Only when a COMMITTED order crosses into a non-fulfilled terminal state.
  -- DELIVERED is fulfilled (the sale stands) → NOT a restock path.
  IF OLD.stock_committed = true
     AND NEW.status IN ('CANCELLED','REJECTED')
     AND OLD.status IS DISTINCT FROM NEW.status THEN
    UPDATE products p
       SET stock_remaining = stock_remaining + oi.qty
      FROM (SELECT product_id, sum(quantity) qty
              FROM order_items
             WHERE order_id = NEW.id
               AND product_id IS NOT NULL                  -- R3-H1: SET-NULL'd lines have no counter (accept-risk)
             GROUP BY product_id ORDER BY product_id) oi   -- sorted: deadlock-free (M4)
     WHERE p.id = oi.product_id
       AND p.location_id = NEW.location_id                 -- tenant scope: derived from the ORDER ROW, not caller
       AND p.stock_remaining IS NOT NULL;                  -- NULL (unlimited) = no-op
    NEW.stock_committed := false;                          -- idempotency: flip in the SAME row write
  END IF;
  RETURN NEW;
END;
$$;

CREATE TRIGGER orders_restock_on_terminal_trg
  BEFORE UPDATE OF status ON orders          -- fires only when status is in the SET list
  FOR EACH ROW EXECUTE FUNCTION orders_restock_on_terminal();
```

**SECURITY DEFINER abuse-safety proof (cannot cross tenants).** A SECURITY DEFINER function is only dangerous if
its target scope is **caller-controlled**. Here it is not: the function takes **no argument and reads no `app.*`
setting**; the only location it ever touches is `NEW.location_id` — the `location_id` of the order row whose
status is being flipped, which is itself written under that order's own tenant constraints and immutable on this
path. A caller in tenant X firing a cancel on an order in tenant X can only ever restock products
`WHERE p.location_id = <that order's location>` — i.e. tenant X's own products. There is no input by which a
caller selects another tenant's `location_id`; the scope is **narrowed to one row-derived value**, never widened
to a dynamic predicate. (`search_path` is pinned so the DEFINER body cannot be hijacked via a shadowed
`products`/`order_items` in a caller-controlled schema.) DEFINER here buys "the restock lands even with no member
context"; it does NOT buy any cross-tenant reach.

**R3-H1 — restock-line integrity under `order_items.product_id ON DELETE SET NULL` (verified, accept-risk).**
`order_items.product_id … ON DELETE SET NULL` (`1780338982023:6-8`) nulls the linkage when a product is
hard-deleted. The subquery's `AND product_id IS NOT NULL` makes that explicit (the join would skip NULLs anyway).
**Accept-risk:** a product that has been hard-deleted has **no `stock_remaining` counter to restock into** — the
restock target no longer exists, so the "leaked unit" is definitionally unobservable for that product. In a
multi-item order mixing a deleted product with a live one, the live line restocks correctly and the dead line is
skipped — the correct outcome; `stock_committed=false` is still correct because every restockable line was
restocked. A snapshot `product_id` is **rejected as over-engineering** (it would defend a state with no
observable effect). Owner: **Stock-runtime follow-up lead.**

**Why a trigger over "audit every raw writer and force them through `updateOrderStatus`":** weighed both.
- **(chosen) DB trigger** — robust against the customer-cancel raw UPDATE *and every future raw writer*; the
  invariant ("a committed unit is returned on any non-fulfilled terminal transition") lives where the data
  lives, so it cannot regress when someone adds a sixth status writer. Cost: a small amount of restock logic
  in plpgsql (logic-in-DB), and it shares the hot-orders-UPDATE path with the set-once trigger — but it is
  gated on `BEFORE UPDATE OF status` (only fires when `status` is in the UPDATE's SET list, so a pure
  timestamp/notes UPDATE never enters the function) and the body short-circuits on `stock_committed=false`
  (the common case: PENDING orders, already-restocked orders) with no query.
- (rejected) refactor all raw writers through `updateOrderStatus` — requires finding and rewriting every raw
  `UPDATE orders SET status` now AND forever; the customer-cancel path also runs courier-assignment + shift
  + cash-reversal logic in the same txn (`customer/orders.ts:295-317`) that does not belong in the generic
  status service. One missed or future-added raw writer re-opens the leak. Strictly more fragile than a guard
  the database itself enforces.

**Idempotency / no double-restock:** the trigger only acts on `OLD.stock_committed = true` and flips
`NEW.stock_committed := false` in the *same* row write (`BEFORE UPDATE`, so the flip is part of the committed
row, not a second statement). A re-fired CANCELLED→CANCELLED has `OLD.status = NEW.status` (the
`IS DISTINCT FROM` guard is false) AND `stock_committed` is already false → double no-op. A never-confirmed
PENDING order has `stock_committed=false` → no-op. A DELIVERED order is excluded by the `status IN
('CANCELLED','REJECTED')` test → the sale stands.

> NOTE the trigger replaces the service-method restock entirely; `updateOrderStatus` no longer needs its own
> restock branch — the owner-REJECT path (`dashboard.ts`) and any future confirmed-cancel are now covered by
> the same DB guarantee as the raw customer-cancel. The decrement (§2) stays in `orderStatusService` because
> it must be able to *reject* the confirm (0 rows → 422), which a BEFORE trigger cannot surface cleanly; the
> restock has no such return-value need, so the DB is the right home.

### 4. Claim-first idempotency — txn semantics PINNED (Breaker H1 + R2-H2)

**The one decision that determines correctness is the claim's txn placement. v2 left it unpinned; R2-H2
showed same-txn and separate-txn are mutually exclusive for the stated guarantee and a separate-txn claim can
crash-poison the key. v3 pins ONE mechanism: a SINGLE-TXN claim, with a `state` column that makes the replay
contract crash-safe.** The claim row carries an explicit lifecycle, so a crash never bricks the key and a peer
always gets the right body.

```sql
-- one-time DDL on idempotency_keys (idempotent ALTER; order_id is already nullable + REFERENCES orders):
ALTER TABLE idempotency_keys ADD COLUMN IF NOT EXISTS state text NOT NULL DEFAULT 'completed'
  CHECK (state IN ('claimed','completed'));   -- legacy rows default 'completed' (back-compat)
ALTER TABLE idempotency_keys ADD COLUMN IF NOT EXISTS claimed_at timestamptz;
```

**The single create txn (one BEGIN…COMMIT) does, in order:**

```sql
-- (1) CLAIM the key first, BEFORE building the order:
INSERT INTO idempotency_keys (location_id, key, request_hash, response_code, state, claimed_at, order_id)
VALUES ($loc, $key, $hash, 201, 'claimed', now(), NULL)
ON CONFLICT (location_id, key) DO NOTHING
RETURNING key;
--   1 row → THIS txn owns the claim → proceed.
--   0 rows → a peer already holds the row → go to the replay branch (below).

-- (2) build + INSERT the order (PENDING), then BEFORE COMMIT back-fill + complete the claim:
UPDATE idempotency_keys SET order_id = $newOrderId, state = 'completed'
 WHERE location_id = $loc AND key = $key;

-- (3) COMMIT.  Claim → order → completion all commit ATOMICALLY (one txn).
```

**Why single-txn is correct here (resolves R2-H2's "mutually exclusive" objection).** R2-H2 argued the
concurrency benefit requires the claim to be visible *before* the order commits — true only if you want a
peer to read a half-state. We do not: with single-txn, the unique index on `(location_id, key)` is the
serialization point. The first INSERT takes the index lock; **a concurrent peer's `ON CONFLICT DO NOTHING`
blocks on that lock until the first txn COMMITs or ROLLBACKs** (Postgres holds the speculative-insert lock for
the duration). On the first txn's COMMIT, the peer's INSERT sees the now-committed row → 0 rows. Crucially, at
that exact moment the order is *also* committed (same txn) — so the peer can immediately read a `completed`
key with a valid `order_id`. The "claim visible but order not" window R2-H2 feared **does not exist**: the
index lock makes the peer wait for the whole txn, and there is exactly one decrement (at confirm, later).

**Replay branch (the `0 rows` / prior-key path) — crash-safe by `state`:**

```sql
SELECT order_id, request_hash, state, claimed_at FROM idempotency_keys
 WHERE location_id = $loc AND key = $key;
```
- `request_hash` mismatch → `422 IDEMPOTENCY_KEY_REUSED` (existing behaviour).
- **`state = 'completed'`** → re-read the order body and return the **200-replay**
  (`SELECT id, status, subtotal, total, created_at, timeout_at FROM orders WHERE id = order_id`) — the prior
  order body, exactly the existing replay at `orders.ts:375-378`. **The replay re-enters that existing SELECT
  block; v3 explicitly wires the claim-first path to fall through into it rather than returning a bare 200**
  (R2-H2 part 3 closed — the peer returns the full order body, not an empty 200).
- **`state = 'claimed'` AND `claimed_at` is recent (< `CLAIM_STALE_MS`, e.g. 30 s)** → a peer is mid-flight in
  another txn (it has not committed). This is the rare lost-the-index-race-but-peer-still-running case → return
  `409 { code:'IDEMPOTENCY_IN_FLIGHT' }`; the client retries and hits the now-`completed` row. (In practice the
  index lock makes the peer block until commit, so this state is near-unobservable; it exists only for the
  separate-connection edge.)
- **`state = 'claimed'` AND `claimed_at` is STALE (≥ `CLAIM_STALE_MS`)** → the owning txn **crashed between
  claim and completion** (the only way a `claimed` row outlives the stale window, since claim+order+completion
  share one txn — a crash rolls the *whole* txn back, so a surviving committed `claimed` row can only come from
  a partial-commit pathology or an orphaned separate connection). The key is **reclaimable**: atomically
  `DELETE … WHERE state='claimed' AND claimed_at < now() - interval` then re-attempt the claim. This is a
  *guarded* reclaim (predicate on `state='claimed'` + age), NOT the v2 unconditional `DELETE WHERE id=NULL`
  that R2-H2 showed could double-create when two retries raced — only one `DELETE…RETURNING` wins the row, the
  loser sees 0 rows deleted and re-reads the winner's `completed` key.

**Why this can't double-create on crash (R2-H2 part 2 closed).** The v2 hole was: a separate-txn claim commits
a `(…, order_id=NULL)` row, the order txn crashes, two retries both `SELECT WHERE id=NULL → 0 rows`, both
`DELETE`, both create. v3 forecloses it three ways: (a) claim+order+completion are ONE txn, so a crash rolls
back the claim too — there is normally no surviving `claimed` row at all; (b) if one nonetheless survives
(orphaned connection), the reclaim is a guarded `DELETE … WHERE state='claimed' AND claimed_at < threshold
RETURNING` — exactly one concurrent retry wins the delete, the other gets 0 rows and re-reads; (c) the unique
index still serializes the subsequent re-claim. There is no path to two committed orders for one key.

Because the create path no longer decrements (decrement is at CONFIRM), a double-tap can never double-decrement
at create; claim-first additionally guarantees a double-tap never creates two orders, and the `state` machine
guarantees a crash leaves the key recoverable (never permanently poisoned) with a correct replay body.

### 5. Composition with existing rollback paths

The CONFIRM decrement and all create-path writes each sit inside their own `BEGIN…COMMIT`; any rule failure
(cash too low `:568`, delivery `:551`, velocity `:261`, enqueue error) rolls the whole txn back and Postgres
undoes the work. No write needs its own application-layer compensation **except** the post-confirm restock,
which is the deliberate, flag-guarded exception — and it is now enforced by the **DB trigger** (§3), so it
composes with *every* status-flip path (service method AND raw UPDATE) automatically, not just the ones routed
through `orderStatusService`.

## Race-correctness argument

**Two concurrent CONFIRMs of the last unit** (`stock_remaining = 1`, each order needs 1):
- Txn A's `UPDATE … WHERE … >= 1` takes a **row write-lock**, sets it to 0, returns 1 row → A confirms.
- Txn B blocks on the lock; on A's COMMIT, B re-evaluates the predicate against the committed value 0;
  `0 >= 1` false → 0 rows → B's confirm rolls back → the order stays PENDING (re-confirmable when restocked).
- **Exactly one** confirm succeeds. Same single-statement primitive as the status machine
  (`orderStatusService.ts:119-121`); no `SELECT … FOR UPDATE` window.

**No leak on ANY terminal path** (DoD matrix):
| Terminal path | Decremented at confirm? | Restocked? | Net unit effect |
|---|---|---|---|
| PENDING → timeout-CANCELLED | no (never confirmed) | no (flag false) | 0 |
| PENDING → owner-REJECTED | no | no | 0 |
| CONFIRMED → … → DELIVERED | yes | no (fulfilled) | correct sale |
| CONFIRMED → REJECTED/CANCELLED | yes | yes (once, flag-guarded) | 0 |
| any terminal transition fired twice | — | guarded by `stock_committed` flip | no double-restock |

## Proof / DoD (mandatory — red→green guardrail)

1. **Confirm-race test** (`apps/api/test/orders.stock-race.spec.ts`): seed product `stock_remaining = 1`; fire
   N concurrent CONFIRMs of N distinct PENDING orders → exactly one CONFIRMED, N−1 `OUT_OF_STOCK`-rejected;
   `stock_remaining = 0`, never negative.
2. **Lifecycle no-leak test**: assert each row of the matrix above — PENDING-timeout and PENDING-reject leak 0;
   CONFIRMED→CANCELLED restocks exactly once; a re-fired CANCELLED does not double-restock.
3. **R2-C1/R3-C1 ANTI-CHEAT-GREEN test — the restock must fire on the LEAKING route, in the REAL empty RLS
   context, and the row must actually MOVE.** Seed `stock_remaining = 5`; drive an order CONFIRMED→IN_DELIVERY
   (decrements to 4); then exercise the **`POST /orders/:orderId/cancel`** customer route (`customer/orders.ts:289`,
   the raw-UPDATE path that bypasses `updateOrderStatus`) within the cancel window → assert `stock_remaining` is
   back to **5** AND `orders.stock_committed = false`. **Hard requirements that make this catch R3-C1 instead of
   cheat-greening past it:** (a) the test MUST drive the HTTP route (or replay its EXACT raw-pool context — a
   `db.connect()` with only `app.settlement_reversal` set, NO `app.user_id`/`app.current_tenant`); (b) the test
   MUST NOT pre-set `app.user_id`, MUST NOT use a BYPASSRLS / superuser test role, and MUST run against
   `products` with FORCE-RLS active; (c) the assertion MUST confirm the row VALUE moved (5, not "no error") —
   "the handler returned 200" is NOT proof the `UPDATE products` landed. Under SECURITY INVOKER this test goes
   **RED** (0 rows under FORCE-RLS); under the v4 SECURITY DEFINER fix it goes **GREEN**. (This DoD is the follow
   -up's gate; in the sensor batch the runtime is deferred, so this test ships with the follow-up.)
4. **Claim-first idempotency test** (H1/R2-H2): concurrent same-key pair → exactly one order created + one
   **full-body 200-replay** (asserts the replay carries `id/status/total`, not a bare 200), never a 500; one
   decrement at confirm.
5. **R2-H2 crash-recovery test**: simulate a surviving `state='claimed'` row with a stale `claimed_at` (no
   completing order) → a retry RECLAIMS the key and creates exactly one order; two concurrent retries on the
   same stale `claimed` row → exactly one order (the guarded `DELETE … RETURNING` lets only one win), never two.
   A `state='completed'` key replay returns the prior order body.
6. **Deadlock test** (M4): two overlapping multi-item orders ({A,B} and {B,A}) confirmed concurrently → no
   40P01 (sorted lock order; the restock trigger also iterates `ORDER BY product_id`).
7. **NULL test**: `stock_remaining = NULL` → confirm succeeds, value stays NULL (regression-free); a cancel of
   a NULL-stock confirmed order is a restock no-op (the trigger's `stock_remaining IS NOT NULL` guard).
8. **Cause-hint test** (Counsel #4): the 422 body carries the product name + "out of stock", not a bare 422.
9. **Trigger hot-path test (R2-C1 perf)**: a pure-timestamp UPDATE (e.g. the bulk sweep's
   `UPDATE … status='CANCELLED' WHERE status='PENDING'`) on the restock-trigger table shows no material timing
   delta — the trigger's `BEFORE UPDATE OF status` + `stock_committed=false` short-circuit means non-confirmed
   rows never enter the restock body.
10. Regression-ledger row + the `CHECK (stock_remaining >= 0)` belt-and-suspenders invariant.

## Consequences

- Oversell is structurally impossible while stock is tracked; **no unit leaks on any terminal path** (C1
  closed) — and the restock is now a **DB trigger**, so it is UNBYPASSABLE by any `UPDATE orders SET status`
  writer, including the raw customer-cancel route that bypassed the v2 service-method restock (R2-C1 closed).
  The feature is dormant at `stock_remaining = NULL`.
- The decrement is the **one** order-correctness write licensed to fail an order (shared-resource control,
  brief §0.1). Every true sensor stays non-blocking (proposal §4.2).
- Latency: the decrement now rides the CONFIRM transition (a single guarded UPDATE that already runs), not the
  hot create path — the create path is **lighter** than v1. ≤N extra single-statement round-trips at confirm
  (N = distinct products, ~1–8), inside a transition that already runs (proposal §1c).
- M5: with decrement-at-confirm, a flood of never-confirmed PENDING orders decrements nothing → the create-time
  DoS-on-availability is neutralised; the cited velocity throttle (now phone+IP, proposal §4.3) bounds PENDING
  spam, not stock.
