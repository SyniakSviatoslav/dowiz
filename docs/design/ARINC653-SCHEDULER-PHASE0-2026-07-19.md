# ARINC-653-style two-level scheduler — Phase 0 design doc (design-only, no code)

- **Item:** 11 (roadmap §E item 11) · **Phase:** 0 (design doc + TLC model only)
- **Date:** 2026-07-19 · **Tier:** 4 · **Status:** BLUEPRINT-derived design artifact — **no scheduler code**
- **Design-only ruling:** roadmap §0 gate + §E item 11 — the TLC model and the falsifiable
  slice-guarantee statement can be produced now; **no `kernel/src/` code until item 9's breaker
  exists**. This document creates **zero** Rust code. `git diff` against the parent of this work
  touches only `docs/`.
- **Ground-truth sources read this session:**
  - `kernel/src/token_bucket.rs` — the GCRA / cell-rate primitive (temporal slices map here)
  - `kernel/src/decision/import.rs` — the six ordered structural-gate checks (partition admission maps here)
  - `kernel/src/order_machine.rs` — the `fsm_graph_report()` golden-signature / drift-gate precedent
    for any falsifiable model (this doc + `.tla` follow that discipline)
- **Companion artifact:** `docs/formal/PartitionSchedule.tla` (+ `PartitionSchedule.cfg`) — the
  TLC-checkable temporal model whose invariants *are* the falsifiable slice-guarantee.

---

## 1. Scope / goal (one paragraph)

Phase 0 is a from-scratch, zero-dep, Rust-native **two-level partitioning scheduler** design — the
one item in the synthesis where the kernel would be *creating* precedent, not following it (synthesis
§3.1: "no ARINC-653-compliant Rust RTOS exists anywhere"). This is **not code**: it binds ARINC-653's
two concepts onto primitives the kernel already owns — **temporal partitioning** (a fixed cyclic
*major frame* dividing guaranteed slices among partitions) onto the token bucket's proven refill law
(`token_bucket.rs`), and **partition admission** onto the `decision/import.rs` §1.5 structural-gate
pattern. It honestly scopes **spatial partitioning** (MMU-enforced memory isolation) as the hard
part requiring OS/bare-metal work, with a nearer-term process-per-partition approximation. The
load-bearing output is a **falsifiable slice-guarantee statement**, made checkable by a TLC temporal
model (`PartitionSchedule.tla`) whose invariants a buggy/over-admitted schedule must violate.

---

## 2. Verified current state (grounded — read this session)

- **The GCRA/cell-rate primitive exists.** `token_bucket.rs`:
  - `pub struct TokenBucket` (`:45`) with `capacity: f64` and a **private, immutable**
    `refill_rate: f64` (`:47`) that has **no setter**. The proven refill law is
    `tokens = (tokens + refill_rate * elapsed).min(capacity)` (`refill_locked`, `:81`), with the
    module-doc invariant (`:4`): *total granted over `elapsed` seconds NEVER exceeds
    `capacity + refill_rate * elapsed`*. `try_acquire` (`:101`) is **degrade-closed**: on
    insufficient tokens it returns `false` (never a partial grant, never a silent downgrade).
  - The over-grant ceiling is a *tested falsifier*: `token_bucket_never_over_grants_under_refill`
    (`:266`) asserts `granted <= capacity + refill_rate*elapsed + 1e-6`.
  - **Phase-0 constraint on Phase-1 code:** because `refill_rate` is private-immutable with no
    setter, a scheduler that needs to *reconfigure* slice budgets at runtime requires a *bounded*
    rate-API on `TokenBucket` — that is **item 21's territory** (cross-referenced, NOT duplicated
    here). Phase 0 records this; it does not build it.
- **The partition-admission structural-gate pattern exists.** `decision/import.rs` performs **six
  ordered checks** (`:8–16`, `import_unit` `:81`): (1) size check, (2) integrity (`sha3`), (3)
  instance-set pin, (4) independent replay, (5) epoch check, (6) lineage parent. On any reject,
  **nothing is persisted** (degrade-closed, `:78`). Synthesis §3.1: "partition admission is a
  §1.5-style structural gate." Phase 0 models partition admission as the *same ordered-check-pipeline
  shape* — a partition manifest is admitted only after an ordered set of structural checks, never a
  per-call boolean.
- **The F´-pattern heartbeat + native fuel-meter primitives are NOT yet built.** Grep confirms no
  `heartbeat`/kernel fuel-meter exists in `kernel/src/` today (the `FuelMeter` that exists is in
  `agent-adapters` for wasmtime — a different crate). Phase 0 names these as **Phase-1 prerequisites**,
  does not build them.
- **No scheduler exists.** No `kernel/src/scheduler/`, no partition/slice types. Green field.
- **Grounded keyword precedent:** `order_machine.rs` `fsm_graph_report()` (`:492`+) is the
  falsifiable-model precedent — it captures a **golden structural signature** and cross-validates it
  so silent drift is caught. This Phase-0 model adopts the same discipline: the TLA+ invariants are
  the golden signature of the schedule, and the broken variant (§3.2) is the drift detector.

---

## 3. The two-level structure, mapped onto owned primitives

ARINC-653 partitions time into a fixed cyclic **major frame** of duration `T`, subdivided into
**minor frames / slices**, one guaranteed slice per partition. Two scheduling levels:

- **Level 1 — temporal partitioning (the kernel's job, global).** The major frame is a fixed cyclic
  schedule. In each major frame, partition `P_i` is guaranteed a contiguous slice of length `s_i`.
  **Budget authority = the token bucket.** Each partition's slice is modeled as a `TokenBucket`
  whose `capacity == s_i` and whose `refill_rate` is zero *within* a frame — i.e. the slice is a
  one-shot positive-refill-budget that drains to zero across the partition's execution. The token
  bucket's proven refill law (`(tokens + refill_rate*elapsed).min(capacity)`) is the *correctness*
  primitive guaranteeing a partition cannot be granted execution time it was not allocated: when the
  bucket is empty, `try_acquire` returns `false` and the partition is preempted. (Phase 1 maps this
  to the real `TokenBucket`; the TLA+ model abstracts the arithmetic to the `Σ s_i ≤ T` framing.)
- **Level 2 — priority-preemptive scheduling *within* a slice (the partition's job, local).** While
  `P_i` holds its slice, an ordinary fixed-priority scheduler runs `P_i`'s internal threads. This is
  out of the kernel's temporal-partitioning guarantee; the kernel only guarantees the *slice window*,
  not what the partition does inside it.

```
        ┌──────────────────── MAJOR FRAME (T) ────────────────────┐
        │  [ slice s_0 ] [ slice s_1 ] ... [ slice s_n ] [slack]  │
        │   P_0 runs     P_1 runs         P_n runs    (unused)     │
        └──────────────────────────────────────────────────────────┘
           ↑ each slice guarded by a TokenBucket(capacity=s_i, rate=0)
             try_acquire(N) fails ⇒ preempt P_i back to idle (degrade-closed)
```

---

## 4. The falsifiable slice-guarantee statement (load-bearing)

> **Slice-Guarantee (SG).** In every major frame of length `T`, for every admitted partition `P_i`
> with declared slice `s_i`, `P_i`'s guaranteed slice `s_i` is *available to `P_i`* — i.e. no other
> partition executes during `P_i`'s slice window — **regardless of any other partition's behavior**,
> and the slices fit the frame: `Σ_i s_i ≤ T`, with the remainder as slack.
>
> **Falsifiable formulation (what TLC must exclude):** a reachable bad state is either
> (a) **overrun** — some partition `P_j` executes at a time `t` that lies outside its own slice
> window; or (b) **sum-overflow** — `Σ_i s_i > T` is admitted. SG holds iff neither bad state is
> reachable. The TLA+ model below encodes exactly these as invariants `NoOverrun` and
> `SliceSumFitsFrame`; the deliberately broken variant (§3.2 of the `.tla`, / `PartitionSchedule.cfg`
> `BROKEN` model) admits a partition whose `s_i` exceeds the frame remainder, and **must** violate
> `SliceSumFitsFrame` under TLC — that violation *is* the falsifiability proof.

This is falsifiable (not a vague "the schedule is correct"): a concrete bad state is named, the model
can reach it only if the guarantee is violated, and the broken variant is engineered so TLC *does*
reach it. Same discipline as `order_machine.rs`'s `fsm_graph_report` golden signature.

---

## 5. Partition admission as an ordered structural-gate pipeline (§1.5 shape)

A partition manifest declares `(slice_budget s_i, priority p_i, resource_scope r_i)`. It is admitted
**only after an ordered set of structural checks**, modeled on `decision/import.rs`'s six-check
`import_unit` (degrade-closed: any reject admits nothing). Order is fixed; a later check may assume
earlier ones passed.

| # | Check (name) | Source pattern | Admission rule |
|---|--------------|----------------|----------------|
| 1 | `SliceSumFitsFrame` | import `:8` size check | admitting `P_i` keeps `Σ s_j ≤ T` (frame not oversubscribed) |
| 2 | `ScopeWithinParent` | import `:11` instance-set pin | `r_i ⊆ parent_scope` (no escaping the partition's address/resource envelope) |
| 3 | `PriorityInRange` | import `:12–13` replay | `p_i ∈ [0, MAX_PRIORITY]` (well-formed internal priority) |
| 4 | `SlicePositive` | import `:14` replay-agreement | `s_i > 0` and `s_i ≤ T` (a usable, frame-bounded slice) |
| 5 | `NoEpochDowngrade` | import `:14–15` epoch check | new manifest's epoch > existing admitted epoch for that partition id |
| 6 | `LineageParentExists` | import `:15–16` lineage parent | if `prev_content_id` set, that prior manifest exists in the admit log |

On all-six-pass the manifest is appended to the admit log and the partition becomes **admitted**
(state `Live`, mirroring `import_unit`'s return at `:152`). On any reject, **nothing is persisted**
(degrade-closed, mirroring `:78`). The TLA+ model encodes checks 1, 4, and the sum invariant as the
`AdmitCheck` action precondition; checks 2/3/5/6 are recorded as the same pipeline shape (the model
keeps `priority`/`scope`/`epoch`/`prev` fields and validates them in `AdmitCheck`, consistent with
the `import_unit` ordering).

---

## 6. Slice-exhaustion = the fuel trap (synthesis §3) — wiring stated, code is Phase 1

A partition that consumes its entire slice budget (`try_acquire` exhausts the bucket) is **preempted
deterministically** — the token bucket refuses further grants, exactly the degrade-closed contract at
`token_bucket.rs:101`. Synthesis §3 ("any kernel path that executes less-trusted logic carries an
explicit pre-committed step budget with a deterministic trap on exhaustion") is satisfied at the slice
level by the bucket's `false` return.

**Breaker wiring (Phase 1, gated on item 9):** a partition that *repeatedly* overruns its slice —
i.e. attempts execution after its bucket is empty, frame after frame — is the exact overrun class
`NoOverrun` forbids. Once **item 9's breaker** exists, that repeated-overrun signal trips the breaker
and the partition is evicted. Phase 0 **states this wiring**; it is **Phase-1 code**, out of scope
here. The TLA+ model records the *signal* (`overrunAttempt` counter) but does not itself trip a
breaker — that is the explicit dependency gate (§7).

---

## 7. Spatial partitioning — the honest hard part (NOT claimed, NOT in Phase-1 scope)

MMU-enforced memory isolation between partitions is genuine OS/bare-metal work. Synthesis §3.1: the
nearer-term Rust-native approximation is **process-per-partition with the kernel as supervisor**
(each partition is a separate OS process; the kernel supervises scheduling and IPC). This is **not**
true spatial isolation — a buggy/compromised process can still affect the host OS.

- **Blocked on operator ruling (roadmap §0, synthesis §3.1 / §3 Hermit-OS note):** whether the kernel
  eventually runs as a bare process, a microVM, or bare-metal decides whether true MMU spatial
  isolation is reachable at all, or whether process-per-partition is the ceiling. **This is a
  deployment-architecture decision only the operator can make.** Phase 0 designs the *temporal* half
  fully (deployment-independent) and marks the *spatial* half as **blocked / not invented**.
- **Explicitly not claimed:** this Phase-0 artifact provides **no spatial isolation**. Any statement
  to that effect would be an over-claim and is disclaimed here.

---

## 8. Phase-0 deliverables produced

1. **This design doc** — `docs/design/ARINC653-SCHEDULER-PHASE0-2026-07-19.md`.
2. **The TLC-checkable temporal model** — `docs/formal/PartitionSchedule.tla` + `PartitionSchedule.cfg`.
   Invariants: `SliceSumFitsFrame` (`[] Σ s_i ≤ T`), `SliceGuarantee` (`[]` every admitted partition
   receives its slice each frame), `NoOverrun` (`[]` no partition executes outside its slice),
   `NoStarvation` (`<> ` every admitted partition eventually runs). A deliberately broken variant
   (`BROKEN` model in the `.tla`, selected by `PartitionSchedule.cfg` `BROKEN`) admits a partition
   whose `s_i` exceeds the frame remainder and **must** violate `SliceSumFitsFrame` under TLC.

**No `kernel/src/` file created or edited in Phase 0.** The Cargo build, the hot-path manifest, and
the zero-dep gate are untouched.

---

## 9. Acceptance criteria — Phase 0 only (from blueprint §5)

1. **Design doc exists** with a falsifiable slice-guarantee statement (§4) — a concrete bad state
   (overrun / sum-overflow) is named and the doc states how it is excluded. ✔ (this doc)
2. **TLC-checkable temporal model exists** (`PartitionSchedule.tla`) with the four invariants. ✔
3. **A deliberately broken model variant fails TLC** (over-admitted partition violates
   `SliceSumFitsFrame`) — recorded in the `.tla` + `.cfg`. ✔ (broken model variant provided)
4. **No code landed** — `git diff` touches only `docs/`. ✔ (no `kernel/src/` edits)
5. **Breaker dependency stated explicitly** — item 9 named as the Phase-1 gate; overrun→trip wiring
   is Phase-1 work (§6). ✔

---

## 10. Dependency gates (from blueprint §6)

- **Phase 0 (this doc):** design + model only; **no dependency**; can start now.
- **Phase 1 (scheduler code — OUT OF SCOPE):** gated strictly **after item 9** (the breaker overrun-
  trip wiring), and requires the F´-heartbeat + native fuel-meter primitives (synthesis §3) built
  first, plus a bounded rate-reconfiguration API on `TokenBucket` (overlaps item 21, since
  `refill_rate` is private-immutable with no setter — see §2). Phase 0 records these as the Phase-1
  gate.
- **Operator-gated (roadmap §0):** the whole scheduler arc is "PURSUE, design-only." Phase-1 code
  requires a fresh operator go, not implied by this Phase-0 blueprint.

---

## 11. Open questions (operator ruling — from blueprint §7)

1. **Deployment shape decides spatial partitioning** (synthesis §3.1, §3 Hermit-OS note). Operator
   decision only. Flagged in §7; not invented.
2. **Is Phase 1 worth the arc?** Synthesis §3.1 is honest that this is "a large arc, operator-gated."
   Phase 0 is cheap and creates precedent-value on its own (a falsifiable Rust-native ARINC-653 model
   is publishable/reference-grade). Whether to fund Phase-1 code is a separate operator call after
   Phase 0 lands. Named, not pre-decided.

---

### Blueprint ambiguities resolved (reported to parent)

- **"slice maps to token bucket"** — resolved concretely: each slice is a `TokenBucket` with
  `capacity = s_i`, `refill_rate = 0` *within* a frame (a one-shot positive-refill budget), guarded by
  the proven `(tokens + rate*elapsed).min(capacity)` law and the degrade-closed `try_acquire` refusal.
  The `refill_rate` private-immutable/no-setter fact is surfaced as a Phase-1 constraint (item 21),
  not silently assumed away.
- **"admission gates to §1.5 pattern"** — resolved as a six-check ordered pipeline isomorphic to
  `import_unit`'s six checks, with degrade-closed reject (nothing persisted) and a `Live` admit state.
- **"falsifiable"** — resolved by naming the exact reachable bad states (overrun, sum-overflow) and
  providing a broken model variant engineered to violate `SliceSumFitsFrame`, mirroring the
  `order_machine.rs` golden-signature discipline.
- **Spatial partitioning** — resolved honestly as blocked-on-operator, not claimed, not in Phase-1
  scope; process-per-partition is named as the only near-term approximation.

*This artifact is DESIGN-ONLY. No scheduler code was written, compiled, or committed.*
