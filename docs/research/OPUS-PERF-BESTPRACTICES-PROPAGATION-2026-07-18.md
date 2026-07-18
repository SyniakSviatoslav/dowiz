# Best-Practice Propagation Audit — event-driven · DoD · TDD · concurrency

**Date:** 2026-07-18 · **Model:** Opus 4.8 · **Scope:** dowiz/DeliveryOS Rust kernel + engine +
`bebop-repo` (`crates/bebop`, `bebop2/`). Feeds a later blueprint-writing pass.

**Method.** For each of the four disciplines, first anchor the *good* pattern already proven in this
tree (read first-hand, cited `file:line`), then grep the rest of the tree for structures that violate
it, then keep only the violations that are a **real** gap — cross-referenced against
`kernel/benches/criterion.rs` (the authoritative hot-path list) for DoD/concurrency, against role
(money/order/dispatch/crypto) for TDD, and against actual call sites/re-entrancy for the rest.
Speculative or cold-path candidates are listed and **rejected** with reasons (Appendix A) so a
blueprint author does not re-investigate them.

**Verdict up front.** The dowiz **kernel** is the reference implementation for all four disciplines —
`event_log.rs`, `mat.rs`/`csr.rs`/`arena.rs`, `money.rs`, and the admission/`token_bucket` atomics are
each a battle-tested exemplar. The genuine gaps cluster in two places: (1) the **`bebop-repo` crates**
(a money ledger that mutates balances in place, a "ledger" that is counter-only, a pub/sub bus that
holds its lock across dispatch), and (2) a **CI/build-config defect** — the `bebop2/delivery-domain`
settlement/intake tests exist but are gated OFF in the default `cargo test`. Inside the kernel, the
only real residual gaps are small (`budget.rs`) or latent (`ppr.rs`, `PgStore`). The propagation plan
(§5) is therefore ranked: cheapest-highest-impact first is the TDD gate fix, then the bebop ledger and
bus, then the one benchmark-backed DoD rewrite (`causal.rs`).

---

## 1. Event-driven / event-sourcing

### 1.1 The gold standard (proven in-repo)
`kernel/src/event_log.rs` — a per-node, content-addressed, append-only log. The load-bearing
properties:

- **Content-addressing = idempotency key.** `MeshEvent::event_id() = SHA3-256(prev ‖ actor_pubkey ‖
  actor_seq ‖ payload)` (`event_log.rs:148-155`). A duplicate is a *structural* no-op, not a TTL
  dedup (`event_log.rs:6-7`).
- **Decide-before-commit, idempotent on replay.** `commit_after_decide` dedups on the raw content-id
  first (`event_log.rs:378-381`), runs `decide` only if new (`event_log.rs:384-385`), and persists
  under the *same* raw id via `append_raw` — never re-running the side effect on replay
  (`event_log.rs:366-391`; the money-law reason is spelled out `event_log.rs:349-365`: a replayed
  `SettlementClaimed` must never re-run its hashlock).
- **Typed failure poles.** `CommitError::{Rejected, Store}` (`event_log.rs:270-275`) — a Law rejection
  (never retry) is categorically distinct from a durability fault (retry/alarm). `StoreError`
  enumerates the four durable-append failure points (`event_log.rs:167-176`).
- **Replayability / tamper-evidence.** The store keeps full event **bodies** (`MemEventStore.by_event`,
  `event_log.rs:213`) so `verify_chain()` (`event_log.rs:475`) can detect corruption at rest.

Four more legitimate instances of this exact shape already exist and are **correct** (cite as
secondary exemplars, do not "fix"): `kernel/src/mesh.rs:193-225` (signed prev-hash-chained log),
`kernel/src/spine.rs:147-199` (tamper-evident knowledge chain), `kernel/src/pq/codesign.rs:75-88`
(`ApplyLedger` = explicit audit/replay guard), and — most important — the **settlement-money
authority** `kernel/src/ports/payment.rs:181-226` (`SettlementState::fold_event` keeps
`events: Vec<SettlementEvent>` + a derived projection; `decide_settlement` at `payment.rs:367` appends
nothing, the caller folds only on `Recorded`). Order money in `domain.rs` is threaded **by value**
through pure decide/fold (`place_order`/`apply_event`/`compensate` all *return* new `Order`s —
`domain.rs:256-277, 377-400`), which is event-sourcing-compatible and not a gap.

### 1.2 Real gaps

**G-E1 — bebop money ledger mutates balances in place, discards the transfer log. (STRONG)**
`bebop-repo/crates/bebop/src/ledger.rs:109-111`:
```rust
self.accounts[from_idx].balance -= t.amount;
self.accounts[to_idx].balance   += t.amount;
self.applied.insert(id);
```
This is the closest analogue to the good pattern that stops halfway. It *has* content-addressed
idempotency (`transfer_id = H(from‖to‖amount‖nonce)`, `ledger.rs:38-49`; `applied: HashSet<String>`,
`ledger.rs:55`) and a conservation invariant (`conserved`, `ledger.rs:79-81`), but `applied` retains
only transfer **ids** — never the `Transfer` bodies — so the ledger cannot replay itself or produce an
audit trail of *which* movements produced the current balances. Balances (i128 money) are the source
of truth and are mutated directly. The module doc (`ledger.rs:9-13`) assumes an *external*
event-sourced substrate holds the transfer log, but the module ships none, so standalone it is not
auditable/replayable. **Why it's a real gap, not style:** money state with idempotency-but-no-history
is exactly the class `event_log.rs` + `ports/payment.rs` were built to close.

**G-E2 — bebop reputation "ledger" is counter-only, and is a NO-COURIER-SCORING red-line divergence. (MEDIUM, two distinct concerns)**
`bebop-repo/crates/bebop/src/reputation.rs:39-46`:
```rust
self.records.entry(node.to_string()).or_default().deliveries += 1;  // record_delivery
r.suspensions += 1;                                                  // record_suspension
```
The doc claims "Deterministic, additive, fully auditable" and names it a "ledger"
(`reputation.rs:15`), but only aggregate counters are stored (`TrustRecord`, `reputation.rs:30-33`);
there is no append-only record of the POD proofs/suspensions that moved trust, and `decay`
(`reputation.rs:55`) mutates it lossily — so you cannot audit or replay how a node reached its score.
**Second, separate concern for the blueprint author:** this is *courier/node scoring*, which the dowiz
kernel explicitly forbids (`event_log.rs:22` NO-COURIER-SCORING; `domain.rs:39-40`; project memory
records dormant `no-courier-scoring` guards). Trust in the canonical stance is a signed *capability*,
never a reputation score. So `reputation.rs` is simultaneously an event-sourcing gap **and** a
governance red-line divergence — flag both.

**G-E3 — kernel compute-budget spend accumulator, shared, no per-debit trail. (LITE)**
`kernel/src/budget.rs:117` `self.spent += amount;` on `ComputeBudget.spent: f64`
(`budget.rs:86-89`), wrapped in a `Mutex` and shared via `BudgetedJobPort` (`budget.rs:133`). It is
correctly degrade-closed (`budget.rs:114-119`) so the *invariant* is safe, but it is financial (cost)
state mutated in place with no event/audit trail of individual debits. Lower severity because it is a
resettable monthly runtime budget, not order money.

**G-E4 — bebop entropy-budget "ledger", same accumulator-only shape. (LITE)**
`bebop-repo/crates/bebop/src/entropy_ledger.rs:155, 181` — content-addressed idempotency
(`applied.insert(id)`) but the `debt` accumulator is mutated in place and entry bodies are not
retained as a replayable log. Self-improvement accounting, lowest business impact.

*(Borderline, not counted: `domain.rs:98,116` mutates an **owned local** `Order` value, not a shared
store — value-passing decide/fold, replay-protected inside `money.rs::ledger_append` at
`money.rs:190`. The only residual note is that per-order money movements are not *separately*
committed through `event_log::commit_after_decide`, so a caller wanting cross-restart order-level
replay must event-source them itself.)*

### 1.3 Propagation recommendation
Do **exactly what `ports/payment.rs::SettlementState` already does** — retain event bodies + a derived
projection, decide-before-fold — rather than invent anything.
- **G-E1 (bebop `ledger.rs`):** give `Ledger` an append-only `Vec<Transfer>` (or fold over an external
  `EventLog<S>`), make balances a *derived projection* re-foldable from the transfer log, and keep the
  existing `transfer_id` as the content-id/idempotency key. This is the settlement-money pattern ported
  one crate over.
- **G-E2 (bebop `reputation.rs`):** first resolve the governance question (does node scoring belong at
  all, given NO-COURIER-SCORING?). If it stays, store the POD/suspension **events** append-only and
  derive the counters; if it goes, delete per the red-line. Blueprint should route this through the
  governance owner, not treat it as a pure refactor.
- **G-E3 (`budget.rs`):** optional — record debits as append-only entries if per-debit audit is wanted;
  low priority given it is resettable runtime state.

---

## 2. Data-oriented design (AoS → SoA)

### 2.1 The gold standard (proven in-repo)
- `kernel/src/mat.rs` — one contiguous row-major `Vec<f64>`, element `(i,j)` at `data[i*ncols+j]`
  (`mat.rs:12-21, 67-75`); `matmul_contig` strides linear memory and auto-vectorizes
  (`mat.rs:129-151`); arena-aware twin `matmul_contig_in` (`mat.rs:161-185`). The module header
  (`mat.rs:3-8`) states its own reason for existing: replace the historical `Vec<Vec<f64>>`
  pointer-chase.
- `kernel/src/csr.rs` — parallel `row_ptr`/`col_idx`/`val` SoA sparse layout (`csr.rs:39-55`), with an
  arena-aware `row_normalize_in` (`csr.rs:157`).
- `kernel/src/arena.rs` — `BumpArena`, O(1) reset, degrade-closed to heap on exhaustion
  (`arena.rs:37-125`).
- `kernel/src/simd.rs` — `f64x4` struct-of-arrays SIMD lane + the N-courier **Kalman SoA consumer**,
  with a bit-identity parity test *and* a measured speedup bench that gates regressions
  (`simd.rs:190-360, 630-710`).

**The propagation already happened once and is benchmarked:** the PageRank rebuild path has a
`heap` vs `arena` bench (`kernel/benches/criterion.rs:151-185`) proving the CSR+arena SoA path against
the Vec-of-Vec heap path; `engine/src/field_energy.rs:35-119` is already all `Csr`+`Incidence`+flat
`Vec<f64>`. So this is a pattern with an in-repo track record, not a proposal.

### 2.2 Real gaps (ranked by benchmark-backed impact)

**G-D1 — `causal.rs` samples are an AoS of 20 000 tiny heap Vecs. (TOP — benched at production scale)**
`Joint::from_samples(cards, samples: &[Vec<usize>])` (`causal.rs:1056`), fed by `sample_backdoor`
which does `rows.push(vec![x, z, y])` per sample (`causal.rs:1307-1321`) → 20 000 separately
heap-allocated 3-element `Vec<usize>`. **Hotness is real and at the benched size:** both
`empirical_identify/20k_samples` and `empirical_identify/end_to_end_20k`
(`criterion.rs:73-91`, `sample_backdoor(20_000, …)` at `criterion.rs:80/87`). The hot consumers walk
all 20k scattered rows — the validate loop `for row in samples { for (i,&v) in row.iter() … }`
(`causal.rs:1061-1077`) and the count loop `counts[encode_static(&cards,row)] += 1`
(`causal.rs:1079-1081`), plus `infer_cards` (`causal.rs:1366`); `end_to_end_20k` additionally pays 20k
`vec![x,z,y]` allocations. **Rewrite:** a flat row-major sample matrix `struct Samples { n_cols:
usize, data: Vec<usize> }` (the `Mat` pattern for `usize`), row `i = data[i*n_cols..(i+1)*n_cols]` —
one allocation instead of 20k, linear-stride counting. **Win:** the largest absolute allocation-count
win in the audit, directly visible on the two existing benches. Confidence: HIGH.

**G-D2 — `retrieval/ppr.rs` dense transition matrix is `Vec<Vec<f64>>`. (LATENT — do it with the scaled path, not standalone)**
`struct Ppr { n, w: Vec<Vec<f64>> }` (`ppr.rs:20-23`, self-labelled "dense `Vec<Vec<f64>>`"); the hot
inner loop `nxt[j] += pii * ((1.0-alpha) * self.w[i][j])` (`ppr.rs:49-56`) chases a per-row heap
pointer and breaks the contiguous `j` stride that would auto-vectorize. Structurally this is the exact
anti-pattern `mat.rs` exists to cure, and it *is* benched (`ppr/rank_32x32_k20`,
`criterion.rs:186-200`). **However** — cross-referencing the sibling analysis
`docs/research/OPUS-PERF-PPR-ANALYSIS-2026-07-18.md`: the only production caller is `diffusion.rs`
(`wiki_ppr`/`related`) at **n=20 over a frozen fixture**, and the kernel already owns a deterministic
*sparse* PPR (`csr.rs::personalized_pagerank`) for the scaled case. So the measurable win today is
absent — the `mat::Mat` swap is trivially correct and low-risk but should be scheduled **with** the
scaled retrieval path (or when `n` grows), not as an isolated perf change. Confidence: HIGH that it is
the anti-pattern; HIGH (per sibling doc) that it is currently latent. Blueprint: bundle, don't rush.

**G-D3 — `retrieval/bm25.rs` per-doc term-frequency is `Vec<HashMap<String,u32>>`. (LATENT — corpus-scale only)**
`tf: Vec<HashMap<String,u32>>` + `df: HashMap<String,u32>` (`bm25.rs:121-125`); the benched
`recall_at_k` → `Bm25::rank` → `score_doc` does a String-keyed HashMap lookup per query term in the
inner loop (`bm25.rs:208`, benched via `criterion.rs:222-227`). **But** the benched corpus is only 12
short docs (`recall.rs:45-70`), so the String-hash-per-doc-per-term cost is unmeasurable today; the SoA
win (intern tokens → `u32`, store `tf` as a CSR posting layout `doc_ptr`/`term_id`/`count`, `df` as
`Vec<u32>`) materializes only at production corpus scale. Weak flag by the "no speculative rewrite"
rule; record the revisit threshold rather than act now.

### 2.3 Propagation recommendation
Reuse `mat::Mat` / `csr::Csr` / `BumpArena` verbatim — all three are proven and benched.
1. **G-D1 now:** convert `causal.rs` samples to a flat `Vec<usize>` matrix; it is the one candidate
   with a benchmark that will *move* on the change. Add/keep the `empirical_identify` benches as the
   regression gate.
2. **G-D2 bundled:** `ppr.rs` → `mat::Mat` when the scaled retrieval path lands (cross-ref the PPR
   analysis doc; do not present as a standalone speedup).
3. **G-D3 deferred:** write down a corpus-size threshold in `bm25.rs`; convert to interned CSR postings
   only when a real corpus crosses it.

---

## 3. TDD discipline (RED→GREEN falsifiable-test hygiene)

Census for framing: the kernel alone carries **767 `#[test]`** across **88** `#[cfg(test)]` files.

### 3.1 The gold standard (proven in-repo, with git RED→GREEN pairs)
- `kernel/src/money.rs` — **FLAGSHIP.** 854 LOC; `#[cfg(test)]` at `money.rs:435`; ~420 test lines
  (≈49%); 31 `#[test]`. Tests named to the falsifiable convention: `red_money_add_overflow_is_err`,
  `red_tax_negative_rate_is_err_not_divzero` (`money.rs:543`), `red_tax_i128_overflow_is_err_not_panic`
  (`money.rs:555`), `red_ledger_duplicate_entry_id_is_err`, plus KAT parity
  `apply_tax_generated_parity_exact_integers`. **Git RED→GREEN pair:** `afde0fd05` ("RED: … exact-integer
  parity pin (fails vs Ok(0))") → `b2801d313` ("GREEN: … parity pin passing"); reinforcing test-first
  fixes `89017c482`, `96f51d249`, `ee6c96394`.
- `kernel/src/domain.rs` — 1015 LOC; `#[cfg(test)]` at `domain.rs:402`; ≈60% test lines; 29 `#[test]`,
  incl. `red_compensated_refund_not_reachable_via_apply_event` (`domain.rs:425`),
  `red_legacy_place_order_trusts_client_price` (`domain.rs:621`). Git RED-LINE money commits
  `e71b5af28`, `7bc2a82ae`.
- `bebop-repo/bebop2/core/src/pq_dsa.rs` — crypto/signing; 12 in-file tests + a sibling NIST ACVP KAT
  suite `pq_dsa/acvp_tests.rs` (9 KAT tests). Git KAT RED→GREEN: `ed2af6e` → `fb4e651`.

### 3.2 Real gaps

**G-T1 — `bebop2/delivery-domain` settlement/intake/dispatch tests are silently excluded from the default `cargo test`. (HEADLINE — CI/config defect, cheapest high-impact fix in this audit)**
`finalization.rs`, `intake.rs`, `hub_ring.rs` (and one module in `lib.rs`) all gate their test modules
behind `#[cfg(all(feature = "kernel-rlib", test))]` — `finalization.rs:172`, `intake.rs:294`,
`hub_ring.rs:94`, `lib.rs:382` — while `kernel-rlib` is `default = []` (OFF) in that crate's
`Cargo.toml`. A developer running the default `cargo test -p bebop-delivery-domain` sees green while
**every finalization and order-intake test is excluded**. The excluded logic is safety-critical:
`PartitionMerge::reconcile`/`detect_conflict` (the split-brain / double-finalization gate that must
reject two hubs settling one order to Delivered-vs-Cancelled, `finalization.rs:104-169`) and
`admit_and_fold`/status-mapping in `intake.rs:234, 54, 69`. The falsifiable tests **exist** but do not
run in the default gate — this directly undercuts the stated RED→GREEN discipline. **This is not a
"write more tests" gap; it is a one-line-per-file feature-gate / CI-matrix fix.**

**G-T2 — kernel `intake.rs` is thin for its role.** 583 prod LOC, in-tree `#[cfg(test)]` at
`intake.rs:584` but only 8 tests (~1 test / 73 prod lines) over a tiered-admission state machine;
core `admit()` (`intake.rs:552`) and `tier_c_smt_stub` (`intake.rs:389`) have limited branch coverage
vs the money/order exemplars. Role: order intake — deserves money.rs-grade coverage.

**G-T3 — kernel `budget.rs` money-gate undertested.** 208 prod LOC, 4 tests; `debit()`
(`budget.rs:113`) and `MonthlyBudget` ceiling (`budget.rs:138`) are **f64** money accounting with no
obvious negative-amount / rounding / ceiling-boundary tests (contrast `money.rs`: integer minor-units,
31 tests). Note the double smell: f64 money *and* thin tests.

**G-T4 — kernel `json_api.rs` untrusted boundary undertested.** 214 prod LOC, 4 tests (~32% test
lines) on a parse/serialize boundary — low for an untrusted-input surface.

*(Verified NOT gaps, so the blueprint author skips them: `kernel/src/pq/dsa.rs` pulls in a 9-test ACVP
KAT sibling `pq/dsa/dsa_acvp_tests.rs`; `agent-adapters/src/dispatch.rs` is covered by
`tests/e2e_admission.rs`; `kernel/src/ports/payment.rs` is ≈49% tests + a `proptest!`; `router.rs` is
adequate for routing.)*

### 3.3 Propagation recommendation
- **G-T1 first (cheapest, highest impact):** either make the `delivery-domain` test modules plain
  `#[cfg(test)]`, or add a CI matrix leg that runs `cargo test -p bebop-delivery-domain --features
  kernel-rlib`. The tests are already written — just un-gate/run them.
- **G-T2–T4:** raise coverage to the `money.rs` bar, prioritizing the order/money paths. For `budget.rs`
  specifically, pair the test work with a decision on migrating f64 money → integer minor-units (the
  `money.rs` convention).

---

## 4. Concurrency

**Framing (verified, load-bearing):** the adapter crates expected to hold async I/O —
`agent-loop/`, `llm-adapters/`, `agent-adapters/`, `agent-facade/` — contain **zero** `async fn` /
`.await` / `tokio::` in source; they are deliberately synchronous over `std::thread` + `std::sync::mpsc`
+ blocking backends. So the classic "`Arc<Mutex>` held across `.await`" gap **cannot exist** there
(checked; it does not). The only genuinely async surfaces are `tools/native-spa-server/` (axum/tokio)
and the feature-gated `PgStore`. The sync-first determinism discipline is honored.

### 4.1 The gold standard (proven in-repo)
- **Lock-free atomic counters/IDs:** `admission.rs:151,192` (`check_count: AtomicUsize` via `fetch_add`
  on the admission hot path); `wasm.rs:58,214` + `json_api.rs:28,180` (`static ORDER_SEQ: AtomicU64`,
  `fetch_add(SeqCst)`); `arena.rs:276,288` (`AtomicUsize` stats with correct `Relaxed`);
  `spectral_cache.rs:38-83` (falsifier `AtomicU64`, **no interior mutex** by design).
- **Atomic bulkhead with RAII Drop-guard:** `tools/native-spa-server/src/api.rs:303,355-363,428-433`
  — `inflight: AtomicI64`, `cap_middleware` `fetch_add`+ceiling, `BulkheadGuard::drop` does `fetch_sub`
  on every exit path. Lock-free in-flight limiting.
- **Expensive-work-outside-the-lock (textbook critical-section discipline):** `admission.rs:201 vs
  238-249` (crypto `verify_chain`/PQ verify runs lock-free; the `seen`-nonce `Mutex<HashSet>` is held
  only for the cheap insert *after* verification); `llm-adapters/src/cache.rs:107-122` (lock→get→drop
  →network `inner.chat()` unlocked→re-lock→put); `agent-adapters/src/dispatch.rs:213-248` (same shape).
- **Channel + immutable `Arc` sharing:** `llm-adapters/src/dispatch.rs:55-115` (`Dispatcher` shares
  `Arc<B>` + `Arc<TokenBucket>` immutably, `mpsc::channel` for worker→caller handoff, bounds
  concurrency with the `TokenBucket` not a lock).
- **Sharded rate-limiter:** `admission.rs:264-294` (`AdmissionLimiter::try_admit` — one global ceiling
  bucket + a fixed-size sharded array indexed by `conn_id % shards.len()`; offloads per-source
  contention off the global path, deliberately avoids an attacker-growable per-source map).
- **Deliberate mutex + poison recovery:** `token_bucket.rs:74-98` — short `f64` refill+decrement
  critical section, monotonic-clock refill, poison-cascade hardening via
  `lock().unwrap_or_else(|e| e.into_inner())`; the header (`token_bucket.rs:12-15`) documents *why* a
  mutex, not a CAS loop, is correct here.

### 4.2 Real gaps (ranked)

**G-C1 — pub/sub bus holds its single lock across the entire subscriber-dispatch loop. (STRONGEST)**
`bebop-repo/crates/bebop/src/portkey.rs:81-97` and `bebop-repo/crates/bebop/src/zenoh.rs:77-93`.
`Portkey::publish` / `Mesh::publish` acquire the one `Arc<Mutex<Inner>>` bus lock
(`portkey.rs:82` / `zenoh.rs:78`) and then invoke **every** subscriber handler `h(env)` *while still
holding the guard* (`portkey.rs:90-95`, `zenoh.rs:84-91` — zenoh even mutates `g.log` inside the loop).
Why real:
- It is the **publish hot path** of a bus — every message pays it.
- Handler execution is serialized under one global lock: no two publishes run concurrently; one slow
  handler stalls the whole bus.
- **Re-entrancy self-deadlock:** any handler that re-enters the bus (`publish`/`subscribe`/
  `unsubscribe`) — the natural "react to a message by emitting another" pattern — re-locks the same
  `std::sync::Mutex` and deadlocks. Concrete liveness hazard.
- Honesty caveat: both files are documented offline stand-ins for a future real Zenoh/Portkey, so
  today's blast radius is limited — but the defect lives in the reusable bus and ships the moment a
  real handler does work or re-publishes.
- Root cause + fix: `handlers: HashMap<SubId, Box<dyn Fn…>>` — the `portkey.rs:88` comment ("boxed
  closures can't be cloned") is *why* they hold the lock across dispatch. Change `Box<dyn Fn>` →
  `Arc<dyn Fn>`, **snapshot** the `Arc` handles under the lock, **drop the guard**, then invoke handlers
  outside the lock. Removes both the serialization and the re-entrancy deadlock in one change. Reuse
  the `llm-adapters` "lock/read/unlock/work" shape (§4.1) as the template.

**G-C2 — `PgStore` drives async sqlx via `block_on` behind a sync trait. (LATENT — feature-gated)**
`kernel/src/retrieval/memory_store.rs:167-216` — each sync `MemoryStore` method does
`self.rt.block_on(async {…})` against a captured `tokio::runtime::Handle` (`put` 169, `get` 185,
`keys` 198, `snapshot_root` 208). `Handle::block_on` called from within a tokio worker panics/blocks
the worker — the well-known sync-over-async footgun; each call also blocks for a full DB round-trip.
**But** it is feature-gated off by default (default path `InMemoryStore`) and no current caller drives
it from an async context (`native-spa-server` uses `json_api` directly, not `PgStore`). Latent, not a
live hot path. Fix: expose a genuinely async SQL API (callers `.await`) or confine `PgStore` to a
`spawn_blocking` pool so it never runs `block_on` on a runtime worker.

**G-C3 — per-call blocking file re-open on the dispatch path. (MINOR)**
`llm-adapters/src/dispatch.rs:137-150` (`append_harvest`) and
`agent-adapters/src/dispatch.rs:88-99` (`FileHarvest::record`) re-open `track_record.jsonl` with
`OpenOptions…append…open` + `write_all` on **every** dispatch, unbuffered, from the worker thread.
Real per-call blocking I/O, but it runs *after* the far slower LLM/bridge network call (not the primary
bottleneck) and POSIX `O_APPEND` writes are atomic (not a correctness bug). Fix: a persistent buffered
append writer, or an `mpsc` telemetry channel to a single batch-flushing writer thread.

*(Rejected as NOT gaps — short sections / cold config / lock released before IO:
`token_bucket.rs:74-98`, `llm cache.rs:107-122`, `ollama.rs:59-74`, `InMemoryStore`
`memory_store.rs:45-101`, `budget.rs:131-174` (debit released before `inner.submit`),
`native-spa-server api.rs` global store mutex (fine at `MAX_INFLIGHT_API=64`),
`admission.rs:238-249`. See Appendix A.)*

### 4.3 Propagation recommendation
- **G-C1 now:** apply the "snapshot handlers under lock, dispatch outside lock" fix to `portkey.rs` and
  `zenoh.rs`, using the `llm-adapters` cache pattern as the in-repo template. Highest concurrency
  impact; also closes a real deadlock.
- **G-C2 before `PgStore` goes live:** move to async API or `spawn_blocking` isolation. Gate on the
  pgrust rollout, not now.
- **G-C3 opportunistic:** buffered/channelled harvest writer.

---

## 5. Consolidated propagation plan (ranked by impact × how proven the source pattern is)

The source pattern column names the in-repo exemplar the fix should *copy* — every recommendation is
"do what X already does", not "invent something".

| # | Fix | Target file:line | Source exemplar (proven) | Impact | Effort | Priority |
|---|-----|------------------|--------------------------|--------|--------|----------|
| 1 | Un-gate/run `delivery-domain` settlement+intake tests | `bebop2/delivery-domain/{finalization,intake,hub_ring}.rs` gate lines 172/294/94 + Cargo `kernel-rlib` | `money.rs` RED→GREEN culture | High (safety-critical tests currently invisible) | Tiny (1 line/file or CI matrix leg) | **P0** |
| 2 | Bus: snapshot handlers under lock, dispatch outside | `crates/bebop/src/portkey.rs:81-97`, `zenoh.rs:77-93` | `llm-adapters/src/cache.rs:107-122` lock/unlock/work | High (serialization + re-entrancy deadlock) | Small (`Box`→`Arc`, reorder) | **P0** |
| 3 | Money ledger → append-only log + derived balances | `crates/bebop/src/ledger.rs:109-111` | `kernel/src/ports/payment.rs:181-226` SettlementState | High (auditable/replayable money) | Medium | **P1** |
| 4 | `causal.rs` samples AoS → flat `Vec<usize>` matrix | `kernel/src/causal.rs:1056,1307-1321` | `kernel/src/mat.rs` contiguous `Mat` | Medium-High (benched at 20k; kills 20k allocs) | Medium | **P1** |
| 5 | Reputation ledger: resolve NO-COURIER-SCORING, then event-source or delete | `crates/bebop/src/reputation.rs:39-46` | governance red-line + `event_log.rs:22` | Medium (governance + audit) | Medium (needs owner decision) | **P1 (gated)** |
| 6 | Raise TDD coverage on kernel order/money boundaries | `intake.rs`, `budget.rs`, `json_api.rs` | `money.rs`/`domain.rs` test bar | Medium | Medium | **P2** |
| 7 | `budget.rs` f64 money → integer minor-units (+ audit entries) | `kernel/src/budget.rs:113-138` | `money.rs` integer minor-units | Medium (correctness + audit) | Medium | **P2** |
| 8 | `ppr.rs` `Vec<Vec<f64>>` → `mat::Mat` (bundle with scaled path) | `kernel/src/retrieval/ppr.rs:20-56` | `mat.rs`; cross-ref `OPUS-PERF-PPR-ANALYSIS-2026-07-18.md` | Latent (prod n=20 today) | Small | **P3** |
| 9 | `PgStore` async API / `spawn_blocking` isolation | `kernel/src/retrieval/memory_store.rs:167-216` | native-spa-server async surface | Latent (feature-gated) | Medium | **P3 (gate on pgrust)** |
| 10 | `bm25.rs` interned CSR postings + write revisit threshold | `kernel/src/retrieval/bm25.rs:121-208` | `csr.rs` posting layout | Latent (12-doc fixture) | Medium | **P3** |
| 11 | Buffered/channelled harvest writer | `llm-adapters/src/dispatch.rs:137-150`, `agent-adapters/src/dispatch.rs:88-99` | `llm-adapters` mpsc dispatcher | Minor (after network call) | Small | **P4** |

**Sequencing logic.** P0 items are near-free and remove a *silent* safety hole (tests that do not run)
and a *concrete* deadlock — do them first. P1 items are the substantive best-pattern ports, each with a
proven in-repo template and (for #3/#4) a real audit/benchmark payoff. #5 is P1 but **gated on a
governance decision** (NO-COURIER-SCORING), not a pure refactor. P2 raises the money/order test+type
bar. P3/P4 are latent — schedule with their triggering path (scaled retrieval, pgrust rollout, larger
corpus), and write the revisit thresholds down now so the latency is not forgotten.

---

## Appendix A — Rejected candidates (do not re-investigate)

**DoD, rejected (cold / small-n / already addressed):** `markov.rs:142-177` (small-n, unbenched),
`cgraph.rs:40-42` + `causal.rs` graph Vecs (3-node bench graph), `spectral.rs:114-137` charpoly (n>32
fallback with "no consumer" + already has `charpoly_in` twin), `spectral.rs:280` topk evecs (top-k
small vs O(iters·nnz) spmv), `analytics.rs:42-141` (reporting reducer, correct HashMap use),
`engine/field_frame.rs:514` (test-only), `engine/scene.rs:72` (tiny shape count, flat field buffer
already), `engine/{friction,motion}.rs` (FSM/scalar, no collections). `engine/field_energy.rs` is
already the gold pattern.

**Event-sourcing, correctly NOT event-sourced (ephemeral/derived/already-good):**
`ports/payment.rs` (already event-sourced — the exemplar), `mesh.rs`/`spine.rs`/`pq/codesign.rs`
(append-only logs), `analytics.rs` (re-derivable projection), `hydra.rs:225-236` (re-derived
supervisory state; the breach *fact* IS event-sourced via `append_raw` at `hydra.rs:366`),
`cart.rs:42-129` (ephemeral session cart), `catalog.rs:40` (config/reference data),
`spool.rs:70-129` (consume-once work queue), `token_bucket.rs:85` (rate-limiter),
`ports/customer.rs:255-261` (dies-with-order subscription), `online.rs:287-289` (streaming estimator),
`engine/widget_store.rs`/`scene.rs`/`bridge.rs` (per-frame UI/GPU state),
`bebop2/delivery-domain/lib.rs:329-359` (returns new `AppliedEvent`), `finalization.rs:40` + `pod.rs`
(hash-chained quorum certs / multisig builder).

**Concurrency, rejected (short section / cold / lock released before IO):** `token_bucket.rs:74-98`,
`llm-adapters/src/cache.rs:107-122`, `agent-adapters/src/cache.rs:63-76`, `ollama.rs:26,59-74`,
`memory_store.rs:45-101` (`InMemoryStore`), `budget.rs:131-174` (debit released before `inner.submit`
at 174), `native-spa-server/src/api.rs:101-133,331-342,463-510` (kernel logic runs outside the lock,
fine at `MAX_INFLIGHT_API=64`), `admission.rs:238-249` (crypto outside the lock). No `crossbeam` in
source (grep hits are `target/` + `Cargo.lock` only).

**TDD, verified NOT gaps:** `kernel/src/pq/dsa.rs` (+9-test ACVP KAT sibling),
`agent-adapters/src/dispatch.rs` (+`tests/e2e_admission.rs`), `kernel/src/ports/payment.rs` (≈49% +
`proptest!`), `kernel/src/router.rs` (adequate for routing).
