# Reliability Gate — Rust cutover (staging), 2026-07-05

Traced one order across the flipped stack (9/10 surfaces on Rust) + audited the Rust port's
lifecycle guarantees with two decorrelated agents. This is the CUTOVER gate: does the Rust
stack preserve DeliveryOS's exactly-once / recoverable / cross-surface-consistent lifecycle.

## Live trace (against https://dowiz-staging.fly.dev, flags = rust)

| Stage | Result | Served by |
|---|---|---|
| L0 entry `/s/demo` | 200 | `astro:S1` (human UA) |
| L1 menu read | 200 | `rust:S1` |
| L2 order create | 201, correct money | `rust:S5` |
| L2 idempotent double-POST | 200, same order id | `rust:S5` |
| L4 owner CONFIRM | 200 (after `148ee7ea`) | `rust:S5` |
| L4 double-CONFIRM (race guard) | **409** — anti-race UPDATE holds | `rust:S5` |
| L5 PREPARING / READY | 200 / 200 | `rust:S5` |
| L5 illegal READY→PENDING (matrix guard) | **409** — frozen matrix rejects | `rust:S5` |
| L6 READY→PICKED_UP (pickup terminal) | 200 | `rust:S5` |
| L11 owner order list shows PICKED_UP | 200, order visible + correct status | `node` (keep-gated, same DB) |
| L7 courier DELIVERED (delivery path) | **LIVE GREEN** — 200, order→DELIVERED, delivery_trace paid_full cash=total exact, cash_ledger hold, assignment delivered (after enum-read `148ee7ea` + payment_outcome-write `b96a7790` fixes) | `rust:S7` |

The L4/L7 500s were the read-side enum-decode bug (`o.type`, `o.payment_method` read into Rust
`String` without `::text`) — same root class as every other cutover SQL bug; fixed and re-traced
green. Race guard (double-CONFIRM→409) and matrix guard (illegal transition→409) both PROVEN LIVE.
Cross-surface consistency holds across the node/rust split because both stacks read one Postgres.

## Agent audit 1 — exactly-once + idempotency (create) — PASS

- ONE `pool.begin()`→`commit` binds the order row + idempotency_keys row atomically (pg.rs:95/394/420/434).
- Hash-first Replay/Reuse dedup (state.rs:177-187): double-POST → same order (Replay 200);
  same-key-different-hash → 422; missing order → DeleteAndRecreate. Backstopped by the
  `idempotency_keys(key,location_id)` unique → loser 23505 → 409 (no orphan).
- Money server-recomputed from `products.price` (pg.rs:232-239 → pricing composition); no client
  total trusted; `cash_pay_with` is tender-only, gated ≥ total.
- Create tx seats NO app.user_id GUC (REV-S5-1); no rollback-surviving partial write on any arm.

## Agent audit 2 — DELIVERED termination + state machine — PASS (2 nuances)

- Race guard PASS: `UPDATE … WHERE status=$expected RETURNING`; 0 rows → 409 (owner surface).
- **State machine byte-identical to Node** — diffed `order-machine.ts:18-40` vs Rust
  `order_status.rs:56-63`, all transitions/precedence/terminals identical; 100-pair exhaustive test.
- `delivery_trace` idempotent (`ON CONFLICT (order_id) DO NOTHING`); `courier_cash_ledger` hold on
  cash-collected (`ON CONFLICT (order_id,type)`), exact-equality cash-as-proof precheck before write.
- Single-tx `with_tenant` = kill-before-commit rolls back everything → no orphan/partial state.

### Nuance A (FLAG, not a durability bug) — courier deliver discards the transition conflict-bool
`assignments.rs:1160-1164` calls `apply_transition(...).await?` and DROPS the returned `bool`. If
the order raced out of `IN_DELIVERY` (a concurrent `customer_cancel`, which locks only the order row
and never the assignment), the order-side WHERE-guard correctly no-ops BUT the assignment terminalize
+ cash-ledger hold + `delivery_trace` + `payment_outcome='paid_full'` still get written. Order-side
stays consistent; the assignment/ledger writes are NOT gated on the order transition landing.
**This is a money/delivery-path cross-surface race — a deliberate business decision (the courier
already collected cash), not a reflexive patch. → S7/S5 council item.**

### Nuance B (by design) — `order_status_history` SAVEPOINT swallow
The audit row is best-effort inside `SAVEPOINT osh` (pg.rs:726-748) — a history INSERT failure
rolls back to the savepoint and does NOT fail the transition. With the `::order_status` casts
(148ee7ea's sibling fix, 886c83d7) it now succeeds on valid enums; the swallow structure is
intentional (audit never blocks a transition).

## Verdict (cutover)

**GO for the flipped surfaces' core lifecycle**, conditional on the enum-read redeploy landing green
(L4/L7 re-trace). Exactly-once ✅, recoverable ✅, state-machine byte-identical ✅, money-recompute ✅.

**NOT-YET-GO for prod**, gated on: Nuance A (council), S8 webhook path-secret parity, the strangler
tail (unmounted routes still on Node), S5 deferred `preflight`+track-token, and the operator's
per-surface prod-flip gos (S5-money, S9-GDPR explicit). Deferred lifecycle infra (feedback-reminder
queue, honest-dispatch) is the documented pre-existing debt, not a cutover regression.

## The one systemic ratchet
Every SQL bug this cutover — numeric→f64, int4→i64, text→enum (bind AND decode), missing NOT-NULL —
is the same family: **passes as a psql literal / text render, fails as a sqlx bind/decode.** The
`#[ignore]` live-PG cargo suite exercises exactly these against a real Postgres. Wire it into CI and
this entire class is caught at build time instead of one redeploy at a time.
