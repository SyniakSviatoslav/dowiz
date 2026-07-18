# Generational-index arenas (slotmap / generational-arena / thunderdome) — deeper dive: broader need-sweep, real crate comparison, compile-verified hand-rolled `SlotArena`

> Research, 2026-07-18. Follow-up to `OPUS-PERF-POINTER-ARENA-ANALYSIS-2026-07-18.md` on the
> operator directive: *"look at slotmap/generational-arena/thunderdome in MORE detail, and if
> they genuinely give a real performance/ergonomics gain, add the equivalent NATIVELY
> (hand-rolled, matching `BumpArena`'s precedent — not as an external Cargo dependency)."*
> Grounded in live source + the new P57–P74 blueprints, not memory. Every claim is
> re-verifiable via the commands shown.

## 0. TL;DR — second honest "no adoption yet", but now with the code parked and the first trigger named

1. **The prior premise survives a much broader test.** The `swap_remove`-empty grep was one
   signal; this pass added five more (manual `Vec<Option<T>>` tombstone patterns, free-list /
   slab patterns, `git log` for stale/dangling/UAF history, a stored-index-across-removal scan,
   and a full read of the P57–P74 blueprints). **Every one comes back negative.** The
   stale-handle / ABA bug class these three crates exist to prevent is absent in the current tree
   *and* is designed **out** of the near-term blueprints — most pointedly **P65's dispatch
   orchestrator**, the exact "objects that come and go" case the task flagged, which explicitly
   **rejected** a stored `next_rank: usize` cursor and keeps only logical keys + an
   `Option<LiveOffer>` + a cleared-each-round skip-set (`BLUEPRINT-P65-dispatch-orchestrator.md:320`).
   P66 likewise rejects tombstones outright (`:154`).

2. **The three crates are real and good; thunderdome's edge is real but is a memory-layout edge,
   not a published-throughput edge.** thunderdome's citable advantage is an **8-byte key with an
   8-byte `Option<Index>`** (vs generational-arena's 16 / 24 bytes) — half the key memory traffic,
   which for cache-bound workloads reads through as speed. There is **no widely-published
   head-to-head criterion latency table**; "better performance" is an architectural argument
   (small keys, dense single-`Vec`), and I mark it as such rather than repeating it as a measured
   number. Critically, **thunderdome has zero runtime dependencies** (its `Cargo.toml` has no
   `[dependencies]` section at all) — it is the *one* crate here that would clear a
   "single well-audited zero-transitive-dependency crate" bar.

3. **A hand-rolled `SlotArena<T>` in `arena.rs`'s style is genuinely small and I proved it: the
   sketch in §3 compiles and passes ABA/stale/double-remove/reuse assertions** (`rustc -O`,
   `Handle` = 8 bytes). The prior report's "~50 lines" is right for the *logic* (~60 lines); a
   merge-ready module matching `arena.rs`'s doc + Miri + test discipline is ~150–200 lines (arena.rs
   itself is 318 for a simpler allocator). It is O(1) on every op and adequate; it does **not**
   beat thunderdome's packed layout on the last byte (`Option<Handle>` is 12 bytes, not 8 — §3.2),
   an honest and fixable gap.

4. **Verdict = (c) with teeth: no adoption now — neither the crate nor the hand-roll — but the
   code is drafted, compile-checked, and parked in §3 so the first real need is a copy-in-and-test
   job, not a design pass.** The first plausible trigger is named in §5. Adopting anything today
   would be a solution in search of a problem *and* would break the CI-verified zero-dependency
   default build for negligible gain.

---

## 1. Re-verifying the premise the hard way (broader than one grep)

All commands run live this pass from `/root/dowiz` (and `/root/bebop-repo` where noted).

### 1.1 The six signals of a real generational-index need — all negative

| Signal (what a genuine need looks like) | Command | Result |
|---|---|---|
| `swap_remove` (index invalidation on removal — the classic tell) | `grep -rn swap_remove kernel/src engine/src` | **empty** (re-confirmed) |
| Manual `Vec<Option<T>>` tombstone slab | `grep -rn "Vec<Option<" kernel/src engine/src` | **1 hit**, `simd.rs:517` — a test helper `random_observations() -> Vec<Option<f64>>` (missing-data fixture), **not** a slab |
| Free-list / slab / recycle machinery | `grep -rni "free_list\|freelist\|tombstone\|slab\|recycle\|slot reuse" kernel/src engine/src` | **0 relevant** (only the words "generation"/"regenerate" in unrelated senses — key-*generation*, benchmark-*generation*) |
| Stale/dangling/UAF ever fixed in history | `git log --all --grep="stale\|dangling\|use-after-free\|UAF" -i` | hits are about **stale docs/wasm/EMA** and a *dangling FSM ref* + *dangling test target* — **zero** memory-safety stale-index/UAF fixes |
| A stored `Vec` index held across a removal | manual read of the removal sites the prior report catalogued (`spool.rs`, `order_machine.rs`, `analytics.rs`, `intent.rs`) | none store an index across the removal; all recompute `position(...)` fresh or key by logical id |
| A **new** blueprint that introduces the pattern | full sweep of `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P57..P74*` + `BLUEPRINT-CACHE-…-ARENA` + `BLUEPRINT-EVENT-DRIVEN-ORCHESTRATOR` | see §1.2 — **negative, and actively designed against** |

### 1.2 The load-bearing new check: do the P57–P74 blueprints create the need? (No — P65 designs it out)

The task correctly singled out **P65's dispatch orchestrator** — it manages in-flight courier
offers with accept/timeout/decline, the textbook "objects come and go" pattern that usually grows
a generational arena. I read it in full (`BLUEPRINT-P65-dispatch-orchestrator.md`). It goes the
**opposite** way, on purpose:

- The entire per-order state is `DispatchSession` (`:259-268`): `order_id: u64` (logical key),
  `kind`, `offered_this_round: Vec<CourierKey>` (an **ephemeral skip-set, cleared every round and
  destroyed when the order leaves `Ready`**), `live_offer: Option<LiveOffer>` (a **single**
  outstanding offer, `OFFER_WAVE_SIZE = 1`), plus scalar backoff/timestamps. There is **no shared
  collection of offers with per-element removal**, hence nothing to hold a stale index into.
- It **explicitly rejects** the one construct that would have needed generational safety
  (`:320-324`): *"A `next_rank: usize` cursor instead of `offered_this_round` — rejected … the
  online set can change between offers, so a raw index into a stale list points at the wrong
  courier; re-running `assign` over the current online set each tick is the requeue-never-drop
  contract."* That is a designer choosing **re-derivation from current truth over a stored
  index** — the same instinct a generational index encodes, resolved at the design layer instead.
- Couriers are keyed by `CourierKey = [u8;32]` (a public key), orders by `u64` id. If a hub ever
  holds many live sessions, the fit is `HashMap<order_id, DispatchSession>` — a **keyed** map
  (like `analytics.rs`), not a `Vec`-index arena. No ABA surface.

Other new blueprints, swept: **P66 (offline drafts)** explicitly rejects tombstones — *"no
tombstones, no merge machinery. Single-writer LWW is strictly correct"* (`:154`). **P62 (catalog
multi-vendor)** is a vendor-scoped tree with a leaf invariant, build/validate-shaped, not
delete-with-held-handles. **The `CACHE-REFERENCE-GRAPH-TENSOR-ARENA` blueprint** is the origin of
`BumpArena` itself (region/bump, no per-element free) and asks for a *persistent tensor* arena
(huge-page seam, `arena.rs:137`), again not generational. No P-number blueprint contains
`generational`, `slotmap`, `thunderdome`, `swap_remove`, `free-list`, or a stored index cursor
(one grep over the whole set; the only `next_rank`/`tombstone` hits are the two **rejections**
above).

**Conclusion of §1:** the negative result is now robust across six independent signals and the
forward-looking blueprint set. The bug these crates prevent is neither present nor coming.

---

## 2. The three crates — real technical comparison (sources cited)

All three are legitimate, well-tested generational-index arenas. They pair each slot with a
**generation/version** counter and reject a lookup whose generation no longer matches the slot —
defeating the **ABA / stale-handle** bug (a handle that outlives its element and silently reads a
recycled slot).

| Property | **slotmap** | **generational-arena** | **thunderdome** |
|---|---|---|---|
| Key size | **8 B** (`DefaultKey`: `KeyData` = u32 index + u32 version = u64) | **16 B** (`Index` = usize index + u64 generation) | **8 B** (`Index`: u32 slot + `NonZeroU32` generation) |
| `Option<Key>` size | 8 B (version is `NonZero`-parity) | **24 B** | **8 B** (niche in the `NonZero` generation) |
| Insert / get / remove | O(1), "low overhead" | O(1) (deletion ABA-safe via generational indices) | O(1) "constant time insertion, lookup, and removal" |
| Iteration | `SlotMap` slow (skips empties); `HopSlotMap` fast (skip-field); `DenseSlotMap` = `Vec`-speed (extra indirection ⇒ slower random access) | single density | dense single `Vec<Slot<T>>` |
| Runtime deps | **none mandatory** (`serde` optional; `fxhash`/`quickcheck` are dev-deps); `no_std` via disabling `std` | **`cfg-if`** (+ optional `serde`); `no_std` | **NONE** — `Cargo.toml` has no `[dependencies]`; `no_std` via disabling `std`; **zero `unsafe`? no — it uses `unsafe` for the packing** |
| Maturity / adoption | most mature & widely used; multiple map variants | older, simplest, "zero `unsafe`", quickcheck-tested; effectively the reference the other two were "inspired by" | newer; explicitly *"inspired by generational-arena, slotmap, and slab"*, optimizing for small keys |
| Generation wrap horizon | 2³¹ deletions to the same slot (version parity) | u64 generation (effectively unbounded) | 2³²-ish per slot (`NonZeroU32`) |

**On thunderdome's "better performance" claim (verified, not repeated):** thunderdome's own
README/docs advertise **small (8-byte) keys** and constant-time ops but publish **no comparative
throughput/latency benchmark table**. The concrete, citable win is **memory**: an `Index` and an
`Option<Index>` are each **8 bytes vs generational-arena's 16 and 24** — a 2×/3× reduction in key
footprint. That halves key memory traffic and keeps `Option<Index>` free (niche-packed), which on
cache-bound access patterns manifests as speed — but as an **architectural** argument, not a
measured number. slotmap's `DefaultKey` is also 8 bytes, so on key size **slotmap and thunderdome
tie**; slotmap's differentiator is its three iteration-vs-access variants, thunderdome's is
minimalism + zero deps. generational-arena is the honest laggard on footprint (usize index + u64
generation, no niche packing) and is essentially superseded — kept alive by its simplicity and
zero-`unsafe` guarantee.

**Dependency-tree reality (the fact that matters most for the kernel):**
- **thunderdome**: `[dependencies]` — **none**. `std`/`alloc` only. This is the crate that could
  clear the "one zero-transitive-dependency crate" bar.
- **slotmap**: no *mandatory* external runtime dep (only optional `serde`); `no_std`-capable.
- **generational-arena**: pulls **`cfg-if`** (a tiny, ubiquitous macro crate) + optional `serde`.

---

## 3. Hand-rolled `SlotArena<T>` — real, compile-verified code (not pseudocode)

Drafted in `arena.rs`'s house style and **actually compiled and behaviorally tested this pass**
(`rustc -O --edition 2021`; all assertions pass; `size_of::<Handle>()` = 8). It is a **sibling**
to `BumpArena`, not a replacement: `BumpArena` is a *phase/region* allocator (bump + O(1) whole-
region `reset`, no per-element free); `SlotArena` is a *per-element* arena with stable `Copy`
handles that survive removal-and-reuse without the stale-index / ABA bug. Safe (`no unsafe`),
zero-dependency, `no_std`-ready (swap `std::mem::replace` for `core::mem::replace`).

### 3.1 The module (drop-in for `kernel/src/arena.rs`, or a new `slot_arena.rs`)

```rust
/// A `Copy` handle into a `SlotArena`. 8 bytes (`u32` index + `u32` generation). A handle is
/// valid ONLY while its `generation` matches the slot's current generation — a handle to a
/// since-removed element is a safe `None`, defeating the ABA / stale-index bug by construction
/// (the exact guarantee slotmap/thunderdome exist to provide), with zero external dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    index: u32,
    generation: u32,
}

enum Entry<T> {
    /// A live value.
    Occupied(T),
    /// A free slot; `next_free` threads the free-list (`None` = list tail).
    Free { next_free: Option<u32> },
}

struct Slot<T> {
    /// Bumped on every removal, so a stale handle's generation can never match again (until
    /// 2³² reuses of THIS slot — the documented wrap horizon, same class as slotmap's 2³¹).
    generation: u32,
    entry: Entry<T>,
}

/// A generational slot arena. `O(1)` insert / get / remove; removed slots are recycled via a
/// free-list, and generation counters make dangling handles a safe `None`, never a silent read
/// of a recycled value. Degrade-safe: every fallible op returns `Option`, never panics on a
/// bad handle (only `unreachable!` on a broken internal invariant — see the free-list arms).
pub struct SlotArena<T> {
    slots: Vec<Slot<T>>,
    free_head: Option<u32>,
    len: usize,
}

impl<T> Default for SlotArena<T> {
    fn default() -> Self { Self::new() }
}

impl<T> SlotArena<T> {
    pub const fn new() -> Self {
        SlotArena { slots: Vec::new(), free_head: None, len: 0 }
    }

    pub fn with_capacity(cap: usize) -> Self {
        SlotArena { slots: Vec::with_capacity(cap), free_head: None, len: 0 }
    }

    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }

    /// Insert `value`, returning a stable `Copy` handle. Reuses a free slot if one exists (O(1),
    /// no allocation), else pushes a new slot. The handle carries the slot's CURRENT generation,
    /// so it stays valid exactly until that element is removed.
    pub fn insert(&mut self, value: T) -> Handle {
        self.len += 1;
        match self.free_head {
            Some(index) => {
                let slot = &mut self.slots[index as usize];
                let next_free = match slot.entry {
                    Entry::Free { next_free } => next_free,
                    Entry::Occupied(_) => unreachable!("free_head pointed at an occupied slot"),
                };
                self.free_head = next_free;
                slot.entry = Entry::Occupied(value);
                Handle { index, generation: slot.generation }
            }
            None => {
                let index = self.slots.len() as u32;
                self.slots.push(Slot { generation: 0, entry: Entry::Occupied(value) });
                Handle { index, generation: 0 }
            }
        }
    }

    /// `Some(&T)` iff `handle` names a live element (index in range, slot occupied, generation
    /// matches). A handle to a removed/recycled slot is `None` — stale-index is unrepresentable.
    pub fn get(&self, handle: Handle) -> Option<&T> {
        match self.slots.get(handle.index as usize) {
            Some(Slot { generation, entry: Entry::Occupied(v) })
                if *generation == handle.generation => Some(v),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        match self.slots.get_mut(handle.index as usize) {
            Some(Slot { generation, entry: Entry::Occupied(v) })
                if *generation == handle.generation => Some(v),
            _ => None,
        }
    }

    pub fn contains(&self, handle: Handle) -> bool { self.get(handle).is_some() }

    /// Remove the element named by `handle`, returning it. Bumps the slot's generation
    /// (invalidating every outstanding copy of the handle) and pushes the slot onto the
    /// free-list. A double-remove or a stale remove is a safe `None`.
    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        let slot = self.slots.get_mut(handle.index as usize)?;
        let occupied_matching =
            matches!(slot.entry, Entry::Occupied(_)) && slot.generation == handle.generation;
        if !occupied_matching {
            return None;
        }
        slot.generation = slot.generation.wrapping_add(1); // bump FIRST → outstanding handles stale
        let old = std::mem::replace(&mut slot.entry, Entry::Free { next_free: self.free_head });
        self.free_head = Some(handle.index);
        self.len -= 1;
        match old {
            Entry::Occupied(v) => Some(v),
            Entry::Free { .. } => unreachable!("validated occupied above"),
        }
    }
}
```

Behavior actually exercised this pass (all pass): insert→get; remove returns the value; the
**stale handle is rejected** (`get` → `None`, `contains` → `false`); **double-remove** is `None`;
the freed slot is **reused** with the **same index but a bumped generation**, and the **old handle
to that slot stays rejected** (ABA defeated); `get_mut` mutates in place.

### 3.2 Honest performance/footprint vs the crates

- **Handle size: 8 bytes — ties slotmap and thunderdome.** Verified (`size_of::<Handle>()` = 8).
- **`Option<Handle>`: 12 bytes — loses to thunderdome's 8.** My `generation: u32` has no niche, so
  `Option<Handle>` can't pack. **Fixable** by making generation `NonZeroU32` (start at 1, bump with
  wrap-skipping 0), reclaiming the niche exactly as thunderdome does — costs ~5 lines and a
  slightly fiddlier bump. Left out of the base sketch for clarity; noted as the one real ergonomic
  gap.
- **Per-slot memory: loses to thunderdome for small `T`.** thunderdome overlaps the free-list index
  with the value's storage (union-like) and packs the generation; my `Slot<T>` carries a separate
  `u32` generation + an `Entry<T>` enum whose `Free` variant holds an `Option<u32>` (8 B) + a
  discriminant, so `size_of::<Slot<T>>()` ≥ `max(size_of::<T>(), 8) + 8`-ish. For large `T` this is
  noise; for `T = u32` it roughly doubles the slot. This is the classic **simplicity-vs-last-byte**
  trade — and re-deriving thunderdome's packed layout by hand is materially more than 90 lines of
  `unsafe`-touching code, which is the honest counter-argument in §4.
- **Throughput: O(1) all ops, allocation-free on reuse — adequate.** A naive hand-roll won't *beat*
  thunderdome's cache layout, but for any plausible kernel use (courier sessions, an incremental
  mesh index) the op counts are identical; the difference is constant-factor cache behavior on hot
  loops that **do not exist in this codebase**.

### 3.3 Real implementation-complexity estimate (correcting the prior "~50 lines")

- **Core logic:** ~60 lines (the impl above minus doc-comments) — the prior report's "~50" is
  essentially right for *logic*.
- **Merge-ready in `arena.rs`'s discipline** (module doc explaining why-it-exists + soundness note +
  `#[cfg(test)]` covering insert/get/remove/stale/double-remove/reuse/ABA + a Miri run like
  `arena.rs`'s done-check): **~150–200 lines.** For scale, `arena.rs` is 318 lines for the *simpler*
  bump allocator. So "small, but not a one-liner" — a focused half-day, not a sprint.

---

## 4. DECART: "adopt thunderdome" vs "hand-roll `SlotArena`" — honest, not assumed

Per the task, I did **not** assume hand-rolled always wins. thunderdome is the strongest adoption
candidate precisely because it clears the usual disqualifier: **zero runtime dependencies.**

| Axis | Adopt **thunderdome** | Hand-roll **`SlotArena`** (§3) |
|---|---|---|
| Dependency footprint | One crate line; **zero transitive deps** (verified: no `[dependencies]`) — but still breaks the CI-asserted **zero-dependency *default build*** property (`cargo tree -p dowiz-kernel --no-default-features -e no-dev \| grep -c serde → 0`; every current dep is behind an opt-in feature). A dep is a dep: a version to track, a supply-chain trust anchor, an audit surface. | **Zero.** Preserves the default-build invariant exactly. |
| Correctness/perf ceiling | Best-in-class packed 8/8-byte keys, battle-tested, dense layout. Uses `unsafe` internally (audited upstream). | O(1), safe (`no unsafe`), 8-byte handle; `Option<Handle>` 12 B and larger slots for small `T` unless you re-derive the packing (then you're writing thunderdome by hand). |
| In-tree proof story | Determinism / `no_std` / red-line guarantees delegated to the crate's feature flags & upstream tests. | Guarantees provable **in-tree**, under this repo's Miri done-check — the same reason `BumpArena` was hand-rolled instead of pulling `bumpalo`. |
| Effort | ~0 (add + use). | ~150–200 lines / half-day for a merge-ready module (§3.3). |
| When it's the right call | **Only** if a *measured* profile shows a hot per-element arena where the packed 8-byte-`Option` layout is load-bearing, AND the team's dep-minimalism tolerance flexes for that one case. | The default here: any non-hot need, or any need at all given no measured hot path exists. |

**The honest tie-breaker:** the deciding fact is not "hand-rolled is purer" — it's that **there is
no need to serve at all** (§1). With zero live or near-term consumers, adopting a dependency (even
a clean one) is speculative complexity, and hand-rolling now would be dead code. If that changes,
the table above says: hand-roll unless a profiler proves the packed layout matters, in which case
read thunderdome as the reference and hand-roll the packed version (or, if dep-minimalism flexes,
adopt thunderdome as the single defensible exception).

---

## 5. Recommendation

**(c) — No action now. A second, better-supported "no adoption yet."** Do not add
`slotmap`/`generational-arena`/`thunderdome`, and do not hand-roll `SlotArena` into the tree
today. The ABA/stale-index bug is absent across six signals and is designed **out** of the
forward blueprints (P65 rejects the index cursor; P66 rejects tombstones). Building the machinery
now yields dead code and, for the crate path, breaks the CI-verified zero-dependency default build.

**What to actually do:**

1. **Park the compile-verified `SlotArena<T>` (§3) in this doc** so the first real need is a
   copy-in + test + Miri job (~half a day), not a design pass. When wired, prefer this hand-roll to
   any crate — same reasoning that produced `BumpArena`, and it keeps the zero-dep default intact.
2. **Name the first plausible trigger to watch:** an **incremental mesh/graph index that deletes
   nodes while other structures hold references to them** (the prior report's exact condition), or
   a hub that ends up holding a *`Vec`-indexed* pool of live `DispatchSession`s with per-element
   removal *and* cross-references by slot position (note: P65 as written avoids this by keying on
   `order_id`/`CourierKey` — so even this would be a *deviation* from the current design, not a
   consequence of it). If either appears, lift §3, add the `NonZeroU32`-generation niche packing
   (§3.2) so `Option<Handle>` is 8 bytes, and gate it with Miri.
3. **Unchanged from the prior pass:** the one genuinely worthwhile arena refactor is still
   `micrograd.rs`'s `Rc<RefCell<ValueData>>` → a `Vec<ValueData>` + `NodeId` **tape** — and that is
   **non-generational** (build-once / backward-once / drop-all; no node is ever removed), so it
   needs neither these crates nor `SlotArena`. Optional perf/quality, 0 product consumers.

Net: the operator's "add it natively if it genuinely helps" is answered — it does not *genuinely
help yet*, the native version is drafted and proven so adoption is frictionless when it does, and
the zero-dependency discipline is preserved.

---

### Sources

- **thunderdome** (key sizes 8/8, "constant time insertion, lookup, removal", zero
  `[dependencies]`, `std`/`no_std` feature): [github.com/LPGhatguy/thunderdome](https://github.com/LPGhatguy/thunderdome),
  [Cargo.toml](https://raw.githubusercontent.com/LPGhatguy/thunderdome/main/Cargo.toml)
- **slotmap** (O(1) low-overhead insert/remove/access; `SlotMap`/`HopSlotMap`/`DenseSlotMap`
  iteration-vs-access trade; `(value, version)` slots, 2³¹ wrap; `no_std` via disabling `std`;
  optional `serde`): [docs.rs/slotmap](https://docs.rs/slotmap/latest/slotmap/)
- **generational-arena** (ABA-safe generational indices, zero `unsafe`, quickcheck-tested,
  `cfg-if` + optional `serde`, `no_std`): [docs.rs/generational-arena](https://docs.rs/generational-arena/latest/generational_arena/)
- **Codebase / blueprint ground truth (re-verifiable):** `kernel/src/arena.rs` (BumpArena style +
  the `count-allocs`/Miri discipline); `grep -rn swap_remove kernel/src engine/src` → empty;
  `grep -rn "Vec<Option<" kernel/src engine/src` → only `simd.rs:517` (test helper);
  `git log --all --grep="stale\|dangling\|use-after-free\|UAF" -i` → no memory-safety UAF fixes;
  `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P65-dispatch-orchestrator.md` (§3 `DispatchSession`,
  `:320` `next_rank` rejection); `…/BLUEPRINT-P66-data-wallet-offline-drafts.md:154` (tombstone
  rejection); `kernel/Cargo.toml` (every dep behind an opt-in feature). Hand-rolled `SlotArena`
  compile-checked at `rustc -O --edition 2021` (Handle = 8 B; ABA/stale/double-remove/reuse
  assertions pass).
- Prior pass this extends: `docs/research/OPUS-PERF-POINTER-ARENA-ANALYSIS-2026-07-18.md`.

---

## 6. UPDATE 2026-07-18 — operator OVERRODE this report's verdict; thunderdome integrated

This report's own recommendation (§0 / §5) was **(c) no adoption now**. After hearing that
verdict, the **operator explicitly overrode it** and directed that `thunderdome` be adopted
**now**, as forward-looking infrastructure, even though nothing in the current tree depends on it.
Recording the override honestly here so the doc's history is not silently contradicted: the research
still holds (no *current* code needs generational-index safety — §1's six-signal sweep stands), but
the decision was reversed at the operator layer, which is where "speculative-vs-warranted" calls
belong.

What was integrated (all landed in `/root/dowiz`, `main`):

- **`kernel/Cargo.toml`** — `thunderdome = { version = "0.6", optional = true }` behind a **new
  `slot-arena` feature** (`slot-arena = ["dep:thunderdome"]`), the same opt-in discipline as
  `pq` / `gpu` / `pgrust`. thunderdome pulls **zero transitive runtime deps**
  (`cargo tree --features slot-arena` shows it with no children).
- **`kernel/src/slot_arena.rs`** (new) — a thin, dowiz-flavored wrapper: `SlotArena<T>` over
  `thunderdome::Arena<T>` and an **opaque** `Handle` newtype over `thunderdome::Index` (private
  payload, so call sites can't unpack it and the backing crate stays swappable — e.g. for the §3
  hand-roll). API mirrors `arena.rs` vocabulary: `new` / `with_capacity` / `len` / `is_empty` /
  `capacity` / `insert` / `get` / `get_mut` / `contains` / `remove` / `clear` / `iter`. Doc style,
  degrade-closed posture (every fallible op returns `Option`, never panics on a stale handle), and
  the ABA argument all match `arena.rs`.
- **`kernel/src/lib.rs`** — `#[cfg(feature = "slot-arena")] pub mod slot_arena;`.
- **Tests** (8, all green) — insert/get/get_mut/remove roundtrip; **stale-handle rejection after
  removal**; **double-remove is a safe `None`**; **ABA defeated across removal + slot reuse** (old
  handle stays `None` while the recycled slot holds a new live value under a fresh handle);
  many-handles independence; `iter` visits only live elements; `clear` invalidates all handles; and
  **`size_of::<Handle>() == 8` AND `size_of::<Option<Handle>>() == 8`** (the niche-packed memory edge
  §3.2 cited — now asserted, not assumed).

**Zero-dependency default build preserved** (the property §4 flagged as the cost of adoption): the
feature is OFF by default, so `cargo tree -p dowiz-kernel -e no-dev | grep -c thunderdome` → **0**
and the default `cargo build` / `cargo test --lib` (704 passed) are byte-for-byte unchanged. With
`--features slot-arena`, the lib suite is 712 passed / 0 failed (704 + the 8 new). The §3 hand-rolled
`SlotArena` remains parked as the swap-in target if dep-minimalism is ever re-tightened — the opaque
`Handle` was designed precisely so that swap touches no call site.

> Net: the deep-dive's *analysis* stands (no current need); the *verdict* was overridden by the
> operator, and thunderdome is now integrated behind an off-by-default feature so it costs the
> canonical build nothing until the first real consumer (the §5 mesh-index trigger) arrives.
