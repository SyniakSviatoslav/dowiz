# OPUS Kernel Performance Audit — complexity + atomicity/branchless sweep

**Date:** 2026-07-18
**Scope:** `/root/dowiz/kernel/src/` (44,613 LOC), EXCLUDING `retrieval/ppr.rs` (O(k·n²)) and
`absorbing.rs` (O(n³)) — those are covered by sibling reports
(`OPUS-PERF-PPR-ANALYSIS-2026-07-18.md`, `OPUS-PERF-ABSORBING-MARKOV-ANALYSIS-2026-07-18.md`).
**Method:** brace-depth nested-loop detection (304 candidates), linear-scan-in-loop grep (128
candidates), dense-matrix (`Vec<Vec<f64>>`) signature sweep, lock/atomic sweep, then targeted reads
to establish real Big-O, the meaning of `n` in practice, and bench coverage
(`kernel/benches/criterion.rs`).
**Framing (operator's):** rank by *real-scale-risk × currently-uncovered-by-bench*. The load-bearing
distinction is whether `n` is a **bounded-small fixed structure** (FSM states, capped tile, causal
variables) or **scales with real data/mesh/order/corpus volume**.

---

## TL;DR

The math-heavy core is **already well-guarded** — the two biggest theoretical offenders
(`eigenvalues`/`charpoly`) are dispatched so the O(n⁴) path is a documented dead-fallback and every
live caller runs at n ≤ 32 or n = 1. The sweep did **not** find a second `ppr`/`absorbing`-class
landmine on the money/order critical path. It found:

- **1 genuine, cleanly-fixable O(N²)** on the durability/dispatch path (`spool.rs` FIFO drain) —
  n scales with outbox backlog, unbenched. **Highest-ranked complexity finding.**
- **1 low-medium O(R²)/O(n)** in the knowledge-spine (`retrieval/spine.rs`) — n scales with the
  memory corpus, advisory path, safe HashSet/HashMap fix.
- **1 low O(n²)-densification** of a sparse graph (`spectral_laplacian.rs`) on the n > 32 render path.
- **3 atomicity candidates** (Mutex on request-per-call paths): `token_bucket` (benched, but only
  single-threaded — contention unproven), `budget`, `admission` seen-set.

Everything else checked out as bounded-small or already benched. **I did not manufacture findings to
pad** — the negative results below are as load-bearing as the positives.

---

## Complexity findings (ranked)

### C1 — `spool.rs` FIFO drain is O(N²)  ·  MEDIUM (scales, unbenched, dispatch/durability path)

`kernel/src/spool.rs`
- `claim_next` (spool.rs:88): `self.records.iter().position(|r| !r.claimed)` — O(n) linear scan.
- `ack` (spool.rs:98–105) and `compact_drop` (spool.rs:122–129): `position(|r| r.id == id)` O(n) scan
  **plus** `Vec::remove(pos)` which shifts every trailing element — O(n).
- `reclaim` (spool.rs:110): `iter_mut().find(...)` O(n).

**Real complexity:** a strict-FIFO lock-step drain (the documented usage: `claim_next` the front,
`ack` the front) makes each `ack` a `Vec::remove(0)` = O(n) element shift. Draining N queued records
is therefore **O(N²)** — the textbook "linear remove inside a drain loop" quadratic.

**What `n` is:** the outbox/spool depth = pending work-unit backlog. `Spool::new(capacity)` is
caller-set; under sustained backpressure (the exact condition the `bounded_drainer` exists to handle,
per `bounded_drainer.rs` header) this is the backlog, not a fixed small constant. Scales with real
enqueue volume.

**Bench:** none. The `bounded_drainer` benches/tests count units but never exercise `Spool`'s
`remove`-shift cost.

**Recommendation — SAFE OPTIMIZATION.** Replace the `Vec<Record>` + `remove` with a `VecDeque` +
a claimed cursor (or a `head` index with lazy compaction) so FIFO claim/ack is O(1) amortized. If
random-access ack-by-id must stay, add a small `id → index` map. Then add a criterion bench that
drains N records so the fix is verified-by-math per the standing rule. No behavioral/red-line change
(pure data-structure swap; ordering and crash-safety semantics preserved).

---

### C2 — `retrieval/spine.rs` backlinks/related O(R²) dedup + O(n) id scan  ·  LOW-MEDIUM (corpus scales, advisory, unbenched)

`kernel/src/retrieval/spine.rs`
- `backlinks` (spine.rs:210–223): iterates **every** tag bucket in the index; for each bucket
  containing `id`, `!related.contains(other)` is an O(R) linear scan on a `Vec` per candidate →
  **O(R²)** dedup, plus it visits buckets that don't contain `id` at all.
- `related` (spine.rs:315–332): same `!out.contains(other)` O(R²) dedup (better outer loop — only
  the doc's own tags).
- `build_map` (spine.rs:231–259): `groups.iter_mut().find(|(t,_)| *t == tag)` inside the doc loop →
  O(docs · distinct-first-tags).
- `lookup_by_id` (spine.rs:296): O(docs) linear scan — **acknowledged** in the doc comment.

**What `n` is:** knowledge-spine document count / tag-incidence. Per MEMORY the memory corpus lives
on a "no limit" Hetzner volume and grows over sessions — this scales, unlike the FSM/causal structures.

**Bench:** none for `SpineIndex`. `bench_retrieval_recall` covers `PrimaryRecall` (a different
BM25+trigram structure), not the spine.

**Recommendation — SAFE OPTIMIZATION.** Use a `HashSet` for the dedup accumulators (O(R) not O(R²))
and a `HashMap<id, idx>` for `lookup_by_id`/`related`'s self-lookup. Low urgency: this is the
advisory living-knowledge/dev-tooling read path, not money/order. Bench-then-fix if corpus growth
makes it visible.

---

### C3 — `spectral_laplacian.rs::build_laplacian` densifies a sparse graph (O(n²)) for the n > 32 path  ·  LOW (render/UI, unbenched)

`kernel/src/spectral_laplacian.rs:43–54`
- `build_laplacian` calls `csr.to_adjacency()` and materializes a dense `n × n` `Vec<Vec<f64>>`
  `L = D − A`. For **n > 32** the module then routes this dense L into the *sparse* tier
  `topk_symmetric` (per the doc at spectral_laplacian.rs:67–74) — i.e. it densifies a sparse graph
  to O(n²) memory/time specifically on the branch that exists because the graph is too big for the
  dense path. Self-defeating for the sparse case.

**What `n` is:** field-UI graph node count. Can scale, but this is the render/field-UI basis producer,
not money/order. The module explicitly documents "the load-bearing tests exercise the n ≤ 32 path."

**Bench:** none.

**Recommendation — BENCH-ONLY / note.** For the n > 32 branch, build `L = D − A` directly as a `Csr`
(the Laplacian preserves the adjacency's sparsity pattern plus the diagonal) instead of densifying.
Low priority; gate on evidence that the field-UI ever drives n > 32.

---

### C4 — `money.rs` ledger is O(n²)  ·  INFO/LOW (n = per-order entries, bounded-small — NOT a scaling risk)

`kernel/src/money.rs`
- `ledger_sum` (money.rs:230–245): for each `Earn`, `ledger.iter().any(|r| r.reverses == Some(e.id))`
  → **O(n²)**.
- `ledger_append` (money.rs:185–224): O(n) duplicate-id scan + O(n) reversal-target/reversed-once
  scans per append → **O(N²)** to build a ledger of N entries.

**What `n` is:** ledger entries **per order** — an `Earn` plus at most one `Reversal` (the doc:
"A compensated terminal order (Earn + its Reversal) sums to exactly 0"). Bounded-small by construction.

**This is money-authority code and I am deliberately NOT recommending a change.** n is tiny, the
linear scans are the fail-closed conservation/idempotency probes, and correctness-first beats a
micro-opt here. Listed only for honesty/completeness. Bench: N/A.

---

## Negative results (checked and cleared — these are load-bearing, not omissions)

- **`spectral.rs` `eigenvalues`/`charpoly`** — the scariest theoretical target (Faddeev-LeVerrier
  charpoly is O(n⁴)). **Already guarded:** `eigenvalues` (spectral.rs:225) dispatches n ≤ 32 to the
  O(n³) Householder engine; the O(n⁴) charpoly+Durand-Kerner path is only the n > 32 fallback, and
  the code documents "n > 32 dense-symmetric has **no consumer and no path**." `eigh` is capped
  n ≤ 32. **Not a scaling concern.**
- **Drift gate (`classify_drift`/`spectral_radius`)** — on the event-commit path
  (`event_log.rs:432`, `commit_after_decide_drift_gate`) and the order path
  (`order_machine.rs:358`). Every live caller runs at **n = 1** (`classify_drift(&[vec![rho]])`) or a
  capped spectral-cache tile (`spectral_cache.rs:268`). Bounded-small.
- **`causal.rs` (2,335 LOC)** — node-set ops (`.contains`, `topo.position`) are over tiny fixed
  causal-variable index vectors; `JointDist` tables are bounded by domain cardinalities; the sample
  path (`from_samples` → identify → reduce) **is** benched (`empirical_identify`, 20k samples).
  Bounded.
- **`mesh.rs`** — `append` is O(1) (hashes only the tip); `verify_chain` (mesh.rs:225) is O(n)
  signature verifies, which is *inherent* (each entry must be verified once) — no hidden O(n²)
  (append does not re-verify the chain).
- **`event_log.rs`** — `MemEventStore` backs dedup with a `HashSet` (event_log.rs:210) → O(1)
  `contains`/`insert`. Append path is fine.
- **`cart.rs` / `order_machine::place_order`** — O(items); cart `add` is O(lines) with lines bounded
  by items-per-order. Benched at 5 items. The only super-linear cost would be a caller-supplied
  `unit_price` closure doing an O(catalog) lookup — that's outside the kernel.
- **`retrieval/bm25.rs::rank`** (bm25.rs:222) — O(D · Q) linear-in-corpus (no inverted-index posting
  lists), **linear not quadratic**, and exercised by `bench_retrieval_recall` at fixture scale. A
  posting-list index would help at large D but this is a documented design point, not a hidden
  quadratic.
- **`retrieval/diffusion.rs`** — frozen 20-node fixture (N = 20 constant); delegates to `Ppr`.
- **`intake.rs` AC-3** (intake.rs:343) — domains capped by `MAX_ENUM_WIDTH = 4096`; fields/rules are
  human-authored spec size (small). Bounded. (Micro-note: `supported` for `Lt/Le/Gt/Ge` does an
  O(|dj|) `.any()` scan that could be O(1) via min/max — but bounded domain, cold, not worth it.)

---

## Atomicity / branchless opportunities (per the 2026-07-18 standing rule)

The rule is explicit: flag **only** where there is real hotness evidence (a bench or a documented
critical-path role) AND a poorly-predicted branch or a **contended** lock/CAS loop — never a blanket
mandate. Each item below carries its hotness evidence and an honest statement of what's *not* yet
proven.

### A1 — `token_bucket.rs` `Mutex<Inner>` on the F33 dispatch hot path  ·  MEDIUM (benched hot path; contention UNPROVEN)

`kernel/src/token_bucket.rs:29,74–90` — `try_acquire` takes `self.inner.lock()` per call over an
`Inner { tokens: f64, last_refill: Instant }`.

**Hotness evidence:** it has a dedicated criterion bench (`token_bucket/try_acquire_permit`), and both
the bench comment and the module doc (§4.2) state the `Dispatcher` calls `try_acquire` **once per
chat request** — it is the per-request compute-budget gate.

**Honest caveats (why this is NOT auto-"convert to CAS"):**
1. The existing bench is **single-threaded / uncontended**. The critical section is a handful of float
   ops. A `Mutex` only serializes under *real concurrency on a shared bucket* — and there is currently
   **no contended bench** proving that cost.
2. The bench comment mislabels the impl as "refill + **CAS**" — there is no CAS; it's a `Mutex`. The
   comment is aspirational; the code is a lock.
3. The module deliberately argues the lock keeps refill+decrement one atomic section for the
   over-grant invariant — a lock-free rewrite must preserve that invariant exactly.

**Recommendation — NEEDS-BENCH-FIRST.** Add a multi-threaded contended bench (K threads hammering one
bucket). *Only if* it shows lock serialization, convert to a lock-free CAS loop over an `AtomicU64`
(bit-cast the `f64` token count) + `AtomicU64` monotonic-nanos for `last_refill`, re-proving the
over-grant falsifier and adding the contended bench as the authority. Per the standing rule a
lock-free rewrite with no contended bench "is not done." **Do not convert on the single-thread
number alone.**

### A2 — `budget.rs` `Mutex<ComputeBudget>`  ·  LOW-MEDIUM (sibling of A1, UNBENCHED)

`kernel/src/budget.rs:133,148,167` — `budget: Mutex<ComputeBudget>` debited under lock, same
`unwrap_or_else(into_inner)` poison-recovery shape as `token_bucket`.

**Hotness evidence:** the compute-budget primitive the `Dispatcher` reuses on the same per-request
path (per `token_bucket` §4.2). **But there is no criterion bench for it** — hotness is by
association, not measured.

**Recommendation — BENCH-FIRST**, same treatment as A1 with lower prior confidence. Add a bench
before touching it.

### A3 — `ports/agent/admission.rs` `Mutex<HashSet<[u8;8]>>` seen-set  ·  LOW (admission path, unbenched, bigger change)

`kernel/src/ports/agent/admission.rs:150,240` — `seen: Mutex<HashSet<…>>` locked on every admission
check (nonce replay dedup).

**Hotness evidence:** on the agent-admission critical path (every agent request performs the replay
check). **But unbenched**, and the realistic fix (a sharded or lock-free set) implies a new dependency
or a nontrivial rewrite — higher cost, unproven payoff.

**Recommendation — BENCH-FIRST, then consider sharding.** Do not rewrite speculatively.

### Explicit NON-findings (branchless) — flagging these would violate the rule's own caveat

- **`SeqCst` on standalone metric counters** — `admission.rs:192` (`check_count.fetch_add`),
  `spectral_cache.rs:75` (`recomputes.fetch_add`), `json_api.rs:180` (`ORDER_SEQ.fetch_add`). Relaxed
  ordering would be equally correct for these pure counters, **but on x86-64 `fetch_add` compiles to
  `lock xadd` regardless of ordering** — there is literally no instruction-level difference, so the
  benefit is **zero**. Changing them would add reasoning cost for no measured win — exactly the
  "readability cost not paid back" the standing rule forbids. **Deliberately NOT flagged.**
- No poorly-predicted hot-loop branch worth arithmetic/mask conversion was found: the tight numeric
  loops (matmul, spmv, Householder, softmax_batch_lane) are already branch-light or bounded-small, and
  `simd.rs` already provides the branchless softmax lane path.

---

## Priority ranking (real-scale-risk × uncovered-by-bench)

| Rank | Finding | Class | n scales? | Bench? | Recommendation |
|------|---------|-------|-----------|--------|----------------|
| 1 | `spool.rs` FIFO drain | O(N²) | **Yes** (backlog) | No | SAFE OPT (VecDeque/cursor) + bench |
| 2 | `token_bucket` Mutex (A1) | lock | contention | single-thread only | BENCH contended → maybe CAS |
| 3 | `spine.rs` backlinks/related (C2) | O(R²)/O(n) | **Yes** (corpus) | No | SAFE OPT (HashSet/HashMap) |
| 4 | `budget.rs` Mutex (A2) | lock | contention | No | BENCH-FIRST |
| 5 | `spectral_laplacian` densify (C3) | O(n²) mem | Yes (UI, n>32) | No | BENCH-ONLY / build L as Csr |
| 6 | `admission` seen-set Mutex (A3) | lock | contention | No | BENCH-FIRST |
| — | `money.rs` ledger (C4) | O(n²) | No (per-order) | N/A | NO ACTION (correctness-first) |

**Bottom line:** no second ppr/absorbing-scale money-path landmine exists — the kernel's heavy math
is correctly tiered by n. The real, actionable item is the `spool.rs` O(N²) drain; the atomicity
items all reduce to "add a contended bench before rewriting," per the standing rule's own
verified-by-math gate.
