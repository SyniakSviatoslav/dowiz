# BLUEPRINT-B3 — `ExposureLedger` + Hierarchical Budget Envelopes

> **Anchors:** R5 §3 (15c3-5 pre-trade, in-path, per-counterparty exposure; Knight Capital) ×
> R5 §1 (LULD graduated limit-state + designed re-open) × R3 §5 (hierarchical budget envelopes)
> × Hermetic P4 ("every intermediate degree is a named variant on one axis") × RC-2 (no
> self-certified fast lane) × SYNTHESIS §2.5/§3.3.
> **Depends-on:** **B1** for the source of `CapabilityClass` (read from the signed
> `AgentManifest`'s fail-closed capability set — this blueprint defines no competing concept);
> **B2** for the settlement/commitment event TLV — specifically its `expiry_tick` field and the
> timeout-refund sweep. Per SYNTHESIS §5.3, the **rate-envelope half (§2.2) can land before B2
> finalizes**; the ledger half (§2.1/§2.3–2.5) lands after B2's event shape freezes.
> **Parallel-safe-with:** B4 (crypto bench — disjoint files).
> **Status:** PLANNING ARTIFACT ONLY. No `.rs` file is edited by this document.
> **Re-verified live** against `feat/agentic-mesh-protocol-2026-07-17` on 2026-07-17.

---

## §0 — The problem

The kernel bounds *flow* but not *stock*. `TokenBucket` refuses excess rate — but a
counterparty that accepts 50 tasks and completes none never trips it, because elapsed time
alone refills the bucket. R5 §3's 15c3-5 finding: a **position limit heals only on settlement,
never by a clock**, must be **per-counterparty** (one bad peer must not consume the node's
whole headroom), and must sit **in the order path, not advisory** (Knight Capital: dashboards
lost $460M in 45 minutes because the control was not structural). The exchange plane (B1/B2)
creates exactly this stock — open budget commitments awaiting DvP settlement — and nothing
bounds it today. Two R5 §1 refinements ride along: graduated response (LULD's 15-second Limit
State before any halt) and a *designed automatic re-open* for every non-tamper halt.

## §1 — Current-state evidence (live re-read)

**`kernel/src/token_bucket.rs:27-89` — confirmed a rate limiter, not an exposure limiter.**
`try_acquire` (`:46-63`) calls `refill()` first; `refill` (`:67-82`) adds
`refill_rate * elapsed.as_secs_f64()` on every probe, capped at `capacity`. The healing input
is **wall-clock elapsed time and nothing else** — the API (`new`/`try_acquire`/`available`)
has no settlement, release, or refund entry point. The synthesis's distinction ("TokenBucket
heals with time; ExposureLedger heals only on settlement") is real, verbatim, in the refill
arithmetic. Degrade-closed refusal (`:52`) and the falsifiable bound (test `:96-113`) are the
character this blueprint inherits.

**`kernel/src/event_log.rs:389-419` — the pre-persist gate slot exists and composes.**
`commit_after_decide_drift_gate` runs `classify_drift` **before** `decide`; an `Unstable`
spectrum returns `CommitError::Rejected(..)` — the Law pole (`:409-414`), nothing persisted —
then delegates to `commit_after_decide` (`:339-361`), where the durability barrier (`:359`) is
the Store pole; the poles are typed and distinct (`CommitError`, `:263-268`). `try_commit`
must occupy this slot with the same discipline. Duplicates short-circuit before `decide`
(`:350-351`) — a replayed commitment-open must likewise never double-reserve.

**`kernel/src/hydra.rs:332-348` — `ingest_peer_breach` is the convergence hook.** A verified
peer `BreachAlert` is durably recorded as an external-witness row (`BREACH_WITNESS_ACTOR`,
stable content-id via `append_raw`, idempotent on replay). This is R5 §1's "stop trading with
a burnt peer" mechanism; today it only *records* — nothing reads it into any limit.

**`kernel/src/order_machine.rs:140-153` — the fold-reducer pattern.** `fold_transitions` is
the house pattern for state derived deterministically from an event sequence: replay, stop
typed on first violation. The exposure projection (§2.4) copies this shape.

## §2 — Target-state design

### 2.1 The typed `ExposureLedger` and `Commitment`

One refinement to the synthesis sketch, with justification: `per_peer` cannot map to a single
`Commitment` — settlement events must match a *specific* commitment (B2's hashlock pairing is
per-exchange), and expiry is per-commitment. So the peer entry is a rollup holding its open set:

```
pub type PeerId = [u8; 32];            // NodeId = SHA3-256(pq_pub ‖ classical_pub); same 32-byte
                                       // id hydra's BreachAlert.node_id carries.

pub struct Commitment {
    pub commitment_id: [u8; 32],       // content-id of the commitment-open MeshEvent
    pub peer: PeerId,
    pub class: CapabilityClass,        // B1's type, read from the verified manifest/chain
    pub amount: u64,                   // integer budget units (manifest-declared denomination;
                                       // integer-money discipline, no floats on this path)
    pub opened_tick: u64,
    pub expiry_tick: u64,              // READ from B2's settlement TLV — B3 does not define it
}

pub struct PeerExposure {
    pub outstanding: u64,              // checked_add sum of open amounts (overflow = reject)
    pub open: BTreeMap<[u8; 32], Commitment>,
    pub cap: u64,                      // per-peer cap; 0 = burnt (§2.5)
}

pub struct ExposureLedger {
    pub per_peer: BTreeMap<PeerId, PeerExposure>,
    pub default_per_peer_cap: u64,
    pub aggregate_cap: u64,
    pub aggregate_outstanding: u64,    // cached invariant: Σ per_peer.outstanding
    pub regime: ExchangeRegime,        // §2.3
}
```

**Heals only on settlement — including expiry.** The ledger has no clock. A stalled settlement
does not permanently consume room because B2's timeout sweep emits a **timeout-refund
settlement event** at `expiry_tick`; that *event*, folded like any other, releases the room.
Expiry is metadata carried for B2's sweep and the open-commitments query — never a decrement
the ledger performs itself. (Precise dependency: B3 reads B2's `expiry_tick` field and B2's
two settlement-outcome event kinds — completed, timeout-refunded — as its only decrement
sources. *What fires the sweep* is B2's P5 obligation, flagged there.)

**`try_commit` — signature and slot.** Split check from apply, because the ledger must never
reserve room for an event the store then loses:

```
pub fn try_commit(&self, c: &Commitment) -> Result<(), ExposureError>   // pure, read-only
pub fn apply(&mut self, ev: &ExposureEvent)                             // post-durability fold step
```

`try_commit` refuses when (a) `regime != Open` for new commitments, (b)
`peer.outstanding + amount > peer.cap`, or (c) `aggregate_outstanding + amount >
aggregate_cap` — typed `ExposureError::{Paused, PeerCapExceeded, AggregateCapExceeded}` (three
named refusal poles, P4). It runs inside a sibling gate on `EventLog`:

```
pub fn commit_after_decide_exposure_gate<D, T, E>(
    &mut self, ev: MeshEvent, adjacency: &[Vec<f64>], intervention: bool,
    ledger: &ExposureLedger, decide: D,
) -> Result<(AppendOutcome, Option<T>), CommitError>
```

Order: duplicate short-circuit → drift gate → **`try_commit` (mapped to
`CommitError::Rejected`)** → `decide` → `append` (durability barrier) → caller runs
`ledger.apply` only on `AppendOutcome::Committed`. Reject-before-persist, same slot and
discipline as the drift gate; the single-writer commit path (R5 §2's confirmed convergence)
makes check-then-apply race-free, and because `apply` fires only after `Committed`, the
in-memory ledger is always exactly the §2.4 fold of the durable log. This is 15c3-5's "direct
and exclusive control": the check lives in the kernel commit path, never in an agent's
advisory logic — B2's exchange event kinds route exclusively through this gate.

### 2.2 Hierarchical envelopes over `TokenBucket` (landable before B2)

```
pub struct EnvelopeMap {
    envelopes: BTreeMap<(PeerId, CapabilityClass), TokenBucket>,
    aggregate: TokenBucket,
}
```

**`CapabilityClass` comes from B1.** It is derived from the `AgentManifest`'s declared,
fail-closed capability set as carried in the peer's anchor-rooted delegation chain and verified
by `HybridGate::check` — a field B1 defines; this blueprint only *reads* it. Envelope
parameters (`capacity`, `refill_rate`) are node-local config keyed by class.

**Two-level check, refund on the second level.** `try_dispatch(peer, class, n)`:
1. envelope `try_acquire(n)` — `false` ⇒ typed `EnvelopeExhausted`: the peer's own lane is dry,
   the aggregate untouched, other peers unaffected (R3 §5's "ten looping agents = $5,000"
   containment).
2. aggregate `try_acquire(n)` — `false` ⇒ **`envelope.release(n)`**, then typed
   `AggregateExhausted`. This is the one `TokenBucket` API addition made here:
   `pub fn release(&self, n: u64)` — add `n` back, capped at `capacity` (release never mints
   beyond what was acquired; F33 bound preserved). Without it, node-wide congestion silently
   taxes whichever peer probed first.

**Priority = envelope selection, never a wire flag (RC-2).** A peer's "priority" is exactly
the `(capacity, refill_rate)` of the envelope its *verified* `CapabilityClass` keys into; any
wire priority hint is checked against the class derived from the capability chain, mismatch ⇒
refusal. No queue, no reordering, no new kernel scheduler — the earlier-rejected
"priority-flag dispatcher" reduces to this selector, exactly as SYNTHESIS §2.5 resolved.

### 2.3 Graduated limit-state (Hermetic P4: one axis, named variants)

```
pub enum ExchangeRegime {
    Open,
    LimitState { entered_tick: u64 },   // pause NEW commitments; in-flight settles freely
}
```

- **Entry:** on `apply`, if `aggregate_outstanding * HIGH_WATER_DEN >= aggregate_cap *
  HIGH_WATER_NUM` with `HIGH_WATER = 17/20` (85%), regime → `LimitState` (integer fraction —
  no floats). This fires *before* the hard cap would start refusing, LULD-style: friction
  before wall.
- **Behavior difference:** in `LimitState`, `try_commit` refuses every NEW commitment with
  `ExposureError::Paused`; settlement-completion and timeout-refund events still fold normally
  (in-flight exchanges finish — that is the entire point of the intermediate pole).
- **Auto-reopen (defined, automatic):** regime → `Open` when **both** (a)
  `aggregate_outstanding <= aggregate_cap * 7/10` (70% low-water — hysteresis band prevents
  flapping) and (b) `now_tick - entered_tick >= LIMIT_DWELL_TICKS` (a named const, the LULD
  15-second analog — a one-tick dip does not reopen). Both thresholds are single-authority
  consts with H2-style pin tests.
- **Not conflated with `Locked`.** Hydra's `OrganismState::Locked` is the tamper pole —
  entered by `integrity_check`, exited only by owner re-seed / M9. `ExchangeRegime` is the
  exchange-anomaly axis, entered and exited automatically by the thresholds above. Two axes,
  two reopen rules: exposure anomalies never enter `Locked`; tamper never auto-reopens. The
  effective ladder: Open → LimitState (automatic both ways) → organism-`Locked` (manual,
  different axis, refuses everything anyway).

### 2.4 Read projection for open commitments (no SQL)

`fold_exposure(events) -> ExposureLedger` — a deterministic reducer over the WORM log in the
exact shape of `order_machine.rs::fold_transitions` (`:140-153`): iterate in log order;
commitment-open ⇒ reserve; settlement-completed / timeout-refunded ⇒ release (unknown
`commitment_id` ⇒ typed fold error, stop); breach-witness row for peer P ⇒ `per_peer[P].cap =
0` (§2.5). "Find all open commitments with peer X" is then `ledger.per_peer[X].open` — a
BTreeMap walk, no query language, per MESH-09's sqlless stance (events stay content-addressed
rows the existing retrieval organs already index). The §2.1 in-memory ledger is precisely this
fold's running value: boot = replay; optional `BlockStore` snapshots are an optimization,
never authority.

### 2.5 Burnt-peer zeroing via `ingest_peer_breach`

When `ingest_peer_breach` (`hydra.rs:332-348`) persists a verified external-witness row for
peer P, the exposure fold interprets it: **`per_peer[P].cap = 0` immediately** — every new
commitment involving P is refused from that event onward (Law pole, pre-persist).

**In-flight commitments with P are NOT force-failed.** They stay in the open set, frozen
(outstanding may only decrease), resolving solely through B2's own paths: settlement-complete
if a preimage claim is live, else timeout-refund at `expiry_tick`. Justification: B2's DvP
guarantee (Herlihy PODC 2018 — "no conforming party ends up worse off under any deviating
coalition") holds *because* the claim/refund legs are unconditional given hashlock and
timelock. Force-failing on a locally-ingested, gossip-timing-dependent breach alert would
confiscate a conforming party's claim leg — if this node already delivered work to P, zeroing
must not strand its payment; and the refund leg is what returns this node's locked budget if P
defaults. The cost is bounded and already priced: worst case = exposure outstanding at breach
time (≤ `per_peer_cap`), grief-locked until `expiry_tick` — exactly B2's stated grief-lock
caveat. Containment comes from zeroing *new stock*, not tearing up in-flight contracts.

## §3 — Migration steps (dependency order)

1. `TokenBucket::release(n)` + its bound-preserving test (RED first: prove release cannot
   exceed capacity). Kernel-local, no dependencies.
2. `EnvelopeMap` + two-level `try_dispatch` with refund + the two typed refusal poles; wire
   into the `Dispatcher` path behind B1's `CapabilityClass` (this is the pre-B2 landable half).
3. RC-2 guard test: a frame asserting a priority its verified capability class does not grant
   is refused.
4. *(after B2's TLV freezes)* `Commitment` / `PeerExposure` / `ExposureLedger` types +
   `try_commit`/`apply` + `fold_exposure` reducer with replay tests.
5. `commit_after_decide_exposure_gate` on `EventLog`, mirroring the drift-gate shape;
   route B2's exchange event kinds through it exclusively.
6. `ExchangeRegime` with high/low-water consts + `LIMIT_DWELL_TICKS` (pin tests per H2), entry
   on `apply`, reopen check on tick advance.
7. Breach-row interpretation in the fold; integration test with `ingest_peer_breach`.

One edit per turn; each step's tests seen RED against the pre-fix behavior before GREEN.

## §4 — Acceptance criteria (falsifiable)

1. **Pre-persist refusal:** a commitment that would push `aggregate_outstanding` past
   `aggregate_cap` returns `CommitError::Rejected` from the gate; the event log's `len()` and
   `tip()` are unchanged (never persisted-then-rolled-back); replaying the log through
   `fold_exposure` yields a ledger identical to the in-memory one.
2. **Heals only on settlement (the anti-`TokenBucket` falsifier):** with the ledger at cap,
   arbitrary elapsed time/ticks with no settlement event leaves `try_commit` refusing;
   folding one settlement-completed event makes the same commit succeed.
3. **Limit-state semantics:** driving exposure to ≥ 85% flips regime to `LimitState`; a new
   commitment is refused `Paused` while a settlement-completion event for an in-flight
   commitment still applies and decrements.
4. **Auto-reopen fires:** after settlements bring exposure ≤ 70% AND `LIMIT_DWELL_TICKS`
   elapse, the next `try_commit` succeeds with no operator action; at 71% or before the dwell,
   it still refuses (hysteresis + dwell each independently demonstrated).
5. **Two-level envelopes:** exhausting one `(peer, class)` envelope refuses that peer with
   `EnvelopeExhausted` while another peer's dispatch still succeeds; an aggregate-level refusal
   refunds the envelope tokens (`available()` restored, F33 bound never violated).
6. **RC-2:** a self-asserted priority flag inconsistent with the verified `CapabilityClass`
   is refused; no code path selects an envelope from an unverified wire field (grep-provable:
   envelope key derives only from `HybridGate`-verified chain output).
7. **Burnt-peer:** after `ingest_peer_breach(P)`, a new commitment with P is refused
   (`cap == 0`); an in-flight commitment with P still resolves via its timeout-refund event,
   restoring the node's locked budget — the refund is not orphaned.

## §5 — What this unblocks

This lands the third layer of the Agent Exchange Plane (SYNTHESIS §3.3): admitted agents (B1)
and settled work (B2) now carry **bounded blast radius per counterparty and in aggregate** —
the one containment gap no clock-healing bucket covers. It makes the rejected
priority-dispatcher permanently unnecessary (envelope selection subsumes it), gives
`ingest_peer_breach` market-level teeth (the Knight / MiFID-RTS-6 "stop the flow now" property,
structurally), and provides the open-commitments projection B2's timeout sweep and any future
F44 arbitration hook will read. Out of scope, unchanged: pricing/market logic (§2.1's
sealed-batch preconditions stay dormant law), multi-party netting, and any reputation-derived
limit (caps are config + capability-derived, never history-scored).
