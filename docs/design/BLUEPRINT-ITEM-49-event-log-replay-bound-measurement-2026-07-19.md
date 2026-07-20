# BLUEPRINT — Item 49: event-log replay-bound measurement + Hybrid/LSM park

- **Date:** 2026-07-19 · **Tier:** 1-class (measurement + park) · **Status:** BLUEPRINT (planning
  artifact, no code) · **Arc:** §I "Whole-System Determinism & AI-Optional Arc".
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §I item 49
  (lines 725–739), item 2 (composition-root wiring gap — the hard gate), item 26 (measurement-only
  discipline); `docs/audits/hardening/CHECKLIST.md`. Code ground truth: `kernel/src/event_log.rs`,
  `kernel/src/fdr/ring.rs`, `kernel/src/hub_supervisor.rs`.
- **Dependency status:** **STRICTLY GATED on item 2's wiring-gap fix.** Today no production
  composition root constructs the durable store, so replay of it is an *unreachable path* —
  measuring it would optimize code nothing runs (roadmap lines 725, 731). Item 49 does not start
  until item 2 lands.

---

## 1. Problem + non-goals

### Problem
The raw-prompt dialogue recommended a **Hybrid** durability design (WAL + periodic snapshot) to
bound recovery/replay time. Item 49 dispositions that recommendation **per surface** — because the
two log surfaces have opposite replay profiles — and, for the surface that genuinely has unbounded
replay, **measures before optimizing** (item-26 discipline). No snapshot/LSM code is landed.

### Non-goals (explicit — roadmap lines 727–733)
- **NOT** a snapshot/LSM implementation. Item 49 lands **zero** snapshot code (scope law, item-26
  precedent). The design is *recorded and parked* behind a measured trigger.
- **NOT** a change to the FDR ring — for it, Hybrid is **permanently rejected** (§2.1).
- **NOT** measuring the durable `EventLog` until it is actually wired (item 2) — measuring an unwired
  store optimizes an unreachable path.

## 2. Current-state grounding (verified this session)

### 2.1 FDR ring — Hybrid PERMANENTLY REJECTED (bounded by construction)
`fdr/ring.rs:33`: `DEFAULT_SEG_CAP = 1 << 20` (1 MiB), two alternating segments A/B (`ring.rs:35–36`,
`switch()` at `:141`). Recovery reads back **at most 2×1 MiB** (`recover` at `:230` reads both
segments). Replay is bounded by construction — last-N-seconds retention. A snapshot buys nothing
here; **rejected permanently.** Record this rationale in `fdr/ring.rs`'s module doc when next touched
(roadmap line 739).

### 2.2 Durable `EventLog` — genuinely unbounded hash-chain replay (the parked surface)
`kernel/src/event_log.rs` is a content-addressed hash chain. `verify_chain` walks the WHOLE chain from
tip to genesis, recomputing each event-id (`event_log.rs:481–510`; `max_hops = self.store.len()` at
`:487`). Startup integrity/replay is therefore **O(N)** in event count — genuinely unbounded as the
log grows. This is the surface Hybrid is *parked behind measurement* for.

### 2.3 `hub_supervisor`'s `StateSnapshot` is NOT a replay-speedup (roadmap line 730)
`hub_supervisor.rs:334` `StateSnapshot` holds "the event-log chain-tip content-id at snapshot time"
(`:326`); `snapshot()` returns the tip as an epoch anchor (`:392–394`) and `restore()` re-points the
tip to that epoch (`:402–405`). It is an **update-rollback epoch pointer**, not a replay
checkpoint — restoring it does not skip replay, it changes which epoch is current. So the durable
`EventLog`'s replay is genuinely un-accelerated today; the snapshot machinery that exists solves a
different problem (update rollback, `hub_supervisor.rs:4`).

### 2.4 The item-2 wiring gap (the hard gate)
The durable store is not constructed by any production composition root today:
- `event_log.rs:14`: "The real `PgEventStore` (backed by pgrust) is wired in the node binary, NOT
  here." The default `MemEventStore` (`:213`) is non-durable, single-process.
- The only std-durable variant is `hydra::FileEventStore` (`event_log.rs:186`).
- `fdr/ring.rs:289` (`emit_post_mortem` doc): routing the post-mortem "into the durable `EventLog` is
  DEFERRED behind item 2's composition-root fix."

So there is no live, durable, growing `EventLog` to measure. **Item 2 must wire one first.**

### 2.5 The measurement-only discipline (item 26)
Item 26's precedent: measure real numbers, land no code. Item 49 inherits it — the deliverable is a
dated measurement doc + a recorded budget/trigger, with **zero** snapshot/LSM code in the diff.

## 3. Options considered (≥2)

**Option A — measure-then-park (RECOMMENDED, the roadmap design).**
Once item 2 wires a durable store, measure startup replay time vs event count at
N ∈ {1e3, 1e4, 1e5}; state a replay budget; park the Hybrid/snapshot design behind the named trigger
(measured replay > budget at realistic volume).
- Concept: *measurement-first optimization* (item-26 discipline; anti "premature optimization").
- Tradeoff: no speedup lands now — correct, because there is no evidence one is needed and the store
  isn't even wired. Cheap, honest, reversible.

**Option B — build the Hybrid (WAL + periodic snapshot) durable EventLog now.**
- Concept: *bounded-replay durable log up front*.
- Tradeoff: **rejected for now** — optimizes an unwired, unmeasured path; adds a snapshot fsync
  ordering hazard (§6) and real complexity to the substrate everything replays from, with zero
  evidence the O(N) replay is a problem at realistic volume. Reserve for the measured trigger.

## 4. Decision + rationale (ADR-format)

**ADR-049: per-surface disposition — FDR ring Hybrid REJECTED permanently; durable `EventLog`
Hybrid PARKED behind measurement (after item 2).**

Rationale: the FDR ring is bounded by construction (2×1 MiB), so a snapshot is pure over-design there.
The durable `EventLog` has genuine O(N) replay, but it is not even wired into a production composition
root (item 2), and there is no measurement showing the replay is a problem — so building a snapshot
now would optimize an unreachable, unmeasured path (exactly the anti-pattern item 26 exists to
prevent). The proportionate move: wait for item 2, measure, set a budget, and park the design behind
a trigger that fires only on measured evidence. `hub_supervisor`'s `StateSnapshot` is not a substitute
(it is a rollback pointer, not a replay checkpoint), so this is a real, un-accelerated replay path
worth *measuring* — later.

## 5. Implementation plan (numbered — measurement-only, after item 2)

1. **GATE:** confirm item 2's wiring-gap fix landed — a production composition root that constructs a
   durable store (`FileEventStore`/`PgEventStore`) and runs startup replay. Until then, **item 49
   does not start.**
2. **Measurement harness** (bench/test-only, item-26 discipline, ZERO snapshot code): build a durable
   `EventLog` populated with N ∈ {1e3, 1e4, 1e5} events; measure startup replay time (the
   `verify_chain` walk + any `fold`), recording µs at each N with methodology stated (host, build
   profile, event shape, cold/warm cache).
3. **State a replay budget:** e.g. "startup replay completes within X ms at realistic volume Y" — the
   value is operator-set (§10), informed by the measurement.
4. **Park the Hybrid/snapshot design** with its named reopening trigger: *measured replay exceeding
   budget at realistic event volume*. Record the design sketch + trigger in the measurement doc and
   the relevant module doc.
5. **Record the FDR permanent-rejection rationale** in `fdr/ring.rs`'s module doc when next touched
   (§2.1).
6. **Carried-forward correctness note (if ever built):** data-file fsync **strictly BEFORE** pointer
   swap — the dialogue's caveat, consistent with `ring.rs`'s kill-9-vs-power-loss separation
   (`ring.rs:13–22`: `write` reaches page cache for kill-9; `sync_data` before claim for power-loss).
   Recorded so a future implementer inherits the ordering constraint, not built now.

## 6. Failure + degradation (failure-first, for the parked design)

- The parked Hybrid's dangerous pole is a torn snapshot: a pointer swapped to a not-yet-fsynced data
  file → power-loss recovery to a partial snapshot. §5.6's fsync-before-swap ordering is the
  mitigation, recorded now so it is not rediscovered later.
- Today (measure-only), the degradation story is unchanged: durable replay is O(N) and correct; the
  measurement just tells us *when* that becomes a budget problem.

## 7. Required tests / proofs (per CHECKLIST.md 5-point standard)

Item 49 lands **no algorithm** — the 5-point algorithmic-oracle checklist does not directly apply.
The honest mapping:

1. **Oracle:** N/A — no new algorithm/hot path. The deliverable is a *reproducible measurement*, not
   a gate-able oracle. Record `N/A(measurement-only, item-26)`.
2. **dudect gate:** N/A.
3. **Debug cross-check:** N/A.
4. **Assembly spot-check:** N/A.
5. **Scope-law compliance (the real check):** a diff review confirms **zero snapshot/LSM code
   landed** — the item-26 no-code-landed law is the falsifiable property here.

**Falsifiable acceptance criteria (roadmap 736–739):**
- A **dated measurement doc** with replay µs at N ∈ {1e3, 1e4, 1e5} events and methodology stated.
- The **budget + reopening trigger** recorded.
- **Zero snapshot/LSM code landed** (scope law, item-26 precedent) — verifiable by diff.
- The FDR permanent-rejection rationale recorded in `fdr/ring.rs`'s module doc.

## 8. Security + tenant isolation

No tenant/PII/money surface. The durable `EventLog` is content-addressed and hash-chain-verified
(`verify_chain` is the integrity walk); the measurement does not touch that property. If the parked
snapshot is ever built, the snapshot file inherits the same local-first, non-gossiped discipline as
the FDR ring and `hub_supervisor`'s `StateSnapshot` ("PLAINTEXT, on the vendor's own box, NEVER
[transmitted]", `hub_supervisor.rs:330`).

## 9. Operability

- **Health:** replay time is a startup-latency signal; the budget is the threshold that (if breached
  at real volume) fires the reopening trigger.
- **Observability (<1 min):** the measurement doc is the artifact; once wired, startup replay µs can
  ride an FDR `Event` record so live replay time is observable, not just benched.
- **Rollback:** nothing to roll back — measurement + doc only.
- **Scaling gate:** the recorded budget/trigger IS the scaling gate for the parked snapshot work.

## 10. Open / accepted risks + operator-decision points

- **[HARD GATE] Item 2's wiring-gap fix.** No production composition root constructs the durable store
  today (`event_log.rs:14`; `fdr/ring.rs:289`). Item 49 cannot measure a store nothing constructs;
  it starts only after item 2. *Owner: item-2 lead → item-49 executor.*
- **[OPERATOR-DECISION] The replay budget value.** "X ms at volume Y" is a product/SLA decision
  informed by the measurement — not invented here. *Owner: operator.*
- **[OPERATOR-DECISION] "Realistic event volume."** Depends on deployment (local-first single-node vs.
  mesh hub aggregating many actors); the measurement points (1e3/1e4/1e5) bracket it but the
  *realistic* figure is a deployment input. *Owner: operator + deployment layer.*
- **[ACCEPTED] FDR ring Hybrid permanently rejected.** Bounded by construction (2×1 MiB); recorded,
  not revisited. *Owner: item-49 executor.*
- **[RECORDED, not built] fsync-before-pointer-swap.** The correctness constraint for the parked
  snapshot is captured now so a future implementer inherits it. *Owner: future snapshot-ticket
  implementer.*
