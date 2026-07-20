# BLUEPRINT — Item 42: Fixed-Sequence Scheduler (the engine's cyclomatic-1 spine)

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — the scheduler is
  the ruling as control flow: **the whole model as one compiled call sequence, cyclomatic complexity
  1, no dynamic graph traversal, no hash-map dispatch.** You know exactly which instructions run, in
  which order, every time (dialogue part-5 §4: "You don't run the model — you execute a program that
  has the shape of a neural network").
- **Sources read this session:** roadmap §H item 42 (lines 587–593); `RAW-PROMPT-4` part-5 §4
  (const function-pointer array / straight-line sequence, cyclomatic complexity 1, no context switch,
  no dynamic graph analysis); `kernel/src/order_machine.rs` cyclomatic lens (`cyclomatic_number`,
  cited item-7 blueprint §2 at `:627`) — the kernel **already measures** cyclomatic complexity and
  pins it to a golden signature; item 14's toolchain-keyed **assembly-audit** format; item 40's
  per-layer golden checksums (identity-across-runs proof surface).
- **Dependency gate:** **after items 38 + 39 + 41** (workspace + kernels + weights — the three things
  the sequence composes).

---

## 1. Scope / goal + non-goals

**Goal.** A straight-line layer sequence — `const` function-pointer array or inlined call chain —
that runs the pilot graph as one compiled program: layer `i` reads/writes item-38's `const`
workspace offsets, calling item-39's kernels with item-41's embedded weights, in **fixed order**,
with **cyclomatic complexity 1 at the dispatch level** (no dynamic dispatch, no hash-map lookup, no
data-driven graph walk). End-to-end output is **bit-identical across repeated runs and across
native/wasm32**, and item-40's per-layer checksums are identical across both.

**Non-goals.** NOT a general graph runtime — the pilot graph is fixed, so the schedule is `const`.
NOT a dynamic interpreter (PyTorch/TF-style dispatch is the jitter source the arc rejects — dialogue
part-5 §4, synthesis Q1). NOT parallel/multi-core (Q2: core migration adds host-scheduler
nondeterminism, the opposite of the ruling). The one data-dependent branch class (ReLU) is item 43's
concern (mask/cmov for the secret plane; ordinary for the public toy pilot).

## 2. Grounding

- **Cyclomatic complexity is already a first-class, gated kernel property.** `order_machine.rs`'s
  `cyclomatic_number` graph lens pins the FSM's complexity to a golden signature (item-7 blueprint
  §2). The scheduler asserts complexity **1** at the dispatch level using the same measurement
  discipline — this is not a novel metric for this codebase.
- **The assembly-audit format exists** (item 14, toolchain-keyed) — the "no dynamic dispatch,
  straight-line" claim is filed there, re-run on compiler bumps.
- **Integer determinism is the whole reason cross-target bit-identity is achievable** — item 35's
  integer domain (no IEEE-754) means native and wasm32 compute the *same* bits (synthesis §2 Q5:
  "the same result independent of where it runs — ThinkPad, server, or embedded controller").

## 3. Implementation plan

1. **Straight-line dispatch.** `fn infer(input) -> output` calls `layer_0, layer_1, …, layer_L` in
   fixed order — either a `const [fn(&mut Workspace); L]` array iterated with a fixed trip count, or
   an inlined call chain. No dynamic function-pointer indirection driven by input data; no hash-map
   op-dispatch.
2. **Each layer reads/writes `const` workspace offsets** (item 38) and calls the item-39 kernel with
   item-41 weights — zero-copy layer-to-layer.
3. **Cyclomatic-1 dispatch.** The only control flow at the dispatch level is the fixed-length
   iteration/chain; every branch inside a kernel is either input-independent (SIMD feature detect,
   resolved once) or the ReLU clamp (item 43). A source-structure test asserts the sequence is
   `const` and the dispatch is branch-free.
4. **Assembly spot-check** of the dispatch path, filed in item 14's toolchain-keyed audit format:
   no data-dependent branch, no dynamic dispatch, straight-line call sequence.
5. **Cross-target end-to-end proof.** Run `infer` over D on native AND wasm32; assert bit-identical
   outputs and (via item 40) identical per-layer checksums.

## 4. Required proofs (5-point checklist mapping)

- **1 (oracle):** end-to-end `infer` vs item 37's oracle over D — **bit-identical**.
- **4 (asm):** assembly spot-check of the dispatch path (item 14 format) — no dynamic dispatch,
  straight-line, cyclomatic-1.
- **5 (kani/native-structural):** the cyclomatic-complexity-1 property, asserted via the
  `order_machine` cyclomatic-lens discipline (a source-structure/native check — no Kani toolchain
  needed for a complexity count, per the item-7 rescope logic).
- **3 (differential):** the cross-target (native vs wasm32) and cross-run bit-identity checks are the
  differential.
- **2 (dudect):** deferred to item 43 (the ReLU branch is the only data-dependent surface; public
  plane ⇒ cheap branch for the toy pilot).

## 5. Falsifiable acceptance criteria

1. A source-structure test asserts the layer sequence is **`const`** and the dispatch is **branch-free
   (cyclomatic complexity 1)**. **RED→GREEN:** introducing a data-driven `match` on op type turns it
   RED.
2. An **assembly spot-check** of the dispatch path is recorded (item 14 format): no data-dependent
   branch, no hash-map/dynamic dispatch, straight-line call sequence.
3. End-to-end `infer` over D is **bit-identical across repeated runs**.
4. End-to-end `infer` over D is **bit-identical across native and wasm32** — integer determinism
   proven cross-target. **RED→GREEN:** any accidental float in a layer breaks this.
5. Per-layer checksums (item 40) are **identical across runs AND across native/wasm32**.

## 6. Dependency gate + operator-decision-needed

- **Gate:** after items 38 + 39 + 41.
- **Operator-decision-needed:** **none** (engineering choice only, FLAGGED for clarity): `const
  [fn; L]` function-pointer array vs a fully-inlined straight-line call chain. **Architect
  recommendation:** the **inlined straight-line chain** for the fixed toy graph — it yields
  cyclomatic-1 most cleanly and inlines best (the fn-pointer array reintroduces an indirect call the
  asm audit then has to reason about). The fn-pointer form is the right shape only if the graph
  becomes data-configurable, which the fixed pilot is not. Not an operator gate.
