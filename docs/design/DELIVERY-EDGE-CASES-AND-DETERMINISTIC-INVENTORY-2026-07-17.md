# DELIVERY EDGE CASES (CONFLICTS / REORDERING / CANCELLATION) + DETERMINISTIC REAL-TIME INVENTORY (2026-07-17)

> **What this is:** (1) a code-grounded catalog of the hardest edge cases a real delivery platform
> hits — concurrent writers on one order, ordering/ID determinism, cancellation and its money
> consequences, and the surrounding cases (idempotent submission, substitutions, address changes,
> reassignment, payment failure) — each traced against what `kernel/src/` actually does today;
> (2) a design for deterministic, event-sourced inventory counting where the software's own
> bookkeeping never needs a human, and a human is needed ONLY to input a physical recount.
> **Planning artifact only — no product code written or edited.** Branch: `feat/harness-llm-backend`.
> **Builds directly on** (does not re-derive): `DELIVERY-FLOWS-BACKEND-AUDIT-AND-MULTITENANT-TESTING-2026-07-17.md`
> (its findings R1–R8 and §3.2 are treated as established evidence) and
> `HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md` (gap G1 + addendum D1/D4 are the menu/store-state
> context this design plugs into). Every new claim below carries a file:line read this session.

---

## 1. Problem + non-goals

**Problem.** The audit established WHAT is missing (no cancel past `Pending`, one global `ORDER_SEQ`,
untrusted-price boundary). This doc answers the next question: what CONCRETELY goes wrong at those
seams under real delivery load — two writers racing on one order, retries, mid-flight cancellations —
and what conflict/ordering/cancellation RULES the kernel must adopt, stated precisely enough to
implement. Separately, the operator's inventory vision needs a mechanism, not a policy statement:
counts that are a pure function of an event log, verified by replay, with humans confined to
physical-recount input.

**Non-goals:** no implementation; no new phase (every fix names its owner among P07/P13/P16 + the
audit's T-layers); no re-derivation of the market research; no operator rulings made here (O3, O20,
and the FSM-signature bump R3 are flagged where they bite); dispute/refund adjudication content is
P14's (gated on O3) — only its state-machine boundary is settled here.

---

## 2. Back-of-envelope (sizes every decision below)

- One hub (audit §2): peak 2–4 orders/min, 300–600 orders/day, ~5k lifecycle events/day.
- Inventory events at item-level tracking (~3 tracked items/order + restocks + one weekly stocktake
  of ~200 lines): **~1–2k stock events/day**; ingredient-level BOM later multiplies ~×5 → ~10k/day.
- Fold cost: each stock event is one `BTreeMap` lookup + checked add (~0.1–1 µs). A full day refolds
  in <10 ms; a full **year** (~0.5–4M events) refolds in ~0.5–2 s. Consequence: the paranoid
  strategy — refold the whole ledger from genesis at boot, the same shape as
  `kernel_boot_verify_fsm` (`lib.rs:177`) — is affordable at hub scale. Snapshotting (the
  content-addressed `backup.rs` store exists for it) is an optimization with a named trigger
  (boot refold >5 s), not a v1 requirement. Boring-first.
- Storage: ~100 B/event → 50–400 MB/year in-log per hub. Fine for a sovereign node.
- No connection-pool budget applies: native per-hub store, no shared DB (audit §2, D1 canon).
- Conflict-rule cost: the state-fenced decide proposed in §3.1 adds one projection lookup per
  commit (~µs). At 4 orders/min it is unmeasurable; it would still be fine at 1000× that.

---

## 3. PART 1 — Edge-case catalog

### 3.1 Order conflicts: two writers race on the same order

**What the code does today — traced, not assumed:**

1. `apply_event` (`domain.rs:220-231`) is a pure function `(&Order, OrderStatus) → Result<Order>`.
   It validates the transition against the CALLER'S snapshot of the order and returns a new value.
   There is no version field, no compare-and-swap, no store. Concurrency is simply out of its scope.
2. The wasm surface makes this concrete: `apply_event_js` (`wasm.rs:295`) takes the full order JSON
   from JS and hands the updated JSON back — **the client is the state store**. Two admin tabs
   holding the same order at `Confirmed`: tab A applies `PREPARING`, tab B applies `IN_DELIVERY` —
   both legal from `Confirmed` (`order_machine.rs:68`), both succeed, and there are now two
   divergent orders with the same id, with no arbiter anywhere. Today nothing persists either one
   (audit §9 Q2: no runtime backend), so the "resolution" is whichever JSON a future store happens
   to receive last — last-write-wins by accident, not by rule.
3. The event-log path is better but incomplete. `EventLog::commit_after_decide`
   (`event_log.rs:339-361`) serializes writers mechanically — `&mut self` means one writer per log
   instance, and the hash chain (`append`, `event_log.rs:293-311`) gives a **total order per hub**.
   But the `decide` closure receives only the event (`FnOnce(&MeshEvent) → Result<T,E>`); it has no
   access to the folded aggregate state. The only test composing the log with the order Law
   validates a transition between two CONSTANTS (`Pending→Confirmed` hardcoded,
   `event_log.rs:580-609`). So a stale-snapshot race passes the Law: writer A commits
   `Confirmed→Preparing`, writer B (holding the same stale `Confirmed`) commits
   `Confirmed→InDelivery` — `assert_transition` approves each in isolation, both events land in the
   chain, and the replay fold (`fold_transitions`, `order_machine.rs:140-152`) then hits
   `Preparing→InDelivery`, which is illegal, and **stops permanently at the first invalid step**.

**Verdict: conflict resolution is UNDEFINED today, and the failure mode is worse than
last-write-wins.** The chain is append-only and content-addressed — the losing event cannot be
removed. The order's history is now permanently poisoned: every replayer deterministically stops at
the same point (determinism holds), but the order is stuck, and the only surface that notices is
`reduce_anomalies` counting it as an anomaly after the fact (`analytics.rs`, audit row 18). Neither
optimistic concurrency, nor LWW, nor CRDT — a commit-time gap with an append-only blast radius.

**Two adjacent mechanical findings (new this session, both in `event_log.rs`):**

- **F1 — the idempotency check dedupes the wrong id.** `commit_after_decide` computes the dedup id
  BEFORE chaining (`event_log.rs:348` — `ev.prev` still zeroed), but `append` re-binds `prev` to
  the tip and stores the POST-chain id (`event_log.rs:297-302`). Consequence: on any non-empty log,
  a byte-identical re-submission with zeroed `prev` is **not** detected as a duplicate — the
  unchained id is never in the store — so `decide` re-runs and a second chain entry commits. The
  passing test `dup_event_is_idempotent_no_state_change` (`event_log.rs:533-561`) only covers the
  **genesis** event, where tip is `None` and the two ids coincide. The audit's consistency note
  ("a replayed duplicate is a structural no-op", audit §5) is therefore true only for replays that
  carry the exact chained `prev` bytes (network sync) — NOT for local client retries. See §3.4.
- **F2 — a non-zero `prev` is never validated against the tip.** `append` binds only the zeroed
  case (`event_log.rs:297-301`); a caller-supplied wrong `prev` is accepted verbatim and becomes
  the new tip's ancestor claim. The stored sequence stays linear, but the content-id chain is no
  longer walkable — replay verification by hash-chain traversal would break silently. (`append_raw`
  at `:321-329` is deliberately unchained for self-witness rows; the hazard is `append` accepting
  arbitrary `prev` on the ORDER path.)

**The rule to adopt (design, ≥2 options per protocol):**

| | **A — State-fenced decide (optimistic concurrency)** — chosen | **B — Last-write-wins** | **C — CRDT merge** |
|---|---|---|---|
| Concept | Compare-and-swap on the aggregate's folded state; the chain tip is the version/fencing token | Timestamp ordering, later overwrite | Commutative merge function |
| Mechanics | Intent carries `expected_from`; `decide` folds the order's current status from the log projection and rejects when `folded ≠ expected_from`. First-to-commit wins; loser gets a **typed conflict error** and re-reads | Whichever event lands later replaces state | Define a join for concurrent statuses |
| Why / why not | Fits the existing seam exactly — `commit_after_decide` already runs decide-before-persist; only the closure's INPUT changes (folded state, not caller snapshot). No new runtime | Silently discards an actor's action on a money-adjacent lifecycle — a courier's `PickedUp` or an owner's cancel vanishing is unacceptable | The lifecycle is a DAG with non-commutative transitions; `Cancelled ∥ PickedUp` has no lattice join that respects the transition table. CRDTs fit availability-biased data, not a CP intra-hub aggregate (audit §5: intra-hub = single-writer fold) |

The operator's example resolves cleanly under A: owner submits "out-of-stock substitution" at the
same instant the courier submits `Ready→PickedUp`. Both enter one per-hub chain; the chain
serializes them; the second one's `decide` sees the first one's fold. If pickup landed first, the
substitution intent is refused typed ("order already PICKED_UP") and the owner's UI says so; if the
substitution landed first, the pickup proceeds against the amended order. A definite winner, both
actors informed, zero corruption — versus today, where both would "succeed."

Closing F1+F2 rides the same fix: the fence must include a **per-actor sequence registry**
(`(actor_pubkey, actor_seq)` seen-set, derived as a fold projection of the log — never a separate
mutable set) so a retried intent with the same `actor_seq` is a structural duplicate regardless of
tip movement, and `append` must reject a non-zero `prev` that does not equal the current tip
(typed `StaleTip` error — the wire-sync path that legitimately carries foreign chains goes through
its own verify path per audit T5, not through local `append`). **Owner: P13 intake spine** (it owns
the order↔log binding per audit R1/R4); the F1 fix is small enough to land with the audit's T-layer
test pass and should, because T2's harness will otherwise sit on a broken dedup.

### 3.2 Reordering: what the global `ORDER_SEQ` actually breaks

`wasm.rs:51` — `static ORDER_SEQ: AtomicU64 = AtomicU64::new(0)`, used at `wasm.rs:191-193`:
`id = format!("ord_{seq}")` and `created_at_ms = seq as i64`. Specific failures, concretest first:

1. **Reset-on-instantiation ID reuse — single-tenant, today.** The counter is per-wasm-instance
   with no persistence. Every page load starts at 0: two browser tabs each mint `ord_0`; a reload
   re-mints ids already used. This is not a multi-tenant hypothetical — it is two customers in one
   restaurant tonight.
2. **Cross-tenant collision — the audit's §5.2 hazard.** Every hub's instance counts from 0, so any
   cross-hub aggregation (analytics, mesh sync) sees colliding `ord_N` ids.
3. **Silent misattribution downstream.** `ChannelLedger.ingest` locks an order's channel at first
   sighting and silently ignores duplicates (`analytics.rs:60-66`, audit §4.2) — colliding ids
   don't error, they corrupt attribution with `false` as the only signal.
4. **Fake timestamps.** `created_at_ms = seq` means order times are 0,1,2… — every downstream
   consumer of `at_ms` (funnel ordering, out-of-order detection in the `geo.rs` genre) receives
   fabricated clocks.
5. **Replay/linkage unsoundness.** The moment order ids appear inside other event families (the
   §4 stock ledger links `Reserved/Consumed/Released` to `order_id`), colliding ids make the
   linkage invariant unverifiable.

**What determinism guarantee is needed, and what already exists:**

- **Total order per hub: REQUIRED and PROVIDED.** The hash chain gives one linear, content-addressed
  history per hub (`event_log.rs:293-311`), and `&mut` single-writer access plus process-per-hub
  (audit §4.3 Option A) makes it exclusive. `fold_transitions` replaying that order is
  deterministic by construction. Nothing more is needed intra-hub.
- **Partial order across hubs: SUFFICIENT and (correctly) all that is promised.** No global
  sequencer exists or should (audit §5: AP inter-hub). Cross-hub causality is carried by `prev`
  links and per-actor `actor_seq` (`event_log.rs:139-142`); cross-hub analytics only needs ids to
  be **collision-free**, which T4 already derived as a requirement.
- **The missing piece is ID MINTING, and the kernel already owns the right primitive.** The order
  id should BE the content-id of its placement event — `MeshEvent::event_id()`
  (`event_log.rs:148-155`), SHA3-256, collision-free with no counter, no global, no coordination.
  `created_at_ms` becomes an explicit host-clock input to the placement call (the kernel never
  samples a clock — same purity rule the whole crate follows). This deletes `ORDER_SEQ` outright,
  which also empties the audit's T1 allowlist by one entry. **Owner: P13 intake (audit R4 already
  assigns id minting there); this section supplies the mechanism.**

### 3.3 Cancellation: where the FSM forbids what the business needs

The transition table (`order_machine.rs:64-78`) reaches `Cancelled`/`Rejected` **only from
`Pending`** (`:67`). Checked against the states where a real delivery business needs cancellation:

| From-state | Real-world need | FSM today | Money/state consequence if allowed |
|---|---|---|---|
| `Pending` | Customer withdraws / owner rejects before accept | **Supported** (`:67`) | None — pre-money |
| `Confirmed` | Customer grace-window cancel; owner discovers an item is gone before prep; **payment-auth failure** (§3.4) | **Forbidden** | Must void the payment authorization (when P13's charge plane exists); zero waste |
| `Preparing` | Mid-prep cancel — food partially made | **Forbidden** | Waste is real: needs a compensation policy slot (the market publishes these — DoorDash error-charge rules, HUB §2.1); couples to a `Wasted` stock event (§4) |
| `Ready` | Pickup no-show; courier unavailable and food dying | **Forbidden** | Waste + possible re-shelf; same compensation slot |
| `InDelivery` | **Failed delivery** — customer unreachable, wrong address, courier abandons | **Forbidden — and worse: `InDelivery` has exactly one exit, `Delivered` (`:71`).** A delivery that fails is UNREPRESENTABLE: the honest options today are lying (`Delivered`) or an order stuck forever | Return-to-store flow; charge policy; the sharpest structural hole in the table |
| `Delivered`/`PickedUp` | Dispute-driven refund | Correctly forbidden | **Refund is a money-plane event, not a state edit.** The delivery physically happened; rewriting the state would falsify the PoD evidence chain that D5 explicitly makes the dispute evidence (HUB §4-D5). A refund is a P13 ledger entry referencing the order + a P14 dispute record — the FSM stays untouched. This is the line where "allowing cancel" would violate the money/state invariant |

The audit already flagged the UI contradiction (Dashboard plans owner "Reject→CANCELLED" on live
orders the FSM would refuse — audit row 8) and assigned compensation edges to P07 (R3). This
section settles the SHAPE of the fix:

| | **A — Four forward edges into the existing `Cancelled`, reason as payload** — chosen | **B — Saga states (`CancelRequested`, `DeliveryFailed` terminals)** |
|---|---|---|
| Edges added | `Confirmed→Cancelled`, `Preparing→Cancelled`, `Ready→Cancelled`, `InDelivery→Cancelled` | Same + intermediate request/ack states and a distinct failed-delivery terminal |
| Who/why cancelled | Payload data on the cancellation event (`actor: Customer\|Owner\|System`, `reason` code, waste flag) — schema rich, runtime minimal; the FSM does not need N states to record N reasons | Encoded as states — doubles the state count to represent metadata |
| The in-flight race (cancel arrives while courier drives) | Handled by §3.1's chain serialization: cancel and `Delivered` race to commit; first wins, loser gets typed conflict. No ack states needed | The ack state exists to model exactly this race — redundant once commits are fenced |
| Graph consequences | Still a DAG (all new edges enter a terminal): `edges 9→13`, `μ = 13−10+2 = 5`, `ρ = 0`, `is_acyclic` true, reachability unchanged (767 — `Cancelled` was already reachable) | Vertices change too; larger signature diff |
| Gate discipline | `FSM_GOLDEN_SIGNATURE` (`order_machine.rs:472-483`) must be consciously bumped with these exact values + rationale — the gate exists precisely to force this (audit R3: operator-gated) | Same, bigger |

**The one non-negotiable coupling (P07's law):** a cancellation edge from any post-`Confirmed`
state may only fold together with its compensation record — void/refund ledger entry once P13's
double-entry plane exists, and the `Released`/`Wasted` stock events of §4. An FSM that permits
`Preparing→Cancelled` without forcing the linked money+stock events would re-open money
conservation (Σ≠0) at the exact place the ledger design closes it. Concretely: the cancel intent is
ONE event whose fold updates order status AND emits the compensation entries — never two
independently-committable events. This is why R3 stays P07-owned rather than a quick FSM edit.

### 3.4 The remaining catalog, grounded

**Duplicate submission / idempotency.** The kernel's only idempotency mechanism is event
content-addressing (`event_log.rs:148`), and §3.1-F1 shows it does not survive a local retry on a
non-empty log; separately, the wasm order path has NO idempotency at all — a double-tap or a
timeout-retry mints two orders with two seq ids (`wasm.rs:191`). The historical system scoped
idempotency keys by `location_id` (As-Built, cited at HUB §1.2) — that behavior has no successor at
HEAD. Fix = §3.1's per-actor seq fence: the placement intent carries `(actor_pubkey, actor_seq)`;
a retry reuses the seq and is refused as a structural duplicate returning the ORIGINAL order id
(the `Duplicate(id)` outcome already exists, `event_log.rs:246-252`). Owner: P13 intake, with F1
fixed first.

**Substitutions / partial fulfillment.** `Order.items` is fixed at placement — no API in
`domain.rs` amends items, and any amendment changes `subtotal`, so it is money-red-line work: an
`OrderAmended` intent must re-derive every price through `PriceCatalog` exactly as
`place_order_priced` does (`domain.rs:163-204`), fold under §3.1's fence (so it cannot race a
pickup), and record a customer-consent flag. Named as a P13 design item; not designed further here
(scope). Its stock-side twin (a substitution releases one item's reservation and reserves another)
falls out of §4's event vocabulary for free.

**Address change mid-delivery.** Structurally unrepresentable for a simpler reason than the FSM:
**the kernel `Order` has no delivery address at all** (`domain.rs:39-57` — id, customer, status,
items, money fields, channel, `cash_pay_with`, `price_trusted`; grep for `address` across
`kernel/src` finds only content-address plumbing). The geo/ETA modules operate on ad-hoc
coordinates. Before "change address mid-delivery" can be an edge case, address must become a
domain field with a re-quote rule (distance-tiered fee may change → re-price under the same
catalog discipline; ties to audit R2, the missing fee authority). Flagged as a P13/P16 model gap —
the second structurally-missing field this catalog surfaces, after cancellation reasons.

**Courier reassignment.** No courier handle exists on any dowiz order type (deliberate — the
no-courier-scoring red line keeps courier identity off `Order`; assignment lives in bebop2's HRW
matcher, audit row 10). Reassignment therefore has no dowiz-side state to corrupt today; when
HUB-D3's `DispatchProvider` lands, reassignment = `cancel(handle)` + fresh `quote/dispatch`
(the trait already carries `cancel` — HUB §4-D3), and the courier-abandons-mid-delivery case
collapses into §3.3's `InDelivery→Cancelled`-or-redispatch policy rather than a new mechanism.

**Payment failure after acceptance.** No charge plane exists (audit row 4), so today this is
unrepresentable like everything money-moving. The design consequence worth pinning NOW: the charge
saga's natural shape is authorize-at-`Confirmed`, capture-at-terminal (`Delivered`/`PickedUp`),
and an authorization failure AFTER acceptance needs `Confirmed→Cancelled{actor: System,
reason: AuthFailed}` — i.e. §3.3's edges are a PREREQUISITE for P13's payment saga, not a
nice-to-have. And the first enforcement point of that saga is the one the audit already named R1:
refuse to charge any order with `price_trusted == false` (`domain.rs:56`, currently read by
nothing).

---

## 4. PART 2 — Deterministic real-time inventory

### 4.1 The conservation pattern the codebase already proves, applied to stock

Three existing mechanisms are the design's whole foundation — nothing new is invented, only
composed:

- **Integer, checked, fail-closed arithmetic on conserved quantities** — `money.rs`: i64 minor
  units, `checked_add`/`checked_mul` everywhere, typed `Err` on overflow, and degrade-to-`None`
  rather than fabricate (`money.rs:219-241`). Stock quantities get the identical treatment.
- **State as a pure fold over an append-only, content-addressed log** — `event_log.rs`'s
  decide-before-commit + replay, and `fold_transitions`' determinism. The current stock count is
  ALWAYS `fold(events)`, never a stored counter that can drift from its own history.
- **Executable conservation checks** — `noether.rs:19` `step_preserves` already exists as a tested
  kernel organ ("given a transition `f` and an invariant `I`, verify `I` is conserved along the
  trajectory"). Stock needs the integer sibling of exactly this check.

**Proposed module: `kernel/src/stock.rs`** (name chosen to avoid the `intake.rs`-style collision
the audit flagged as R8; "inventory" additionally collides with the UI plan's localStorage page,
audit row 21).

```rust
pub enum StockEvent {
    // qty is i64 in the item's base unit (DECART-B): pieces, grams, ml. Always > 0.
    Received  { item: ItemId, qty: i64 },                          // restock:        on_hand += qty
    Reserved  { item: ItemId, qty: i64, order_id: OrderId },       // order placed:   reserved += qty
    Consumed  { item: ItemId, qty: i64, order_id: OrderId },       // prep started:   on_hand -= qty, reserved -= qty
    Released  { item: ItemId, qty: i64, order_id: OrderId },       // cancel/reject:  reserved -= qty
    Wasted    { item: ItemId, qty: i64, reason: WasteReason },     // spoilage/drop:  on_hand -= qty
    Stocktake { item: ItemId, observed: i64, stocktake_id: String }, // human input:  on_hand := observed (basis reset)
}

pub struct StockLevel { pub on_hand: i64, pub reserved: i64 }     // available = on_hand − reserved

pub struct StockLedger { levels: BTreeMap<ItemId, StockLevel> }   // a PROJECTION — rebuilt by fold, never edited
impl StockLedger {
    pub fn decide(&self, ev: &StockEvent) -> Result<(), StockError>;   // fail-closed gate, run pre-commit
    pub fn fold(events: &[StockEvent]) -> Result<StockLedger, StockError>; // pure; checked; deterministic
    pub fn available(&self, item: &ItemId) -> i64;
}
```

`OrderId` is the content-address id of §3.2 — the linkage invariant I3 below is meaningless with
colliding `ord_N` ids, which makes §3.2's fix a named dependency, not a preference.

**Invariants (the conservation law, enforced at `decide`, re-provable at fold):**

- **I1 — non-negativity:** `on_hand ≥ 0`, `reserved ≥ 0`, `reserved ≤ on_hand` after every event.
  A violating event is refused at decide time (the Law pole of `CommitError`, `event_log.rs:262-268`
  — never retried, nothing persisted). An order reserving more than `available` is refused with a
  typed `OutOfStock` — **this refusal IS the automated 86**, closing the D1 done-check (a) from the
  supply side.
- **I2 — conservation:** for every item, `on_hand = basis + Σreceived − Σconsumed − Σwasted`,
  where `basis` resets at the latest `Stocktake`. The fold IS this sum — there is no second counter
  to disagree with it. Checked with the integer analog of `noether::step_preserves` in-suite.
- **I3 — order linkage:** every `Reserved{order_id}` is eventually matched by exactly one
  `Consumed` or exactly one `Released` with the same `order_id`, never both, never neither once the
  order is terminal. This binds stock to the order FSM: entry into `Preparing` emits `Consumed`;
  every §3.3 cancellation edge emits `Released` (before `Consumed`) or `Wasted` (after). **Without
  the §3.3 cancel edges, a rejected-in-flight order strands its reservation forever — the cancel
  work is a prerequisite of inventory correctness, not just of customer service.**
- **I4 — replay determinism:** `fold` of the same event sequence yields byte-identical projections
  in a second process (the hermetic-remediation quick-win #19 pattern: serialize → re-read → assert
  in a fresh path, `HERMETIC-REMEDIATION-PLAN.md:196-198`), and the log's hash chain makes the
  sequence itself tamper-evident (any edit changes every downstream content-id).

**Lifecycle coupling (one commit, not two):** the order events and stock events of one intent fold
atomically — a placement intent's fold produces `Order{Pending}` + `Reserved` rows together through
one `commit_after_decide`; a cancel intent produces the status change + `Released`/`Wasted` + (post
P13) the money compensation entry together (§3.3's coupling law). This is what makes oversell and
stranded reservations structurally impossible rather than procedurally avoided.

### 4.2 "100% correct retrieval and counting" — what breaks elsewhere, and the structural closure

| # | How counts go wrong in typical systems | Structural closure here | Grounding |
|---|---|---|---|
| 1 | **Double-decrement on retry** — a timed-out POST retried, the decrement applied twice | The intent is content-addressed + per-actor-seq fenced; a retry is a `Duplicate` no-op and `decide` never re-runs | `event_log.rs:349-352` + the §3.1-F1 fix (which is exactly why F1 must land first — today the no-op guarantee holds only at genesis) |
| 2 | **Lost update under concurrency** — two writers read `5`, both write `4` | There is no read-modify-write and no mutable counter to race on: the count is derived, and appends serialize through one per-hub chain (`&mut`, process-per-hub) with decide validating against the CURRENT fold (§3.1 rule A) | `event_log.rs:275-311`; audit §4.3 Option A |
| 3 | **Silent drift** — the counter and its audit trail diverge and nobody notices | The counter does not exist; the projection is refoldable from genesis at boot (<2 s/year, §2) and the second-process test pins byte-identity; hash-chaining makes history append-only | I4; `lib.rs:177` boot-gate pattern; quick-win #19 |
| 4 | **Float/rounding on fractional units** (0.1 kg × 3 ≠ 0.30000000000000004) | Integer base units per item with checked arithmetic — the `money.rs` discipline verbatim, including typed overflow errors | `money.rs:119-132`; DECART-B |
| 5 | **Oversell under load** — two concurrent orders both see the last portion | `decide` gates against folded `available` inside the serialized commit; the second order gets typed `OutOfStock`, which doubles as the automated 86 | I1; `event_log.rs:339-361` |
| 6 | **Books "fixed" by hand** — a human edits the count, destroying the audit trail | There is no mutable count to edit. The only human-writable event is `Stocktake{observed}`; every adjustment is itself a logged, signed, replayable event | §4.3 |

**Honesty scoping of "100%".** The ledger is exactly correct about what it MODELS: every recorded
receipt, reservation, consumption, waste, and recount — retrieval and counting of those is
structurally exact, replay-verified, and needs no human ever. Physical-shelf fidelity is bounded by
what gets recorded (unlogged spillage, theft, over-portioning drift outside the recipe model).
That residual is precisely what the operator's own division assigns to humans — physical recounts —
and §4.3 is the mechanism that imports each recount as data and quantifies the residual instead of
hiding it. The claim this design can honestly make: **the software's bookkeeping never needs
reconciliation; the physical world is reconciled through one typed event, on a schedule, with
drift surfaced — never through anyone editing the books.**

### 4.3 `StockReconciliation` — the exact mechanism, not a policy

1. **Trigger (structural, not remembered):** a scheduled stocktake job — same P5-RHYTHM
   registered-job pattern D1 already specifies for menu safeguard re-sync (HUB §4-D1) — opens a
   stocktake `stocktake_id` for a configured item set; ad-hoc stocktakes (post-incident) use the
   same path.
2. **Human input (the ONLY human surface):** the person counts the shelf and enters `observed` per
   item. Each line becomes a `Stocktake{item, observed, stocktake_id}` event through the standard
   decide/commit gate. The human never sees, computes, or enters a delta.
3. **Machine derivation:** at decide time the runtime computes `drift = observed − folded.on_hand`.
   The fold applies `Stocktake` as a basis reset (`on_hand := observed`; `reserved` untouched — see
   the race note below), so the correction is intrinsic to the event, with no second "correction"
   event to atomically pair (one event kind, no multi-event atomicity problem).
4. **Surfacing rule:** `drift == 0` → nothing is raised; the event is a silent audit row.
   `drift ≠ 0` → a typed discrepancy report `{item, drift, stocktake_id, window: (prev_stocktake,
   now)}` goes to the owner surface and the local telemetry sink (per the AGENTS.md mandatory-
   telemetry rule). The human's follow-up is PHYSICAL — investigate theft/damage/miscount — and
   optionally a `Wasted{reason}` event if the cause is identified. The books have already adjusted
   themselves; drift history accumulates as queryable shrinkage data (the HUB-G7 food-cost surface
   gets its input for free).
5. **Race honesty:** a stocktake taken while orders are in flight can be off by the units consumed
   between the physical count and the commit. Bounded mitigation, not denial: take stocktakes under
   `Paused` store state (D4's machinery) or accept the ≤minutes window and let the next cycle
   absorb it — flagged as accepted risk R-a below, never silently ignored.

**Falsifiable done-checks:** (a) seeded property run (kernel `rng.rs`, audit-DECART harness style):
fold twice + second-process serialize→re-read are byte-identical; (b) RED: over-reserve refused
typed, log unchanged; (c) a retried `Reserved` intent returns the original outcome, `decide` runs
once (fails today until §3.1-F1 — that is the RED proving the fence has teeth); (d) `Stocktake`
with `observed == folded` raises nothing; with `observed ≠ folded` raises exactly one report naming
item + drift; (e) integer `step_preserves` over the run confirms I2 at every prefix; (f) stock
events join the audit's T2 twin-hub harness: hub A's stocktake never moves hub B's counts.

### 4.4 Relation to HUB-G1 (`MenuRevision` / `AvailabilitySet`): separate module, and it completes D1

**Decision: `stock.rs` is NOT part of `MenuRevision`, and not a replacement for `AvailabilitySet`
— it is the missing PRODUCER of `AvailabilitySet`'s automated half.** Three derivable reasons:

1. **Frequency/authority split — D1's own argument, extended one level.** D1 already separates
   `AvailabilitySet` from `MenuRevision` because "86ing is a high-frequency operational event, not
   a menu edit" (HUB §4-D1). Stock events are a third, higher-frequency layer of PHYSICAL FACT,
   distinct from OWNER INTENT (a manual 86 for a broken fryer has nothing to do with counts).
   Folding counts into the menu entity would repeat the incumbents' conflation D1 names as the
   reason Checkmate's 5-minute poll exists.
2. **One enforcement point, two feeds (P2-CORRESPONDENCE).** Intake refuses an item iff
   `eighty_sixed(item) ∨ (tracked(item) ∧ available(item) ≤ 0)` — a single check consulting both
   sources. The stock side auto-emits `AvailabilityChanged{cause: StockDerived}` when `available`
   hits 0, and lifts it on restock **only when the standing 86's cause is `StockDerived`** — a
   manual 86 is never silently un-86'd by a delivery of ingredients. This requires one addition to
   D1's type: `AvailabilitySet` entries carry `cause: Manual | StockDerived` (proposed to the D1
   implementer; without it, auto-lift is unsafe and the two systems fight).
3. **Projection symmetry.** D1 demotes `PriceCatalog` to a projection of `MenuRevision`;
   `StockLedger` is the same shape over a different event family. The menu binds them with one
   optional field per `MenuItem`: `stock_policy: Untracked | Tracked{unit}` (later
   `Recipe{Vec<(ingredient, qty)>}` for BOM/ingredient-level accounting — explicitly a named
   extension into HUB-G7's food-cost territory, NOT v1; per-item sellable units first, per the
   HUB doc's own granularity caution, its §7 Q1-5).

---

## 5. DECART tables (concrete data-structure/algorithm choices made by this doc)

### DECART-A — Stock-count representation

| Criterion | **Fold over content-addressed event log** (chosen) | Mutable counter + audit trail | PN-counter CRDT |
|---|---|---|---|
| Correctness under retry/concurrency | Idempotent + serialized by construction (§4.2 rows 1–2) | The audit trail and the counter can disagree — the classic drift this design exists to kill | Converges, but convergence ACCEPTS concurrent oversell: two decrements below zero commute; a non-negative resource needs a serialization point, which is a CP choice the CRDT deliberately avoids |
| Fits repo | Reuses `event_log.rs` + `noether.rs` verbatim; zero new deps | New mutable state in a kernel with almost none (audit T1) | New crate or ~300 lines; wrong consistency pole for intra-hub (audit §5: CP intra-hub) |
| Perf at hub scale | Refold <2 s/year (§2); µs per event | µs | µs |
| Reversibility | Delete module | — | — |

**Probe (strongest case against):** at chain-scale (O20-a, hundreds of locations aggregating), a
per-hub fold does not give consolidated real-time counts without a cross-hub reader. Answer: the
consolidated view is a READER over per-hub ledgers (HUB-D5's statement pattern), never a shared
counter — and that reader inherits collision-free order/item ids from §3.2, which is another reason
the id fix precedes any aggregation work.

### DECART-B — Quantity representation

| Criterion | **i64 base units per item** (chosen: pieces/grams/ml, unit declared in `stock_policy`) | f64 | Fixed-point decimal type |
|---|---|---|---|
| Exactness | Exact; checked ops; typed overflow (the `money.rs` discipline, `money.rs:119-132`) | 0.1+0.2 class errors accumulate in a fold — disqualifying for a "100% correct counting" mandate | Exact but a new abstraction the repo doesn't have |
| Unit-mixing hazard | Same shape as `Currency` on `Money` (`money.rs:57-87`): the unit lives on the item's policy; cross-item arithmetic doesn't exist in the fold, so no cross-unit add is expressible | Same hazard, plus float | Same as chosen, more machinery |
| Cost | Zero new code beyond the enum | — | A type nobody else needs |

### DECART-C — When stock leaves the count (reservation timing)

| Criterion | **Reserve-at-place / consume-at-prep / release-on-cancel** (chosen — try-confirm/cancel shape) | Decrement at placement | Decrement at prep only |
|---|---|---|---|
| Oversell window | None: `available = on_hand − reserved` gates intake | None | Open window between accept and prep — two accepted orders can both claim the last portion |
| Stranded stock | Possible only if an order never reaches a terminal — closed by I3 + the §3.3 cancel edges (named dependency) | Cancel must "un-decrement" — an increment indistinguishable from a restock in the history, muddying I2 | None |
| Semantic honesty | Reserved food is still on the shelf (matches physical stocktakes counting it, §4.3) | Books say the food is gone while it sits on the shelf — every stocktake would show phantom drift | — |

---

## 6. Consistency + idempotency summary (concepts named, per canon)

Intra-hub: CP — one writer, one chain, decide-fenced compare-and-swap on folded state (§3.1-A);
conflicts are refused typed at commit, never merged, never silently dropped. Inter-hub: AP —
signed events, partial order via `prev`/`actor_seq`, no distributed transaction anywhere in this
design; the only cross-hub requirement is id collision-freedom (content-address minting, §3.2).
Idempotency: content-addressing for network replays (works today), plus the per-actor seq fence
for local retries (does NOT work today — §3.1-F1 — and is this doc's highest-priority fix).
The saga concept appears exactly once, where it belongs: cancellation-with-compensation (§3.3)
and reserve/consume/release (DECART-C) are the same try-confirm/cancel discipline applied to state
and stock respectively; money legs join it under P13.

## 7. Failure / degradation

No mechanism in this doc introduces an external call, a standing service, or a timeout surface —
everything is in-process, so the failure taxonomy is inherited whole from `event_log.rs`:
Law-pole rejection (typed, never retried, nothing persisted) vs store-pole durability fault
(typed, retriable, alarming) — `event_log.rs:262-268`, tested at `:724-745`. New typed errors this
design adds to the Law pole: `Conflict{expected, found}` (§3.1), `StaleTip` (F2 fix),
`OutOfStock{item}` (I1), `StrandedReservation{order_id}` (I3 audit check). Degradation stance for
inventory reads mirrors `estimate_order_total` (`money.rs:219-241`): a projection that cannot be
computed (fold error, corrupted store) returns a typed error and the intake check FAILS CLOSED for
tracked items (refuse to sell what cannot be counted) while untracked items keep flowing — degraded,
never fabricating a count.

## 8. Security + tenant isolation

`StockLedger`/stock events are per-hub instances like every kernel primitive (audit §4.2's rule);
they add no global state (T1's allowlist gains nothing; §3.2 removes its one order-path entry) and
must be added to the audit's T2 twin-hub non-interference harness and T4 collision pins (done-check
f). Stock events carry no PII and no courier identity (`WasteReason`/`actor` enums, ids only) —
the no-courier-scoring fence is untouched. Stocktake input is an owner-capability surface: the
human entering counts authenticates as the hub's actor key; a forged `Stocktake` is exactly as
hard as any forged event (signature verification arriving with P9, audit R6 — inherited, not
worsened).

## 9. Operability + sequencing

- **Observability <1 min:** every drift report, conflict rejection, and `OutOfStock` refusal is a
  typed local telemetry line (AGENTS.md native-telemetry rule); the boot refold emits
  `(events, fold_ms, chain_tip)` — a corrupted ledger is visible at boot, not at month-end.
- **Rollback:** all additive (one module + FSM edges behind the golden-signature bump + F1/F2
  fixes). The FSM edit is the only change with behavioral blast radius; it is operator-gated by
  design (R3) and reverts by restoring the 9-edge table + signature.
- **Sequencing:** (1) F1/F2 fixes + their RED tests land with the audit's T-layer pass (they share
  files and the T2 harness needs a working dedup under it); (2) §3.3 cancel edges = P07, operator-
  gated; (3) id minting + intent fence = P13 intake (audit R1/R4 homes); (4) `stock.rs` is
  Wave-0-grade pure kernel (no dependency on 1–3 to EXIST, but its done-checks c and I3 stay RED
  until 1 and 2 land — born-enforced, per the audit §7 discipline); (5) `AvailabilitySet.cause`
  rides D1.

## 10. Open / accepted risks (owner named)

| # | Risk | Status | Owner |
|---|---|---|---|
| R-a | Stocktake-vs-in-flight race (§4.3.5) — counts taken while orders move can drift by the window's consumption | ACCEPTED with named mitigation (stocktake under `Paused`, or next-cycle absorption); never silent | D4 implementer (store-state) |
| R-b | §3.1-F1 double-commit is live NOW in the only tested commit path; any interim consumer of `commit_after_decide` with zeroed-prev retries double-runs `decide` | OPEN — highest-priority mechanical fix in this doc | next kernel pass (with audit T-layers) |
| R-c | Cancel edges require the `FSM_GOLDEN_SIGNATURE` bump (exact target values in §3.3) — a lifecycle change the gate correctly makes operator-visible | OPEN, gated | P07 + operator (audit R3) |
| R-d | The per-actor seq registry must itself be a fold-derived projection; implemented naively as a standalone mutable set it recreates drift risk #3 | OPEN design constraint, named so the implementer can't miss it | P13 intake |
| R-e | Recipe/BOM granularity unknown (per-item v1 chosen; ingredient-level deferred) — echoes HUB §7 Q1-5's same caution for prep-time | ACCEPTED; trigger = first hub that actually tracks ingredients | G7 extension work |
| R-f | Multi-location stock (a chain moving stock between locations) prejudged by nothing here, but inter-location transfer events only make sense after O20 | ACCEPTED, rides O20 | operator (O20) |
| R-g | Address field absence (§3.4) is out of this doc's scope but blocks two catalog rows (address change, distance-fee re-quote) | OPEN, flagged to owner | P13/P16 |

## 11. 2-question doubt audit (AGENTS.md ritual)

**Q1 — least confident about (5 items):**
1. **F1's severity read.** The double-commit is verified by tracing `event_log.rs:348` vs `:302`
   against the test at `:533-561` (genesis-only) — but not by executing a failing test (no Bash
   this session, same epistemic caveat as the audit's §9 Q1-1). The RED test in done-check (c) is
   the proof obligation; if it unexpectedly passes, this doc's top finding downgrades and §3.1's
   fence loses its "fixes a live bug" urgency (not its correctness value).
2. **`Stocktake`-as-basis-reset** makes I2 piecewise (conservation between stocktakes). I judged
   one event kind + intrinsic correction worth that complication vs a paired Correction event
   needing multi-event atomicity; a reviewer could reasonably prefer the pair with an explicit
   batch-commit primitive — which the log does not have today, which is why I didn't.
3. **Reserved-food semantics** (DECART-C: reserved items still counted in `on_hand` until
   `Consumed`) matches shelf-physical stocktakes for made-to-order restaurants but may mismatch
   grab-and-go formats where "reserved" leaves the shelf immediately; the unit-policy enum leaves
   room, the doc doesn't design it.
4. **The §3.3 golden-signature arithmetic** (edges 13, μ=5, reachability 767) is hand-computed from
   `allowed_next`; the implementer must let `fsm_graph_report()` produce the real values rather
   than trust these — the gate exists so that they cannot silently diverge.
5. **`AvailabilitySet.cause` is proposed onto D1's type without the D1 author's pass** — it is a
   one-field addendum, but if D1 lands first without it, auto-86 lift must ship disabled.

**Q2 — the biggest thing I might be missing:** the whole Part-2 design presumes the per-hub
event log is the SINGLE intake path for stock mutations — true in the target architecture, but the
UI plan's supplies page currently lives in localStorage (audit row 21), and any transition period
where both exist creates exactly the dual-authority drift this design eliminates. The honest rule:
there is no incremental migration in which localStorage keeps write authority; the browser surface
must become a submitter of `StockEvent` intents on day one of `stock.rs` consumption, or the
"no human reconciliation" property is void during the overlap. Named as a hard sequencing
constraint for P16 rather than assumed away.

## 12. Anu / Ananke check

- **Anu (derivable, not asserted):** every Part-1 verdict traces to file:line read this session
  (`order_machine.rs:67/71`, `wasm.rs:51/191/295`, `domain.rs:39-57/220`, `event_log.rs:293-361/
  348/533-561/580-609`, `analytics.rs:60-66`); the two new findings (F1/F2) are argued from the
  code path step-by-step and carry their own falsification route (done-check c); each chosen rule
  sits in a ≥2-option table with the rejection reasons stated in failure-mode terms, not taste;
  the inventory design cites the three in-repo mechanisms it composes (`money.rs`, `event_log.rs`,
  `noether.rs:19`) and its one honest scope limit (physical-fidelity bound, §4.2) instead of
  claiming "100%" beyond what structure delivers.
- **Ananke (structure forces the outcome):** the count cannot drift because no mutable count
  exists (a structural impossibility, not a discipline); a retry cannot double-decrement because
  dedup is content+seq-keyed inside the commit gate; oversell cannot happen because the gate folds
  before it admits; the books cannot be hand-edited because the only human-writable surface is one
  typed event; cancellation cannot skip compensation because cancel+compensation is one fold; the
  FSM cannot change silently because the golden-signature gate trips (`order_machine.rs:472-483`);
  and every property lands as a RED-first done-check scheduled WITH the CI job that runs it (audit
  §7 rule, inherited). Where structure cannot force it yet — the stocktake race window (R-a), the
  signature-verification gap until P9 (R6), the localStorage overlap (Q2) — the doc says so with
  an owner attached instead of hoping.

*This document plans; it changes no code and no canon. Follow-ups proposed to owners: F1/F2 +
fence → next kernel pass with audit T1–T4; cancel edges + signature bump → P07 (operator-gated);
id minting + intent idempotency + address field → P13; `stock.rs` + done-checks → Wave-0 kernel
work; `AvailabilitySet.cause` → D1; localStorage write-authority cutover rule → P16.*
