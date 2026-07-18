# BLUEPRINT P65 — Dispatch orchestrator: stateful offer-timeout/advance-rank coordination above the deterministic HRW matcher (2026-07-18)

> **Planning document — writes no product code.** Written against the 20-point contract in
> `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (compliance map §10). Wave **W2**,
> component **DELIVERY / dispatch**. Source scope: `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md`
> §5 W2 table row **P65** and §3.2 point 5 ("Dispatch orchestrator is the largest un-designed
> order-flow piece"), grounded by `OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md` §0,
> §4.2, §5.3, §8 risk #1. Structural template: `BLUEPRINT-P51-open-map-routing.md` (numbering
> mirrored); sibling W2 precedents: `BLUEPRINT-P60-payment-adapter-core.md`,
> `BLUEPRINT-P62-catalog-multivendor-data-model.md`.
>
> **The one-sentence thesis:** the HRW matcher (`assign`) already produces a deterministic,
> coordination-free *ranked candidate list*, and the claim machine (`ClaimStatus`) already
> models a single offer's *lifecycle* — the genuinely missing piece is the **stateful driver
> that walks that ranked list under an accept-timeout**, advancing on decline/timeout, holding
> a no-courier order at `Ready` and re-polling, **without ever accumulating one byte of
> courier-identity-keyed history that could bias a future ranking.** P65 builds that driver on
> top of two tested primitives; it re-implements neither.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

Fresh reads, 2026-07-18. **Cross-repo note (binding, per memory `cross-branch-todo-map`):** the
matcher and claim machine are **bebop2 / proto-cap files in `/root/bebop-repo`**, the order
machine is a **kernel file in `/root/dowiz`**. P65's new module lands in `bebop-repo`
(`bebop2/proto-cap/src/dispatch.rs`), NOT in `/root/dowiz` — it is mesh-coordination Law, it sits
with its siblings, and it mirrors the order machine's *shape* without importing the kernel crate
(the exact precedent `claim_machine.rs` already set, `claim_machine.rs:1-5`).

| Claim | Fresh `file:line` (this pass) | Status |
|---|---|---|
| **HRW matcher EXISTS and is tested.** `assign(order, candidates, max) -> Vec<CourierKey>` sorts by FNV-1a weight DESC, tie-breaks pubkey ASC, truncates to `max`; top entry = primary, remainder = deterministic fallbacks | `bebop2/proto-cap/src/matcher.rs:63` (sig), `:70` (tie-break), `:64-72` (body) | **VERIFIED — P65 consumes, never re-derives** |
| `hrw_weight(order_id, courier)` is a **pure** function of `(u64, &CourierKey)` — no history, score, or count parameter exists to thread past behavior into a ranking | `matcher.rs:41` (sig), `:176-180` (`green_mesh05_hrw_weight_pure`) | **VERIFIED — the structural no-scoring floor P65 must not breach** |
| `Courier { pubkey: CourierKey }` — the struct **structurally cannot** carry a score/rating/rank field; module doc states the NO-COURIER-SCORING guarantee | `matcher.rs:33-36` (struct), `:15-18` (doc) | VERIFIED |
| Requeue-never-drop already coded: re-running `assign` over the same candidate set returns the same ordering, so a refused order is **never dropped** | `matcher.rs:75-84` (`primary_for` + doc), tested `:138-156` (`r_mesh05_refused_order_requeued_never_dropped`) | **VERIFIED — P65's re-poll is this invariant, driven** |
| `CourierKey = [u8; 32]` (bare Ed25519 public key) | `bebop2/proto-cap/src/event_dict.rs:25` | VERIFIED |
| **Per-offer claim FSM EXISTS and is tested.** `ClaimStatus { Offered, Claimed, Released, PickedUp }`; `Offered => [Claimed, Released]`, `Claimed => [Released, PickedUp]`; `Released`/`PickedUp` terminal-legal | `bebop2/proto-cap/src/claim_machine.rs:21-30` (enum), `:72-81` (table) | **VERIFIED — a P65 offer IS a `ClaimStatus` instance; advance = `Offered→Released`, accept = `Offered→Claimed`** |
| Claim wire discriminants pinned: Offered `0x20`, Claimed `0x21`, Released `0x22`, PickedUp `0x23` | `claim_machine.rs:34-50`, tested `:186-192` | VERIFIED |
| A late accept on an already-released offer is **structurally illegal**: `Released → Claimed` is not in `allowed_next` | `claim_machine.rs:78` (`Released => &[]`), tested `:124-131` (`r_mesh04_released_cannot_move`) | **VERIFIED — the late-accept race is refused by an existing gate, not new code (§4.3)** |
| Claim machine is **pure Law, NO kernel dependency**, deliberately mirroring `order_machine`'s shape | `claim_machine.rs:1-5`, `:13-17` | **VERIFIED — P65 adopts this exact boundary** |
| **Unified order FSM EXISTS.** `OrderStatus::Ready` and its branch `Ready => [InDelivery, PickedUp, Refunding]` — delivery (`Ready→InDelivery`, needs dispatch) and pickup (`Ready→PickedUp`, no dispatch) **already branch at `Ready`** | `kernel/src/order_machine.rs:12` (state), `:84` (edges), `:734-741` (`green_pickup_path_ready_to_pickedup`) | **VERIFIED — §16.60 "pickup is the same flow minus dispatch" is the existing code, not new design** |
| The order FSM has **NO edge that auto-rejects a waiting order**: from `Ready` the only exits are `InDelivery` (a courier accepted), `PickedUp` (pickup), or `Refunding` (an explicit, human-initiated cancel after money moved) | `order_machine.rs:84`; `Confirmed => [..Refunding]` `:82`; `is_terminal` `:64-73` | **VERIFIED — "never silently rejected" (§16.49) is structural: no give-up edge exists to add one** |
| The order FSM is guarded by a golden drift-signature: `edges: 14`, `is_acyclic: true`, `ρ=0` | `order_machine.rs:465-476` (`FSM_GOLDEN_SIGNATURE`), gate `:495-534` | **VERIFIED — P65 adds ZERO order-FSM edges (it reuses `Ready→InDelivery`/`Ready→PickedUp`), so this signature MUST stay `edges:14` after P65 (§6 DoD)** |
| **The no-scoring CI gate EXISTS** and already scans the whole `bebop2/` tree except `bebop2/core/` for score/rating/reputation/rank/`*_count`-shaped fields on structs | `bebop-repo/scripts/ci-no-courier-scoring.sh:25` (regex), `:29` (scope), `:18-19` (double-lock + `test-no-courier-scoring.sh` regression precedent) | **VERIFIED — a new `bebop2/proto-cap/src/dispatch.rs` is automatically under this gate; §4.6 extends the red-case corpus** |
| Courier admission / trust-anchor is the **capability-cert layer** (P59): anchors are *identities* (public keys), explicitly "no score, no reputation, no trusted mover" | `bebop2/proto-cap/src/roster.rs:26-27` (AnchorRoster no-reputation doc), `:192-198` | VERIFIED — P59's home; P65 depends on it *interface-only* (it needs the `CourierKey`, not the cert machinery) |
| **Name-collision disambiguation:** `kernel/src/loops.rs` already has a `struct Orchestrator` + `DispatchTicket` — that is the **self-improvement loop-card dispatcher** (certifies `LoopCard`s), NOT courier dispatch. P65 deliberately names its type `DispatchSession` to avoid the collision | `kernel/src/loops.rs:161-190` | VERIFIED — recorded so no future reader conflates the two |
| **proto-cap has NO wall-clock / time source** (grep for `SystemTime`/`Instant`/`now()` over `src/*.rs` → 0 non-comment hits); the crate is `no_std`-lean (`bebop2-core` `default-features=false`) | `bebop2/proto-cap/Cargo.toml:12`; grep this pass | **VERIFIED — P65 MUST inject `now_ts: i64`, never read the clock (pure-Law discipline, §3)** |
| Module registration is `pub mod <name>;` in `lib.rs`, roughly alphabetical | `bebop2/proto-cap/src/lib.rs:32-48` (`claim_machine` at `:33`) | VERIFIED — register `pub mod dispatch;` immediately after `claim_machine` |

Ground truth is non-discussible; everything below builds on this table only. **The load-bearing
finding:** two of P65's four sub-problems (the ranked list, the offer lifecycle) are *already
tested code*; P65 is the ~250-line stateful glue between them plus the re-poll loop — and the
whole risk is keeping that glue **memoryless per courier**.

---

## 1. The design problem, answered (standard §2 items 3, 6, 19) — the four open decisions closed

R4 §8 ranks dispatch orchestration as **risk #1** and enumerates exactly four undesigned pieces
(R4 §4.2 "Timeout-then-reassign", §5.3, §8.1). Each is closed here with a falsifiable choice.

### 1.1 Offer-to-primary policy — **sequential single-offer** (one courier at a time, down the ranked list)

**Choice: `OFFER_WAVE_SIZE = 1`** (Wave-0). The order is offered to the current highest-HRW
*online* courier; only on their decline/timeout does the next-ranked courier see it. Not
fan-out-to-N-with-first-accept-wins.

**Why, given the no-scoring constraint (the load-bearing reasoning):**

1. **HRW already *is* the total order.** `assign` returns a deterministic ranked list
   (`matcher.rs:63`). Offering sequentially *honors that exact order* — the primary gets first
   refusal, then the deterministic fallback, and so on. Fan-out throws that order away and
   replaces it with a **latency race** ("whoever's device taps accept first"), which (a) is not
   HRW rank, and (b) is the seed of scoring: "who tends to accept fastest" is precisely the kind
   of per-courier behavioral signal §16.26's red line forbids. Sequential offering **cannot
   observe** accept-latency-as-selection because only one courier is ever racing (themselves,
   against the clock).
2. **Determinism / mesh-replay.** Sequential single-offer makes the offer sequence a pure
   function of `(order_id, online-set-over-time, accept/decline/timeout events)` — replayable
   from the event log with no wall-clock tiebreak. Fan-out first-accept-wins is only
   deterministic if every accept is timestamped and ordered in the log, and even then it burns
   `N-1` fallback candidates on every single offer (they all get "too late"), degrading the
   fallback pool the requeue-never-drop invariant depends on.
3. **UX honesty.** One courier is asked at a time; nobody is told "you got it" then "actually
   no." This matches the operator's protocol-not-marketplace stance (§16.3).

**Honest tradeoff (stated, not hidden):** sequential offering pays up to `OFFER_TIMEOUT_SECS` of
latency when the primary is online-but-AFK. The mitigation is a *tight* timeout (§1.2), not
fan-out. A **bounded wave `OFFER_WAVE_SIZE = K > 1`** (offer to the top-K simultaneously, first
*claim-transition* in event order wins, losers get `Released`) is a **named future unit** with a
pre-registered caveat: raising K past 1 requires re-running the no-scoring adversarial test (§4.6)
because it introduces the accept-order signal — recorded here so it is a deliberate, reviewed
change, never a silent optimization. Wave-0 ships K=1.

### 1.2 Accept-timeout → advance-rank — a **fixed, courier-independent** constant

**Choice: `OFFER_TIMEOUT_SECS = 30`.** When a courier is offered the order (claim enters
`Offered`, `claim_machine.rs:23`), they have exactly 30 seconds to accept. On expiry the
orchestrator releases that offer (`Offered → Released`) and advances to the next-ranked online
courier.

**The anti-scoring property is structural, not policy:** the timeout is **one constant applied
identically to every courier** — there is no `timeout_for(courier)` function, no per-courier
value, no field keyed by `CourierKey` that could make a courier's window shorter or longer based
on anything about them. Making the timeout courier-specific would be de-facto scoring by the back
door; the type system forbids it because the deadline is computed as `now_ts + OFFER_TIMEOUT_SECS`
with **no courier argument** (§3, `LiveOffer::deadline`). Falsifiable by grep: no dispatch symbol
takes a `CourierKey` and returns a duration.

### 1.3 Decline handling — decline advances immediately; it does **NOT** touch future rankings

An explicit decline and a timeout are **identical for ranking purposes** — both release the offer
(`Offered → Released`) and advance to the next-ranked online courier. The *only* difference is
timing: a decline advances the instant it arrives (no need to wait out the clock); a timeout
advances when `now_ts ≥ deadline`. Both emit the same `Advanced { reason }` event; the `reason`
distinguishes them *for telemetry only* (§5.6), never for ranking.

**The invariant that makes this not-scoring (verbatim the task's red line):** a decline on *this*
order **cannot** penalize a courier on the *next* order. This holds by construction:

- The next order re-runs `assign(next_order, online_set)` from scratch (`matcher.rs:63`). `assign`
  takes only `(order, candidates, max)` — **there is no parameter through which a prior decline
  could enter.**
- The orchestrator's *only* memory of who-declined is `DispatchSession.offered_this_round`
  (§3) — a `Vec<CourierKey>` scoped to **one order's one round**, **cleared at the start of every
  round** and **destroyed when the order leaves `Ready`**. It exists solely to walk down the list
  without re-offering to the same courier within a single sweep; it is never read by `assign`,
  never persisted, never keyed across orders.

So the decline "memory" is horizon-bounded to a single sweep of a single order and evaporates.
This is proven, not asserted, by the two-world adversarial test (§4.6 Test A).

### 1.4 Queued no-courier orders — wait at `Ready`, re-poll with capped backoff, **never give up silently**

When a round is exhausted (every currently-online courier has declined/timed-out) **or** the
online set is empty from the start, the order **stays at `OrderStatus::Ready`** (verified state,
`order_machine.rs:12`,`:84`) — no new FSM state is invented (R4 §5.3: "a `Delivery` order sitting
at `Ready` with an empty HRW candidate set"). The orchestrator then:

- **Clears `offered_this_round`** (the next round is a *fresh full list* — a courier who declined
  last round returns at their deterministic HRW rank, unpenalized — this IS the no-scoring
  guarantee in the re-poll path) and schedules the next poll at
  `now_ts + backoff`, where `backoff` starts at `REPOLL_INTERVAL_SECS = 20` and doubles up to
  `REPOLL_MAX_SECS = 120` while nothing changes (busy-poll guard, §5.2).
- **Re-polls immediately** (backoff reset to base) whenever the online-set changes — a courier
  coming online is an event that wakes the loop, so a newly-available courier is offered without
  waiting out the backoff. (This is the requeue-never-drop invariant `matcher.rs:75-84` driven by
  a presence event.)

**Termination condition (the §16.49 / §16.14 honesty requirement, stated exactly):** the dispatch
loop **has no internal give-up.** It terminates **only** when the order leaves `Ready`, which can
happen in exactly two ways: forward (`Ready → InDelivery`, a courier accepted) or via an
**explicit, human-initiated** cancel/refund (`Ready → Refunding`, the customer or vendor
cancelled — money already moved after `Confirmed`, `order_machine.rs:82,84`). There is **no
timeout-to-reject and no auto-cancel** — inventing one would be the "silently rejected" outcome
§16.49 forbids, and there is *no FSM edge from `Ready` to `Rejected`/`Cancelled` to even express
it* (`order_machine.rs:84` — the only terminal-ward edge is `Refunding`, which is human-gated).
Throughout the wait, the customer sees honest client-side status **"waiting for courier"** derived
from the `Ready`-entry timestamp (§16.14, §5.5) — real and pending, never fabricated as
"assigned."

---

## 2. Scope — what P65 owns vs deliberately does NOT

**P65 owns (build items §4):**

| Item | Content |
|---|---|
| M1 | `FulfillmentKind` discriminator + the **dispatch gate**: a non-`Delivery` order can never construct a `DispatchSession` (constructor refusal — pickup/dine-in are unrepresentable in this module) |
| M2 | `DispatchSession` + `tick()` — the pure, clock-injected driver: offer→await→advance over `assign`'s ranked list, emitting `DispatchEvent`s; drives `ClaimStatus` transitions |
| M3 | Accept / decline / timeout handling (the offer lifecycle), incl. the late-accept race resolved by the existing `Released → Claimed` illegal gate |
| M4 | Queued no-courier re-poll: exhaustion → clear round → capped backoff → event-woken re-poll; the honest-wait status derivation |
| M5 | The **no-scoring structural proof**: the two-world adversarial test, the CI-gate red-case extension, the aggregate-only-telemetry guard |
| M6 | Integration seams (interfaces only): the `Assigned` → kernel `Ready→InDelivery` signal, the offer-out over the P61 transport, the online-candidate-set input |

**P65 explicitly does NOT own:**

- **NOT courier matching / ranking.** `assign`/`hrw_weight` (`matcher.rs`) are consumed
  unchanged. A diff that re-implements ranking, adds a weight parameter, or edits `matcher.rs` is
  a scope violation regardless of test state.
- **NOT the claim FSM.** `ClaimStatus`/`assert_transition` (`claim_machine.rs`) are consumed
  unchanged; P65 *drives* transitions, it does not redefine the table.
- **NOT the order FSM.** `order_machine.rs` is authority for the order lifecycle; P65 adds **zero
  edges** and reuses the existing `Ready→InDelivery` / `Ready→PickedUp` branch. `FSM_GOLDEN_SIGNATURE`
  (`order_machine.rs:465`) must read `edges:14` unchanged after P65 (§6).
- **NOT any courier scoring / reputation / history — RED LINE, restated as an in-blueprint
  anti-scope with a falsifiable test (§4.6, §5.6).** No persistent, courier-identity-keyed value
  that influences a future ranking may exist anywhere in this module. This is the whole reason
  P65 is risky and the reason its DoD includes a structural proof, not just behavioral tests.
- **NOT the persisted `fulfillment_type` order field.** P65 defines the *type* and the *gate*;
  where the field lives on the persisted order intent is the **order-record owner's diff**
  (P62 catalog/data-model or P69 order intake) — cited, not re-specified, exactly the seam
  convention P51 used for `GeoPin`/the order address field (`BLUEPRINT-P51` §2). M6's end-to-end
  "order record carries `fulfillment_type`" test is `#[ignore = "order-record-field"]` until that
  seam closes.
- **NOT the courier certificate / admission machinery.** P59 (`roster.rs`) decides *who is an
  authorized courier*; P65 consumes the resulting `CourierKey`s (interface only). A courier's
  cert being valid is a precondition to being in the candidate set, checked upstream.
- **NOT the notification transport.** The offer message reaching the courier's device rides the
  **P61 notification fabric** / the P34/P37 wire; P65 emits `Offered { courier, deadline_ts }`
  and consumes accept/decline as inbound events — it does not open sockets (proto-cap is
  transport-free, `Cargo.toml`).
- **NOT presence/liveness.** "Which of the venue's couriers are online right now" is caller-supplied
  (`candidates: &[Courier]`); P65 does not track heartbeats (that is the hub's presence layer,
  §16.53). P65 only requires that the set it is handed reflects current online couriers.
- **NOT a central dowiz queue.** The orchestrator is **hub-local** (§16.6). There is no
  dowiz-central dispatch state; each hub runs its own loop over its own pool (§5.3).
- **NOT pickup/dine-in fulfillment logic.** Those orders are gated *out* at construction (M1) and
  complete via the existing `Ready→PickedUp` edge with no dispatch — §16.60's "same flow minus
  the dispatch step" realized as a compile-/constructor-time skip, not a runtime branch littered
  through the code.

---

## 3. Predefined types & constants (standard item 4 — named BEFORE implementation)

```rust
// ── bebop2/proto-cap/src/dispatch.rs — NEW module. Pure Law, NO kernel dep,
//    NO wall-clock (now_ts injected) — the claim_machine.rs:1-5 precedent, applied. ──

use crate::event_dict::CourierKey;         // [u8; 32] — matcher.rs already keys on this
use crate::matcher::{assign, Courier, Order};
use crate::claim_machine::{ClaimStatus, assert_transition as claim_transition};

// ── Fixed, courier-INDEPENDENT timing. There is deliberately NO per-courier value. ──
pub const OFFER_TIMEOUT_SECS: i64  = 30;   // accept window — identical for EVERY courier (§1.2)
pub const REPOLL_INTERVAL_SECS: i64 = 20;  // queued no-courier re-poll base interval (§1.4)
pub const REPOLL_MAX_SECS: i64      = 120; // backoff cap — busy-poll guard (§5.2)
pub const OFFER_WAVE_SIZE: usize    = 1;   // sequential single-offer (§1.1); K>1 = named future

/// The fulfillment discriminator (M1). Mirrors the order-record's `fulfillment_type`
/// WITHOUT importing the kernel — the same "mirror the shape, no kernel dep" discipline
/// claim_machine.rs:1-5 uses for OrderStatus. 1:1 correspondence to the persisted field,
/// whose home is the order-record owner's diff (P62/P69, §2 seam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FulfillmentKind { Delivery, Pickup, DineIn }

/// Why an offer advanced. Distinguishes decline from timeout FOR TELEMETRY ONLY (§5.6) —
/// both advance identically; neither ever influences a ranking (§1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceReason { Declined, TimedOut }

/// A live offer: exactly one courier, one deadline. The deadline is `now_ts +
/// OFFER_TIMEOUT_SECS` — NOTE it is computed with NO courier input, so no courier can
/// get a longer/shorter window (§1.2, structural anti-scoring).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveOffer { pub courier: CourierKey, pub deadline_ts: i64 }

/// The ENTIRE dispatch state for one order. This is the only place P65 stores anything,
/// and everything in it is per-order ephemeral (destroyed when the order leaves Ready).
///
/// STRUCTURAL NO-COURIER-SCORING (the whole point): the only courier-keyed field is
/// `offered_this_round`, a within-one-sweep skip-set that is CLEARED every round (§1.3-1.4)
/// and NEVER read by assign()/hrw_weight(). There is deliberately NO field named or shaped
/// like a score/rating/rank/decline_count/accept_rate — the CI gate (ci-no-courier-scoring.sh:25)
/// scans this file automatically and §4.6 adds a red-case that proves it goes RED if one is added.
#[derive(Debug, Clone)]
pub struct DispatchSession {
    pub order_id: u64,
    pub kind: FulfillmentKind,               // always Delivery once constructed (M1 gate)
    offered_this_round: Vec<CourierKey>,     // ephemeral skip-set; cleared each round
    live_offer: Option<LiveOffer>,           // the single outstanding offer (WAVE_SIZE=1)
    repoll_at_ts: Option<i64>,               // set while queued (no online candidate)
    backoff_secs: i64,                       // capped exponential; reset on online-set change
    ready_since_ts: i64,                     // for honest "waiting Ns" status (§5.5)
}

/// The driver's output — an EVENT STREAM (standard item 3: tests assert on sequences,
/// mirroring the kernel decide/fold law). Every state change is one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchEvent {
    Offered   { courier: CourierKey, deadline_ts: i64 },  // claim Offered; rides P61 (M6)
    Advanced  { from: CourierKey, reason: AdvanceReason }, // claim Offered→Released; next courier
    Assigned  { courier: CourierKey },                     // claim Offered→Claimed; → Ready→InDelivery
    RoundExhausted,                                        // all online tried; order stays at Ready
    Requeued  { repoll_at_ts: i64 },                       // queued no-courier; honest waiting status
    StaleAccept { courier: CourierKey },                   // late accept on a released offer (§4.3)
}

/// Inbound stimulus for one tick. The clock and presence are INJECTED (proto-cap has no
/// clock, Cargo.toml; presence is the hub's, §2) — this keeps tick() a pure function.
#[derive(Debug, Clone)]
pub enum DispatchInput {
    Tick,                                     // wall-clock advanced to now_ts (timeout/backoff check)
    Accept  { courier: CourierKey },          // courier tapped accept
    Decline { courier: CourierKey },          // courier tapped decline
    OnlineSetChanged,                         // a courier came online/offline — re-poll now
}

/// Typed construction refusal — the M1 gate. A Pickup/DineIn order CANNOT make a session,
/// so it can never emit an Offered event: pickup-never-dispatched is unrepresentable (§5.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError { NotDeliverable(FulfillmentKind) }

impl DispatchSession {
    /// M1 gate: ONLY a Delivery order yields a session. Everything else = typed refusal.
    pub fn open(order_id: u64, kind: FulfillmentKind, now_ts: i64)
        -> Result<Self, DispatchError>;

    /// M2/M3/M4: pure driver. `candidates` = the caller's CURRENTLY-ONLINE couriers
    /// (presence is upstream, §2). `now_ts` = injected clock. Returns the events this
    /// tick produced. Internally: assign(order, candidates, len) → walk skip `offered_this_round`
    /// → offer/advance/assign/exhaust/requeue. NEVER calls a system clock; NEVER persists a
    /// courier key beyond `offered_this_round` (cleared each round).
    pub fn tick(&mut self, input: DispatchInput, candidates: &[Courier], now_ts: i64)
        -> Vec<DispatchEvent>;
}
```

Rejected alternatives (DECART one-liners, standard item 19):

- **Fan-out first-accept-wins as Wave-0 default** — rejected: introduces accept-latency-as-selection
  (a scoring vector) and burns fallback candidates every offer (§1.1). `OFFER_WAVE_SIZE=1`;
  K>1 is the named future with a mandatory no-scoring re-review.
- **A new `OrderStatus::AwaitingCourier` state** — rejected: R4 §5.3 + the FSM already express
  "waiting" as `Ready` with an empty/exhausted candidate set. A new state would move
  `FSM_GOLDEN_SIGNATURE` (`order_machine.rs:465`) and is redundant. Waiting is `Ready`, honestly.
- **A `next_rank: usize` cursor instead of `offered_this_round`** — rejected on two counts:
  (a) the online set can change between offers, so a raw index into a stale list points at the
  wrong courier; re-running `assign` over the *current* online set each tick is the requeue-never-drop
  contract (`matcher.rs:75`). (b) a field literally named `*rank*` risks confusing readers of the
  no-scoring gate — `offered_this_round` (a skip-set) is unambiguous and provably ephemeral.
- **Reading the clock inside `tick`** — rejected: proto-cap is clock-free by design (`Cargo.toml`);
  injecting `now_ts` keeps `tick` a pure, replayable function (deterministic-mesh property, §1.1).
- **A durable per-hub dispatch store** — rejected: the session is re-derivable from the order's
  persisted `Ready` status + the current online set + the logged offer/release events
  (Snapshot-Re-entry, §5.4); a second store is state with nothing to add.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (items 3, 5)

Order below is dependency order; all buildable today against the existing matcher/claim/order code.

### 4.1 M1 — `FulfillmentKind` + the dispatch gate (pickup never enters)

`DispatchSession::open(order_id, kind, now_ts)` returns `Ok(session)` **iff**
`kind == FulfillmentKind::Delivery`, else `Err(NotDeliverable(kind))`. A Pickup/DineIn order thus
**cannot construct a session and therefore cannot emit a single `Offered` event** — §16.60's
"same flow minus dispatch" as a constructor-time skip, not a runtime `if` scattered through the
driver. RED→GREEN: `pickup_order_is_not_deliverable` — `open(_, Pickup, _)` is `Err`,
`open(_, DineIn, _)` is `Err`, `open(_, Delivery, _)` is `Ok`. **Adversarial:** drive a full
tick-loop attempt on a session that was refused → impossible by construction (there is no session
value to call `tick` on); the test asserts the type-level impossibility (the `Err` arm carries no
session).

### 4.2 M2 — the driver: offer → await → advance over the ranked list

`tick`, on a `Delivery` session with a non-empty online set and no live offer: run
`assign(order, candidates, candidates.len())` (`matcher.rs:63`), take the highest-ranked courier
**not in `offered_this_round`**, set `live_offer = Some(LiveOffer{ courier, deadline_ts: now_ts +
OFFER_TIMEOUT_SECS })`, push it to `offered_this_round`, emit `Offered{...}` (which the M6 layer
maps to `ClaimStatus::Offered` + the P61 send). RED→GREEN (event-sequence form, standard item 3):
- `happy_primary_accepts`: `[Offered(rank0)]` then on `Accept(rank0)` → `[Assigned(rank0)]`.
- `advance_on_decline`: `Offered(rank0)`, then `Decline(rank0)` →
  `[Advanced(rank0, Declined), Offered(rank1)]`.
- `advance_on_timeout`: `Offered(rank0)`, then `Tick` at `now_ts = deadline` →
  `[Advanced(rank0, TimedOut), Offered(rank1)]`.

**Adversarial:** a `Tick` at `now_ts = deadline - 1` must **not** advance (off-by-one on the
deadline is a real dispatch bug — the test pins `>= deadline`, not `>`); an `Accept` from a
courier who is *not* the `live_offer.courier` (a stale/foreign accept) is ignored, not assigned
(§4.3); an empty online set from the start → §4.4.

### 4.3 M3 — accept / decline / late-accept race (reuses an existing illegal-transition gate)

Accept of the live offer → claim `Offered → Claimed` (`claim_machine.rs:85`, legal), emit
`Assigned`, drop the session's activity (order will leave `Ready`). Decline → `Offered → Released`
(legal), advance (§4.2). **The late-accept race** — a courier accepts *after* their offer already
timed-out and advanced: their claim is already `Released`, and `Released → Claimed` is **illegal
by the existing table** (`claim_machine.rs:78`, tested `:124-131`). P65 does not add new code for
this — it surfaces `StaleAccept{courier}` (informational) and the claim transition is refused by
the primitive. RED→GREEN: `late_accept_after_timeout_is_stale` — `Offered(C0)`, timeout advances
to `Offered(C1)`, then `Accept(C0)` arrives → `[StaleAccept(C0)]`, order still governed by the
C1 offer, exactly one eventual `Assigned`. **Adversarial (double-assign attempt):** `Accept(C0)`
and `Accept(C1)` in the same tick batch → **exactly one** `Assigned` (the first in event order);
the second hits an order already leaving `Ready` (`InDelivery` has no inbound offer edge,
`order_machine.rs:84`) → refused. Money is untouched by assignment (assignment is coordination,
not a money leg — §5.1).

### 4.4 M4 — queued no-courier re-poll (wait at `Ready`, never give up)

When `assign(order, candidates)` minus `offered_this_round` is empty (all online tried) **or**
`candidates` is empty (nobody online): emit `RoundExhausted`, **clear `offered_this_round`**, set
`repoll_at_ts = now_ts + backoff`, `backoff = min(backoff*2, REPOLL_MAX_SECS)`, emit
`Requeued{repoll_at_ts}`. A later `Tick` with `now_ts >= repoll_at_ts` **or** an
`OnlineSetChanged` input (backoff reset to `REPOLL_INTERVAL_SECS`) starts a **fresh full round**.
RED→GREEN (event-sequence): `no_courier_from_start` — empty pool → `[RoundExhausted, Requeued]`,
and the order status is asserted **still `Ready`** (never `Rejected`); `exhaust_then_new_courier`
— all decline → `[..., RoundExhausted, Requeued]`, then `OnlineSetChanged` with a new courier →
`[Offered(new)]`. **Adversarial (the §16.49 teeth):** `all_decline_never_rejects` — every online
courier declines across many rounds; assert the emitted stream contains **no** terminal-order
event and the order never transitions toward `Rejected`/`Cancelled` (there is no such edge to
emit — `order_machine.rs:84`); the only way the loop ends in the test is an injected explicit
`Ready→Refunding` cancel, asserted to be human-initiated (a distinct input, not an orchestrator
timer). `backoff_caps_at_max` — repeated empty polls; `backoff_secs` never exceeds
`REPOLL_MAX_SECS` (busy-poll guard).

### 4.5 M6 — integration seams (interfaces only; wiring tests `#[ignore]` until seams close)

Three seams, each an interface P65 emits/consumes, owned jointly with the named phase:
(a) **`Assigned` → kernel `Ready→InDelivery`**: the hub order-service observes `Assigned` and
calls `order_machine::assert_transition(Ready, InDelivery)` (`order_machine.rs:139`); P65 does not
mutate the kernel FSM (proto-cap has no kernel dep). Test `assigned_drives_ready_to_indelivery` is
`#[ignore = "order-service-wiring"]` until the hub order-service exists.
(b) **`Offered` → P61 send**: the offer message to the courier device rides the notification
fabric; P65 emits the event, P61 delivers it. `#[ignore = "P61-transport"]`.
(c) **`fulfillment_type` field**: the persisted order carries it; the order-service maps it to
`FulfillmentKind` at `open`. `#[ignore = "order-record-field"]` (P62/P69 seam).
Ignored-not-deleted — the P51/P38 honesty convention (`BLUEPRINT-P51` §2). Everything in §4.1–4.4
is buildable and testable **now** with in-crate fixtures (couriers = `[u8;32]` keys, `now_ts` =
an `i64` fixture clock).

### 4.6 M5 — the NO-COURIER-SCORING structural proof (the red line, made falsifiable)

This is the item the whole blueprint exists to protect. Three independent gates:

**Test A — two-world offer-sequence determinism (`decline_never_penalizes_future_ranking`).**
Fix a courier pool `P` and two orders. *World-A:* run dispatch for order 1; courier `C` (whatever
rank they hold) is offered and **declines** (advance). *World-B:* order 1 never happens. In **both**
worlds, run dispatch for order 2 over `P` and assert the emitted `Offered` sub-sequence is
**byte-identical**. Because the orchestrator carries zero cross-order courier state (§1.3), the two
must match exactly. This test goes **RED the instant anyone threads a decline/timeout count into
ranking** — it is the behavioral falsifier for the entire red line.

**Test B — matcher purity anchor (`ranking_takes_no_history`).** Assert (reusing
`matcher.rs:176-180` `green_mesh05_hrw_weight_pure`) that the orchestrator only ever calls
`assign(order, online_set, len)` — never with any history/count argument, which does not exist in
the signature (`matcher.rs:63`). A compile-level guard: `assign`'s type is fixed upstream; P65's
call sites are grep-asserted to pass only `(&order, &candidates, candidates.len())`.

**Test C — CI-gate red-case extension.** `bebop2/proto-cap/src/dispatch.rs` is **already** under
`ci-no-courier-scoring.sh` (it scans all `bebop2/` non-`core/` structs, `:29`). Extend
`scripts/test-no-courier-scoring.sh` (the existing regression harness, `ci-no-courier-scoring.sh:18-19`)
with a red-case that writes a `DispatchSession`-shaped struct carrying `pub decline_count: u32`
(and separately `pub accept_rate: f32`) and asserts the gate goes **RED**. This turns "someone
adds a per-courier counter field" into a **CI-time failure**, not a runtime surprise (standard
item 14).

**Test D — aggregate-only telemetry (`telemetry_carries_no_courier_key`).** Dispatch counters
(offers/accepts/declines/timeouts, §5.6) are **per-hub totals**, never labeled by `CourierKey`.
Assert the telemetry emission takes no `CourierKey` argument — a per-courier decline counter would
be a scoring ledger wearing a metrics hat. Grep/type check: no telemetry symbol in `dispatch.rs`
accepts or maps over a `CourierKey`.

**Not-done clause:** any of Tests A–D failing = **NOT done**, regardless of behavioral green. A
courier-identity-keyed persistent field anywhere in the dispatch path = NOT done even if all
sequence tests pass.

---

## 5. Cross-cutting design obligations (items 6, 8, 9, 11–16)

### 5.1 Hazard-safety as math (item 6) — reachability, not prose

- **No-scoring is a data-flow reachability argument:** the ranking functions `hrw_weight`/`assign`
  have no history parameter (`matcher.rs:41,63`); the orchestrator's only courier-keyed state,
  `offered_this_round`, is cleared each round and read by no ranking function; therefore **no path
  exists from "a courier's past behavior" to "a future rank."** The unsafe state (behavior biases
  future ranking) is *structurally unreachable*, falsified by §4.6 Test A + gate C.
- **Silent rejection is unrepresentable:** the order FSM has no `Ready→Rejected`/`Ready→Cancelled`
  edge (`order_machine.rs:84`); the orchestrator emits no order-terminal event; the only exits
  from `Ready` are a real accept or a human cancel. There is nothing to emit an auto-reject *with*.
- **Pickup-never-dispatched is unrepresentable:** the M1 constructor refuses non-`Delivery`
  (§4.1) — a Pickup order holds no `DispatchSession`, so `Offered` is unreachable for it.
- **No double-assign:** `Ready→InDelivery` fires once (`order_machine.rs:84`); a second accept
  meets a non-`Ready` order and is refused; a late accept meets `Released → Claimed` illegal
  (`claim_machine.rs:78`). Two independent gates, both pre-existing.
- **Money is untouched:** dispatch is pure coordination (courier ↔ order), not a money leg; no
  `Refunding`/payment transition is reachable from any `DispatchEvent` (they carry no money).

### 5.2 Schemas & scaling axes (item 8)

`DispatchSession` is **O(1) state per active `Delivery` order** (one optional live offer + a
skip-set bounded by the venue's courier count). Axis 1 = **couriers per venue**: `assign` is
`O(n log n)` in the sort (`matcher.rs:70`); at a venue's realistic `n` (tens, low hundreds) this
is microseconds — break point is `n ≳ 10⁴` couriers-per-venue (implausible for one venue; §16.3
scopes the pool to *this venue's own* couriers), at which the sort would want a partial-select
(top-K heap) instead of full sort. Axis 2 = **concurrent waiting orders per hub**: one session
each, O(1); break point is memory at `≳10⁵` simultaneous queued orders (a hub far past its
capacity). Axis 3 = **re-poll frequency**: the capped backoff (`REPOLL_MAX_SECS`) bounds wake-ups
for a long-queued order at ≤1 poll / 120 s — the busy-poll guard; without it a no-courier order
would spin. No break point in sight for a single venue.

### 5.3 Isolation (item 11), mesh awareness (item 12), living memory (item 15)

**Isolation / bulkhead:** the orchestrator is **hub-local** — each hub runs its own loop over its
own pool (§16.6 isolated hubs); a hub's dispatch failure cannot propagate to another hub because
there is no shared dispatch state. `tick` is a pure function of injected inputs, so a bad input
cannot corrupt a sibling order's session (sessions are independent values). **No central dowiz
queue** (explicit task constraint; §16.14 no-central-state). **Mesh:** the only mesh-borne
payloads are (a) the **offer-out** message `Offered{courier, deadline_ts}` → the courier device
(≤ ~48 B: 32-B key + 8-B deadline + tag) and (b) the **accept/decline-in** event (≤ ~40 B), both
riding the P61 / P34–P37 wire at human cadence (one per courier interaction, not a stream) — a
trivial budget, no gossip, no consensus (the HRW ordering is coordination-free by construction,
`matcher.rs:5-13`, so even under future replication nodes agree without a round trip). **Living
memory (temporal access pattern):** the honest "waiting for courier" status is derived from
`ready_since_ts` (elapsed = `now_ts - ready_since_ts`) — a temporal read, not a stored countdown;
superseded offers demote (claim `Released`), never mutate in place — the same
content/epoch discipline as `internal-retrieval-living-memory-arc`.

### 5.4 Rollback / self-healing vocabulary (item 13, used precisely)

- **Self-Termination leg (claimed):** typed refusals (`DispatchError::NotDeliverable`, the
  empty-set → no-offer path); the offered-set cleared each round (bounded state); the *absence*
  of a give-up edge (silent-reject unrepresentable, §5.1). These are hard invariant boundaries,
  not a supervisor's policy.
- **Self-Healing leg (claimed narrowly):** requeue-never-drop + re-poll (§4.4) is genuine
  error-correction — when an offer fails (decline/timeout) or the pool changes, the driver
  regenerates a fresh deterministic offer from *current* truth (`assign` over the live online
  set). Claimed for the **offer plan only**, not for order/money state.
- **Snapshot Re-entry (claimed):** the `DispatchSession` is **derived state** — on hub restart it
  is cheaply regenerated from the order's persisted `Ready` status + the current online set + the
  logged offer/release events; the last valid epoch is the order's committed status. No separate
  durable dispatch store is needed (§3 rejected alternative). Mechanical rollback: the module is
  additive (one new `dispatch.rs`, one `pub mod` line) — deletion restores today's tree.

### 5.5 Honest status derivation (§16.14) & 5.6 telemetry (item 10 hook, item 14 gate)

**Status:** the customer-facing "waiting for courier / offered to a courier" status is a pure read
of session state (`live_offer.is_some()` → "a courier is being asked"; `repoll_at_ts.is_some()` →
"waiting for a courier to come online, `now_ts - ready_since_ts` s so far") — never a fabricated
"assigned" (§16.49). **Telemetry:** per-hub aggregate counters — `offers_made`, `accepts`,
`declines`, `timeouts`, `orders_queued_now`, `max_wait_secs` — ride the existing native-telemetry
hooks (P-H lane). **Explicit gate (item 14):** these counters are **never keyed by `CourierKey`**
(§4.6 Test D) — an aggregate decline total is operations data; a *per-courier* decline total is a
scoring ledger, so the type simply offers no courier-labeled counter. A regression in this class
surfaces at CI (gate C), not at review.

### 5.7 Linux discipline (item 9) — the adoption-framework verdicts

- **ALREADY-EQUIVALENT:** one ranking authority (`matcher.rs`, reused), one claim-lifecycle
  authority (`claim_machine.rs`, reused), one order-FSM authority (`order_machine.rs`, reused,
  zero edges added) — P65 forks none of them, exactly the single-source-of-truth discipline.
- **REINFORCES:** pure, clock-injected Law (deterministic, replayable) — the same "no I/O, no
  float, WASM-safe" kernel discipline `order_machine.rs:1-5` and `claim_machine.rs:1-5` state.
- **EXTENDS:** the event-stream-as-output contract (`DispatchEvent`) as a new gate class for
  *coordination* logic (tests assert on sequences, standard item 3), matching the decide/fold law.
- **GAP (honestly named):** presence/liveness ("who is online") is upstream and not yet built as a
  first-class hub service — Wave-0 P65 depends on the caller supplying an accurate online set; the
  presence layer (§16.53 heartbeat) is the named prerequisite for M6's live wiring, not P65's scope.

### 5.8 Tensor/spectral + eqc reuse (item 16) — honestly NOT invoked

Dispatch is a sequential FSM-driver over a small ranked list; there is **no spectral or tensor
structure to exploit**, and inventing one would be the ritual-math the Anu/Ananke discipline
forbids (`anu-ananke-strict-discipline-feedback`). The *order FSM's own* spectral drift-gate
(`FSM_GOLDEN_SIGNATURE`, ρ=0, `order_machine.rs:334,465`) already guards the `Ready` branch P65
rides — and because P65 adds **zero FSM edges**, that gate stays green untouched (`edges:14`), which
is itself the smart-index catch (item 14) for "someone changed the order lifecycle under dispatch."
eqc/`tools/eqc-rs`: no closed-form math here (deadline = `now_ts + const`), so not applicable.

---

## 6. DoD — falsifiable, RED→GREEN, per item (item 2)

| Item | RED (fails before) | GREEN (passes after) | Permanent regression (item 17) |
|---|---|---|---|
| M1 gate | no `FulfillmentKind`; `pickup_order_is_not_deliverable` RED | Pickup/DineIn → `Err(NotDeliverable)`, Delivery → `Ok`; no session ⇒ no `Offered` | pickup-gate test |
| M2 driver | no `DispatchSession`/`tick`; sequence tests RED | `happy`, `advance_on_decline`, `advance_on_timeout` exact sequences; deadline is `>=` not `>` | offer-sequence tests |
| M3 offer race | late-accept path absent | accept→`Assigned`; decline→`Advanced`; late accept→`StaleAccept` (claim `Released→Claimed` refused); exactly one `Assigned` under double-accept | late-accept + double-assign tests |
| M4 re-poll | no queue path; `no_courier_from_start` RED | exhaust→`[RoundExhausted, Requeued]`, order **still `Ready`**; new courier wakes an offer; backoff caps | `all_decline_never_rejects` + backoff-cap tests |
| **M5 no-scoring** | Tests A–D absent | **two-world offer sequence byte-identical**; matcher-purity anchor; CI red-case (`decline_count`→RED); telemetry carries no `CourierKey` | **all four (A–D) as ledger rows** |
| M6 seams | wiring absent | `Assigned`/`Offered`/`fulfillment_type` seam tests present (`#[ignore]` until owners land) | seam-marker presence |
| FSM invariant | — | `FSM_GOLDEN_SIGNATURE` still `edges:14`, `is_acyclic:true` after P65 (proves zero edges added) | golden-signature test (existing, `order_machine.rs:957`) |

**Not-done clauses:** a per-courier persistent counter/score/rate field anywhere in the dispatch
path = NOT done (§4.6). An orchestrator-internal auto-reject/auto-cancel timer = NOT done (§16.49 —
must have no give-up edge). A courier-specific timeout value = NOT done (§1.2). Any edit to
`matcher.rs`/`claim_machine.rs`/`order_machine.rs` that forks their authority = NOT done (§2).
`FSM_GOLDEN_SIGNATURE` moved off `edges:14` = NOT done (P65 must add no order-FSM edges).

---

## 7. Benchmark plan (item 10) — existing harness, three benches, zero new infra

Criterion harness (the proto-cap/`native-trackers` baseline discipline, P-A §6 / `BLUEPRINT-P51`
§7 precedent). Dispatch is not a hot path, but measured, not assumed:
- `dispatch/offer_cycle_100_couriers` (< 200 µs — a full assign+advance sweep over a 100-courier
  online set; makes "microseconds at venue scale" falsifiable; if RED, the top-K partial-select of
  §5.2 engages).
- `dispatch/tick_steady_state` (< 5 µs — a no-op tick while a live offer is pending; the loop must
  be cheap enough to run frequently).
- `dispatch/repoll_backoff` (O(1) — asserts the backoff arithmetic is constant-time, no scan).
All added RED-commit-first so baselines auto-seed; results to `BENCH_HISTORY.md`, never prose
estimates. Telemetry: the §5.6 aggregate counters ride the existing native-telemetry hooks (P-H
lane) so a latency/queue-depth regression surfaces without review.

---

## 8. Links to docs & memory (item 7)

Depends on / cites: `CORE-ROADMAP-STANDARD-2026-07-17.md` (contract) ·
`SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 W2 (P65 row), §3.2 pt 5, §3 (M2 gate) ·
`OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md` §0, §4.2, §5.3, §7.3–7.4, §8 risk #1 ·
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.3 (venue-brings-own-couriers, no scoring),
§16.14 (honest client-side status, no central state), §16.26 (HRW-automatic, no-scoring red line),
§16.49 (no-courier-available waits, never rejected), §16.59/§16.60 (no quality bar; pickup = same
flow minus dispatch) · `BLUEPRINT-P51-open-map-routing.md` (structural template; `#[ignore]` seam
convention) · `BLUEPRINT-P59-capability-cert-chain.md` (courier admission — interface-only
dependency) · `BLUEPRINT-P60-payment-adapter-core.md` / `BLUEPRINT-P62-catalog-multivendor-data-model.md`
(order-record / `fulfillment_type` field home; money boundary) · `BLUEPRINT-P61-notification-fabric.md`
(offer-out transport) · `HERMETIC-ARCHITECTURE-PRINCIPLES.md` (§9) ·
`docs/regressions/REGRESSION-LEDGER.md` (five rows named in §6). Code cited (live): `matcher.rs`,
`claim_machine.rs`, `event_dict.rs`, `roster.rs`, `Cargo.toml` (bebop2/proto-cap);
`kernel/src/order_machine.rs`, `kernel/src/loops.rs` (dowiz); `scripts/ci-no-courier-scoring.sh`,
`scripts/test-no-courier-scoring.sh` (bebop-repo). Memory:
`cross-branch-todo-map-2026-07-10` (bebop2/wire-native files → `/root/bebop-repo`, honored — the
new module lands there) · `SOVEREIGN-EVENT-EXCHANGE-BLUEPRINT-2026-07-14` (trust = signed
capability, NEVER reputation/blacklist — the exact stance this blueprint enforces at the dispatch
layer) · `crypto-safe-first-pass-2026-07-14` (independent-review discipline) ·
`anu-ananke-strict-discipline-feedback-2026-07-17` (§5.8 — no decorative spectral) ·
`verified-by-math-2026-07-07` · `never-bypass-human-gates-2026-06-29` (the `Ready→Refunding`
cancel is human-gated, §1.4). Supersedes: nothing — additive; **feeds M2 (first delivery order)**
and is consumed by the courier surface **P71** (accept/decline UI against P65).

---

## 9. Hermetic principles honored (item 20 — load-bearing only)

- **P1 MENTALISM** (spec is source): the `DispatchEvent`/`DispatchInput` types + timing consts §3
  precede the driver; the session is derived, regenerable from the order's `Ready` status.
- **P2 CORRESPONDENCE** (one concept, one primitive): one ranking authority (`matcher.rs`), one
  claim-lifecycle authority (`claim_machine.rs`), one order-FSM authority (`order_machine.rs`),
  one timeout constant for all couriers — P65 forks none, adds one thin driver.
- **P6 CAUSE-AND-EFFECT** (determinism as law): `tick` is pure and clock-injected; the two-world
  test (§4.6 A) makes "no hidden cause (past behavior) affects effect (future rank)" a falsifier;
  offer sequences are replayable from the event log.
- **P7 GENDER** (paired verification, no self-certification): the driver's offers are refereed by
  the *independent* `matcher.rs` ranking and `claim_machine.rs` transition tables (P65 asserts on
  their output, never re-implements them); the no-scoring claim is refereed by an *independent* CI
  gate (`ci-no-courier-scoring.sh`), not by the module vouching for itself.

(P3/P4/P5 not load-bearing here; not claimed decoratively — §5.8.)

---

## 10. Standard-compliance map (all 20 points, checkable)

| §2 item | Where satisfied |
|---|---|
| 1 ground truth | §0 (fresh cross-repo cites; matcher/claim/order-FSM findings; the loops.rs name-collision; the no-clock finding) |
| 2 DoD | §6 |
| 3 spec/event-driven TDD | §3 spec-first; §4 RED-first; `DispatchEvent`-sequence assertions throughout §4.2–4.4 |
| 4 predefined types/consts | §3 |
| 5 adversarial/breaking tests | §4.2 (deadline off-by-one), §4.3 (late-accept, double-assign), §4.4 (`all_decline_never_rejects`, backoff cap), §4.6 (two-world scoring falsifier) |
| 6 hazard-safety as math | §5.1 (four reachability arguments) |
| 7 links docs/memory | §8 |
| 8 scaling axes | §5.2 (three axes, each with a break point) |
| 9 Linux discipline | §5.7 (all four verdict classes incl. an honest GAP: presence layer) |
| 10 benchmarks+telemetry | §7, §5.6 |
| 11 isolation/bulkhead | §5.3 (hub-local, no central queue) |
| 12 mesh awareness | §5.3 (offer-out/accept-in payload budget; no gossip; coordination-free) |
| 13 rollback/self-heal vocabulary | §5.4 (three legs claimed precisely) |
| 14 error-propagation gates | §4.6 gate C (CI), §5.1 (typed refusals), §5.8 (FSM golden signature) |
| 15 living memory | §5.3 (temporal status read; demote-never-mutate offers) |
| 16 tensor/spectral + eqc reuse | §5.8 (honestly NOT invoked; FSM drift-gate untouched) |
| 17 regression ledger | §6 (five rows, incl. the four no-scoring rows) |
| 18 agent-executable instructions | §11 |
| 19 reuse-first | §0/§2 (matcher/claim/order-FSM all reused, zero forks; five rejected alternatives §3) |
| 20 Hermetic citations | §9 |

---

## 11. Clear instructions for other agentic workers (item 18 — zero session context assumed)

**Repo:** all edits land in **`/root/bebop-repo`** (bebop2/proto-cap) — NOT `/root/dowiz`
(memory `cross-branch-todo-map`; the matcher/claim machine live here). The dowiz kernel
`order_machine.rs` is **read-only reference** for this phase (P65 adds no kernel code).

1. **T1 (M1+M2+M3 — the driver is the contract).** Create
   `bebop2/proto-cap/src/dispatch.rs` per §3 (types/consts verbatim). Register `pub mod dispatch;`
   in `bebop2/proto-cap/src/lib.rs` immediately after `pub mod claim_machine;` (line ~34). Write
   the RED tests FIRST: `pickup_order_is_not_deliverable`; the `happy`/`advance_on_decline`/
   `advance_on_timeout` sequence tests using an `i64` fixture clock and `[u8;32]` fixture couriers;
   the deadline `>=` off-by-one test; `late_accept_after_timeout_is_stale`; the double-accept
   single-`Assigned` test. Consume `matcher::assign` and `claim_machine::assert_transition` —
   do NOT re-implement ranking or the claim table. Acceptance: `cargo test -p bebop-proto-cap
   dispatch` green.
2. **T2 (M4).** Add the re-poll path per §4.4 to `tick` (exhaust → clear `offered_this_round` →
   backoff → `Requeued`; `OnlineSetChanged`/expired `repoll_at_ts` → fresh round). RED tests
   first: `no_courier_from_start`, `all_decline_never_rejects` (assert order never leaves `Ready`
   toward a terminal), `exhaust_then_new_courier`, `backoff_caps_at_max`. Acceptance: green;
   assert `REPOLL_MAX_SECS` is never exceeded.
3. **T3 (M5 — the red line).** Write §4.6 Test A (`decline_never_penalizes_future_ranking`,
   two-world byte-identical offer sequence) and Test B (matcher-purity call-site anchor). Extend
   `bebop-repo/scripts/test-no-courier-scoring.sh` with a red-case writing a dispatch struct with
   `pub decline_count: u32` and assert `ci-no-courier-scoring.sh` exits non-zero (Test C). Add the
   telemetry-no-`CourierKey` assertion (Test D). Acceptance: A/B/D green; C proves the gate goes
   RED on the injected field, then remove the injected field and confirm the real `dispatch.rs`
   passes the gate.
4. **T4 (M6 seams — interfaces only).** Add the three seam tests with `#[ignore]` markers keyed
   `order-service-wiring` / `P61-transport` / `order-record-field` (§4.5). Do NOT wire the kernel
   `Ready→InDelivery` transition here — leave the named marker; the hub order-service (P62/P69
   territory) closes it. Add the five §6 ledger rows to `docs/regressions/REGRESSION-LEDGER.md`.
5. **T5 (benches).** Add the three §7 benches RED-commit-first so baselines seed; results to
   `BENCH_HISTORY.md`. Acceptance: benches run; `dispatch/offer_cycle_100_couriers` < 200 µs.
6. **Verify the FSM invariant held.** Run the existing `green_live_signature_matches_golden`
   (`order_machine.rs:957`) — it must still pass with `edges:14`, proving P65 added no order-FSM
   edge. If it moved, P65 touched the order lifecycle and is NOT done.
