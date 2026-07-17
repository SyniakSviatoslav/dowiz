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

---

## Extended Context

The Agent Exchange Plane is built in three layers, and the first two are both *flow* controls.
B1 admits an identity and mints it a `TokenBucket` envelope — a rate limiter. B2 lets two admitted
peers exchange work for value under a hashlock — a per-exchange atomicity guarantee. Neither bounds
the one quantity that actually determines blast radius: the *total commitment a single counterparty
can have open at one instant*. That is **stock**, and `ExposureLedger` is the only primitive in the
entire plane that bounds it. The distinction is not stylistic. A `TokenBucket` heals with the wall
clock — `refill()` adds `refill_rate × elapsed` on every probe (`token_bucket.rs:67-82`), so a peer
that accepts fifty commitments and settles none never trips it: time alone refills the lane. An
`ExposureLedger` has no clock; its only decrement source is a settlement event (completion or
timeout-refund) folded from the durable log (§2.4). Flow bounds *how fast*; stock bounds *how much
is outstanding right now*. Every other layer answers the first question; only this one answers the
second, and the second is the question that sizes a default.

Remove this layer and B2 is unbounded in exactly the dimension B2 does not police. B2 guarantees
each individual exchange settles atomically — but says nothing about how many exchanges one peer may
have in-flight concurrently. A single compromised or malicious counterparty, staying comfortably
under its `TokenBucket` *rate*, could open commitment after commitment — each one individually
rate-legal, each one a real locked-budget obligation on this node — until the node's entire
settlement headroom is consumed by one peer that never intends to settle. The refills keep coming;
the outstanding stock keeps climbing; nothing refuses. R3 §5's containment number ("ten looping
agents = $5,000") assumes a bound on concurrent commitment that, without this ledger, does not
exist. The rate envelopes (§2.2) narrow the blast radius per lane, but only the ledger caps the
accumulated stock — and only per-counterparty, so one bad peer cannot consume the node's whole room
(§2.1 (b)/(c)).

This is not a novel invention; it is the mesh analog of a rule finance already wrote in blood. SEC
Rule 15c3-5 (R5 §3) requires pre-trade risk controls that are **per-counterparty**, that **heal only
on settlement rather than by a clock**, and — the load-bearing clause — that sit under the broker's
*"direct and exclusive control"* and *"in the path"* of the order, never in advisory logic a desk
can route around. Knight Capital is the counter-example the rule exists to prevent: dashboards that
*observed* the runaway had no authority to *stop* it, and $460M left in 45 minutes. `try_commit` is
that clause made structural: the check lives in the kernel commit path, in the same pre-persist slot
as the drift gate, so an agent's advisory logic can neither see around it nor relax it (§2.1).
ExposureLedger is 15c3-5's "direct and exclusive control, in the path" requirement, ported to a mesh
where the "broker" is every sovereign hub.

## Definition of Done

This blueprint's DoD is **two-phase and gated** — the two halves reach "done" at different times and
the gate between them is a hard, checkable dependency, not a soft preference:

- **Phase A — the rate-envelope half (migration steps 1–3).** Depends only on **B1** (for
  `CapabilityClass`). May be marked done independently, before B2 exists. Done when:
  1. `TokenBucket::release` exists with a bound-preserving RED-first test (sub-DoD below), and every
     pre-existing `token_bucket.rs` test still green.
  2. `EnvelopeMap` + two-level `try_dispatch` with second-level refund + the two typed poles
     (`EnvelopeExhausted`, `AggregateExhausted`) is wired into the `Dispatcher` path behind B1's
     *verified* `CapabilityClass`, acceptance §4.5 demonstrated.
  3. The RC-2 guard (§4.6) is a passing, grep-provable test: no envelope key derives from an
     unverified wire field.
- **Phase B — the ledger half (migration steps 4–7).** **BLOCKED on B2's settlement-TLV freeze.**
  This is the explicit gate, stated as a rule not prose: **Phase B MUST NOT be marked done while
  `expiry_tick` and B2's two settlement-outcome event kinds (completed, timeout-refunded) are not
  frozen in B2's wire schema.** Checkable form: a Phase-B "done" claim is invalid unless `git grep`
  shows B2's TLV constants landed and B3's `fold_exposure` decrement sources cite them *by symbol*,
  not by placeholder. Closing Phase B against an unfrozen B2 TLV is a failing precondition — exactly
  the shape B2's own P07 gate takes, structurally enforced rather than remembered.

**`TokenBucket::release` — sharpened sub-DoD** (this blueprint's one API addition; §2.2 specifies it
and migration step 1 mandates RED-first — confirmed and made exact here):
- **Signature:** `pub fn release(&self, n: u64)` — `&self`, not `&mut self`, mirroring the existing
  `try_acquire(&self, …)` interior-mutability receiver (`token_bucket.rs:46`); `release` is a sibling
  of `acquire`, not a new mutation discipline.
- **Semantics:** add `n` back to the available count, **saturating at `capacity`** — `release` can
  never mint tokens beyond what a prior `try_acquire` removed, so the F33 bound (`available ≤
  capacity`, ever) is preserved by construction, not by convention.
- **RED-first test that proves the cap:** acquire `n`, then `release(2n)` (release past capacity);
  assert `available() == capacity` and never `> capacity`. Written to fail first against the absence
  of the method / a naive unclamped add, green after the saturating implementation. This is the
  falsifier for "release mints free budget."
- **Existing tests:** the addition is **purely additive** — it adds new tests and touches no existing
  `token_bucket.rs` test. The pre-existing `try_acquire`/`refill`/`available` tests remaining green
  is itself the regression proof that `release` did not perturb the shipped rate-limiter behavior.
  (If closing this step required *editing* an existing test, that is a red flag under the
  test-integrity rule, not a normal outcome.)

**Cross-cutting done items (both phases):** integer-only arithmetic on the exposure path (the
high/low-water comparisons stay integer fractions, no floats — §2.3); every named const
(`HIGH_WATER`, the low-water fraction, `LIMIT_DWELL_TICKS`, `default_per_peer_cap`, `aggregate_cap`)
carries an H2-style pin test; `try_commit` is read-only and `apply` runs only on
`AppendOutcome::Committed`, proven by acceptance §4.1's "log `len()`/`tip()` unchanged on refusal";
and no path force-fails an in-flight commitment on burn (§2.5, acceptance §4.7). The consolidated
Wave-0 discriminant-allocation act (CONSOLIDATED §4) is a precondition to Phase-B event routing —
not owned here, but Phase B is not done if B3's exchange events collide with B1/B2 discriminants.

## Event-Driven Architecture Treatment

**`try_commit` is a pre-persist check, not a stored decision.** It runs in the same commit-path slot
as the drift gate — after the duplicate short-circuit, before `decide`, before the durability barrier
(§2.1 gate order). It produces a `Result`, never a `MeshEvent`: a refusal maps to
`CommitError::Rejected` and nothing is written; an acceptance lets the *underlying* exchange event
proceed to `append`, and only `AppendOutcome::Committed` triggers `ledger.apply`. The ledger emits no
events of its own — it *reacts to* B2's exchange events. This is the invariant that makes the whole
thing sound: because `apply` fires only post-durability, the in-memory ledger is at every instant
exactly the fold of the durable log, never ahead of it.

**Ledger state is rebuilt as a pure fold on restart — no bespoke snapshot mechanism is required for
correctness.** This is the kernel's established discipline, not a new one. Hydra's `boot_verify`
(`hydra.rs:253`) and its `hydra_durable_closed_loop_across_restart` test (`hydra.rs:958-1004`) show
the pattern verbatim: commit through a `FileEventStore`, drop the process, reopen with
`Hydra::new(FileEventStore::open(…))`, and the organism re-derives its state from the durable log —
the committed event is present and `boot_verify` recomputes clean. `fold_exposure` (§2.4) is the
exposure-plane instance of exactly this: boot = replay the WORM log; the
`BTreeMap<PeerId, PeerExposure>` is the fold's running value, not an independently-persisted structure
that could drift from the log. The events it folds are precisely B2's — commitment-open (reserve),
settlement-completed and timeout-refunded (release) — plus Hydra's breach-witness rows
(`per_peer[P].cap = 0`). Nothing else is authoritative.

**Is per-boot replay cheap enough, or is snapshotting needed?** The honest answer: *cheap enough now,
snapshot later as a pure optimization*. Exposure-relevant events are a strict subset of the log (only
exchange + breach events; orders, retrieval, and every other organ's events are skipped by the
reducer's match), and each fold step is a `BTreeMap` insert/remove plus one `checked_add` —
microseconds. At 10⁵–10⁶ total log events a full replay is sub-second and runs once per boot; that is
not a bottleneck a delivery mesh will feel. It becomes one only at a log measured in many millions of
events over long uptime — and the mitigation is already the kernel's `snapshot_root` pattern
(`retrieval/memory_store.rs:36`): a `BTreeMap`-backed store yields a deterministic content root, "any
change to any entry changes the root," stable across runs. The exposure ledger's map can produce the
same digest, a `BlockStore` (`backup.rs:29`) can persist a periodic snapshot, and boot then replays
only the tail since the snapshot. Critically, per §2.4 the snapshot is **an optimization, never
authority**: a loaded snapshot is accepted only if its `snapshot_root` matches a recompute over the
snapshotted prefix, so the log — not the snapshot — stays the single source of truth. Don't build the
snapshot until the replay cost is measured to matter (YAGNI); the fold is correct without it.

**The `ExchangeRegime` transition is derived state, NOT a first-class event — and it should stay that
way.** Entering `LimitState` at the 85% high-water mark and reopening at ≤70% + dwell (§2.3) are
deterministic functions of quantities already in the fold: `aggregate_outstanding` (folded), the
entry tick (the first replayed tick at which the threshold crossed, recoverable during the same
fold), the current tick (available live at `try_commit`), and two single-authority consts. Nothing
about the transition is un-derivable from the exposure events plus the pinned constants. Making the
transition its own `RegimeChanged` WORM event would therefore manufacture a **second source of truth
for a value the fold already fully determines** — the RC-4 "unpinned mirror" / dual-authority shape
the arc explicitly flags (CONSOLIDATED §5 Q1.5), the same class as the kernel's historical
3-eigensolver dual-authority bug. If a stored `RegimeChanged` ever disagreed with the recomputed
regime, there would be no principled winner. Keeping it derived loses no auditability: "when and why
did this node enter limit-state" is fully reconstructable by replaying the exposure events against
the two named consts, and an advisory (non-WORM) observability line can still be logged at the
transition. The `regime` field on `ExposureLedger` (§2.1) is thus a *cached memoization* of the
derivation, not an authority — with one determinism obligation to pin: **on snapshot load the regime
must be recomputed, never trusted from the snapshot** (identical treatment to `snapshot_root` being
non-authoritative), so two nodes replaying the same log always agree on the regime regardless of
which one snapshotted. Simpler design, and the correct one: derived beats event here for a concrete
soundness reason, not merely for parsimony.

## Long-Term Consequences, Safety, Scalability

### (a) Scalability — the `per_peer` map and the cost of the check

Two costs are distinct and only one grows. The **aggregate-cap check is O(1)**: §2.1 caches
`aggregate_outstanding` as an invariant (`Σ per_peer.outstanding`), so `try_commit`'s aggregate test
is a single field read plus a `checked_add`, independent of peer count. The **per-peer check is
O(log n)**: one `BTreeMap` lookup. Neither degrades meaningfully with mesh size. The only quantity
that grows is **memory** — one `PeerExposure` retained per distinct counterparty this node has *ever*
exchanged with. A dormant peer's entry is small (two `u64`s, an empty `open` map, a 32-byte key ≈
~100–200 B). Order-of-magnitude: 10⁴ peers ≈ a few MB, 10⁵ ≈ tens of MB, 10⁶ ≈ ~200 MB. For a
delivery mesh the realistic distinct-counterparty count per node is *hundreds to low thousands* — so
**this does not matter yet, and a pruning policy is genuine future work, not a launch blocker.** When
it does matter, the eviction rule is already clean *because the ledger is a pure fold*: a peer with
`outstanding == 0` and no event in the last N ticks can be dropped from the hot map and lazily
re-derived from the log (fold the tail) if it ever returns — evicting a zero-outstanding peer changes
nothing derivable, so correctness is untouched. Recommendation: ship without pruning; name the
trigger (hot-map cardinality past a named bound, or measured memory pressure) as the future-work
condition. Building it now is premature optimization against a peer count the mesh will not reach
soon.

### (b) Safety — who owns the thresholds, and what a wrong value costs

The blueprint carries two *kinds* of tunable, and they deserve *different* ownership answers —
conflating them is the trap:

- **The caps (`default_per_peer_cap`, `aggregate_cap`) are magnitudes of risk appetite** — how much
  outstanding commitment this node will extend to any one peer and in total. Set too low, legitimate
  work is refused and availability suffers; too high, the containment budget is simply larger. Risk
  *tolerance* genuinely varies per node (a well-resourced hub vs. a hobbyist relay), and this is
  exactly what M5 sovereignty means: **the caps are a LOCAL operator choice, per node**, and the
  blueprint already treats them as config (§2.2, §5). Correct as-is.
- **The graduated-response ratios (85% enter, 70% reopen, `LIMIT_DWELL_TICKS`) are the *shape of the
  brake*, not the appetite.** Set too tight (enter at, say, 50%), the node spuriously pauses new
  commitments far below its real budget and starves honest peers — an availability self-harm. Set too
  loose (enter at 99%, or reopen at 98% with zero dwell), the graduated pole collapses: no
  friction-before-wall, and the hysteresis that prevents flapping is gone, defeating the entire
  LULD-style purpose. Here my recommendation **diverges from a naive "M5 ⇒ everything is local"**:
  the ratios should be **protocol-default constants with a pinned floor invariant**
  (`enter% > reopen% + MIN_HYSTERESIS_BAND`, `dwell ≥ 1`), operator-overridable only in the
  *tightening* direction and never past the floor — the same unrelaxable-floor discipline B1 uses for
  `RequireBoth`. Justification against M5: a node's sovereignty is *already fully expressed by its
  caps* (the magnitude of trust it extends); the ratio encodes not additional appetite but the
  mechanism's stability, and letting it be freely loosened adds a footgun (flap, or a silently
  defeated brake) that buys no real sovereignty. An operator who wants to be *more* cautious may
  tighten within the pinned bounds; no operator may loosen the brake into uselessness. So: **caps
  local, ratios protocol-default-with-floor.** This sharpens §2.3's "single-authority consts" without
  weakening it — it says who may move them and how far.

### (c) Ethics / long-term — the exposure model as a NEW attack surface

This is a genuine risk-management primitive: it governs how much *unverified trust* (open,
not-yet-settled commitment) a node extends to any single peer. Making trust-extension explicit and
bounded is the ethical improvement over the status quo (unbounded, implicit). But the model
introduces its own attack surface, and it must be named honestly rather than assumed away. **The gate
keys on exposure *level*, not *rate of approach*.** `try_commit` compares `aggregate_outstanding`
against a static fraction of the cap; a slow crawl to 84% and a sudden jump to 84% are
indistinguishable to it, and the hysteresis band + dwell (§2.3) suppress *flapping* but do nothing
against a *deliberately sustained* high-occupancy state. The concrete failure mode: an adversary
parks a victim node near the ceiling with many small, individually-legitimate-looking commitments —
each rate-legal, each a real obligation — holding `aggregate_outstanding` at, say, 84% indefinitely.
This (i) starves honest peers of aggregate headroom (a griefing denial-of-service that never trips a
single named refusal), and (ii) lets the attacker choose *when* limit-state fires by adding or
releasing one marginal commitment, i.e. it hands the attacker the trigger. The single-peer version is
contained by `per_peer.cap` (one identity cannot alone dominate the aggregate), and the check itself
is race-free (single-writer commit path, R5 §2 — so there is no TOCTOU exploit *at* the threshold).
The residual, unaddressed case is a **Sybil / coordinated set of separately-admitted identities**,
each under its own per-peer cap, collectively parking the aggregate — B1 admission being per-operator
raises the cost but does not structurally forbid it. **The design has no velocity or
time-at-high-occupancy signal, and I name that as future work rather than pretend it is covered.**
The natural home is the kernel's existing Markov/attractor loop-signal machinery (CONSOLIDATED §5 Q2c
names it as the runtime complement; MEMORY records it LIVE and advisory/fail-open): feed the *rate of
change* of `aggregate_outstanding` and per-peer occupancy-duration into that detector as an advisory
anomaly, distinct from the static structural cap, so a "parked near the ceiling" pattern surfaces as
a signal even though every individual commitment is legal. Not built now (YAGNI until a concrete
Sybil-admission threat is demonstrated), but recorded as a named gap with an owner-shaped trigger, in
the arc's E53 style — the containment primitive is honest about the one pattern its static threshold
cannot see.

---

## Safety Hardening (post-adversarial-review)

> Appended 2026-07-17 after the SYSTEM-BREAKER pass (F3, F4) and the COUNSEL review (§2, §5–§6, §8-2,
> §8-3). **This section ADDS containment; it removes and weakens nothing above** — every bound here
> is a tightening (a smaller effective cap, a walled-off sub-pool), never a loosening. Three
> responses land: **H-1** operator-gated enrollment + a shared *stranger* sub-pool (F3 — Sybil
> fragmentation); **H-2** private first-party *bilateral memory* (COUNSEL §5–§6, safeguard 3 — the
> patient-griefer / information-goods repeat offender); **H-3** a dormant `delivered_value` data hook
> on `Commitment` (COUNSEL §2 / §8-2 — so B2's future incremental-delivery mechanism has a place to
> report irreversibly-delivered work). F4's concurrent-settlement **COUNT** cap is a requirement B2's
> own hardening places *on this ledger*; it is cross-referenced (H-1, last paragraph), **not**
> re-specified here — designing it here would collide with the sibling B2 edit.

### H-1 — F3 (Sybil fragmentation): operator-gated enrollment + a shared stranger sub-pool

**Verdict up front: there is a real fix inside the design's own stated constraints — but it is *not*
the naïve "small per-identity stranger cap," which the task correctly floated and which does **not**
work.** The honest correction is load-bearing, so it leads.

**Why a per-identity stranger cap fails.** F3's arithmetic is `K = aggregate_cap / per_peer_cap`
free identities, each opening up to `per_peer_cap`, summing to the whole aggregate. Replacing
`per_peer_cap` with a *smaller* per-identity stranger cap `s` does not close this: the attacker now
mints `K' = aggregate_cap / s` identities instead — and since identities are free (anchor-delegated,
no cost-to-identity, the design's own rejected door), a *larger* `K'` costs the attacker exactly
nothing. A per-identity default of any positive value, applied against the single shared
`aggregate_cap`, is defeated by fan-out. The fix cannot live in the per-identity dimension at all.

**The fix: partition the aggregate, and pool the un-enrolled tier.** The `aggregate_cap` is split
into two named sub-caps, and a peer's *tier* — not merely its per-peer cap — decides which sub-cap it
draws against:

- **Enrolled tier.** A peer the operator has explicitly admitted (see the enrollment event below)
  gets its OWN `per_peer` allowance (`default_per_peer_cap`, or an operator-set value) and its
  outstanding counts against `enrolled_aggregate`. This is the §2.1 world, unchanged, *for
  operator-vetted peers only*.
- **Stranger tier.** Every never-enrolled peer — including all freshly-minted Sybil identities —
  shares ONE pooled `stranger_pool_cap`, a small operator-set fraction of the aggregate. A stranger's
  outstanding is bounded twice: per-identity by a modest `stranger_cap`, **and collectively by
  `stranger_pool_cap` across all strangers at once.** Because the pool is shared, minting `K` Sybil
  identities can consume at most `stranger_pool_cap` *no matter how large K is* — fan-out buys nothing,
  which is exactly the property a free-identity substrate needs.

```
pub struct PeerExposure {
    pub outstanding: u64,
    pub open: BTreeMap<[u8; 32], Commitment>,
    pub cap: u64,               // 0 = burnt (§2.5). For an enrolled peer, its operator-set allowance;
                                // for a stranger, IGNORED — the stranger_cap/pool govern (below).
    pub enrolled: bool,         // NEW. false = stranger tier (default for any never-seen PeerId).
    // (H-2 adds two more fields to this same struct — see below.)
}

pub struct ExposureLedger {
    pub per_peer: BTreeMap<PeerId, PeerExposure>,
    pub default_per_peer_cap: u64,        // REINTERPRETED: the enrolled-tier default an enrollment grants.
                                          // A never-enrolled peer NO LONGER defaults to this — it defaults
                                          // to the stranger tier. (Tightening, not loosening.)
    pub stranger_cap: u64,                // NEW: per-identity cap for an un-enrolled peer (≪ default_per_peer_cap)
    pub stranger_pool_cap: u64,           // NEW: SHARED aggregate sub-cap for ALL un-enrolled peers together
    pub stranger_pool_outstanding: u64,   // NEW: cached Σ outstanding over { peers | !enrolled }
    pub aggregate_cap: u64,               // now the ENROLLED aggregate (enrolled_aggregate)
    pub aggregate_outstanding: u64,       // cached Σ outstanding over { peers | enrolled }
    pub regime: ExchangeRegime,
}
// Invariant (pin test): stranger_pool_cap + aggregate_cap == total node headroom, and
// stranger_pool_cap ≪ aggregate_cap (the stranger pool is a small opt-in surface, never the bulk).
```

`try_commit` (§2.1) gains a tier branch **before** the existing checks, adding two typed poles to the
§2.1 set (`Paused`/`PeerCapExceeded`/`AggregateCapExceeded`), per Hermetic P4 "one axis, named
variants":

- **stranger** (`!enrolled`): refuse unless `outstanding + amount ≤ stranger_cap`
  (`ExposureError::StrangerCapExceeded`) **and** `stranger_pool_outstanding + amount ≤
  stranger_pool_cap` (`ExposureError::StrangerPoolExhausted`). The Sybil set can grief *the stranger
  pool* and nothing else.
- **enrolled**: the §2.1 checks verbatim against `cap` / `aggregate_cap`.

**Enrollment is an event, not a score — and this is the whole "never reputation" defense.** A peer
becomes `enrolled` only by a durable, operator-authored `PeerEnrolled{peer, cap}` row, folded by
`fold_exposure` (§2.4) *exactly* as the breach-witness row is folded in §2.5 — same
`append_raw`/idempotent-witness discipline, opposite sign (enroll grants an allowance; breach zeroes
it). This is **structurally identical to how money-scoped red-lines already gate the highest-value
leg**: B2 §2.4 arms the red-line gate + operator allow-listing for `LedgerMoney`; H-1 applies the
*same* allow-list discipline one tier down, to counterparty *exposure allowance*. It is a **one-time,
binary admission decision** (in the allow-list or not), never a history-derived, behavior-scored, or
gossiped value — so it does not reintroduce reputation (Cheng–Friedman is about *symmetric aggregated*
scores; a first-party binary allow-list is neither), and it does not reintroduce stake/cost-to-identity
(the attacker still pays nothing; free identities simply land in the pooled tier where fan-out is
inert). It is also consistent with B1's rule "structural trust changes are events; runtime metering is
not" (B1 §, `scope.rs` discussion): enrollment *is* a structural trust change, so it *is* an event.

**Admission (B1) ≠ exposure allowance (B3) — two gates, deliberately.** H-1 requires **no** change to
B1's `admit()` (signature + anchor-rooted chain). An agent still admits and dispatches work under its
B1 `TokenBucket`; enrollment governs only the *exposure allowance* B3 extends it. A peer can transact
permissionlessly *within the stranger pool* while awaiting enrollment — so this is not a human-in-loop
step per transaction (which would be the un-scalable "operator ruling per dispute" COUNSEL §5 warns
of); it is an occasional, batchable roster action. The human touches the roster, not the traffic.

**What is closed, and the honest residual.** Closed: the R5 §3 per-counterparty isolation invariant is
*restored for the enrolled aggregate* — one actor, however many identities, cannot consume the node's
real headroom, because that headroom (`aggregate_cap`) is now reachable only by operator-vetted peers
whose caps the operator sized. **Residual, named plainly:** a Sybil set can still saturate the
*stranger pool* and thereby DoS *other honest first-contact strangers* — Sybil resistance *within* an
anonymous pool is the unsolved problem the design already declined to solve, and H-1 does not claim to.
But the blast radius collapses from "the whole node (`aggregate_cap`)" to "the small opt-in
`stranger_pool_cap`," and the enrolled tier — where all real value lives — is walled off. That is the
Knight/R5 §3 property ("one bad peer must not consume the node's whole headroom") recovered structurally,
with the residual bounded, operator-sized, and confined to the anonymous surface. This is a genuine fix
within scope, not a paper-over; the residual is a strictly smaller, honestly-labelled surface, not the
original gap.

**Cross-reference — F4 (concurrent-settlement count).** F4's per-peer *count* cap (bound the *number*
of concurrent open settlements, not just their summed value) is a requirement **B2's own Safety
Hardening places on this ledger** — per that section it lands as a `max_open_count` check in
`try_commit` alongside the value checks (`peer.open.len() < max_open_count`), typed as a sibling
refusal pole. It is **not** re-specified here to avoid colliding with the sibling B2 edit. Note only
the composition: H-1 caps *which peers get a real allowance*; B2's count cap caps *how many concurrent
settlements each may open*; and because F4 is amplified by F3 (§(c) / F4 point 4), H-1's pooling of the
stranger tier also blunts F4's Sybil-amplified variant — a Sybil set can no longer reach the 85%
high-water trip via the enrolled aggregate at all.

**Acceptance (falsifiable), extending §4:**
- **H-1.1** With zero enrolled peers, `K` freshly-minted stranger identities each opening
  `stranger_cap` reach at most `stranger_pool_outstanding == stranger_pool_cap` and are then refused
  `StrangerPoolExhausted` — `aggregate_outstanding` (enrolled) stays `0`; the regime never trips.
  (Same test at `K = 10` and `K = 10_000` yields the *identical* pool ceiling — the falsifier for
  "fan-out buys the attacker more.")
- **H-1.2** Folding a `PeerEnrolled{P, c}` row flips `per_peer[P].enrolled = true`, moves P's
  outstanding accounting from the stranger pool to the enrolled aggregate, and lets P commit up to
  `c`; replay yields an identical ledger (idempotent witness, §2.4 shape).
- **H-1.3** A never-enrolled peer's first `try_commit` is bounded by `stranger_cap`, never
  `default_per_peer_cap` (grep-provable: the stranger branch reads only `stranger_cap`/`stranger_pool_cap`).

### H-2 — COUNSEL §5–§6 safeguard 3: private first-party bilateral memory

COUNSEL's distinction: "never reputation" must mean "never a *shared/gossiped/aggregated* score,"
NOT "never a *private first-party experience*." H-2 lets a node LOCALLY, PRIVATELY remember "peer X
timed-out / aborted settlements **with me** N times" and use it to **reduce (never increase)** X's
local allowance — never gossiped, never shared, never aggregated across nodes.

**The count is a fold over events the node ALREADY holds — no new event type.** The signal is B2's
existing `SettlementRefunded` (`0x1E`) outcome: a settlement that resolved by *timeout-refund* rather
than by `SettlementClaimed` completion is, from this node's seat, a default by the counterparty (the
information-goods abort of COUNSEL §2 is precisely this — A reads the bytes, then lets the settlement
lapse to refund). `fold_exposure` (§2.4) already iterates the log and interprets settlement events;
H-2 extends the *same* pass with a first-party predicate — no `DefaultCounted` event is ever minted,
the count is *derived*, exactly as §2.3's `regime` is derived and (per the EDA section) deliberately
*not* a first-class event.

**It lives folded INTO `PeerExposure`** (not a separate view — it is per-peer, and the same fold pass
produces it in one walk, keeping a single source of truth):

```
pub struct PeerExposure {
    // ... outstanding, open, cap, enrolled (H-1) ...
    pub default_count: u64,       // NEW: first-party count of SettlementRefunded (timeout) events
                                  // where THIS node was a party to the settlement with this peer.
    pub last_default_tick: u64,   // NEW: tick of the most recent such event (drives H-2 decay).
}
```

**The load-bearing structural rule — first-party by the fold predicate, not by log isolation.**
`default_count[X]` increments on folding a `SettlementRefunded` **iff** the settlement's
`{payer_key, worker_key}` (carried in B2's TLV, §2.3) equals `{self_node, X}`. So even if
anti-entropy (MESH-07 Sync·Pull) has unioned *another* node's settlement events into this log, folding
a refund between two *other* nodes B and X changes **nothing** — the predicate `self ∈ {payer, worker}`
fails. Bilateral memory is first-party because the *derivation* is scoped to this node's own
settlements, independent of whose events happen to sit in the log. This is what makes "never
aggregated" structural rather than conventional: there is no code path by which X's defaults against
*A* can raise *C*'s `default_count[X]`.

**Interaction with the per-peer cap: it lowers the EFFECTIVE cap, by construction never raising it.**
`try_commit` replaces the check `peer.outstanding + amount ≤ peer.cap` with `≤ effective_cap(peer)`,
where

```
effective_cap(peer) = peer.cap.saturating_sub(penalty(aged_count(peer)))   // ≤ peer.cap, always
penalty(n)          = n.saturating_mul(DEFAULT_PENALTY_UNIT)                // pinned const, integer
```

`saturating_sub` guarantees `effective_cap ≤ cap` for every input — "reduce, never increase" holds
*by type*, not by discipline (same proof shape as `TokenBucket::release`'s saturating-at-capacity F33
bound). For a stranger (H-1), the penalty shrinks `stranger_cap` the same way, so a repeat-defaulting
stranger self-ejects from the pool toward zero.

**Decay/reset policy (concrete, named): integer half-life aging — a peer's bad history ages out, it
does NOT persist forever.** The count used in the penalty is discounted by the elapsed half-lives
since the last default:

```
aged_count(peer) = peer.default_count >> ((now - peer.last_default_tick) / PENALTY_HALFLIFE_TICKS)
```

Right-shift = one halving per `PENALTY_HALFLIFE_TICKS` of quiet (integer arithmetic, no floats,
matching §2.3). Rationale, grounded in COUNSEL §6's own steel-man *against* memory: (2) local memory
is an attack surface — an adversary can induce timeouts (or a mere network partition can) to poison an
*honest* peer; aging bounds that damage to a window. (3) whitewash defeats *permanent* memory cheaply
anyway, so forgetting stale history costs little. **Forgiveness is time-based, not
success-based on purpose:** a single good `SettlementClaimed` does NOT reset the count (that would hand
the attacker a "grief-N-times-then-one-cheap-good-settlement-to-wipe-the-slate" reset), whereas a peer
that simply *stops* defaulting is gradually forgiven. There is no hard reset event; only continuous
half-life decay toward zero. (This is deliberately the *opposite* clock-treatment from the exposure
stock, which §0/§2.1 forbid from healing on a clock: the stock is a hard safety bound so it must not
time-heal; the penalty is an advisory dampener, so a dampener that never forgives would itself be the
griefing surface COUNSEL §6(2) names.)

**Composition with H-1 closes COUNSEL §6(3)'s whitewash worry.** A burned peer re-enrolling under a
fresh `NodeId` escapes its `default_count` (new PeerId, count 0) — but by H-1 a fresh identity lands in
the **stranger pool**, not back at its old enrolled allowance. So whitewash no longer buys back the
allowance; H-1 (fresh identity starts small) and H-2 (persistent identity accumulates penalty) reinforce
each other. Neither alone is sufficient; together they make defaulting costly whether the attacker keeps
its identity or discards it.

**The RED-test boundary COUNSEL asked for — structural impossibility that this data ever gossips.**
The gossip / anti-entropy surface (MESH-07 Sync·Pull, and the `RevocationSet` union) transmits **only
`MeshEvent`s** from the WORM `EventStore` (`event_log.rs:134`) — a `MeshEvent` is `{prev, actor_pubkey,
actor_seq, payload}` and nothing else. `PeerExposure` (holding `default_count`/`last_default_tick`) is
an **in-memory derived view** — a fold output, never inserted into an `EventStore`, never a `MeshEvent`,
with no `From<PeerExposure> for MeshEvent` and no serializer into any gossip payload. It is therefore
*type-impossible* for the count to enter a Sync·Pull frame: a gossip payload is `MeshEvent`-typed and
`PeerExposure` is not a `MeshEvent`. Three RED tests pin this, from cheapest to strongest:

- **RED-2a (structural / grep-provable):** the gossip / `gossip_payload` / anti-entropy-union code path
  references only `EventStore` rows, **never** `ExposureLedger` / `PeerExposure` / `default_count` /
  `effective_cap`. A test greps the gossip module for those symbols and asserts **zero** matches — the
  count has no wire representation to leak.
- **RED-2b (first-party fold predicate):** seed a log with a `SettlementRefunded` between two *other*
  nodes B and X (this node a party to neither); fold; assert `per_peer[X].default_count == 0`. Proves a
  node cannot accrue bilateral memory from *other* parties' settlements even when their events sit in
  its unioned log.
- **RED-2c (behavioral / no transfer):** node A drives `default_count[X] = N` via N of its own
  timeout-refunds with X; run a full Sync·Pull anti-entropy round A→C; fold on C; assert C's
  `per_peer[X].default_count == 0`. The penalty never crosses the wire; only the underlying
  `SettlementRefunded` events (A's own settlements) may replicate, and by RED-2b they raise *no*
  bilateral count on C.

**Acceptance (falsifiable), extending §4:** effective cap monotonic-down in `default_count`
(`effective_cap ≤ cap` for all inputs, proven at `default_count = 0` and `= u64::MAX`); a peer that
crosses into repeat-default has its next `try_commit` refused at a *lower* threshold than a
clean-history peer with the same `cap`; after `≥ log2(default_count) × PENALTY_HALFLIFE_TICKS` of quiet
the penalty reaches zero and full `cap` is restored with no operator action (the decay falsifier); and
RED-2a/b/c above are all green.

### H-3 — COUNSEL §2 / §8-2: the information-goods data hook on `Commitment`

**Decision: ADD the field — as a dormant, inert hook, not a live policy.** COUNSEL §2 names the
single largest un-contained surface: a buyer receives delivered work *bytes*, then aborts, keeping the
work free, and **this loss never enters the exposure ledger at all** — the ledger sees only the nominal
*budget* committed (`amount`), which fully refunds on timeout as if nothing were lost. Solving the
information-goods problem is B2's delivery-timing mechanism (incremental/claimable delivery, COUNSEL
§8-2); B3's narrow job is to make the *ledger the right shape* to eventually consume a "value delivered
so far" signal instead of being blind to it. That is a one-`u64` hook, and carrying a dormant field for
a future policy is idiomatic here (§5's sealed-batch preconditions already "stay dormant law").

```
pub struct Commitment {
    pub commitment_id: [u8; 32],
    pub peer: PeerId,
    pub class: CapabilityClass,
    pub amount: u64,               // committed budget (nominal) — unchanged
    pub delivered_value: u64,      // NEW, DORMANT (default 0): cumulative value of work IRREVERSIBLY
                                   // delivered so far. Monotonic non-decreasing, clamped ≤ amount.
                                   // READ from B2's settlement TLV (like expiry_tick, §2.1) — B3 does
                                   // NOT define the wire encoding; absent field ⇒ 0.
    pub opened_tick: u64,
    pub expiry_tick: u64,
}
```

**Hook semantics — what B3 owns, and what it deliberately does not:**
- **Store + fold + expose.** `fold_exposure` carries `delivered_value` on the open `Commitment` and
  updates it when B2's (future) incremental-delivery event declares progress, enforcing **monotonic
  non-decreasing, clamped at `amount`**: `c.delivered_value = new.max(c.delivered_value).min(c.amount)`
  — you cannot un-deliver bytes, and delivered value cannot exceed the committed budget (same
  saturating-bound discipline as `TokenBucket::release`). It appears in the open-commitments projection
  (`ledger.per_peer[X].open`) so B2's sweep and any future policy can read it.
- **Consumed by NOTHING today — provably inert.** `try_commit`'s refusal arithmetic references
  `amount`/`cap`/`effective_cap`/the stranger caps, and **never** `delivered_value`. Grep-provable:
  the field changes *no* admission decision at default (0). This respects "this blueprint does not
  fully implement the policy" — the hook is a no-op until B2 populates it.
- **Where the future policy plugs in (designed, not built).** Two consumers become possible once B2
  reports progress: (i) a future *exposure-weighting* policy could, on `SettlementRefunded`, free only
  `amount − delivered_value` and treat `delivered_value` as realized first-party loss (rather than
  freeing the whole budget as if nothing were delivered); and (ii) H-2's default penalty could be
  *weighted by* `delivered_value` — a peer that aborts after high delivered value is a worse defaulter
  than one who aborts before delivery — replacing the flat `+1`. Both are **future work behind a named
  trigger (B2 ships incremental delivery); H-3 only guarantees the ledger is no longer *shaped* blind
  to the signal.** The "when is delivered value realized vs written off" question is B2's
  delivery-mechanism design (COUNSEL §8-2's claimable increments) and is cross-referenced, not answered
  here.

**Acceptance (falsifiable), extending §4:** `delivered_value` defaults to `0` and a commitment with no
B2 progress report is byte-identical in effect to today's `Commitment`; a fold that attempts to
*decrease* `delivered_value` or push it past `amount` is rejected/clamped (monotone + bounded
falsifier); and a grep proves no `try_commit` / regime-threshold path branches on `delivered_value`
(the dormancy falsifier).

### Migration note (extends §3)

H-1 and H-2 are pure fold + `try_commit` extensions on the §2.1 types and the §2.4 reducer — landable
with the Phase-B ledger half (they consume the same B2 settlement events). `PeerEnrolled` is a new
operator-authored witness row folded like §2.5's breach row (Wave-0 discriminant allocation applies —
see Definition of Done). H-3's `delivered_value` is additive and dormant, landable immediately (default
0, no consumer) and populated later when B2's incremental-delivery TLV freezes. Every new const
(`stranger_cap`, `stranger_pool_cap`, `DEFAULT_PENALTY_UNIT`, `PENALTY_HALFLIFE_TICKS`) carries an
H2-style pin test, and the stranger/enrolled sub-cap partition carries the invariant pin noted in the
H-1 code block. Per the §"Long-Term Consequences (b)" ownership split: `stranger_cap`/`stranger_pool_cap`
are **local operator magnitudes** (risk appetite, per-node), while `PENALTY_HALFLIFE_TICKS` and the
penalty shape are **protocol-defaults with a pinned floor** (a node may tighten — forgive slower,
penalize harder — never loosen the dampener into uselessness).
