# Go's "pointer feature" in kernel/rust — reality check + arena/handle reuse map

> Research, 2026-07-18. Question (operator, paraphrased): *"how realistic is it to
> add a 'pointer' feature from Go into kernel/rust, and reuse the pointer mechanics in
> other, non-standard places."* Grounded in the actual `kernel/` + `engine/` source, not
> memory. Ground-truth commands are shown so every claim is re-verifiable.

## 0. TL;DR

- **A literal port of Go's pointer feature is a category error, not an engineering task.**
  Go pointers are GC-backed "take an address and forget about lifetime"; Rust's entire value
  proposition is *the absence of a GC* plus compile-time ownership. The dowiz kernel already
  has the strictly **richer** pointer model (`&T`/`&mut T`, `*const`/`*mut`, `Box`/`Rc`/`Arc`,
  raw-region bump allocation). There is no missing "Go pointer" primitive to add — adding one
  would mean either bolting on a garbage collector (kills determinism, `no_std`, WASM size,
  and the money/RLS red-line guarantees) or scattering `unsafe` raw pointers (kills the memory
  safety that is the *reason* to be in Rust). Off the table, and correctly so.

- **What is genuinely valuable behind the question is the *idiom*, and dowiz already runs it.**
  The Rust-idiomatic answer to "I want Go-like cheap allocation and easy shared references
  without fighting the borrow checker" is two patterns dowiz already ships:
  1. **Region/bump allocation** — `kernel/src/arena.rs::BumpArena`. This *is* the spiritual
     equivalent of Go's escape-analysis payoff: instead of letting each scratch buffer "escape"
     to its own `malloc`, one contiguous region serves them all and frees them with one `reset`.
  2. **Index-as-handle** — nearly every graph module (`causal.rs`, `cgraph.rs`, `dsu.rs`,
     retrieval) already represents nodes as `usize` indices into a `Vec`, i.e. a "pointer" that
     is a plain integer offset, not a machine address. This is the safe half of what people
     reach for `generational-arena`/`slotmap` for.

- **The single honest reuse target** is `kernel/src/micrograd.rs`, the one module that uses
  `Rc<RefCell<ValueData>>` — the classic pattern that fights Rust's ownership model. A typed
  index-arena (`Vec<ValueData>` + `NodeId(u32)` handles) is the textbook refactor. It is
  **optional** (micrograd has 0 product consumers; only `evals.rs` drives it), so this is a
  quality/perf improvement, not a fix.

- **Do NOT add a `generational` arena right now.** A generational index (`(index, generation)`)
  exists to defeat the *stale-handle / ABA* bug — a handle that outlives the element it named
  and silently reads a recycled slot. **That bug class does not currently exist in the
  codebase**: `grep -rn swap_remove kernel/src engine/src` returns **nothing**, the graph
  structures are build-once/never-mutated, and the two mutable collections look elements up by
  *logical key*, not by a stored index. A generational arena today would be a solution in
  search of a problem.

- **Do NOT pull in `slotmap`/`generational-arena`/`thunderdome`.** The default kernel build is
  deliberately zero-dependency (`cargo tree -p dowiz-kernel --no-default-features -e no-dev
  | grep -c serde → 0`; every dep is behind an opt-in feature). Hand-roll the ~40–60 lines if
  and when the need appears, in `arena.rs`'s existing style — same reasoning that produced the
  hand-rolled `BumpArena`.

---

## 1. What is actually distinctive about Go's pointers (and does Rust want it?)

| Go pointer trait | Rust today | Verdict for the kernel |
|---|---|---|
| **Escape analysis** — compiler auto-decides stack vs heap; no manual `Box`. | Rust is **stack-by-default**; heap is an explicit opt-in (`Box`/`Vec`/`String`). There is no "escape-analysis feature" to import because there is nothing to decide: you already said where the value lives by the type you chose. ([goperf.dev](https://goperf.dev/01-common-patterns/stack-alloc/), [freecodecamp](https://www.freecodecamp.org/news/understanding-escape-analysis-in-go/)) | **Already solved, better.** The *optimization* Go's escape analysis buys (avoid N little heap allocations) is exactly what `BumpArena` buys explicitly and deterministically — see §2. |
| **"Take the address, GC handles the lifetime."** | Fundamentally incompatible with Rust without a GC. Rust replaces the GC with ownership + lifetimes checked at compile time. | **Not portable.** Importing it = importing a GC. That trades away determinism, `no_std`, WASM footprint, and the red-line guarantees. Rejected. |
| **Simplicity: shared mutable references are trivial.** | The borrow checker makes shared *mutable* state deliberately costly (`Rc<RefCell>`/`Arc<Mutex>`). | **This is the real itch** — and the Rust-idiomatic relief is the index/arena handle (§3), which dowiz already uses everywhere except micrograd. |

**Bottom line on "how realistic":** adding Go's pointer *feature* verbatim is ~0% realistic
and undesirable. Re-using the *mechanics that make Go pointers feel nice* (cheap pooled
allocation; integer handles instead of raw addresses) is not only realistic — it is **already
the house style** in this kernel. The useful reframing of the operator's question is therefore
**"where else should dowiz's existing arena/handle mechanics be reused?"**

---

## 2. Ground truth: what `arena.rs` already is

`kernel/src/arena.rs::BumpArena` is a **deterministic bump/region allocator**, *not* a
generational arena and *not* a general slab:

- One `UnsafeCell<Vec<u8>>` region, fixed capacity, monotone bump `offset`, `O(1)` `reset()`.
- `alloc_slice<T: Copy + Default>(len) -> Option<&mut [T]>`. Degrade-closed: returns `None` on
  exhaustion, never grows, never panics.
- Soundness is by *signature*, not convention: `reset(&mut self)` proves no loan is live; the
  `Copy + Default` bound makes the `Drop`-hazard *unrepresentable at compile time*.
- Live callers (the "rebuild-and-rank" phase): `csr.rs` (`from_edges_in`, `row_normalize_in`,
  `personalized_pagerank_in`), `spectral.rs` (`charpoly_in`, `topk_symmetric_in`), `mat.rs`
  (`matmul_contig_in`). Each `_in` variant falls back to a heap `Vec` on `None`.

This is precisely the "Go escape-analysis win, done explicitly" pattern: a family of
same-shaped scratch buffers that would otherwise be ≈2·n+7 individual heap allocations per
rebuild is served from one region and freed in one op. The `count-allocs` feature *measures*
this ("≤ 8 heap allocations on the arena path"), so the win is instrumented, not assumed.

**Key constraint for §3:** because of the `T: Copy + Default` bound, `BumpArena` **cannot**
hold the one type that would most benefit from an arena — `micrograd::ValueData` contains a
`Box<dyn Fn(&Value)>` (neither `Copy` nor `Default`). So the micrograd reuse is *not* "call
`BumpArena`"; it is "add a sibling **typed** index-arena." They are different tools:
`BumpArena` = untyped POD scratch region; a typed arena = `Vec<T>` + integer handles for
non-`Copy` graph nodes.

---

## 3. The reuse map — where handle/arena mechanics genuinely fit (and where they don't)

### 3.1 STRONG candidate — `kernel/src/micrograd.rs` (`Rc<RefCell<ValueData>>` → typed index arena)

The **only** module in `kernel/` or `engine/` that reaches for `Rc<RefCell>` to model a graph
(`micrograd.rs:29 pub struct Value(Rc<RefCell<ValueData>>)`). `ValueData` holds `data`, `grad`,
`prev: Vec<Value>`, and `backward: Option<Box<dyn Fn(&Value)>>`. Every op clones an `Rc` into
`prev` and into the backward closure.

Costs of the current shape:
- **Per-node heap allocation** (one `Rc` box per `Value`) → cache-hostile; the graph is a
  scatter of independent allocations.
- **Refcount traffic** on every `.clone()` (every op clones both operands twice).
- **`RefCell` runtime borrow checks** — a `borrow_mut` while a `borrow` is live is a *panic*,
  i.e. a latent runtime failure mode in a module whose whole selling point is determinism.

Refactor (established Rust idiom — the "tape"/arena autograd, same family as `slotmap` but
without needing generations because the graph is **build-once, backward-once, drop-all** and no
individual node is ever removed):

```
struct Tape { nodes: Vec<ValueData> }          // contiguous, cache-friendly
struct NodeId(u32);                             // the "pointer" — a plain offset
// ValueData.prev: Vec<NodeId>; backward keyed by NodeId; grads: Vec<f64> parallel to nodes.
```

Payoff: contiguous storage, zero per-node allocation, `O(1)` whole-graph free (drop the `Vec`),
and the `RefCell` borrow-panic hazard **disappears** (grads live in one `Vec<f64>` mutated by
index). No generations needed. This is the cleanest possible demonstration of "reuse the
pointer/handle mechanic in a non-standard place," and it is **low-risk**: `micrograd` has 0
product consumers (`evals.rs:747` even documents "0 external consumers"; only the self-eval
harness at `evals.rs:843` constructs `Value`s). So it can be refactored and A/B-verified against
the existing hand-oracle gradient tests without touching any live order/money path.

**Recommendation:** worth doing as a *scoped perf/quality* change (the standing
"performance-priority-over-minimal-change" directive covers exactly this), **not** urgent.

### 3.2 WEAK / NON-candidates — inspected and honestly cleared

Listing these so the reuse map is not oversold. Each was checked; none has the bug or the
pressure that would justify an arena/handle rewrite.

- **`causal.rs`, `cgraph.rs`, `dsu.rs` — graph adjacency as `Vec<Vec<usize>>`.**
  These *already are* index-as-handle graphs (`cgraph.rs:40 parents: Vec<Vec<usize>>`), but
  they are **build-once, validated-at-construction** (e.g. `causal.rs:308` rejects any node
  index ≥ `parents.len()`), and **never mutated** afterward. Indices can never go stale because
  nodes are never removed or reordered. A generational arena adds pure overhead (the generation
  counter never advances). **Leave as-is.**

- **`kernel/src/order_machine.rs:256 queue.remove(0)`.** A local `Vec<usize>` used as a FIFO
  inside `topological_order()` over the **12** fixed lifecycle states. No index is held across
  the removal; `remove(0)` is O(n) but n≤12. `VecDeque` would be marginally tidier and
  pointless at this size. **Not a candidate.**

- **`kernel/src/spool.rs:99–124` (`records: Vec<Record>`, `position(...).remove(pos)`).**
  Records are found by *logical* `id`, and `pos` is computed fresh immediately before
  `remove(pos)` — never stored. So **no stale-index bug**. There is a mild *perf* smell (O(n)
  `position` + O(n) `remove`); if the spool ever grows hot, a `HashMap<Id, Record>` (not a
  generational arena) is the fit. Currently small. **Watch, don't refactor.**

- **`kernel/src/analytics.rs:42 orders: HashMap<String, ...>`.** `get_mut(order_id)` is a
  keyed map lookup, not a `Vec` index. No raw-index fragility. **Not a candidate.**

- **`engine/src/intent.rs:357,639 (matches.remove(0) / items.remove(0))`.** Transient local
  pop-front on small classification/intent vectors. **Not candidates.**

- **`retrieval/` (bm25 / diffusion / ppr / recall).** Uses `node_id` and `Vec<usize>` candidate
  lists, but these are **query-time transient rankings over a build-once index** (no
  `swap_remove`, no `tombstone`, no `free_list` — grep-verified). This is the same
  "rebuild-and-rank" shape `BumpArena` already serves; incremental per-element deletion isn't
  the model. **Not a generational-arena candidate.**

**The load-bearing negative result:** `grep -rn "swap_remove" kernel/src engine/src` (excluding
tests) returns **empty**, and no module stores a `Vec` index across a removal. The exact bug a
generational index prevents is **not present anywhere in the codebase today.**

---

## 4. Should dowiz adopt a crate (`slotmap` / `generational-arena` / `thunderdome`)?

These are real, battle-tested, and well-designed. `thunderdome` gives 8-byte generational keys
(`NonZero`-packed so `Option<Index>` stays 8 bytes) with O(1) insert/lookup/remove; `slotmap`
and `generational-arena` are the older siblings. They solve the **ABA / stale-handle** problem
by pairing each slot with a monotonic generation and rejecting a lookup whose generation no
longer matches. ([thunderdome](https://github.com/LPGhatguy/thunderdome),
[generational-arena docs.rs](https://docs.rs/generational-arena/latest/generational_arena/),
[generational-indices guide](https://lucassardois.medium.com/generational-indices-guide-8e3c5f7fd594))

**Verdict: no — for the same reasons that produced the hand-rolled `BumpArena`, and consistent
with this session's ad-fontes / minimal-dependency ethos.**

1. **Zero-dependency default build is a hard property here**, verified in CI-style
   (`cargo tree ... | grep -c serde → 0`). Every current dep is behind an opt-in feature
   (`wasm`, `pq`, `pgrust`, `gpu`). Pulling a crate for a ~40–60 line pattern breaks that
   property for negligible gain.
2. **No `no_std` / determinism guarantee to inherit.** The kernel is `no_std`-capable and
   determinism-constrained (money/RLS red-lines); a hand-rolled arena in `arena.rs`'s proven
   style keeps those guarantees provable in-tree rather than delegating them to a crate's
   feature flags.
3. **The actual near-term need is non-generational** (micrograd's build-once tape). It doesn't
   even exercise the generational machinery these crates exist to provide.

**If** a real per-element-delete-with-held-handles need ever appears (e.g. an incremental
mesh/graph index that deletes nodes while other structures hold references to them), the right
move is a ~50-line `SlotArena<T> { slots: Vec<Slot<T>>, free: Vec<u32>, gen: Vec<u32> }` living
next to `BumpArena` in `arena.rs`, matching its degrade-closed / soundness-by-signature
discipline — not a new dependency. Read `thunderdome`/`slotmap` as the *design reference* for
the generation-check, then implement in-house.

---

## 5. Concrete verdict

1. **"Add Go's pointer feature to kernel/rust" — not realistic and not desirable.** It is a
   category error: Go pointers presuppose a GC, and the kernel's determinism/`no_std`/red-line
   guarantees are built on *not* having one. Rust already gives the richer, safer model.

2. **"Reuse the pointer mechanics in non-standard places" — already the house style, with one
   clean unclaimed spot.** dowiz already uses region allocation (`BumpArena`) and index-as-handle
   graphs pervasively. The single genuinely non-standard, genuinely worthwhile reuse is
   **`micrograd.rs`: `Rc<RefCell<ValueData>>` → `Vec<ValueData>` + `NodeId` typed index arena**
   (§3.1) — a low-risk, optional perf/quality refactor guarded by existing gradient tests.

3. **Do not add generational indices or a slotmap crate yet** — the bug they prevent
   (`swap_remove`-free, verified) does not exist in the tree, and the dependency would break the
   zero-dep default build. Hand-roll a `SlotArena` in `arena.rs` *if and when* a
   delete-with-held-handles need actually appears.

---

### Sources
- Go escape analysis / stack-vs-heap: [goperf.dev — Stack Allocations and Escape Analysis](https://goperf.dev/01-common-patterns/stack-alloc/), [freeCodeCamp — Understanding Escape Analysis in Go](https://www.freecodecamp.org/news/understanding-escape-analysis-in-go/)
- Generational-index / arena crates + ABA problem: [thunderdome (LPGhatguy)](https://github.com/LPGhatguy/thunderdome), [generational-arena (docs.rs)](https://docs.rs/generational-arena/latest/generational_arena/), [Generational Indices Guide (L. Sardois)](https://lucassardois.medium.com/generational-indices-guide-8e3c5f7fd594)
- Codebase ground truth (re-verifiable): `kernel/src/arena.rs`, `kernel/src/micrograd.rs`, `kernel/src/csr.rs`, `kernel/src/spectral.rs`, `kernel/src/causal.rs`, `kernel/src/cgraph.rs`, `kernel/src/spool.rs`, `kernel/src/analytics.rs`, `kernel/src/order_machine.rs`, `kernel/Cargo.toml`; `grep -rn "swap_remove" kernel/src engine/src` → empty.
