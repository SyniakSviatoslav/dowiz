# BLUEPRINT P77 — Kernel complexity fixes: spool + spine (2026-07-19)

> **Standalone KERNEL blueprint (`dowiz` kernel crate).** One coherent, independently buildable unit
> against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research source:
> `docs/research/OPUS-PERF-KERNEL-AUDIT-2026-07-18.md` ("R3": C1 = spool, rank #1 real O(N²); C2 =
> spine). Synthesis home: `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.2 (Tier-B B1/B2), §5 Wave-1.
> Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding tree:
> `/root/dowiz/kernel/src` at HEAD, read live this pass.
>
> **One sentence:** two Tier-B, strictly **behavior-preserving** algorithmic fixes on the kernel's
> pure-state layer — **B1** turns `spool.rs`'s O(N²) FIFO drain (a `position()` scan + `Vec::remove`
> shift per ack) into an O(1)-amortized `VecDeque` + claim-cursor drain, and **B2** turns
> `retrieval/spine.rs`'s O(R²) `Vec::contains` dedup + O(docs) id scan into `HashSet`/`HashMap`
> accumulators — each shipping with the criterion bench that is the *only* proof either path was ever
> quadratic, because **both are UNBENCHED today**.

---

## VERDICT (stated up front, per session discipline)

**GO — both are safe, behavior-preserving, strictly-better fixes with named in-repo patterns to copy.**
Neither is an emergency and this blueprint says so plainly:

1. **The win is latent/asymptotic, not measured-hot-today.** Per the CLAUDE.md Performance Standing Rule
   (`.claude/CLAUDE.md:182-195`) a rewrite requires a benchmark proving hotness — and **neither `spool.rs`
   nor `retrieval/spine.rs` has any bench today** (§0.4, verified: `grep -i 'spool\|spine\|drain'
   kernel/benches/criterion.rs` → empty). So the **first deliverable of each fix is the baseline bench
   that captures the quadratic curve**, and the win is proven by that curve going linear. We do **not**
   claim a speedup we cannot measure; we prescribe the measurement. The bench IS the gate.

2. **Rank, honestly.** B1 (spool) is R3's **#1-ranked real O(N²) in the whole kernel** (R3 §"Priority
   ranking" rank 1) — but it only bites **under real outbox backlog / backpressure**, the exact
   condition `bounded_drainer.rs` exists to create; it is not hot on an idle queue. B2 (spine) is R3's
   rank-3 **LOW-MEDIUM**, on the **advisory** living-knowledge retrieval path (never money/order), where
   `n` = the knowledge-spine corpus that grows unbounded per session. So: B1 > B2 in leverage, both are
   "fix now while cheap so it never bites," neither is "the site is down."

3. **The NO-REGRESSION floor is a hard gate.** Because the win is asymptotic, the small-`n` end of the
   sweep **must stay a tie-or-win** — a data-structure swap that regressed the common tiny-queue /
   tiny-corpus case would be a net loss (the drainer runs at small depth most of the time). `n = {16}`
   staying ≤ baseline is a DoD row (§6 D-NOREG), not an afterthought.

4. **Semantics are byte-identical.** Spool's FIFO ordering + claim/ack/reclaim/crash-recovery guarantees
   and spine's exact result *set and sort order* must be indistinguishable from the old impl through the
   public API. A **differential equivalence test** (old-vs-new, random op sequences) is a first-class DoD
   item (§6 D-EQUIV), not prose.

**No hazard, no red-line, no operator decision.** Both files are pure `std` state machines with no money,
auth, RLS, migration, or crypto surface; the change is a data-structure swap behind an unchanged public
API. GO.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**
> (`/root/dowiz/kernel/src`, HEAD), not inherited from the synthesis shorthand. One path correction to
> the synthesis is made here because a correct blueprint requires it.

### 0.1 B1 — `spool.rs` is genuinely O(N²) on the FIFO drain (`kernel/src/spool.rs`, 235 lines)

The backing store is a flat `Vec<Record>` (`spool.rs:37`), and every mutating operation is a linear
scan and/or a shifting remove:

| Method | Cite | Quadratic shape |
|---|---|---|
| `claim_next` | `:88-92`; scan at `:89` | `self.records.iter().position(\|r\| !r.claimed)` — O(n) linear scan for the first un-claimed record **every claim**. In a "claim all N" pattern the scan re-walks the claimed prefix → O(N²). |
| `ack` | `:98-105`; scan `:99`, shift `:100` | `position(\|r\| r.id == id)` O(n) scan **plus** `self.records.remove(pos)` which shifts every trailing element O(n). In the documented FIFO lock-step drain (`ack` the front), `remove(0)` shifts all N−1 trailing elements → draining N records is **O(N²)** (the textbook "linear remove inside a drain loop"). |
| `reclaim` | `:109-117`; find `:110` | `iter_mut().find(\|r\| r.id == id)` — O(n) scan. |
| `compact_drop` | `:122-129`; scan `:123`, shift `:124` | same `position` + `remove` shift as `ack` — O(n). |
| `pending` | `:132-134` | `iter().filter(\|r\| !r.claimed).count()` — O(n) (diagnostic peek, not the hot loop). |
| `in_flight` | `:137-139` | `iter().filter(\|r\| r.claimed).count()` — O(n) (diagnostic peek). |

**What `n` is (R3 §C1, verified):** the outbox/spool depth = pending work-unit backlog. `Spool::new(capacity)`
(`:45-51`) is caller-set; under sustained backpressure — the exact condition `bounded_drainer.rs` exists
to handle (`bounded_drainer.rs:1-20`, "heavy op must never monopolise a tick") — this is the real backlog,
**not a fixed small constant**. It scales with real enqueue volume.

**Semantics that MUST be preserved (read from the module doc `:1-20` + the tests `:142-235`):**
- **Strict FIFO claim** — `claim_next` returns the *lowest-id un-acked, un-claimed* record (`:84-88`).
- **Monotonic ids** — `append` returns sequential ids (`:70-82`; test `append_sequential_ids_and_len:147`).
- **Backpressure drop** — at capacity, `append` returns `None` and drops (`:70-73`; test
  `backpressure_at_capacity:183`, `capacity_one_handshake:225`). `is_full` (`:63-65`) flips exactly at the
  watermark over the **un-acked** count.
- **Crash-recovery** — a claimed-but-unacked record is `reclaim`-able and then re-claimed as the *same*
  record, not skipped (test `crash_reclaim_recovers_inflight:196` asserts the reclaimed id is re-claimed
  next).
- **Out-of-order ack** — `ack(id)` may remove a non-front id while others stay claimed, and the remaining
  records keep their ids/relative order (test `claim_fifo_then_ack:158` acks the middle id first, then
  asserts `records[0].id==0`, `records[1].id==2`).
- **Idempotent/fail-closed ack** — `ack` of an unknown id is a no-op returning `false` (test
  `ack_unknown_is_noop:214`).

> **NOTE on `claim_fifo_then_ack:173-174`** — it reads the **private** field `s.records[0].id` /
> `s.records[1].id`. This is a white-box assertion coupled to the internal representation. Because
> `VecDeque<T>` implements `Index<usize>` returning the *logical* front-relative element (identical to
> `Vec` after an order-preserving `remove`), swapping `Vec`→`VecDeque` keeps this test compiling and
> passing **verbatim**. If B1's final internal shape ever tombstones (Design B, §3.1), this one test's
> two field-index lines migrate to the public API (`claim_next`) — called out so the worker is not
> surprised. Every *public-API* test stays byte-identical either way.

### 0.2 B2 — `retrieval/spine.rs` has O(R²) dedup + O(docs) scans (`kernel/src/retrieval/spine.rs`, 646 lines)

| Method | Cite | Quadratic / linear shape |
|---|---|---|
| `backlinks(id, index)` | `:210-223` | (a) `for bucket in index.values()` visits **every** tag bucket, including buckets that don't contain `id`; (b) `bucket.iter().any(\|d\| d == id)` (`:213`) is O(R) membership per bucket; (c) `!related.contains(other)` (`:215`) is an O(R) linear scan on a `Vec` **per candidate** → **O(R²)** dedup; then `related.sort()` (`:221`). |
| `SpineIndex::related(id)` | `:315-332` | better outer loop (only the doc's own tags, `:321-322`), but the same `!out.contains(other)` (`:325`) O(R) linear dedup → **O(R²)**; plus `self.docs.iter().find(\|(i,_,_,_)\| i == id)` (`:316`) = O(docs) scan to fetch the doc's tags; then `out.sort()` (`:330`). |
| `SpineIndex::lookup_by_id(id)` | `:296-302` | `self.docs.iter().any(\|(i,_,_,_)\| i == id)` (`:297`) = O(docs) linear scan — **acknowledged** in the doc comment (`:277-283`). |
| `build_map(docs)` | `:231-259` | `groups.iter_mut().find(\|(t,_)\| *t == tag)` (`:239`) inside the per-doc loop → O(docs · distinct-first-tags). |

**Already-correct, do NOT "fix" (verified):** the `bucket.sort(); bucket.dedup();` pairs in `tag_index`
(`:202-204`) and `SpineIndex::build` (`:287-289`) are **correct** — `dedup()` is called *after* `sort()`,
so it removes all duplicates (not just adjacent-in-arbitrary-order). These are not a bug; leave them.

**What `n` is (R3 §C2, verified):** knowledge-spine document count / tag-incidence. Per MEMORY the corpus
lives on a "no limit" Hetzner volume (`/mnt/volume-fsn1-1/dowiz-memory/`) and grows over sessions —
this **scales**, unlike the FSM/causal fixed structures. It is the **advisory** living-knowledge read
path (`SpineIndex` is the P1..P4 retrieval organ, `:1-11`), never money/order.

**Semantics that MUST be preserved (read from tests `:335-646`):**
- `backlinks`/`related` return the docs sharing ≥1 tag, **sorted ascending, excluding self** (tests
  `spine_backlinks_excludes_self_and_is_sorted:498`, `spine_related_returns_shared_tag_docs:604`).
- `lookup_by_id` returns a single-element `Vec` when found, empty otherwise (`:294-302`; test `:633-634`).
- `lookup_by_tag` is case-insensitive, returns the sorted bucket (`:304-311`; test
  `spine_lookup_by_tag_case_insensitive:563`).
- `build_map` output is byte-stable: `# Knowledge Map` header, sections sorted by tag, entries sorted by
  id, `## (untagged)` last, no trailing-whitespace drift (test `spine_map_grouped_by_tag_and_sorted:525`).
- `tag_index`/`generate_map`/`parse_frontmatter` determinism (tests `:459`, `:387`, `:340`).

### 0.3 The synthesis path mis-cite — corrected here (standard §2 item 1)

`SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.2 B2 row cites the finding as **`kernel/src/spine.rs:210-332`**.
That path is **wrong**: `kernel/src/spine.rs` (line 1: *"W2-7 — event-sourced hash-chain knowledge spine"*)
is a **different file** — a tamper-evident append-only hash-chain log — with **no** `backlinks`/`related`/
`lookup_by_id` API. The real B2 finding lives in **`kernel/src/retrieval/spine.rs`** (line 1: *"Knowledge-spine
validator + MAP.md generator (W3-3 / P1)"*), where `backlinks` starts at `:210` and `related`/`lookup_by_id`
at `:315`/`:296`. The underlying research **R3 cites it correctly** (`OPUS-PERF-KERNEL-AUDIT-2026-07-18.md`
§C2: "`kernel/src/retrieval/spine.rs`"). The synthesis dropped the `retrieval/` prefix. **P77 touches only
`kernel/src/retrieval/spine.rs`; it does NOT touch the top-level `kernel/src/spine.rs`.**

### 0.4 Both paths are UNBENCHED today (the honesty spine of this whole blueprint)

`kernel/benches/criterion.rs` (265 lines) registers benches with the house convention
`c.bench_function("<group>/<n>", …)` (e.g. `place_order/5_items:17`, `empirical_identify/20k_samples:91`,
`ppr/rank_32x32_k20:207`, `absorbing/fundamental_matrix_16:225`). A live grep for `spool`, `spine`, or
`drain` returns **nothing** — confirming R3's "Bench: none" for both C1 and C2. Therefore **the baseline
must be captured by this blueprint**; there is no prior number to compare against. The bench is authored
RED (proving the quadratic curve exists on the *old* code) → the fix flips it GREEN (curve goes linear).

### 0.5 The house data-layout pattern to mirror (standard §2 item 19)

`kernel/src/mat.rs:1-11` is the ONE contiguous-layout discipline in the kernel: *"a single contiguous
`Vec<f64>` laid out row-major so a matmul walks linear memory … This module is the ONE backing store."*
It fixes the identical bug class (`Vec<Vec<f64>>` pointer-chasing → contiguous) with the same values —
zero-dep, plain `std`, deterministic, `#[inline]` small accessors, **byte-identical output** (`matmul_contig`
preserves the `aik == 0` short-circuit exactly, `:130-131`). P77 mirrors `mat.rs`'s *stance*, not its type:
one backing store per queue/index, contiguous where the access is linear, no new dependency, output proven
identical to the old impl.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

Every fix is a standard-library data-structure swap. **Zero new dependencies; zero new abstractions.**

| Prior art | What it is | How P77 uses it — and what it does NOT take |
|---|---|---|
| **`std::collections::VecDeque`** | ring-buffer double-ended queue; `push_back`/`pop_front` are O(1) amortized; `Index<usize>` is logical-front-relative | **B1 backing store.** FIFO append = `push_back`; FIFO ack = `pop_front` (O(1), replacing `Vec::remove(0)`'s O(n) shift). **NOT taken:** `swap_remove`-style reordering — it would break FIFO order (R10/E3 verified the ABA/stale-index bug class *absent* precisely because nothing reorders; we keep it absent). |
| **A claim cursor (head index)** — the "sweep pointer" idiom | one monotone index that only moves forward past claimed records | **B1 `claim_next` O(1).** Replaces the `iter().position(!claimed)` re-scan. **NOT taken:** a second data structure — it is one `usize`. |
| **`std::collections::HashSet`** | O(1) amortized membership/insert | **B2 dedup accumulator.** Replaces `Vec::contains` O(R) membership in `backlinks`/`related` → O(R) total dedup instead of O(R²). Result is materialized to a sorted `Vec` at the end (order-identical). |
| **`std::collections::HashMap<String, usize>`** | O(1) amortized keyed lookup | **B2 id→index (and tag→group) lookup.** Replaces `docs.iter().find/any` O(docs) in `related`/`lookup_by_id` and `groups.iter_mut().find` O(distinct-tags) in `build_map`. R10 (`OPUS-PERF-POINTER-ARENA-ANALYSIS`) concurs: **`HashMap`, NOT a generational arena** — the stale-index/ABA bug class an arena guards is verified absent here (build-once index, keyed by logical id). |
| **`kernel/src/mat.rs` layout discipline** | one contiguous backing store, zero-dep, byte-identical output, `#[inline]` accessors | **Stance mirror** for both fixes (§0.5): one store per structure, output proven identical, no new dep. |
| **`kernel/benches/criterion.rs` `<group>/<n>` convention** | existing sweep-bench naming (`ppr/rank_32x32_k20`, `absorbing/fundamental_matrix_16`) | **B1/B2 benches register into it** — `spool_drain/<n>`, `spine_backlinks/<n>`. **Schema/gate mechanics deferred to P75** (§7.3), not redefined here. |

**Reuse-first conclusion:** every primitive is `std`; the pattern is already in-repo (`mat.rs`, and the
existing `event_log.rs:210` `HashSet` dedup R3 cites as the blessed shape). P77 **adds no dependency and
invents no abstraction** — it deletes quadratics by swapping to the right std container behind an unchanged
public API.

---

## 2. Scope — what P77 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P77 OWNS (exactly two source files + two bench registrations)

1. **`kernel/src/spool.rs`** — the internal representation swap (`Vec<Record>` → `VecDeque<Record>` + claim
   cursor, §3.1) and the O(1)-amortized `claim_next`/`ack`/`reclaim`/`compact_drop`. **Public API unchanged.**
2. **`kernel/src/retrieval/spine.rs`** — `HashSet` dedup accumulators in `backlinks`/`related`; a
   `HashMap<String, usize>` id→index on `SpineIndex` for `lookup_by_id`/`related`; `HashMap` grouping in
   `build_map`. **Public API + output unchanged.**
3. **`spool_drain/<n>` + `spine_backlinks/<n>` (and siblings) benches** — authored into
   `kernel/benches/criterion.rs`, registering into **P75's** `<group>/<n>` schema (§7).
4. **Equivalence + regression tests** proving old-impl-identical behavior (§4, §6).

### 2.2 P77 does NOT own (anti-scope — prevents collision & scope-creep)

- **`kernel/src/money.rs`** — R3 filed `ledger_sum` (`money.rs:230-245`) as **C4 INFO/LOW, NO ACTION**:
  `n ≤ 2` per-order by construction, the scans ARE the fail-closed conservation/idempotency probes,
  correctness-first on money-authority code. The synthesis §2 reconciliation makes this **binding**. P77
  **does not touch `money.rs`** (its optional bench-tripwire is P80's, not P77's).
- **`token_bucket.rs` / `budget.rs` / `admission.rs`** — R3 atomicity items A1–A3 are **BENCH-FIRST**; a
  Mutex→CAS rewrite with no contended bench "is not done" per the standing rule. Not P77's file, not P77's
  change (P80 adds the contended benches; a rewrite is a separate evidence-gated blueprint).
- **`spectral_laplacian.rs`** — R3 C3 densify is **BENCH-ONLY / deferred** (Tier D-4), gated on evidence
  the field-UI drives n>32. Not P77.
- **`SeqCst`→`Relaxed` on metric counters** — R3 **DECLINED** (E4): on x86-64 `fetch_add` is `lock xadd`
  regardless of ordering, zero instruction-level win. P77 does not touch atomics.
- **P79's files** — `kernel/src/causal.rs`, `kernel/src/spectral.rs`, `kernel/src/zerocopy.rs` (B5/B6 data-
  layout ports). **P77 and P79 are disjoint kernel-file lanes** (synthesis §5 collision guard) and may build
  fully in parallel after P75.
- **The top-level `kernel/src/spine.rs`** (the W2-7 hash-chain log) — a *different file* (§0.3), untouched.

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs (in tree):** `Spool`/`Record` (`spool.rs`), `SpineIndex`/`Frontmatter`/`SpineEntry`
(`retrieval/spine.rs`), `kernel/benches/criterion.rs` (bench harness), `std::collections::{VecDeque,
HashMap, HashSet}`. **No new crate.**

**Soft dependency — P75 (bench schema):** the `spool_drain`/`spine_backlinks` benches **land in P75's
`<group>/<n>` bench-id + baseline schema and gate mechanics** — P77 names the groups and sweep sizes but
**does not define the schema or the CI gate** (P75 owns those; §7.3). P77 can be *written* and its code
*built* independently of P75; only the *baseline-registration + regression-gate wiring* waits on P75's
schema landing (synthesis §5 Wave-1: "P75 soft — benches land in its schema").

**Consumers unchanged:** the JSONL-file drainer/adapter outside the kernel (`bounded_drainer.rs` and the
pure-std firewall adapter) calls `append`/`claim_next`/`ack` exactly as today; the living-knowledge retrieval
callers of `SpineIndex` see identical results. No caller edits.

### 2.4 Honest reconciliation with the "no premature optimization" rule (standard §2 item 6)

The CLAUDE.md ponytail/YAGNI stance is **explicitly overridden for performance work** by
`performance-priority-over-minimal-change-2026-07-17.md` — "for perf/speed/stability specifically, pursue
real compounding gains even if significant code changes needed … scope the reconciliation to performance
tradeoffs only." P77 is scoped exactly there: a genuine asymptotic fix (O(N²)→O(N)) on a path that scales
with real volume, justified by a criterion bench, behind an unchanged API. It does **not** gold-plate —
it does not add a lock-free variant, an arena, a generational index, or a config knob (all rejected in §1).

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

### 3.1 B1 — the new `Spool` internal representation (public API unchanged)

The shipped design is **Design A: `VecDeque<Record>` + a claim cursor.** It makes the *documented* strict-
FIFO drain O(1) per operation (the quadratic in the prescribed `spool_drain` bench is exactly the front-
removal shift, which `pop_front` deletes), keeps out-of-order ack correct, and adds **no** map-over-a-
shifting-container staleness trap. Design B (tombstoning) is spec'd as the named escalation and is NOT built
now.

```rust
// kernel/src/spool.rs — internal fields only; `pub struct Record` (:24-31) is UNCHANGED,
// and every `pub fn` signature (new/len/is_empty/is_full/append/claim_next/ack/reclaim/
// compact_drop/pending/in_flight) is UNCHANGED. Only the private fields + bodies change.

use std::collections::VecDeque;

#[derive(Debug, Clone, Default)]
pub struct Spool {
    /// FIFO ring of live (un-acked) records, front = oldest. Replaces `Vec<Record>` (:37).
    /// `push_back` on append; `pop_front` on FIFO ack; order-preserving `remove(pos)` only on
    /// the rare out-of-order ack/drop. `Index<usize>` stays logical-front-relative == old `Vec`.
    records: VecDeque<Record>,
    /// Claim cursor: the VecDeque index of the next candidate for `claim_next`. Monotone-forward
    /// except a `reclaim` that frees an earlier record rewinds it; a `pop_front` decrements it.
    /// Makes `claim_next` O(1) amortized instead of an O(n) `position(!claimed)` re-scan.
    claim_cursor: usize,
    next_id: u64,
    capacity: usize,
}
```

**Cursor invariants (the correctness contract, asserted by tests §4.1):**
- **CI-1** every record at index `< claim_cursor` is `claimed == true` (the cursor never leaves an
  un-claimed record behind it).
- **CI-2** on `pop_front` (front removed), all indices shift down by one ⇒ `claim_cursor =
  claim_cursor.saturating_sub(1)`.
- **CI-3** on order-preserving `remove(pos)` with `pos < claim_cursor`, `claim_cursor -= 1`.
- **CI-4** on `reclaim(id)` that un-claims the record at index `p`, `claim_cursor = min(claim_cursor, p)`
  so `claim_next` re-serves it (preserves `crash_reclaim_recovers_inflight`).
- **CI-5** `len()` / `is_full` / backpressure read the **un-acked count** = `records.len()` (identical to
  the old `Vec` semantics — `records` holds only un-acked records; ack physically removes).

**Design B (NAMED ESCALATION — do not build now).** If a future bench (`spool_ack_scatter/<n>`, §7.1) ever
proves *out-of-order* ack hot (it is not today — the production drainer is strict-FIFO front-ack), escalate
to `records: VecDeque<Option<Record>>` tombstones + `index: HashMap<u64, usize>` keyed on an **absolute
slot** + a `base: usize` front-offset (`physical = index[id] − base`) + amortized full-compaction when
`records.len() > COMPACT_LOAD_FACTOR * live` (`const COMPACT_LOAD_FACTOR: usize = 2`). This is the
`HashMap`-not-arena route R10 blesses; it makes *every* op O(1) amortized. It is recorded so the escalation
is designed, not improvised — but building it now would be gold-plating an un-hot path (§2.4).

### 3.2 B2 — the `SpineIndex` id-index field + dedup accumulator types

```rust
// kernel/src/retrieval/spine.rs — `SpineIndex` (:266-269) gains ONE field; the `docs`/`tag_index`
// fields and every `pub fn` signature are UNCHANGED.

use std::collections::{HashMap, HashSet};

pub struct SpineIndex {
    docs: Vec<(String, String, Vec<String>, String)>,     // unchanged
    tag_index: HashMap<String, Vec<String>>,              // unchanged
    /// id → position in `docs`, built once in `SpineIndex::build`. Turns `lookup_by_id`/`related`'s
    /// O(docs) `docs.iter().find/any` (:297,:316) into O(1). Ids are unique (doc comment :294).
    id_index: HashMap<String, usize>,                     // NEW
}
```

- **Dedup accumulator** (in `backlinks`/`related`): a local `let mut seen: HashSet<String>` replacing the
  `!vec.contains(other)` (`:215`,`:325`) O(R) membership; results collected then `sort()`ed → identical
  sorted, self-excluded `Vec<String>`.
- **Grouping accumulator** (in `build_map`): a local `let mut group_of: HashMap<String, usize>` replacing
  `groups.iter_mut().find(|(t,_)| *t==tag)` (`:239`), with the final `groups.sort_by(tag)` +
  per-group `ids.sort_by(id)` (`:244-247`) unchanged → byte-identical output.

**No new named constants** — B2 introduces no magic numbers; the only new *type* is the `id_index` field.
Sweep sizes for the benches are named in §7.1, not as source constants.

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first, a bench/test that goes RED (or captures the quadratic) before the change, code,
then GREEN.** State transitions modeled as event sequences; the equivalence tests assert on op-sequences,
not just end-state.

### 4.1 B1 — `spool.rs` VecDeque + claim-cursor O(1)-amortized FIFO

- **Spec:**
  - `append(payload)` — unchanged public behavior: reject with `None` at capacity (`records.len() >=
    capacity`), else `push_back(Record{ id, payload, claimed:false })`, return `Some(id)`. O(1).
  - `claim_next()` — advance `claim_cursor` forward over any `claimed` records (CI-1 guarantees they are
    a contiguous claimed prefix except after a reclaim rewind), return a **clone** of the first record at
    `>= claim_cursor` with `claimed==false`, set it `claimed=true`, leave the cursor at that index. O(1)
    amortized (the cursor advances at most N times total across a drain).
  - `ack(id)` — **FIFO fast-path:** if `records.front().map(|r| r.id) == Some(id)`, `pop_front()` (O(1)) and
    apply CI-2. **Out-of-order fallback:** `records.iter().position(|r| r.id==id)` then
    `records.remove(pos)` (order-preserving) and apply CI-3; return `false` on unknown id (fail-closed,
    unchanged). The FIFO drain hits only the fast-path ⇒ O(1)/op ⇒ **O(N) total** (was O(N²)).
  - `reclaim(id)` — find the record, if `claimed` set `claimed=false` and apply CI-4; return whether
    reclaimed. (Lookup stays a scan today; reclaim is the crash-recovery path, not the drain hot loop —
    Design B would make it O(1); not needed now.)
  - `compact_drop(id)` — same fast-path/fallback removal shape as `ack`, returning whether dropped.
  - `pending()`/`in_flight()` — unchanged O(live) filter-counts (diagnostic peeks, not the hot loop).
- **BASELINE-CAPTURE bench `spool_drain/<n>` (RED = quadratic curve on OLD code):** append `n`, then FIFO
  claim+ack all `n`; record wall-time across `n ∈ {16, 64, 256, 1024}`. On the **old** `Vec`+`remove(0)`
  impl the per-`n` time grows ~quadratically; on the **new** impl it grows ~linearly. This bench is the
  *only* proof the path was ever quadratic (§0.4). Land it **first**, capture the old baseline, then fix.
- **RED equivalence test `spool_equiv_random_ops` (differential, old-vs-new):** drive a fixed pseudo-random
  sequence of `append`/`claim_next`/`ack`/`reclaim`/`compact_drop` (seeded LCG, deterministic) against a
  **reference `Vec` impl** kept in the test module and the new `VecDeque` impl; after every op assert
  identical `len`/`pending`/`in_flight`/`is_full` and identical `claim_next` return-id order. RED if any op
  diverges. This is the byte-identical-behavior gate (§6 D-EQUIV).
- **Preserved GREEN tests (must stay verbatim through the public API):** `append_sequential_ids_and_len`,
  `claim_fifo_then_ack`, `backpressure_at_capacity`, `crash_reclaim_recovers_inflight`, `ack_unknown_is_noop`,
  `capacity_one_handshake` (`spool.rs:147-234`).
- **Adversarial cases (each a named RED test):**
  - `spool_ack_out_of_order_preserves_order` — claim 0,1,2; `ack(1)` (middle) → remaining are id 0 then 2,
    in order (exercises the fallback + CI-3; mirrors `claim_fifo_then_ack`).
  - `spool_reclaim_after_crash_rewinds_cursor` — claim 0, `reclaim(0)`, `claim_next` returns id 0 again
    (CI-4); asserts the cursor rewound, not skipped.
  - `spool_ack_nonexistent_id_noop` — `ack(999)` on a non-empty spool returns `false`, mutates nothing
    (len, cursor, order unchanged).
  - `spool_drain_under_backpressure_ordering` — fill to capacity, then interleave `append`(rejected)/
    `claim`/`ack` at capacity 1 and capacity K; assert every accepted id drains in strict FIFO and no id is
    lost or duplicated (the crash-safety + FIFO invariant under the exact backpressure condition `n` scales
    with).
  - `spool_reclaim_then_ack_out_of_order` — claim 0,1; reclaim 0; ack 1 (front is 0, so 1 is out-of-order)
    → 0 remains claimable, 1 gone (fallback + cursor interplay).

### 4.2 B2 — `retrieval/spine.rs` HashSet dedup + HashMap id-index

- **Spec:**
  - `SpineIndex::build(docs)` — additionally build `id_index: HashMap<String, usize>` mapping each doc's id
    to its position in `docs` (ids unique). O(docs) build (already iterating docs for `tag_index`).
  - `lookup_by_id(id)` — `if self.id_index.contains_key(id) { vec![id.to_string()] } else { Vec::new() }`.
    O(1). Output identical.
  - `related(id)` — fetch the doc's tags via `id_index` (O(1)) instead of `docs.iter().find` (`:316`);
    accumulate shared-tag doc ids into a `HashSet<String>` (skip self), then materialize to a `Vec` and
    `sort()`. Output identical (same set, same ascending sort, self excluded).
  - `backlinks(id, index)` — replace the `!related.contains(other)` (`:215`) linear dedup with a
    `HashSet<String>` accumulator; keep the `bucket.iter().any(|d| d==id)` membership but note it may use
    `bucket.binary_search(&id.to_string()).is_ok()` (buckets are sorted+deduped, `tag_index:202-204`) for
    O(log R) instead of O(R) — an optional micro-win, behavior-identical. Final `sort()` unchanged. Output
    identical.
  - `build_map(docs)` — replace `groups.iter_mut().find` (`:239`) with a `HashMap<String, usize>` group
    lookup; keep the final `groups.sort_by(tag)` + per-group `ids.sort_by(id)` (`:244-247`) so the emitted
    markdown is **byte-identical**.
- **BASELINE-CAPTURE benches (RED = quadratic/linear-in-docs curve on OLD code):**
  - `spine_backlinks/<n>` — build a corpus of `n` docs with overlapping tags, call `backlinks` on a high-
    degree id; sweep `n ∈ {16, 64, 256, 1024}`. Old `Vec::contains` dedup grows ~quadratically in the
    result degree; new `HashSet` grows ~linearly.
  - `spine_build_lookup/<n>` — `SpineIndex::build(n)` then `lookup_by_id` × n; old O(docs) scan → O(n²)
    aggregate, new O(1) → O(n). Sweep same sizes.
- **RED equivalence test `spine_equiv_reference`:** for a fixed corpus, assert the new `backlinks`/`related`/
  `lookup_by_id`/`build_map` return **`assert_eq!`-identical** values (same `Vec`, same order, same string)
  to a snapshot captured from the old impl (or a reference re-implementation in the test module). RED if any
  differs by element or order.
- **Preserved GREEN tests (verbatim):** `spine_backlinks_excludes_self_and_is_sorted:498`,
  `spine_related_returns_shared_tag_docs:604`, `spine_lookup_by_tag_case_insensitive:563`,
  `spine_map_grouped_by_tag_and_sorted:525`, `spine_tag_index_deterministic:459`,
  `spine_map_generation_sorted:387`, `spine_parse_frontmatter:340`.
- **Adversarial cases (each a named RED test):**
  - `spine_duplicate_backlinks_deduped` — a doc sharing the *same* other doc across *multiple* tags appears
    **exactly once** in `backlinks`/`related` (the HashSet must dedup exactly like `!contains`).
  - `spine_self_reference_excluded` — a doc tagged identically to itself never appears in its own
    `backlinks`/`related` (self-exclusion survives the HashSet rewrite).
  - `spine_large_corpus_order_stable` — build a 1024-doc corpus **twice in different insertion orders**;
    assert `backlinks`/`related`/`build_map` are byte-identical across both (HashMap iteration order must
    NOT leak into output — the final `sort()` is what guarantees determinism; this test fails if any
    accumulator's iteration order reaches the output un-sorted).
  - `spine_empty_and_isolated` — `backlinks` over an empty index and `related` of an isolated (unique-tag)
    doc both return `Vec::new()` (unchanged edge behavior).

---

## 5. Adversarial self-check — real effort to break the design (standard §2 item 3)

- **Does `VecDeque::Index` really match `Vec::Index` after a middle remove?** Yes — `VecDeque::remove(i)` is
  order-preserving and shifts toward the nearer end; `Index<usize>` returns the logical i-th from the front.
  After `ack(1)` on `[0,1,2]`, `records[0].id==0`, `records[1].id==2` — identical to `Vec`. The one white-box
  test (`claim_fifo_then_ack:173-174`) therefore passes unchanged (§0.1 NOTE). Verified by reasoning about
  the std contract; the equivalence test (`spool_equiv_random_ops`) proves it empirically.
- **Can the claim cursor desync from reality?** The five CI invariants (§3.1) are each covered by a named
  test (§4.1): rewind-on-reclaim (CI-4), decrement-on-pop_front (CI-2), decrement-on-middle-remove (CI-3),
  claimed-prefix (CI-1). The random-op differential test is the catch-all: any cursor bug surfaces as a
  divergent `claim_next` id order.
- **Does the FIFO fast-path/fallback split ever accept a wrong id or double-remove?** `ack` removes at most
  one record (fast-path pops the front *only if `front.id==id`*, else the fallback finds the unique id or
  returns `false`). Ids are unique (monotone `next_id`), so `position` finds at most one. `ack_nonexistent`
  and `ack_out_of_order` tests bound both branches.
- **Could the HashSet/HashMap rewrite leak nondeterminism into spine output?** This is the one real risk:
  `HashSet`/`HashMap` iterate in unspecified order. **Mitigation is structural** — every public method that
  returns a collection ends in an explicit `sort()` (`backlinks:221`, `related:330`) or a sorted-group emit
  (`build_map:244-247`); accumulators are used only for O(1) membership/lookup, never as the output order.
  `spine_large_corpus_order_stable` (§4.2) is the falsifier: it fails if any accumulator's iteration order
  reaches the output unsorted. This is the "smart index" (§8 item 14) for the nondeterminism bug class.
- **Is `is_full`/backpressure still exact after the swap?** `is_full` reads `records.len()` = un-acked count,
  identical to the old `Vec`. `backpressure_at_capacity` and `capacity_one_handshake` (preserved verbatim)
  are the falsifiers.
- **Latent-not-hot honesty:** if the D-BASELINE benches came back *flat* (no measurable quadratic at
  `n ≤ 1024`), the fix would still be correct and strictly-better-or-equal — but we would **say so** and
  down-rank it, not manufacture a speedup. The bench decides; the blueprint does not pre-decide.

---

## 6. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / bench / check) |
|---|---|---|
| D1 | `spool.rs` uses `VecDeque` + claim cursor; FIFO append/claim/ack are O(1)-amortized | `spool_drain/<n>` curve flips from ~quadratic (old) to ~linear (new) across `n∈{16,64,256,1024}` |
| D2 | every spool public-API semantic is byte-identical to the old impl | `spool_equiv_random_ops` (differential vs a reference `Vec` impl) + all six preserved tests (`:147-234`) GREEN |
| D3 | spool adversarial paths hold: out-of-order ack, reclaim-rewind, unknown-id no-op, backpressure ordering | `spool_ack_out_of_order_preserves_order`, `spool_reclaim_after_crash_rewinds_cursor`, `spool_ack_nonexistent_id_noop`, `spool_drain_under_backpressure_ordering`, `spool_reclaim_then_ack_out_of_order` |
| D4 | `retrieval/spine.rs` dedup is `HashSet`, `lookup_by_id`/`related` use the `id_index` HashMap, `build_map` uses a HashMap group lookup | `spine_backlinks/<n>` + `spine_build_lookup/<n>` curves flip from ~quadratic-in-docs to ~linear |
| D5 | every spine public method returns a **set-and-order-identical** result to the old impl | `spine_equiv_reference` (assert_eq vs snapshot/reference) + all seven preserved tests GREEN |
| D6 | spine adversarial paths hold: duplicate dedup, self-exclusion, large-corpus determinism, empty/isolated | `spine_duplicate_backlinks_deduped`, `spine_self_reference_excluded`, `spine_large_corpus_order_stable`, `spine_empty_and_isolated` |
| D-EQUIV | **behavior is provably identical to the old impl** (the #1 invariant) | D2 `spool_equiv_random_ops` + D5 `spine_equiv_reference` — both differential/snapshot, not prose |
| D-BASELINE | the quadratic curve is **captured as a real number**, not asserted | the four new benches produce old + new numbers; the before/after ratio is recorded in `BENCH_RESULTS.md` (per P75's schema, §7.3) |
| D-NOREG | the small-`n` end is a **tie-or-win**, never a regression | `spool_drain/16` and `spine_backlinks/16` new-time ≤ old-time (within the P75 gate's noise band) |
| D-BUILD | the kernel builds and `cargo test --lib` is fully green incl. all new REDs now GREEN, **no dep added** | `cargo test -p <kernel-crate> --lib`; `cargo tree` shows no new dependency |

---

## 7. Benchmarks + telemetry + the measure-first discipline (standard §2 item 10)

Per the standing rule, **the bench is the evidence** — these paths are unbenched (§0.4), so the baseline
must be *captured*, not assumed. A **measured before/after number** is a DoD row (D-BASELINE), not an
estimate.

### 7.1 What to measure (bench groups + honest sweep sizes)

| Bench group | Measures | Sweep `n` | Expected old → new |
|---|---|---|---|
| `spool_drain/<n>` | append `n` then FIFO claim+ack all `n` (the real drainer shape) | `{16, 64, 256, 1024}` | quadratic → linear |
| `spool_ack_scatter/<n>` *(optional, gates Design B)* | append `n`, claim all, ack in a scattered (non-FIFO) order | `{16, 64, 256, 1024}` | quadratic → quadratic-ish *(Design A does not fix this; it stays correct. If this ever benches hot, escalate to Design B, §3.1.)* |
| `spine_backlinks/<n>` | build an `n`-doc overlapping-tag corpus, `backlinks` on a high-degree id | `{16, 64, 256, 1024}` | quadratic → linear |
| `spine_build_lookup/<n>` | `SpineIndex::build(n)` + `lookup_by_id × n` | `{16, 64, 256, 1024}` | quadratic → linear |

Sweep sizes chosen to **expose the curve honestly**: `16` is the realistic small-queue / small-corpus
anchor (the D-NOREG tie point), `1024` is large enough that an O(N²) term dominates wall-clock and a
linear fix is unmistakable, `64`/`256` fill the curve so a reviewer sees the shape (not two points). These
mirror the existing sweep style (`ppr/rank_32x32_k20`, `absorbing/fundamental_matrix_16`).

### 7.2 Telemetry

No new telemetry surface is required (these are pure in-memory state machines, off the network/IO path).
The **bench trend** IS the regression detector: once the baselines are committed into P75's schema, a future
edit that reintroduces a quadratic (e.g. a `Vec::remove` creeping back) trips the P75 gate automatically,
not at review time (item 14). The spool's real production cost is already observable through
`bounded_drainer.rs`'s TokenBucket debit accounting — unchanged by P77.

### 7.3 Bench-schema ownership — DEFERRED to P75 (cross-reference constraint)

**P77 does NOT own the bench-id/baseline schema or the sweep convention.** P75 (CI bench-regression gate
re-architecture, synthesis §5 Wave-0) owns the `<group>/<n>` naming, the `--save-baseline`/`--baseline`
same-runner A/B mechanics, the per-bench thresholds, and the committed-trend storage (`BENCH_HISTORY.md` /
`bench.jsonl`). P77 **registers into** that schema: it names the groups (`spool_drain`, `spine_backlinks`,
…) and the sweep sizes, and authors the `c.bench_function("<group>/<n>", …)` entries in
`kernel/benches/criterion.rs` — but the gate wiring, threshold, and baseline-commit are P75's. Sequence
(synthesis §5): P77 is **Wave-1**, P75 is **Wave-0 (soft dependency)** — "benches land in P75's schema."
P77's code can build before P75; the baseline-registration + regression-gate step waits on P75's schema.

---

## 8. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16, 20)

- **Hazard-safety as math (item 6):** the only reachable "unsafe" state is a **behavioral divergence** from
  the old impl (a different drain order, a lost/duplicated record, a different spine result set). It is made
  unrepresentable-by-test, not by prose: the differential `spool_equiv_random_ops` and snapshot
  `spine_equiv_reference` exhaustively pin the public contract; the five cursor invariants (CI-1..CI-5) are
  each falsified by a named test. There is **no money/auth/RLS/crypto surface** in either file (both are
  pure `std` state machines) — the hazard class is "wrong answer," fully covered by equivalence.
- **Schemas & scaling axis (item 8):** **spool** scaling axis = outbox backlog depth `N` (pending + in-
  flight un-acked records); shape changes only if *out-of-order* ack becomes hot (→ Design B tombstoning at
  `records.len() > 2·live`), stated with its trigger, not timeless. **spine** scaling axis = corpus doc
  count + tag-incidence `R`; `HashSet`/`HashMap` are O(1)-amortized to millions of docs; it would only need
  re-shape (e.g. a persisted on-disk index) if the corpus exceeds RAM — well beyond today's session corpus.
- **Isolation / bulkhead (item 11):** both are leaf pure-`std` modules with no shared mutable state; a bug
  cannot propagate beyond the queue/index it owns. The spool's failure mode is already bulkheaded by
  `bounded_drainer.rs` (degrade-closed: stop draining when the TokenBucket can't pay) — P77 does not touch
  that boundary. The spine is the **advisory** retrieval path; a wrong backlink degrades a suggestion, never
  a money/order decision.
- **Mesh awareness (item 12):** **N/A, honestly** — both are node-local in-memory data structures with no
  gossip/transport/store-and-forward surface. The spool is the pure half of the async work queue; its I/O
  adapter (JSONL marshal + drainer) lives *outside* the kernel (`spool.rs:11-14`) and is untouched. Stated,
  not shoehorned.
- **Rollback / self-healing as math (item 13):** **Self-termination** — the spool's `is_full` backpressure
  invariant (a queue past capacity is unrepresentable: `append` returns `None`) is preserved exactly.
  **Snapshot re-entry** — the spool's crash-recovery (`reclaim` a claimed-but-unacked record and re-serve
  it, unchanged) IS the cheap regenerative recovery; the cursor-rewind (CI-4) keeps it correct. **Self-
  healing is NOT claimed** — there is no error-correcting redundancy here, and claiming it would be false.
- **Error-propagation / smart index (item 14):** the bug classes P77 could introduce are turned into
  automatic failures: (a) a reintroduced quadratic → the P75 bench gate trips (§7.2); (b) spine output
  nondeterminism → `spine_large_corpus_order_stable` fails in CI; (c) a spool ordering/loss bug →
  `spool_equiv_random_ops` diverges. Compile/CI-time, not runtime surprises.
- **Living-memory awareness (item 15) — genuinely relevant to B2:** `retrieval/spine.rs` **is** the
  living-knowledge organ — the P1..P4 seed of the knowledge-spine retrieval layer over the MEMORY corpus
  (`spine.rs:1-11`), whose data has exactly the temporal/topological access pattern
  `internal-retrieval-living-memory-arc-2026-07-14` describes (the 4-layer trigram/BM25/HNSW/diffusion
  stack; living-memory as a growing graph, recall as personalized PageRank). B2's `HashMap` id-index and
  `HashSet` dedup are the O(1) substrate that keeps `backlinks`/`related` — the **topological** (tag-graph)
  access — cheap as the corpus grows unbounded on the "no limit" Hetzner volume (MEMORY: SELF-DEVELOPMENT
  note). This is why B2 is worth doing *before* the corpus is large, even though it is advisory: it is on
  the living-memory hot-as-it-grows path, cross-referenced to that arc. **B1's spool** has no living-memory
  role (it is a transient work queue) — N/A there, honestly.
- **Tensor/spectral (item 16):** **N/A, honestly** — a FIFO queue and a tag-graph dedup are not linear-
  algebra kernels; forcing `spectral.rs`/`mat.rs`'s numeric machinery here would be over-engineering
  (ponytail). The `mat.rs` connection P77 draws is a **layout-discipline stance** (one contiguous store,
  byte-identical output), not a tensor representation. Stated.
- **Linux engineering discipline (item 9):** **REINFORCES** — swapping to the right std container behind an
  unchanged API and proving it with a differential test is textbook "don't reinvent, use the standard
  structure, prove equivalence." **ALREADY-EQUIVALENT** — the `HashSet`-dedup shape is already blessed in
  `event_log.rs:210` (R3's cited exemplar). **DOES-NOT-TRANSFER** — no kernel-module/driver analogue; these
  are pure data structures. **EXTENDS** nothing new; it deletes complexity.
- **Hermetic principles (item 20):** **Correspondence** — the internal representation now *corresponds* to
  the access pattern ("as the FIFO usage, so the FIFO structure"; "as the tag-graph query, so the hashed
  index"), the same "as above, so below" that `mat.rs` embodies. **Cause & Effect** — every quadratic is a
  determinate cause of latency-under-scale; the fix removes the cause, and the bench makes the effect
  measurable. **Polarity / no-middle** — behavior is either byte-identical to the old impl or the
  equivalence test is RED; there is no "mostly the same" middle.

---

## 9. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied (or honest N/A) |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (spool + retrieval/spine methods; the `spine.rs` path correction; unbenched-today proof) |
| 2 | Falsifiable DoD | §6 (D1–D-BUILD, each a bench/test/check) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per B1/B2; equivalence asserts on op-sequences) |
| 4 | Predefined types & constants | §3 (new `Spool` fields + CI invariants; `SpineIndex.id_index`; accumulator types) named before impl |
| 5 | Adversarial/breaking tests | §4 (out-of-order ack, reclaim-rewind, duplicate backlinks, self-ref, large-corpus determinism), §5 |
| 6 | Hazard-safety from structure | §8 (divergence-from-old-impl made unrepresentable-by-test; no money/auth/crypto surface) |
| 7 | Links to docs & memory | §11 |
| 8 | Schemas with scaling axis | §8 (spool backlog `N`; spine corpus `R`; Design-B trigger stated) |
| 9 | Linux engineering discipline | §8 (REINFORCES/ALREADY-EQUIVALENT/DOES-NOT-TRANSFER verdict) |
| 10 | Benchmarks + telemetry + measure-first | §7 (four new benches; baseline-must-be-captured; before/after ratio recorded) |
| 11 | Isolation / bulkhead | §8 (leaf pure-std modules; spool bulkheaded by `bounded_drainer`) |
| 12 | Mesh awareness | §8 (N/A, honestly — node-local in-memory, no transport) |
| 13 | Rollback/self-heal as math | §8 (self-termination = backpressure invariant; snapshot re-entry = reclaim; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §8 (P75 bench gate + `spine_large_corpus_order_stable` + `spool_equiv_random_ops` = compile/CI-time catches) |
| 15 | Living-memory awareness | §8 (B2 IS the living-knowledge spine — cross-ref `internal-retrieval-living-memory-arc-2026-07-14`; B1 N/A honestly) |
| 16 | Tensor/spectral where applicable | §8 (N/A, honestly — FIFO/dedup are not linear-algebra; `mat.rs` used as layout stance only) |
| 17 | Regression tracking | §11 (REGRESSION-LEDGER entries for both fixes; the equivalence + bench tests stay permanently) |
| 18 | Clear worker instructions | §11 (exact file targets, order, acceptance path) |
| 19 | Reuse-first, upgrade-if-needed | §1 (all primitives `std`; `mat.rs`/`event_log.rs` in-repo exemplars; no new dep, no new abstraction) |
| 20 | Hermetic principles | §8 (Correspondence / Cause-&-Effect / Polarity) |

---

## 10. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-KERNEL-AUDIT-2026-07-18.md` ("R3") §C1 (spool, rank #1 real O(N²)), §C2 (spine),
  §"Priority ranking" (spool rank 1, spine rank 3), §C4 (money.rs INFO-only — do NOT touch), §A1–A3 (locks
  bench-first — not P77), Non-findings (SeqCst→Relaxed declined).
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.2 (Tier-B B1/B2 rows — note the §3.2 path mis-cite this
  blueprint corrects, §0.3), §5 Wave-1 (P77 ∥ P79 after P75; disjoint kernel files), §2 (money.rs binding
  no-action), §6 E4 (SeqCst declined).
- `MASTER-STATUS-LEDGER-2026-07-19.md` §3 Wave-1 (dowiz lane, P77 ∥ P79), §4 item 3 (P77 on the
  blueprint-writing work list).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`.
- **P75** (soft dependency — owns the `<group>/<n>` bench schema + gate; §7.3).
- Memory: `internal-retrieval-living-memory-arc-2026-07-14` (B2's living-memory context, item 15),
  `performance-priority-over-minimal-change-2026-07-17.md` (the perf-priority override of ponytail/YAGNI,
  §2.4), `pattern-ledger-2026-07-18.md` (record the "unbenched-path → bench-first" pattern on close).

**Existing code this blueprint edits (exact targets, `dowiz` kernel — NOT bebop):**
- **EDIT** `kernel/src/spool.rs` — private fields `Vec<Record>`→`VecDeque<Record>` + `claim_cursor`
  (§3.1); `claim_next`/`ack`/`reclaim`/`compact_drop` bodies to O(1)-amortized (CI-1..CI-5). **Every `pub`
  signature and `pub struct Record` unchanged.** Add the §4.1 tests.
- **EDIT** `kernel/src/retrieval/spine.rs` — add `SpineIndex.id_index: HashMap<String,usize>` (built in
  `build`); `backlinks`/`related` `HashSet` dedup; `lookup_by_id`/`related` via `id_index`; `build_map`
  HashMap group lookup. **Every `pub` signature + all outputs unchanged.** Add the §4.2 tests.
- **EDIT** `kernel/benches/criterion.rs` — register `spool_drain/<n>`, `spine_backlinks/<n>`,
  `spine_build_lookup/<n>` (and optional `spool_ack_scatter/<n>`) via `c.bench_function("<group>/<n>", …)`
  into **P75's** schema.
- **DO NOT TOUCH** `kernel/src/money.rs`, `token_bucket.rs`, `budget.rs`, `admission.rs`,
  `spectral_laplacian.rs`, the top-level `kernel/src/spine.rs`, or **any P79 file** (`causal.rs`,
  `spectral.rs`, `zerocopy.rs`).

**For the worker with zero session context — exact acceptance path:**
1. **Author the four baseline benches FIRST** (`spool_drain/<n>`, `spine_backlinks/<n>`,
   `spine_build_lookup/<n>`, optional `spool_ack_scatter/<n>`) against the *old* code and capture the
   numbers — this proves the quadratic exists (§0.4). If a curve is *flat* at `n ≤ 1024`, **say so** and
   down-rank that fix; do not manufacture a speedup.
2. **B1:** swap `spool.rs` to `VecDeque` + `claim_cursor` (§3.1); write the §4.1 RED tests
   (`spool_equiv_random_ops` differential + the five adversarial cases) — they fail before the fix, pass
   after; keep the six existing tests (`:147-234`) verbatim GREEN. `spool_drain/<n>` must flip
   quadratic→linear and `spool_drain/16` must be a **tie-or-win** (D-NOREG).
3. **B2:** add `id_index` + `HashSet` dedup to `retrieval/spine.rs` (§3.2); write `spine_equiv_reference` +
   the four adversarial cases; keep the seven existing tests verbatim GREEN. `spine_large_corpus_order_stable`
   is the determinism gate — output must be byte-identical across insertion orders.
4. `cargo test -p <kernel-crate> --lib` fully green (all new REDs now GREEN, six + seven preserved tests
   GREEN); `cargo tree` shows **no new dependency**.
5. Add two `docs/regressions/REGRESSION-LEDGER.md` rows: "spool FIFO drain O(N²)→O(N), equiv-pinned" and
   "spine backlinks O(R²)→O(R), determinism-pinned."
6. **Register the benches into P75's schema** (do not invent a competing convention); commit the captured
   before/after baseline per P75's `--save-baseline` mechanics (§7.3).
7. Anti-scope: do NOT touch `money.rs` (C4 binding no-action), the locks (A1–A3 bench-first), the atomics
   (E4 declined), the top-level `spine.rs`, or any P79 file. Do NOT add a dependency, an arena, a lock-free
   variant, or a config knob (all rejected in §1). Behavior byte-identical to the old impl is the #1
   invariant — the equivalence tests are the proof, not the prose.
