# BLUEPRINT — Item 38: Static Tensor Workspace on the Arena

- **Date:** 2026-07-19 · **Arc:** §H · **Status:** BLUEPRINT v1 — planning artifact, NO code.
- **Governing ruling (arc-wide):** *"безпека і передбачуваність понад швидкість"* — the workspace is
  the ruling made physical: **zero mid-inference allocation** (no `malloc` jitter), **const offsets**
  (you know where every byte of every layer lives), **illegal-overlap-unrepresentable** (a bad
  layout fails to construct, not at runtime).
- **Sources read this session:** roadmap §H item 38 (lines 554–560, "one preallocated workspace,
  tensors as fixed offsets computed at build time, zero mid-inference allocation, zero-copy
  layer-to-layer, counting-allocator proof, const offsets, overlapping layout fails to construct");
  `RAW-PROMPT-4` part 5 §1 (the "Zero-Allocation Static Graph / Tensors as Offsets / Zero-Copy"
  shape); `kernel/src/arena.rs:43` (`BumpArena`: fixed-capacity `Vec<u8>`, pointer-bump,
  `:75 alloc_slice` degrade-closed → `None`, `:114 reset`, `:137 HugePageHint`), **`:265-317` the
  `count-allocs` counting global allocator** (`ALLOC_COUNT:276`, `snapshot:300`, `since_snapshot:314`
  — item 3's own proof machinery, arena.rs doc §W5); `kernel/Cargo.toml:55-58` (`count-allocs`
  feature, "installs no custom allocator in normal builds").
- **Dependency gate:** **after item 34** (needs the pilot graph's layer shapes to compute offsets),
  **parallel with items 35–37.** Feeds item 42 (the scheduler runs over this workspace).

---

## 1. Scope / goal + non-goals

**Goal.** One preallocated workspace region; every tensor a **build-time `const` byte offset**
computed from the item-34 pilot graph's layer shapes; **zero heap allocation during inference**;
zero-copy layer-to-layer reads (layer `i+1` reads layer `i`'s output in place). A deliberately-
overlapping layout **fails to construct** — illegal state unrepresentable (§1.5 house standard).

**Non-goals.** NOT a general allocator — the shapes are fixed by the pilot graph, so the layout is
`const`, not dynamic. NOT `slot_arena.rs` (that's generational handles for delete-while-referenced
graphs — a different problem). NOT huge-page tuning (the `HugePageHint` seam stays a no-op until its
own `>2 MB` trigger, `arena.rs:145`). NOT growable — a fixed region; the ruling wants a known ceiling.

## 2. Current-state grounding

- **The arena precedent is real and degrade-closed.** `BumpArena` (`arena.rs:43`) is a fixed-capacity
  `Vec<u8>` with monotone-bump `alloc_slice` that returns `None` on exhaustion (never grows, never
  panics — `arena.rs:26,75`). Disjointness of allocations is proven by the monotone offset; `reset`
  takes `&mut self` so use-after-reset is unrepresentable (`arena.rs:112`). The tensor arena is
  named as its intended growth direction (`arena.rs:3`, synthesis §1.5).
- **The zero-allocation *proof* machinery already exists.** `arena.rs:265-317` (`count-allocs`
  feature) installs a counting global allocator: `ALLOC_COUNT` (`:276`), `snapshot()`/`reset_count()`
  /`since_snapshot()` (`:300-315`). Item 3 (W5) uses it to MEASURE "≤ 8 heap allocations on the arena
  path" rather than assume it. **Item 38 reuses this exact machinery** to prove zero mid-inference
  allocation — no new proof tool.
- **The gap `BumpArena` alone doesn't close:** its offsets are *runtime-bump-determined*, not `const`.
  The roadmap explicitly wants **`const` offsets** and a **construct-time overlap check**. So item 38
  adds a thin `const`-layout layer *on top of* the arena's region-ownership pattern.

## 3. Options (≥2) with tradeoffs

- **Option A — reuse `BumpArena` directly.** Each tensor `alloc_slice`d at inference start in layer
  order. *Pro:* zero new code, degrade-closed already. *Con:* offsets are runtime-bump values, not
  `const`; no construct-time overlap check; layer-to-layer is not zero-copy (each is a fresh slice).
  Fails the roadmap's `const`-offset + overlap-unrepresentable requirement.
- **Option B (RULED shape) — a `TensorWorkspace` with `const` offsets, backed by one fixed region.**
  Build-time `const [(offset, len); T]` table from the pilot graph; the region is one fixed-capacity
  buffer (a `BumpArena`-style `Vec<u8>` allocated ONCE at init); each tensor is `&mut region[off..off+len]`
  at its `const` offset; layer `i+1` reads layer `i`'s output slice in place (zero-copy). A `const`
  overlap check (const-fn / const-assert) rejects any two tensors whose `[off, off+len)` ranges
  collide → **fails to construct**. *Pro:* meets every roadmap requirement; reuses the arena's
  region-ownership + counting-allocator proof. *Con:* one new (thin) type. **Chosen.**

## 4. Implementation plan (Option B)

1. **Build-time layout.** From the pilot graph's layer i/o sizes (item 34), compute a `const`
   `LAYOUT: [(usize /*byte offset*/, usize /*len*/); T]` and a `const WORKSPACE_BYTES`. Aligned per
   item 41's `#[repr(align(64))]` needs (64-byte-aligned tensor starts for aligned SIMD loads).
2. **One region, allocated once.** A fixed-capacity `Vec<u8>` (or the `BumpArena` region) of
   `WORKSPACE_BYTES`, allocated at engine init — **never** during inference.
3. **Tensors as const offsets.** Accessor `fn tensor(&mut self, id: TensorId) -> &mut [i8/i32]`
   returns the slice at `LAYOUT[id]`; layer-to-layer reads alias in place (zero-copy).
4. **Overlap-unrepresentable.** A `const` (or const-fn) assertion over `LAYOUT` verifies pairwise
   non-overlap (modulo any *declared* in-place reuse, §6). A deliberately-overlapping `LAYOUT` fails
   at compile time / const-eval — the illegal state is unrepresentable, not caught at runtime.
5. **Prove zero mid-inference allocation** with item 3's `count-allocs` machinery: snapshot
   `ALLOC_COUNT` before the inference call and after; the delta over the *inference region* is 0 (the
   init region allocation is outside the measured window).

## 5. Required proofs (5-point hardening-checklist mapping)

- **1 (oracle):** N/A directly (the workspace holds no arithmetic) — but the full inference *through*
  this workspace must match item 37's oracle bit-exact (that end-to-end check lands in item 42).
- **3 (differential):** the counting-allocator measurement is the continuous cross-check that the
  hot path stays allocation-free.
- **2 (dudect):** the workspace's memory-access pattern is input-independent (fixed offsets) — a
  property item 43's constant-time reasoning relies on; recorded, not gated here (public plane).
- **4 (asm) / 5 (kani):** N/A for the workspace itself; the const overlap check is const-eval-
  enforced (stronger than a runtime asm audit for *this* property).

## 6. Falsifiable acceptance criteria

1. A `count-allocs` test shows **ZERO heap allocations DURING a full inference** (`since_snapshot()`
   over the inference call == 0; the init region alloc is outside the window). **RED→GREEN:**
   inserting a stray `Vec` in a layer turns it RED.
2. Tensor offsets are **`const`** (compile-time) — asserted structurally (the `LAYOUT` is a `const`
   item; a test reads it in const context).
3. A **deliberately-overlapping layout FAILS to construct** (const-eval panic / compile error) —
   illegal state unrepresentable. **RED→GREEN** with a planted colliding pair.
4. Layer-to-layer is zero-copy: no `memcpy`/`clone` between layers (verifiable by inspection + the
   allocation count).
5. The region is fixed-capacity and never grows (degrade-closed like `BumpArena`; exhaustion is a
   build-time layout error, not a runtime grow).

## 7. Dependency gate + operator-decision-needed

- **Gate:** after item 34; parallel with items 35–37; feeds item 42.
- **Operator-decision-needed — FLAGGED (design fork, not an operator gate):** whether v1 permits
  **deliberate in-place tensor reuse** (a layer overwrites its input buffer with its output to save
  workspace bytes). In-place reuse means *some* offset overlaps are legal-by-design, complicating the
  §4 overlap check (it must whitelist declared reuses). **Architect recommendation:** v1 **disallows
  all overlap** — every tensor gets a distinct offset (simplest, most predictable, overlap-check is a
  clean "no collisions"); defer in-place reuse behind a named trigger (measured `WORKSPACE_BYTES`
  exceeds the region budget). Flagged; the pilot is KB-scale so the workspace budget is not a real
  constraint yet.
