# SYNTHESIS — Performance Audit: reconciled plan from ten OPUS-PERF passes (2026-07-18)

> **Planning document — writes no product code.** Synthesizes the ten 2026-07-18 Opus research
> passes (`docs/research/OPUS-PERF-*-2026-07-18.md`) into ONE prioritized action plan with a
> blueprint breakdown, per the operator's "план з блюпринтами" request. Format precedent:
> `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` (§1 exec summary · §4 flagged decisions · §5
> blueprint breakdown). Every blueprint unit below is to be written against the 20-point contract
> in `CORE-ROADMAP-STANDARD-2026-07-17.md`. Numbering continues from the existing **P74**.
> Conducted under the **Performance Standing Rule — atomicity & branchless**
> (`.claude/CLAUDE.md:182-195`): rewrites require a benchmark proving hotness; no blanket
> application; every claimed win carries a criterion bench.
>
> **Honesty preserved:** four of the ten passes returned genuine non-issues (`ppr.rs`,
> `absorbing.rs`, RGB-packing reuse, Go-pointer import). Those negative results are load-bearing
> and are logged in §6 so nobody re-investigates them — they are NOT softened to manufacture work.

---

## 0. Inputs (all ten read in full this pass)

| # | Report | One-line verdict |
|---|---|---|
| R1 | `OPUS-PERF-PPR-ANALYSIS` | **Non-issue today** (n=20 frozen fixture, bench exists+gated); deliverable = growth-sweep bench + written revisit threshold; approximate algorithms REJECTED (break determinism, `csr.rs:21-24` already ruled). |
| R2 | `OPUS-PERF-ABSORBING-MARKOV-ANALYSIS` | **Non-issue** (n=5 fixed FSM, zero production callers); 2 doc errors (true cost O(n⁴); "agentic decision gating" claim false); existing bench measures the pessimal cyclic path, not the real DAG path. |
| R3 | `OPUS-PERF-KERNEL-AUDIT` | 1 real O(N²) (`spool.rs` FIFO drain, highest-ranked), 1 low-med O(R²) (`spine.rs`), 1 low densify (`spectral_laplacian.rs`); `money.rs` **INFO-only, no action**; 3 Mutex candidates all BENCH-FIRST; SeqCst→Relaxed explicitly declined (zero benefit). |
| R4 | `OPUS-PERF-BEBOP-AUDIT` | HRW matcher + crypto verify **clean**; `MerkleDigest::add` O(n²·log n) (highest-ranked, ~5-line fix), `hub_ring::ranked` double-hash-per-comparison (~10 lines), `poly_mul` schoolbook deliberate; branchless/atomicity net action = NONE (no candidate met the evidence bar). |
| R5 | `OPUS-PERF-BENCH-COVERAGE-MAP` | Full both-repo coverage map; flags `money.rs:230 ledger_sum` O(n²) (reconciled §2); `engine` crate ZERO benches; entire kernel PQ lane unbenched; `HybridGate::check` unbenched; wave-structured harness proposal. |
| R6 | `OPUS-PERF-METRICS-ARCHITECTURE` | `tracing` already linked with spans already placed; nothing aggregates → custom Layer → `metric.jsonl` (L1) + on-demand `perf record` on load breach (L2); operator's load1 spike likely `rustc` build noise, and only system-wide `perf` can prove which. |
| R7 | `OPUS-PERF-REGRESSION-TOOLING-AUDIT` | **CRITICAL: CI bench gate cannot execute at all** (exit 2 every fresh runner, broken by `f3c0687cf`); 2 of 3 fail-open paths fixed, cross-host-comparison one OPEN; no statistical significance anywhere (criterion's own stats discarded); prescribes same-runner `--save-baseline`/`--baseline` + critcmp. |
| R8 | `OPUS-PERF-BESTPRACTICES-PROPAGATION` | **P0: `bebop2/delivery-domain` split-brain/double-finalization tests exist but never run** (gated behind default-OFF `kernel-rlib`); P0: bebop bus holds Mutex across subscriber dispatch (self-deadlock); full P0–P4 propagation ladder with in-repo exemplars. |
| R9 | `OPUS-PERF-RGB-PACKING-REUSE` | **Honest refutation**: RGBA interleaving is a display-format constraint, does not generalize; ONE actionable item (spectral evecs → `mat.rs` contiguous-flatten — explicitly NOT the RGBA lesson). |
| R10 | `OPUS-PERF-POINTER-ARENA-ANALYSIS` | **Honest refutation**: Go-pointer import ~0% realistic (category error); house style already has arena+index-handles; ONE optional target (`micrograd.rs` Rc<RefCell> → typed tape); generational-index bug class verified absent (`swap_remove` grep empty) — no slotmap/generational crate. |

---

## 1. Executive summary

One sentence: **the two repos' hot paths are algorithmically healthier than feared — the real
emergencies are two "we think we're protected but we're not" defects in the *protection
machinery itself*** (a CI bench gate that cannot execute, and safety-critical settlement tests
that never run), followed by four small, safe, strictly-better algorithmic fixes, followed by a
large but mechanical bench-coverage expansion.

The reconciled shape of the whole pass:

1. **Tier A (live bugs, fix this week):** the CI bench-regression gate `exit(2)`s on every fresh
   runner (R7 — broken by `f3c0687cf`, compounding the still-open rejected cross-host-comparison
   design), and `bebop2/delivery-domain`'s split-brain/double-finalization + intake tests are
   silently excluded from default `cargo test` (R8 G-T1). Both are cheap; both restore protection
   we currently only *believe* we have. The bebop bus lock-across-dispatch deadlock (R8 G-C1)
   rides in the same wave — small, real, in-repo template exists.
2. **Tier B (algorithmic fixes, safe implementations in hand):** `spool.rs` O(N²) drain,
   `spine.rs` O(R²) dedup, bebop `MerkleDigest::add` per-insert sort, `hub_ring::ranked`
   double-hash comparator, `causal.rs` 20k-alloc AoS, spectral evec flatten. Every one is
   behavior-preserving, has a named in-repo pattern to copy, and ships with a red→green bench.
3. **Tier C (coverage):** `engine` crate gets its first bench harness; the kernel PQ lane, mesh
   chain-verify, money growth-tripwire, spectral/matmul/Kalman surface, and bebop's
   `HybridGate::check` + sign/KEM/AEAD lanes all get benches; the tracing→`metric.jsonl` Layer +
   breach-triggered `perf record` land as continuous observability. **Sequencing insight: the
   gate re-architecture (P75) lands FIRST so ~40 new baselines are written into the fixed
   schema, not the broken one.**
4. **Tier D (deferred with written triggers)** and **Tier E (rejected, §6)** preserve the pass's
   negative results — four full reports resolved to "no action," and that honesty is a
   deliverable, not a disappointment.
5. **The money.rs "conflict" between R3 and R5 dissolves on ground truth** (§2): both reports
   cite the same single per-order call site; they differ only in framing. Verdict: no code
   change; ship the bench as a growth-tripwire with a written revisit threshold.

---

## 2. The money.rs `ledger_sum` reconciliation (R3-C4 vs R5-§2A)

**The apparent conflict.** R3 files `money.rs` under *"C4 — INFO/LOW … NOT a scaling risk …
deliberately NOT recommending a change"*. R5 files the same function as *"O(n²) … quadratic in
ledger length — the highest-signal money finding"* and makes `money_ledger` its #1 bench
priority with sweeps at n ∈ {8, 64, 256, 1024}.

**Ground truth (re-verified this pass, live source, not report claims):**

- `ledger_sum` (`kernel/src/money.rs:230-245`) is genuinely O(n²): for each non-reversal `Earn`
  it runs `ledger.iter().any(|r| r.reverses == Some(e.id))` — a full scan per entry. Both
  reports agree on the code fact.
- The **only** non-test caller is `Order::ledger_balance` (`kernel/src/domain.rs:104-105`) over
  `Order.ledger: Vec<LedgerEntry>` (`domain.rs:63`) — a strictly **per-order** ledger. **R5's
  own table cites this same caller** — it has no evidence of a larger or different `n`.
- The only production writers are `Order::post_earn` (`domain.rs:86-99` — ONE Earn leg, posted
  at confirm) and `compensate` → `reverse_transfer` (`domain.rs:391`) — ONE Reversal.
  `ledger_append` itself enforces at-most-one-reversal-per-earn and duplicate-id rejection
  (`money.rs:216-222`). **No code path exists today that posts multiple Earn legs** (no
  per-item earns, no tip legs, no fee splits). Therefore real-world `n ≤ 2` by construction.

**Resolution — both reports are right about different questions:**

- R3 asked *"is this a scaling risk on the money path?"* → **No.** n is structurally bounded at
  ~2; the linear scans ARE the fail-closed conservation/idempotency probes; correctness-first on
  money-authority code. **R3's no-code-change verdict STANDS.**
- R5 asked *"what is the biggest unbenched asymptotic cliff on the money lane?"* → **This one.**
  Its "highest-signal" label is a *coverage*-tier ranking inside its own T1 table, not a claim
  of production-scale n. The correct reading of R5's row is "highest-signal money-lane **bench
  gap**," and future citations should use that phrasing.

**Reconciled verdict (binding for the blueprints):**

1. **No algorithmic change to `money.rs`.** Do not "fix" the O(n²); do not restructure the
   probes. (If the trigger below ever fires, the fix is an O(n) reversed-id `HashSet` pre-pass
   inside `ledger_sum` — semantics-identical — but not before.)
2. **Ship R5's `money_ledger` bench group re-labeled as a growth-tripwire** (same class as the
   `ppr.rs`/`absorbing.rs` treatment): sweep n ∈ {2, 8, 64, 256} — n=2 is the real-shape anchor;
   the larger sizes exist to keep the quadratic curve on the record, exactly like R1's ppr sweep.
3. **Written revisit threshold** (goes in the bench doc-comment): *revisit `ledger_sum`
   representation only if a future change introduces multi-leg order ledgers (per-item earns,
   tips, fee splits, settlement legs) or any real ledger exceeds ~8 entries.*

**Second cross-report reconciliation (recorded here because it changes a priority):** R8 ranks
the bebop money-ledger event-sourcing port (`crates/bebop/src/ledger.rs`, G-E1) at P1 — but R4
independently established that **`crates/bebop` is the legacy/dev-tooling TUI crate, off the mesh
product path** (`crates/bebop/src/lib.rs:1-10`). Reconciled: the port is architecturally correct
(copy `ports/payment.rs::SettlementState`) but **downgrades to Tier D**, bundled with the B2
settlement work or whenever `crates/bebop` money paths become product. Same downgrade applies to
`reputation.rs` (G-E2) — though its NO-COURIER-SCORING red-line divergence still requires the
governance ruling in §4 regardless of crate status.

---

## 3. Ranked findings across all ten reports

### 3.1 Tier A — LIVE BUGS ("we think we're protected but we're not")

| ID | Finding | Source | Location | Why it's Tier A | Fix shape | Blueprint |
|---|---|---|---|---|---|---|
| A1 | **CI bench-regression gate cannot execute** — `bench_track.py` `exit(2)`s on every fresh runner (`native-trackers` never built there); introduced by `f3c0687cf` which deleted the python fallback without updating `ci.yml:150-168`. Compounds the still-OPEN explicitly-rejected cross-host absolute comparison (fail-open path #c). | R7 §2A/§3 | `kernel/benches/bench_track.py:66-84`, `.github/workflows/ci.yml:150-168` | Every push/PR "passes" a perf gate that carries zero signal; invites `\|\| true` fixes that make it permanently fail-open. | Re-architect to criterion same-runner A/B (`--save-baseline` at merge-base → `--baseline` at HEAD → `critcmp` + thin exit-code parser); keep `native-trackers` as the local Hetzner absolute-tracking cron only. | **P75** |
| A2 | **`bebop2/delivery-domain` safety-critical tests never run** — split-brain / double-finalization (`PartitionMerge::reconcile`/`detect_conflict`) + order-intake tests gated behind `#[cfg(all(feature = "kernel-rlib", test))]` with `kernel-rlib` default-OFF; default `cargo test` shows green with all of them excluded. | R8 G-T1 | `bebop2/delivery-domain/{finalization.rs:172, intake.rs:294, hub_ring.rs:94, lib.rs:382}` | The falsifiable tests EXIST but are invisible — directly undercuts the RED→GREEN discipline on a settlement-safety surface. Near-free fix. | Plain `#[cfg(test)]` where the tests don't need the feature, or a CI matrix leg `cargo test -p bebop-delivery-domain --features kernel-rlib`. | **P76** |
| A3 | **bebop bus holds its Mutex across the entire subscriber-dispatch loop** — serializes all publishes; any handler that re-enters the bus self-deadlocks (std Mutex, non-reentrant). Caveat honored: both files are offline stand-ins, so today's blast radius is limited — but the defect ships the moment a real handler works or re-publishes. | R8 G-C1 | `crates/bebop/src/portkey.rs:81-97`, `zenoh.rs:77-93` | Concrete liveness hazard in the reusable bus; the fix template exists in-repo (`llm-adapters/src/cache.rs:107-122`). | `Box<dyn Fn>` → `Arc<dyn Fn>`; snapshot handles under lock; drop guard; dispatch outside. + regression test: a handler that re-publishes must not deadlock. | **P76** |

### 3.2 Tier B — real algorithmic/complexity fixes (safe implementation in hand)

| ID | Finding | Source | Location | Real n / hotness | Fix (behavior-preserving) | Blueprint |
|---|---|---|---|---|---|---|
| B1 | `spool.rs` FIFO drain is O(N²) — `position` scan + `Vec::remove` shift per ack; draining N records is quadratic. | R3 C1 (rank #1) | `kernel/src/spool.rs:88-129` | n = outbox backlog under backpressure — **scales with real enqueue volume**; unbenched. | `VecDeque` + claimed cursor (or head index + lazy compaction) → O(1) amortized FIFO; `id→index` map if ack-by-id stays. + drain bench (red→green). R10 concurs: `HashMap` not generational arena if id-lookup is kept. | **P77** |
| B2 | `retrieval/spine.rs` backlinks/related O(R²) dedup + O(docs) id scan. | R3 C2 | `kernel/src/spine.rs:210-332` | n = knowledge-spine corpus (grows unbounded per MEMORY); advisory path; unbenched. | `HashSet` dedup accumulators, `HashMap<id,idx>` lookup. + spine bench. | **P77** |
| B3 | bebop `MerkleDigest::add` sorts whole leaf vector on every insert — O(n²·log n) digest build on the anti-entropy fold path. | R4 P1 (rank #1) | `bebop2/proto-wire/src/sync_pull.rs:449-461` | n = event-log size — scales with mesh/order volume; fully uncovered. | Remove per-insert `sort_unstable` (dedup already via `seen` HashSet); sort in `root()` on the clone it already makes. ~5 lines, order-stable root preserved. + `ingest`/`root` bench. | **P78** |
| B4 | bebop `hub_ring::ranked` recomputes HRW hash inside the sort comparator (2×/comparison); `owner_hub` full-sorts to take `[0]`. | R4 P2 | `bebop2/delivery-domain/src/hub_ring.rs:52-92` | hub count bounded-small but called **per-order**. Correct Schwartzian pattern already in `proto-cap/matcher.rs:64-70` — this is a regression from blessed code. | Precompute `(weight, hub)` once, sort tuples; `owner_hub` → single `max_by` scan. ~10 lines, same total order. | **P78** |
| B5 | `causal.rs` samples are 20,000 separately-heap-allocated 3-element Vecs (AoS), walked by benched hot loops. | R8 G-D1 (top DoD) | `kernel/src/causal.rs:1056, 1307-1321` | Hot at the **benched** size (`empirical_identify/20k` ×2) — the one data-layout candidate whose existing bench will visibly move. | Flat row-major `Samples { n_cols, data: Vec<usize> }` (the `mat.rs` pattern for usize). Existing benches are the regression gate. | **P79** |
| B6 | Spectral eigenvector storage `evecs: Vec<Vec<f64>>` — k heap allocations, pointer-chased by full-length dot products (Gram-Schmidt/deflation/Rayleigh). | R9 §3-B (its ONE actionable) | `kernel/src/spectral.rs:280, 400, 421, 529` | Every consumer walks whole vectors contiguously — textbook `mat.rs` contiguous-flatten case. Explicitly NOT RGBA interleaving (which would be harmful here). | One `Vec<f64>` of k·n, vector m at `[m*n..(m+1)*n]`. Coordinate with the Phase-28 single-eigen-surface ruling (`spectral.rs` stays the only eigen surface). + fix `zerocopy.rs:22` "SoA" mislabel (it's AoS). | **P79** |

### 3.3 Tier C — bench-coverage + observability additions

| ID | Addition | Source | Content | Blueprint |
|---|---|---|---|---|
| C1 | **Kernel bench expansion** | R5 §3-W1, R1 §4, R2 §4, R3 A1-A3 | `money_ledger` growth-tripwire (§2 verdict: n ∈ {2,8,64,256} + revisit threshold); `kernel_crypto_pq` (dsa sign/verify, kem encaps/decaps, hybrid_decaps, shake256/sha3 size-swept — the entire kernel PQ lane is unbenched); `mesh_verify` chain sweep {1,8,64,256}; `spectral_math` (eigenvalues **straddling the n=32 QR↔Faddeev step** {8,16,32,48}, matmul_contig, kalman predict/update, classify_drift, laplacian_spmv); `retrieval_geo` (bm25 isolate, progress_along_route, harmonic_centrality); **ppr sweep** n ∈ {32,128,256} @ α=0.15 + written ~256-node/~50µs revisit threshold (R1 §3a); **absorbing**: relabel `_16`→`_cyclic_16`, add `lifecycle_5` (real DAG path) + `dag_chain` sweep, fix the 2 doc errors (O(n⁴) not O(n³); delete the false "agentic decision gating" claim); **contended Mutex benches** for token_bucket/budget/admission (BENCH-FIRST per standing rule — no CAS rewrite without them; fix the bench comment that mislabels the Mutex as CAS). | **P80** |
| C2 | **Engine bench harness (crate has ZERO today, runs every frame)** | R5 §3 (engine) | New `engine/benches/criterion.rs`: `FieldFrame::step` + `laplacian_into` grid-swept {64²,128²,256²}, `frame_rgba`, `compose`, `Scene::render_frame` shape-swept, `Spring::step` ω-swept, `VertexBridge::apply_field` nnz-swept (note its per-call heap alloc in a documented no-alloc loop), `money_guard::present_money` RED-LINE baseline pin. | **P81** |
| C3 | **bebop bench expansion** | R5 §3-W2, R4 P3 | Extend `verify_lane.rs` (or criterion sibling): **sign** timing (currently setup-only), KEM encaps/decaps (gates any future NTT decision — R4 P3), sovereign x25519, size-swept AEAD + sha3. NEW `proto-cap` benches: **`HybridGate::check`** chain-swept {0,1,4,16} (THE per-frame auth gate), verify_pq/verify_classical, tlv_signing_input, roster::verify_chain, matcher::assign. NEW `proto-wire` benches: encode/decode_frame chain-swept, framing, envelope serde_json cost. | **P82** |
| C4 | **Per-function production observability** | R6 (whole report) | Layer 1: `SpanMetricsLayer` (~120 lines, zero new deps — `tracing-subscriber` already linked, 3 spans already placed) → hand-rolled log-bucket histograms → `metric.jsonl` via `telemetry kernel-spans`; instrument the 8 verified functions (place_order_priced, place_order, fold_transitions, commit_after_decide, decide_settlement, cap::verify_chain, mldsa verify behind `pq`, route) — deliberately NOT `assert_transition`/inner loops. Layer 2: extend the existing `load1/nproc ≥ 4` friction branch with system-wide `perf record -a -g -F 99` (answers "kernel or `rustc`?" — R6's reframe of the operator's spike) + alert.jsonl artifact. `pprof` only as feature-gated fallback. | **P83** |
| C5 | **Regression-gate rigor follow-through** (beyond the A1 execution fix) | R7 §6.3-4 | Consume criterion's significance verdict instead of raw mean delta; sample-size >10 for sub-100ns benches; **iai-callgrind instruction-count benches** for the ns-scale benches PERF-06 proved ungateable on this host (±75% swings); per-bench thresholds not one flat 10%; committed trend storage (BENCH_HISTORY is git-ignored today — commit same-runner A/B ratios or resume the `bench.jsonl` feed). | **P75** (rigor half) |

### 3.4 Tier D — deferred/latent (bundle with triggering work; write the trigger down NOW)

| ID | Item | Source | Trigger | Bundled with |
|---|---|---|---|---|
| D1 | `ppr.rs` dense→sparse migration (routes b1 byte-identical / b2 csr-reuse documented in R1 §3b) | R1, R8 G-D2 | diffusion graph > ~256 nodes OR `ppr/rank_*` > ~50µs (the P80 sweep is the tripwire) | scaled-retrieval path |
| D2 | `bm25.rs` interned CSR postings | R8 G-D3 | real corpus ≫ 12-doc fixture; write threshold into `bm25.rs` | corpus-scale retrieval work |
| D3 | `SyncNode::pull` per-actor seq index | R4 P4 | pull-rate × log-size measured hot (P82 bench is the tripwire) | anti-entropy hardening |
| D4 | `spectral_laplacian.rs` n>32 densify → build L directly as `Csr` | R3 C3 | evidence field-UI ever drives n > 32 | field-UI scaling work |
| D5 | `PgStore` `block_on`-behind-sync-trait → async API or `spawn_blocking` | R8 G-C2 | **before pgrust goes live** (feature-gated off today) | pgrust rollout |
| D6 | `micrograd.rs` `Rc<RefCell>` → typed index tape (`Vec<ValueData>` + `NodeId(u32)`) | R10 §3.1 | optional quality/perf; 0 product consumers; covered by the perf-priority directive | self-eval harness work |
| D7 | `budget.rs` f64 money → integer minor-units + tests to `money.rs` bar; `intake.rs`/`json_api.rs` coverage raise | R8 G-T2..T4, #6-7 | next money/order-boundary hardening pass | test-bar wave |
| D8 | bebop `ledger.rs` → append-only log + derived balances (SettlementState pattern) | R8 G-E1, **downgraded per §2 reconciliation** (legacy TUI crate per R4) | `crates/bebop` money paths become product, or B2 settlement work touches them | B2 settlement |
| D9 | `pq_kem` NTT re-introduction (~100× on poly_mul) | R4 P3 | P82's KEM bench proves handshake latency matters; MUST ship with the verifier pair (`intt(ntt(a))==a` AND basemul==schoolbook) per `pq_kem.rs:335`; crypto red-line → operator sign-off | §4-D3 |
| D10 | Harvest-writer buffering (per-dispatch file re-open) | R8 G-C3 | opportunistic; after the far-slower network call, not a bottleneck | adapter housekeeping |

---

## 4. Flagged operator decisions (raise before the affected blueprint is written)

| # | Decision | Blocks | Context |
|---|---|---|---|
| D-1 | **Golden state-digest regression gate** — a committed golden digest over `fold`/`decide` projections so *behavioral* kernel drift trips CI like timing drift. All primitives exist (content-addressing, eqc-rs digest pinning); it is wiring — but it touches the money/FSM red-line surfaces. | a possible P84 (not proposed until ruled) | R7 §5-§6.5 |
| D-2 | **`reputation.rs` — delete or event-source?** It is courier/node scoring, which the kernel forbids (NO-COURIER-SCORING, `event_log.rs:22`; trust = signed capability, never reputation). If it stays it must become append-only events; the canonical stance suggests it goes. | P76's scope note (fix is trivial either way once ruled) | R8 G-E2 |
| D-3 | **`pq_kem` NTT** — only if P82's bench proves handshake latency hot; crypto red-line, requires the verified-pair bar and operator sign-off. | D9 | R4 P3 |
| D-4 | **PPR determinism relaxation** — Forward-Push/MC/FAST-PPR are all approximate and/or order-dependent; the codebase already ruled once (`csr.rs:21-24`). Recorded here so it is never adopted silently; default = REJECTED. | nothing (rejection stands) | R1 §3c |

Engineering decisions the blueprints decide (operator need not): exact critcmp threshold + which
benches move to iai-callgrind (P75); `#[cfg(test)]` vs CI-matrix-leg for A2 (P76); sweep-size
convention `<group>/<n>` shared across all new baselines (P80/P81/P82 cite P75's schema).

---

## 5. Proposed blueprint breakdown (P75–P83, waves W0–W3)

Every unit is one coherent, independently-buildable blueprint against the 20-point standard.
Blueprint **writing** can be fully parallel (different files); waves are **build** order.
Collision guards: P75 owns the baseline/bench-id schema (P80/P81/P82 cite it, never redefine);
P77/P79 both touch `kernel/src` but disjoint files; P76/P78 are bebop-repo lanes (RULE:
bebop files → `/root/bebop-repo`, push to `openbebop`).

### Wave W0 — protection machinery first (2 blueprints, build-parallel, different repos)

| # | Blueprint | Scope | Depends on | Feeds |
|---|---|---|---|---|
| **P75** | **CI bench-regression gate re-architecture** | Same-runner criterion A/B in CI (merge-base `--save-baseline base` → HEAD `--baseline base` → `critcmp` + thin exit-code parser — closes fail-open path #c AND the exit-2 break in one move); `native-trackers bench` confined to the local Hetzner absolute cron (built where invoked, never called unbuilt); consume criterion significance not raw mean delta; per-bench thresholds; iai-callgrind lane for ns-scale benches; committed trend storage (A/B ratios or revived `bench.jsonl` feed); owns the `<group>/<n>` bench-id + baseline schema. DoD: the job goes RED on an injected regression and GREEN on HEAD — proven in CI, not locally. | none | **P80, P81, P82** (they write baselines into its schema) |
| **P76** | **bebop hidden-tests un-gate + bus-lock fix** | Un-gate `delivery-domain` finalization/intake/hub_ring tests (plain `#[cfg(test)]` or CI matrix leg — blueprint decides; DoD: `cargo test` default OR CI provably executes the split-brain/double-finalization tests, count visible in output); `portkey.rs`/`zenoh.rs` snapshot-handlers-then-dispatch-outside-lock (`Box`→`Arc`), template `llm-adapters/src/cache.rs:107-122`; NEW regression test: re-publishing handler must not deadlock; carries the §4/D-2 flag for `reputation.rs` without blocking on it. | none | P78 (same repo, sequenced after to avoid CI churn overlap) |

### Wave W1 — algorithmic fixes (3 blueprints, build-parallel, collision-free lanes)

| # | Blueprint | Scope | Depends on |
|---|---|---|---|
| **P77** | **Kernel complexity fixes: spool + spine** | B1: `spool.rs` `VecDeque`/head-cursor O(1)-amortized FIFO (ordering + crash-safety semantics preserved; `id→index` map if ack-by-id stays) + N-record drain bench proving the win; B2: `spine.rs` HashSet dedup + `HashMap<id,idx>` + spine bench. Both red→green per verified-by-math. | P75 (soft — benches land in its schema) |
| **P78** | **bebop complexity fixes: MerkleDigest + hub_ring** | B3: drop per-insert sort, sort in `root()` on the existing clone (~5 lines; root order-stability preserved — assert it in a test) + `ingest`/`root` bench; B4: Schwartzian precompute mirroring `matcher::assign` + `owner_hub` → `max_by` (~10 lines; same-total-order test). | P76 (repo sequencing), P75 (schema) |
| **P79** | **Kernel data-layout ports: causal flat samples + spectral evec flatten** | B5: `causal.rs` flat `Samples` matrix — the existing `empirical_identify/20k` benches are the before/after gate (a measured number, not an estimate, per standard §10); B6: spectral evecs `Vec<Vec<f64>>` → contiguous k·n buffer (mat.rs lesson; coordinate with Phase-28 single-eigen-surface ruling; explicitly NOT interleaving, per R9); doc fix `zerocopy.rs:22` SoA→AoS label. | P75 (schema); independent of P77 (disjoint files) |

### Wave W2 — bench-coverage expansion (3 blueprints, build-parallel by crate lane)

| # | Blueprint | Scope | Depends on |
|---|---|---|---|
| **P80** | **Kernel bench expansion** | Everything in §3.3-C1: money_ledger growth-tripwire (per §2's binding verdict + revisit threshold), kernel_crypto_pq lane, mesh_verify sweep, spectral_math (straddle n=32), retrieval_geo, ppr sweep + threshold (R1 §4 code as written), absorbing lifecycle/dag benches + relabel + 2 doc fixes, contended-lock benches A1–A3 (**bench-only** — any CAS rewrite is a separate, evidence-gated future blueprint per the standing rule). | **P75 hard** (schema + working gate) |
| **P81** | **Engine bench harness (first ever)** | §3.3-C2: new `engine/benches/criterion.rs` + `[[bench]]` wiring; field_frame/scene/motion/bridge sweeps; money_guard RED-LINE pin. | **P75 hard** |
| **P82** | **bebop bench expansion** | §3.3-C3: verify_lane sign/KEM/AEAD/sha3 extension; NEW proto-cap group (HybridGate::check the headline); NEW proto-wire codec group. Output feeds §4/D-3 (NTT decision) with data. | **P75 hard**; P76/P78 landed (same repo) |

### Wave W3 — observability (1 blueprint)

| # | Blueprint | Scope | Depends on |
|---|---|---|---|
| **P83** | **Kernel span metrics + spike profiler** | §3.3-C4: `SpanMetricsLayer` → `metric.jsonl` (zero new deps); the 8-function span set; `telemetry kernel-spans`; breach-triggered system-wide `perf record` + alert.jsonl artifact; feature-gated `pprof` fallback only if `perf` perms unavailable. | none (parallel-safe with W1/W2; listed W3 only for reviewer bandwidth) |

### Swarm dispatch summary

- **Build order:** W0 both immediately (different repos, zero collision). W1's three in one
  parallel fan-out after P75 merges (P77/P79 disjoint kernel files; P78 after P76 in bebop).
  W2's three in one fan-out after P75 (hard) — ~40 new baselines must land in the fixed schema.
  P83 anytime.
- **Single-owner contracts:** bench-id/baseline schema + gate semantics → P75; bebop CI matrix
  → P76; sweep-size convention `<group>/<n>` → P75 (P80/P81/P82 cite).
- **Before writing:** raise §4 D-1/D-2 with the operator (D-1 gates a possible P84; D-2 is a
  scope note inside P76). D-3/D-4 need no pre-ruling — they are data-gated/default-rejected.
- **Not in any blueprint by design:** everything in §6. A blueprint that resurrects a §6 item
  must cite new evidence overturning the recorded rejection.

---

## 6. Explicitly-rejected / non-actionable log (do NOT re-investigate)

Negative results are load-bearing. Each entry records who rejected it and why, so future passes
cite instead of re-derive.

| # | Item | Verdict | Source + reason |
|---|---|---|---|
| E1 | **RGB/RGBA-packing generalization** to matrices/tensors/Kalman/money | REJECTED | R9: interleaving is a display/GPU-blit **format constraint** (twin: `ParticleBuffer` at the same boundary), and a 1→4 colormap fan-out — not a perf primitive. Interleaving eigenvectors or Kalman state would be **actively harmful** (fights SIMD lanes and contiguous dot products). The real house patterns are SoA (`simd.rs`, `csr.rs`) and contiguous-flatten (`mat.rs`). |
| E2 | **Go-pointer feature import** | REJECTED (~0% realistic) | R10: category error — Go pointers presuppose a GC; importing one kills determinism/`no_std`/WASM/red-line guarantees. The valuable idiom (region alloc + index-as-handle) is already house style (`BumpArena`, index graphs). |
| E3 | **slotmap / generational-arena / thunderdome adoption** | REJECTED | R10: the stale-index/ABA bug class these prevent is **verified absent** (`grep swap_remove kernel/src engine/src` → empty; graphs build-once; mutable collections key by logical id). Zero-dep default build is a hard property. If the need ever appears: hand-roll ~50-line `SlotArena` next to `BumpArena`. |
| E4 | **SeqCst→Relaxed on metric counters** | DECLINED (zero benefit) | R3: on x86-64 `fetch_add` compiles to `lock xadd` regardless of ordering — no instruction-level difference; changing them adds reasoning cost for no measured win (the standing rule's own caveat). |
| E5 | **Approximate PPR** (Forward Push / Monte-Carlo / FAST-PPR) | REJECTED | R1: every faster-than-sparse method trades exact bit-reproducibility for approximation and/or push-order dependence; `csr.rs:21-24` already ruled async local-push out of scope. Reversal = operator decision (§4 D-4), never an engineering default. |
| E6 | **`absorbing.rs` rewrite** | NO ACTION | R2: n fixed at 5 (order-lifecycle FSM; per-state-machine, not per-order), **zero production callers** (bench doc's "agentic decision gating" claim is false — that's one of the two doc fixes in P80). Tier-② topological triangular solve is recorded for if a real n≫5 DAG caller ever appears. |
| E7 | **`ppr.rs` code change today** | NO ACTION | R1: n=20 frozen fixture, bench exists + gated; sparse migration routes (b1/b2) documented and deferred behind the P80 tripwire (Tier D-1). |
| E8 | **`money.rs` algorithmic change** | NO ACTION (binding, §2) | Reconciled R3+R5: per-order n ≤ 2 by construction; scans ARE the fail-closed probes; bench-tripwire only. |
| E9 | **Branchless-ifying `pq_kem::poly_mul`** | REJECTED | R4: wrong fix for a loop whose real remedy is algorithmic (NTT, itself gated on P82 bench + verified pair + operator). "Do not branchless-ify a loop that should be replaced." |
| E10 | **`HybridGate.seen` lock rewrite** | REJECTED (no evidence) | R4: lock held only for an O(1) insert AFTER the dominant crypto (µs–ms verifies); no bench proves contention. Revisit only if a many-core profile shows the seen-lock hot. |
| E11 | **`discovery` PeerDirectory mutex** | NO ACTION | R4: periodic gossip/tick path, cold; correct as written. |
| E12 | **`token_bucket`/`budget`/`admission` Mutex→CAS now** | GATED (bench-first) | R3 A1–A3 + standing rule: no contended bench exists; single-thread numbers don't justify a lock-free rewrite. P80 adds the contended benches; only their results can open a rewrite blueprint. |
| E13 | **`crates/bebop` (legacy TUI) optimization** | OUT OF SCOPE | R4 §5: confirmed dev-tooling off the mesh product path; its Dijkstra/CH routing is textbook-appropriate. (Its ledger/reputation *correctness* items are Tier D-8 / §4 D-2, not perf work.) |
| E14 | **`charpoly` O(n⁴)** (both repos) | NO ACTION | R3 (kernel: documented dead-fallback, n>32 has "no consumer and no path") + R4 P5 (bebop: n≤32, off mesh per-frame path). Note-only. |
| E15 | **Bebop clean paths** — HRW `matcher::assign`, capability-verify chain (`hybrid_gate::check` cost = the two signature verifies), `verify_internal_bytes_many` batch shape, wire codec, FFT, ML-DSA NTT | VERIFIED CLEAN | R4 §2: reported honestly so nobody "optimizes" them; the batch verify is O(1)/sig as expected. |
| E16 | **Kernel clean paths** — `eigenvalues` n≤32 dispatch, drift-gate n=1 callers, `mesh.rs` append O(1), `event_log` HashSet dedup, `cart`/`place_order`, `bm25` linear-not-quadratic, `intake` AC-3 bounded, `order_machine` `queue.remove(0)` n≤12 | VERIFIED CLEAN | R3 negative-results section + R10 §3.2. |
| E17 | **Concurrency non-gaps** — `token_bucket` short section, llm/agent cache lock-shape, `budget` debit-before-submit, native-spa-server store mutex, admission crypto-outside-lock; **no `Arc<Mutex>`-across-`.await` possible** (adapter crates verified zero async) | VERIFIED CLEAN | R8 §4 + Appendix A. |
| E18 | **`assert_transition` span instrumentation** | DELIBERATELY EXCLUDED | R6 §5: per-edge inner loop — a span there violates the hot-path constraint; the `fold_transitions` span + Layer-2 sampler cover it. |

---

*Cross-references: `docs/research/OPUS-PERF-{PPR-ANALYSIS,ABSORBING-MARKOV-ANALYSIS,KERNEL-AUDIT,BEBOP-AUDIT,BENCH-COVERAGE-MAP,METRICS-ARCHITECTURE,REGRESSION-TOOLING-AUDIT,BESTPRACTICES-PROPAGATION,RGB-PACKING-REUSE,POINTER-ARENA-ANALYSIS}-2026-07-18.md` · `.claude/CLAUDE.md:182-195` (standing rule) · `CORE-ROADMAP-STANDARD-2026-07-17.md` (blueprint contract) · `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` (format precedent, P-numbering P57–P74) · `docs/regressions/REGRESSION-LEDGER.md` rows 23–27 · memory: `performance-priority-over-minimal-change-2026-07-17.md`.*
