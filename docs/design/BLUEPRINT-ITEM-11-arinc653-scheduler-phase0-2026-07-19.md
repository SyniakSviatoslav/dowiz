# BLUEPRINT — Item 11: ARINC-653-style two-level scheduler, Phase 0 (design doc + TLC model only)

- **Date:** 2026-07-19 · **Tier:** 4 (roadmap §E) · **Status:** BLUEPRINT (planning artifact, no code)
  — **design-only ruling** (roadmap §0 gate, §E item 11): the TLC model and slice-guarantee statement
  can start now; **no scheduler code until item 9's breaker exists** (the source's own restriction).
- **Sources (read this session):** `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §E item 11
  (lines 378–380), §0 gate ("PURSUE, design-only"); `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md`
  §3 (F´/cFS/fuel patterns, lines 99–104), §3.1 (the ARINC-653 differentiator, lines 106–110), §9
  item 11 (line 175); live source `kernel/src/token_bucket.rs`, `kernel/src/decision/import.rs` (§1.5
  gate), `kernel/src/order_machine.rs`.
- **Relationship to item 9:** the model needs no breaker; the eventual *code* does. Phase 0 is
  explicitly pre-breaker. This blueprint scopes Phase 0 only.

---

## 1. Scope / goal (one paragraph)

Produce the Phase-0 design artifact for a from-scratch, zero-dep, Rust-native **two-level
partitioning scheduler** — the one item in the whole synthesis where the kernel would be *creating*
precedent, not following it (synthesis §3.1: "no ARINC-653-compliant Rust RTOS exists anywhere").
Phase 0 is **not code**: it is (a) a design doc binding temporal slices to the kernel's existing GCRA
cell-rate primitive and partition-admission gates to the §1.5 structural-gate pattern, and (b) a
**TLC-checkable temporal model** with a *falsifiable slice-guarantee statement* (roadmap §E item 11
proof: "a doc with a falsifiable slice-guarantee statement and a TLC-checkable temporal model — code
comes only after the breaker exists"). The design maps ARINC-653's two concepts onto primitives the
kernel already owns: **temporal partitioning** (a fixed cyclic major frame dividing guaranteed slices
among partitions) onto the token bucket's proven refill law, and **partition admission** onto the
decision-import gate; it honestly scopes **spatial partitioning** (MMU-enforced memory isolation) as
the hard part requiring OS/bare-metal work, with a nearer-term process-per-partition approximation.

---

## 2. Verified current state — grounded

- **The GCRA/cell-rate primitive the temporal slices map onto exists.** `token_bucket.rs`:
  `TokenBucket` (`:36`) with a proven refill law (module doc `:4`: "`elapsed` seconds NEVER exceeds
  `capacity + refill_rate * elapsed`"), `try_acquire` (`:92`), the over-grant ceiling invariant test
  `token_bucket_never_over_grants_under_refill`. Synthesis §3.1: "temporal slices are token buckets
  with a proven refill law (§1.3's GCRA is literally a cell-rate scheduler)." **Note:** `refill_rate`
  is a private immutable field (`token_bucket.rs:38`) with **no setter** — Phase 0 records this as a
  constraint on Phase-1 code (a scheduler that reconfigures slice budgets needs a *bounded* rate API,
  which is item 21's territory, cross-referenced not duplicated).
- **The partition-admission structural-gate pattern exists.** `decision/import.rs` — the six ordered
  checks (`:8–16`), `import_unit()` (`:81`), degrade-closed reject (`:78`). Synthesis §3.1: "partition
  admission is a §1.5-style structural gate." Phase 0 models partition admission as the same
  ordered-check-pipeline shape (a partition manifest is admitted only after its resource/slice claim
  passes an ordered set of structural checks), never a per-call boolean.
- **The F´-pattern heartbeat and fuel-trap primitives are named, not yet built.** Synthesis §3
  proposes a `heartbeat()` invariant probe per module (line 99) and a native fuel-budget pattern
  (line 104, the Wasmtime-fuel reimplementation) — "any kernel path that executes less-trusted logic
  carries an explicit pre-committed step budget with a deterministic trap on exhaustion." Grep
  confirms **no `heartbeat`/fuel-meter primitive exists in `kernel/src/` today** (the `FuelMeter` that
  exists is in `agent-adapters`, a different crate, for wasmtime — not the kernel's own). Phase 0
  names these as Phase-1 prerequisites, does not build them.
- **No scheduler exists.** No `kernel/src/scheduler/`, no partition/slice types. Green field. This is
  a genuine first (synthesis §3.1, verified: RTEMS+AIR-II and POK are C/C++-only).

---

## 3. Phase-0 deliverables — exact artifacts (design + model, NO code)

1. **`docs/design/ARINC653-SCHEDULER-PHASE0-2026-07-19.md`** (the design doc). Contents:
   - **The two-level structure**, mapped: level 1 = fixed cyclic *major frame* of *minor slices*, one
     guaranteed slice per partition (the token-bucket refill law is the per-slice budget authority);
     level 2 = ordinary priority-preemptive scheduling *within* a slice.
   - **The falsifiable slice-guarantee statement** (the load-bearing artifact): "In every major frame
     of length `T`, partition `P_i`'s guaranteed slice `s_i` is available to `P_i` regardless of any
     other partition's behavior, and `Σ s_i ≤ T` with the remainder as slack" — expressed so a
     TLC model can *violate* it (a partition overrunning its slice, or `Σ s_i > T`, is a reachable
     bad state the model must exclude).
   - **Partition admission** as an ordered-check pipeline (§1.5 shape): a partition manifest declaring
     `(slice_budget, priority, resource_scope)` is admitted only after checks — slice-sum fits the
     frame, scope is within the parent's, priority is in-range — in fixed order, degrade-closed.
   - **Slice-exhaustion = the fuel trap** (synthesis §3): a partition that consumes its slice budget
     is preempted deterministically (the token bucket refuses further grants), and — **once item 9
     exists** — a partition that *repeatedly* overruns trips the breaker. Phase 0 states this wiring;
     it is Phase-1 code.
   - **Spatial partitioning — the honest hard part.** MMU-enforced memory isolation is OS/bare-metal
     work; the nearer-term Rust-native approximation is **process-per-partition with the kernel as
     supervisor** (synthesis §3.1), upgrading to true MMU enforcement later. Phase 0 marks this
     explicitly as NOT-in-scope-for-Phase-1 and NOT claimed as spatial isolation.
2. **`docs/formal/PartitionSchedule.tla` + `.cfg`** (the TLC-checkable temporal model). Models the
   major frame as a cyclic sequence of slices; the `Schedule` action advances the frame; invariants:
   `SliceSumFitsFrame` (`[](Σ s_i ≤ T)`), `SliceGuaranteed` (`[]` every partition receives its slice
   each frame — the temporal statement of the guarantee), `NoOverrun` (`[]` no partition executes
   outside its slice), `NoStarvation` (`<>` every admitted partition eventually runs). A deliberately
   broken variant (a partition with `s_i` exceeding the frame remainder admitted anyway) must violate
   `SliceSumFitsFrame` under TLC — the falsifiability proof.

**No `kernel/src/` file is created or edited in Phase 0.** The Cargo build, the hot-path manifest,
and the zero-dep gate are untouched.

---

## 4. Tests / proofs — 5-point hardening applicability

Phase 0 produces **no implementation**, so the 5-point checklist applies to *Phase 1's future code*,
not to this artifact. Recorded here so Phase 1 inherits it:

- **Item 5 (formal):** Phase 0's TLC model **is** the formal artifact at this stage — its self-test
  obligation is the broken-variant-fails proof (§3.2), identical discipline to item 10. This is what
  Phase 0 can prove *now*.
- **Items 1–4 (oracle/dudect/debug-differential/asm):** apply to Phase-1 scheduler *code* when it
  lands (the slice-budget arithmetic is an algorithmic hot path → oracle + overflow-proof; no secret
  timing → dudect N/A; no branch-free path → asm N/A). Named as Phase-1 obligations, not discharged
  now.

---

## 5. Acceptance criteria (falsifiable) — Phase 0 only

1. **The design doc exists** with a **falsifiable slice-guarantee statement** (§3.1) — falsifiable
   meaning a concrete bad state (overrun / slice-sum-exceeds-frame) is namable and the doc states how
   it is excluded.
2. **A TLC-checkable temporal model exists** (`PartitionSchedule.tla`) and TLC exhausts it with the
   four invariants GREEN.
3. **A deliberately broken model variant fails TLC** (the over-admitted partition violates
   `SliceSumFitsFrame`) — recorded.
4. **No code landed** — `git diff` touches only `docs/`; `cargo tree`/`HOT-PATHS.tsv` unchanged.
5. **The breaker dependency is stated explicitly** — the doc names item 9 as the gate for Phase 1 and
   the overrun→trip wiring as Phase-1 work.

---

## 6. Dependency gates

- **Phase 0 (this blueprint):** design + model only; **no dependency** — can start now (roadmap §E:
  "can start now as a design artifact; the model itself doesn't need the breaker to exist").
- **Phase 1 (scheduler code — OUT OF SCOPE HERE):** gated **strictly after item 9** (the breaker
  overrun-trip wiring), and needs the F´-heartbeat + native fuel-meter primitives (synthesis §3) built
  first, plus a bounded rate-reconfiguration API on `TokenBucket` (overlaps item 21). Phase 0 records
  these as the Phase-1 gate.
- **Operator-gated (roadmap §0):** the whole scheduler arc is "PURSUE, design-only" — Phase 1 code
  requires a fresh operator go, not implied by this Phase-0 blueprint.

---

## 7. Open questions (operator ruling)

1. **Deployment shape decides spatial partitioning (synthesis §3.1, §3 Hermit-OS note).** Whether the
   kernel eventually runs as a bare process, a microVM, or bare-metal determines whether true MMU
   spatial isolation is reachable or the process-per-partition approximation is the ceiling. This is a
   **deployment-architecture decision only the operator can make** — Phase 0 designs the temporal half
   fully (which is deployment-independent) and marks the spatial half as blocked on this ruling. Flagged;
   not invented.
2. **Is Phase 1 worth the arc?** Synthesis §3.1 is honest that this is "a large arc, operator-gated."
   Phase 0 is cheap and creates precedent-value on its own (a falsifiable Rust-native ARINC-653 model
   is publishable/reference-grade); whether to fund Phase-1 code is a separate operator call after
   Phase 0 lands. Named, not pre-decided.
