# BLUEPRINT — Item 3: `order_machine` const-adjacency zero-heap-allocation proof

- **Date:** 2026-07-20 · **Tier:** space-grade roadmap §A (Tier 0) · **Status:** BLUEPRINT v1
  (planning artifact, no code changed by this pass). Closes the one real gap found by
  `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md`'s successor sweep: item 3 was cited only as a
  byproduct inside `BLUEPRINT-ITEM-06`/`BLUEPRINT-ITEM-07`'s own text (a Kani/hardening-checklist
  cross-reference), never given its own scope/design/acceptance-criteria document.
- **Sources read this session:** `docs/design/ROADMAP.md` Part V §A ("Item 3 — `order_machine`
  const-adjacency + `idx_of` dedup. Golden signature and 1e-12 oracle already cover it.") and §G.5
  ("Proof: zero heap allocations under a counting allocator test; one `idx_of` definition; golden
  signature and 1e-12 oracle both green."); `kernel/src/order_machine.rs` (live, this session);
  `kernel/src/arena.rs` `counting_alloc` module (the existing counting-global-allocator primitive,
  §8.2 of the W5 arena blueprint); `kernel/src/inference/workspace.rs::allocations_during_inference`
  (the item-38 precedent for wiring `counting_alloc` around a measured region — this blueprint
  reuses the exact same pattern, not a new one).

---

## 1. Scope / goal

Item 3's own stated proof obligation (roadmap §G.5) has three clauses. Two are **already
satisfied by live code**, verified this session by direct read — not re-derived, not assumed:

1. **"one `idx_of` definition"** — ✅ already true. `kernel/src/order_machine.rs:264` has exactly
   one `const fn idx_of(s: OrderStatus) -> usize`, and it is the single authority every adjacency
   table, reachability BFS, and topological-order computation in the file goes through (`FSM_ADJ`
   construction at `order_machine.rs:150-155`, the DFS at `:593`, the parent-array init at `:631`,
   and 8 more call sites — `grep -n idx_of kernel/src/order_machine.rs` shows zero duplicate
   definitions, only call sites).
2. **"golden signature and 1e-12 oracle both green"** — ✅ already true. `FSM_GOLDEN_SIGNATURE`
   (`order_machine.rs:512-522`) is a hand-pinned `FsmGraphReport` checked by
   `verify_fsm_signature[_against]` (`:539-549` and tests from `:671` onward); the 1e-12 spectral
   oracle comparison lives at `:980-1022` (`const TOL: f64 = 1e-12`, an iterative power-method
   oracle compared byte-for-byte against the compile-time `FSM_SPECTRAL_RADIUS`). Both are existing
   `#[test]`s in the default (no-feature) `cargo test` run — confirmed still passing as part of the
   kernel's default suite (`ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md`'s fresh count: kernel default
   1137 passed / 8 ignored, zero failures, and neither of these tests is in the ignored set).

3. **"zero heap allocations under a counting allocator test"** — ❌ **NOT satisfied. This is the
   real, still-open piece of item 3**, and the sole thing this blueprint specifies. No test in
   `kernel/src/order_machine.rs` references `count-allocs` or `crate::arena::counting_alloc`
   (`grep -n "count-allocs\|counting_alloc" kernel/src/order_machine.rs` — zero hits, confirmed
   this session). The claim in Part V §A ("already cover it") is accurate for clauses 1-2 and
   **stale/incomplete** for clause 3 — this blueprint's job is to close that specific gap.

**Non-goals:** no change to `FSM_ADJ`'s construction, no new adjacency representation, no touching
`FSM_GOLDEN_SIGNATURE`'s pinned values (a lifecycle change is explicitly out of scope — see the
pin's own header comment, `order_machine.rs:505-511`, "upgrade trigger = a deliberate lifecycle
change... NOT hand-edited to force the gate green"). This is a **measurement addition only**.

## 2. Why this matters (grounding, not narrative)

`FSM_ADJ` and the reachability/topological machinery built on it are `const fn` — evaluated at
**compile time**, baked into the binary as `static` data (`order_machine.rs:150-155`,
`:190-199`). The whole point of Item 3 (space-grade roadmap's "static-data-layout-first" principle,
cited at `ROADMAP.md:4290` — "item 3's const-adjacency is the named fix shape; separate-core stays
rejected") is that this graph costs **zero runtime heap allocation**: it is computed once by
`rustc`, not rebuilt per order-decision call. That property is currently **asserted by code shape**
(the `const fn` keyword) but never **measured** — a `const fn` body can still be *evaluated* again
at runtime in a non-const calling context (Rust does not guarantee const-folding outside a `const`
binding position), so the zero-allocation property genuinely needs the same falsifiable test
discipline this codebase applies everywhere else ("verified, not claimed" — `CLAUDE.md`). Item 38
(`kernel/src/inference/workspace.rs`) already proved this exact class of claim for the tensor
workspace using `counting_alloc`; item 3 is the FSM's turn.

## 3. Design

### 3.1 What gets measured

The **hot decision path**, not just construction: `decide()`/`fold()` (the FSM entry points
`CLAUDE.md` names as load-bearing) call `assert_transition`, which reads `FSM_ADJ[idx_of(from)]`
(`order_machine.rs:155`). The measured region must cover a **realistic sequence of transitions**
through the live lifecycle graph, not a single call — a single call could accidentally look
allocation-free while a multi-step replay (the real workload shape, `decide → Event`, `state =
fold(events)` per `CLAUDE.md`) still allocates somewhere in the loop.

### 3.2 New code — `kernel/src/order_machine.rs`, gated exactly like item 38

Mirror `inference/workspace.rs::allocations_during_inference` byte-for-byte in structure (same
snapshot/consume-init-alloc/re-baseline/measure sequence — that function is the proven pattern,
reused not reinvented):

```rust
/// Item 3 — measure heap allocations during a realistic FSM transition replay.
/// `count-allocs`-gated (see `crate::arena::counting_alloc`); a no-op type outside that
/// feature. Snapshots AFTER any one-time setup so only the transition-replay loop itself
/// is measured (same discipline as `inference::workspace::allocations_during_inference`).
#[cfg(all(feature = "count-allocs", not(target_arch = "wasm32")))]
pub fn allocations_during_fsm_replay<F>(f: F) -> usize
where
    F: FnOnce(),
{
    use crate::arena::counting_alloc;
    counting_alloc::snapshot();
    f();
    counting_alloc::since_snapshot()
}
```

### 3.3 New test — `kernel/src/order_machine.rs` `#[cfg(test)] mod tests`

```rust
#[cfg(all(feature = "count-allocs", not(target_arch = "wasm32")))]
#[test]
fn item3_fsm_replay_allocates_zero_heap_bytes() {
    // A realistic multi-step replay through the live lifecycle graph: every legal
    // transition `assert_transition` will accept, back to back, exercising `FSM_ADJ`
    // lookups + `idx_of` on both ends of each edge — NOT just one call.
    let path = [
        (OrderStatus::Pending, OrderStatus::Confirmed),
        (OrderStatus::Confirmed, OrderStatus::Preparing),
        (OrderStatus::Preparing, OrderStatus::Ready),
        (OrderStatus::Ready, OrderStatus::InDelivery),
        (OrderStatus::InDelivery, OrderStatus::Delivered),
    ];
    let allocs = allocations_during_fsm_replay(|| {
        for (from, to) in path {
            assert!(assert_transition(from, to).is_ok());
        }
        // Also exercise the reachability/topological read paths driven by the same
        // const adjacency, so the proof covers item 3's full stated surface, not just
        // `assert_transition`.
        let _ = reachable(OrderStatus::Pending);
        let _ = verify_fsm_signature();
    });
    assert_eq!(
        allocs, 0,
        "order_machine's const-adjacency FSM path must allocate ZERO heap bytes per \
         item 3's proof obligation (roadmap §G.5) — got {allocs} allocation(s)"
    );
}
```

(Exact function names — `assert_transition`, `reachable`, `verify_fsm_signature` — are the live
public API already in `order_machine.rs`; the test above calls only what already exists, no new
public surface beyond `allocations_during_fsm_replay` itself.)

### 3.4 Cargo wiring

No new feature flag needed — `count-allocs` already exists (`kernel/Cargo.toml:58`) and already
installs the global `#[global_allocator]` (`arena.rs:319`, one process-wide allocator, shared by
every `count-allocs`-gated test in the crate — this is why the existing tests snapshot/measure a
*delta*, never an absolute count, and the new test follows the same discipline). Run with:

```sh
cd kernel && cargo test --features count-allocs item3_fsm_replay_allocates_zero_heap_bytes
```

No change to the default (no-feature) build; `count-allocs` stays off-by-default per the feature
discipline `CLAUDE.md` requires.

## 4. Fits the existing architecture

- **Zero new dependencies, zero new primitives.** Reuses `crate::arena::counting_alloc`
  byte-for-byte (same module, same `snapshot`/`since_snapshot` pair item 38 already proved
  correct) — this blueprint adds one gated function + one gated test, nothing else.
- **Respects the pinned golden signature.** The new test calls `assert_transition`/`reachable`/
  `verify_fsm_signature` as a *consumer*, never mutates `FSM_ADJ` or `FSM_GOLDEN_SIGNATURE`.
- **Matches the space-grade "static-data-layout-first" ruling** (`ROADMAP.md:4290`) by finally
  measuring, not just asserting, that the const-adjacency shape delivers on its own promise.

## 5. Acceptance criteria (RED → GREEN, per this repo's standing culture)

1. **RED first, honestly obtained:** before adding `allocations_during_fsm_replay`, write the test
   body against a *deliberately naive* stand-in that allocates (e.g. temporarily route through a
   `Vec`-backed adjacency instead of `FSM_ADJ` in a throwaway branch) to confirm the test
   *can* fail — a zero-allocation assertion that can never go RED proves nothing. Discard the
   throwaway branch once RED is confirmed; ship only the real `FSM_ADJ`-backed GREEN version.
2. **GREEN:** `cd kernel && cargo test --features count-allocs item3_fsm_replay_allocates_zero_heap_bytes`
   passes, asserting exactly `allocs == 0` over the 5-transition replay + reachability + signature
   check.
3. **No regression to the default build:** `cd kernel && cargo test` (no features) and
   `cargo tree -e no-dev` stay unchanged — `count-allocs` is additive and off-by-default.
4. **Roadmap update:** flip item 3's Part V §A line from "Golden signature and 1e-12 oracle already
   cover it" to record the third clause closed, citing this blueprint and the new test name — so a
   future audit does not have to re-derive that clauses 1-2 were always green while clause 3 was
   silently uncovered.
