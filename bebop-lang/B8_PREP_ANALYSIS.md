# B8 Prep Analysis — W End-to-End Workload

**Blueprint**: `docs/blueprints/B8-workload-W.md`  
**HEAD**: d0cbdfb (main, 2026-09-11; refreshed from 4d86e96/2026-09-10)  
**Date**: 2026-09-10, refreshed 2026-09-11

---

## Executive Summary

B8 is the capstone workload that proves the tensor-graph DB thesis on the real dowiz-core order log (T66). It requires **all B1-B7 + A7** to be complete and green. Current status: **BLOCKED** — multiple dependencies are incomplete or have open defects.

---

## B8 Requirements (from Blueprint)

### Deliverables
| File | Description |
|------|-------------|
| `selfhost/std/wlog.bp` | 8-phase W program (~400 lines): ingest (i), q1-q4, updates (u), crash (c), reopen (r), parallel (p) |
| `bench/oracles/wlog.py` | Folds for q1-q4/u/r via production Rust oracle (ordfsm) + money aggregates |
| `bench/tq_sqlite/wlog_sqlite.py` | SQLite twin: same 8 phases, two tables + two indexes, ctypes + WAL |
| `bench/vs_rust/wlog.sh` | Driver (sgraph2.sh style), writes `RESULT-wlog.md` |
| `bench/vs_rust/std_golden.sh` | Gate additions at :737 for `wlog_q1..q4`, `wlog_u` |
| `docs/LANG-DB-DESIGN.md §10` | W data model + claim table |

### Data Model (Store Objects)
```
Orders    {n, ref meta (created_t, customer, total_money)}           -- SoA arrays
Events    {m, ref order_id[], ref from[], ref to[], ref t[], ref amount[]}  -- append log, time-ordered
EvByOrder GbMatrix (B3/B4 tiered): rows = orders, ci = event ids       -- order -> events
EvByT     GbMatrix: rows = t-buckets (per hour), ci = event ids        -- rank-3 reading (B7)
FSM       12x12 dense adjacency (ordfsm.bp adj masks), A^12 == 0
State     GbVector dense: current state per order (= fold of EvByOrder row through FSM)
```

### Phases (argv letter)
| Phase | Operation | Metric |
|-------|-----------|--------|
| `i` | Ingest N orders × k events from byte file (A7 raw handles) | ms, MB RSS |
| `q1` | Current state of every order (mxv over EvByOrder with FSM fold) | fold: state histogram |
| `q2` | Orders in state s at time t (EvByT bucket + prefix fold) | fold |
| `q3` | Illegal-transition audit (decide over consecutive pairs) | fold = ordfsm codes |
| `q4` | Revenue by state (money op_add over State-masked amounts) | fold = money-exact |
| `u` | Apply E new events (B4 assign + FSM check; B5 partitions by order range) | us/event, stall |
| `c` | Crash during u (B1 harness) | — |
| `r` | Reopen + q1 | rows |
| `p` | q1/q2 with P = 1/2/3 cores (B6) | rows |

### Gates (Non-Negotiable)
- **Every fold == production Rust oracle** (T66: `bench/oracles/rust/src/bin/ordfsm.rs` → `crates/dowiz-core/src/order_machine.rs`)
- Crash trials: reopen at gen k or k-1 only (TRIALS=20 in battery)
- Performance targets (report, not gates until D11-I freeze):
  - ingest ≥ 10x sqlite
  - q1 ≥ 20x sqlite
  - q2 ≥ 10x sqlite
  - u ≥ 5x sqlite
  - r ≤ 2 ms
  - p3/p1 ≥ 1.4

---

## Dependency Status Matrix

| Dep | Blueprint | Status | Blocks B8? | Notes |
|-----|-----------|--------|------------|-------|
| **A7** | `A7-byte-arena-and-str-values.md` | **NEAR-COMPLETE 2026-09-10** | **NO (verify gate)** | Steps 1-3 LANDED: crc32b + c68_strval (2ada1bb), handle migration (7c03111), sys_readbuf/sys_mapb emitters + str_to_cells (6fb9830). Phase `i` ingest path exists; confirm c68 gate green on current HEAD. |
| **B1** | `B1-durability-torn-write.md` | **PARTIAL** | **YES** | Step 1 (sys_fsync) LANDED 2026-09-08. Steps 2-3 (st_commit_batch, st_verify, scrash_torn) LANDED 2026-09-07. **Recovery follow-up card OPEN** (2026-09-09) — `st_verify` scan optimization not done. Crash harness needed for phase `c`. |
| **B2** | `B2-decisive-twins.md` | **COMPLETE** | No | Twins measured 2026-09-09. Join path decided: SpGEMM allowed (D2 probe met gate). CSR build profile RUN: fill pass 30.6x store vs plain, 95.6% of build time. **This IS the B8 profile row**. |
| **B3** | `B3-graphblas-kernels-prejit.md` | **PARTIAL** | **YES** | Steps 1-3 LANDED (objects, gen_gb mxv/vxm/mxm, G9b folds). **Step 4 (pool + PreJIT + background compile) OPEN DEFECT CANDIDATE 2026-09-08** — pool store size bug (4 MiB vs 11.4 MB needed), compiler_digest check exists but answer unread. **FIXED 2026-09-08 per ROADMAP** but verify gate `gb_pool_reuse` passes. |
| **B4** | `B4-functional-tensor-updates.md` | **RE-SCOPED** | **YES** | Step 1 (tail + L0 promotion) — stall target ≤ 10 ms. **Step 2 (RowBlock/blocktab L1) REFUTED 2026-09-08** — sharing never fires on 1M updates/1M rows; whole-L1 rewrite 185 ms vs 10 ms gate. **Step 2''' (tail delta only) built** but L1 merge not done. Phase `u` needs B4 assign/promote working. |
| **B5** | `B5-multi-writer.md` | **STEP 1 LANDED 2026-09-10** | **PARTIAL** | PartTab + _p API, P=1 degenerate case (4d86e96, current HEAD's ancestor). Steps 2 (P writers, G10 ≥2x) + 3 (2PC atomic) still OPEN. Phase `u` single-partition path unblocked; cross-partition + `p`-with-writers still blocked. |
| **B6** | `B6-multi-core-kernels.md` | **REVIVED 2026-09-08** | **YES** | Mandatory per operator. `par_run` over A78 cores. Step 1 (scan) NOT DONE. Needed for phase `p` (q1/q2 at P=1/2/3). Depends on B3, B5. |
| **B7** | `B7-dsl-planner.md` | **STEP 1 LANDED 2026-09-10** | **PARTIAL** | qdsl DSL parser + bpref support + c70 constructs (c99cddb). Steps 2 (planner + Q6/Q1) + 3 (join + rank-3) still OPEN. q2/q4 still blocked on planner. |
| **T66 Oracles** | `ordfsm.bp`, `money.bp` | **COMMITTED** | No | `selfhost/std/ordfsm.bp` (327 lines), `selfhost/std/money.bp` (261 lines). Oracle Python wrappers exist: `bench/oracles/ordfsm.py`, `bench/oracles/money.py`. Rust oracle binary exists: `bench/oracles/rust/src/bin/ordfsm.rs`. |

---

## What's Ready (Green)

1. **T66 Oracles** — `ordfsm.bp` and `money.bp` committed, gates `ordfsm` and `money` in `std_golden.sh` (lines 767-769, 763-765).
2. **B2 Twins** — Measured, join verdict recorded in `RESULT-twins.md`, CSR profile row produced (fill pass = 30.6x store overhead).
3. **B3 Steps 1-3** — `gb.bp` objects, `gen_gb.bp` templates for mxv/vxm/mxm/eWiseAdd/Mult/select/apply/reduce, G9b folds (`gb_tc`, `gb_cc`, `gb_sssp`, `gb_pr`) passing.
4. **B1 Steps 1-3** — `sys_fsync` builtin, `st_commit_batch`, `st_verify`, `scrash_torn` harness (TRIALS=50 gate).
5. **Infrastructure** — `sgraph2.sh` phase driver template, `bench/tq_sqlite` sqlite twin infrastructure, `bench/vs_rust/twins.sh` driver pattern.

---

## What's Missing / Blocked (Red)

### Critical Path Blockers (must land before B8 can start)

| # | Blocker | Blueprint Step | Why B8 Needs It |
|---|---------|----------------|-----------------|
| 1 | **A7 raw-byte ingest + str handles** | A7 steps 1-3 | Phase `i` ingests from byte file via `sys_mapb` + `char`/`str_len`. Without A7, no ingestion path exists. |
| 2 | **B4 Phase `u` assign/promote** | B4 step 1 (tail + L0) + step 2''' (tail delta) | Phase `u` applies E new events via `assign` + FSM check. B4 step 1 must achieve ≤ 10 ms stall (currently 747 ms → needs tail + bounded L0). Step 2''' (tail delta maintenance) built but L1 merge not done — `promote_L0` must work for phase `u` to not stall. |
| 3 | **B5 Multi-writer** | B5 steps 1-3 | Phase `u` partitions by order range (disjoint partitions). Phase `p` runs readers on snapshots while writers run. B5 is **mandatory per operator**. Depends on B1 (fsync/harness), B4 (per-matrix versions). |
| 4 | **B6 Multi-core kernels** | B6 steps 1-3 | Phase `p` runs q1/q2 at P=1/2/3 cores. Needs `par_run` in `gen_gb.bp` + `gb.bp` core discovery. Depends on B3 (kernels), B5 (snapshot readers). |
| 5 | **B7 DSL + Planner** | B7 steps 1-3 | Phases q2/q4 use B7 DSL over EvByT. Phase `u` may use planner for update pipeline. Depends on B3 (gen_gb), B2 (join verdict), B4 (tiered reads), B6 (par). |
| 6 | **B3 Step 4 (Pool + PreJIT)** | B3 step 4 | Instant start requirement: first query ≤ 1 ms (tier-0), specialised ≤ 50 ms background, repeats 0 ms (pool hit). B8's `wlog.bp` queries must hit pool or tier-0. **Defect fixed 2026-09-08** but verify `gb_pool_reuse` gate passes. |

### Secondary Gaps

| Item | Status | Impact |
|------|--------|--------|
| `wlog.bp` | Not created | New file ~400 lines, reuses `ordfsm.bp` `decide`/`step` and `money.bp` `op_*` via `use` |
| `wlog.py` oracle | Not created | Folds q1-q4/u/r via Rust oracle binary + stdlib aggregates |
| `wlog_sqlite.py` twin | Not created | 8 phases, two tables (orders, events), two indexes, ctypes + prepared + WAL |
| `wlog.sh` driver | Not created | `sgraph2.sh` style, writes `RESULT-wlog.md`, TRIALS for crash phase |
| `std_golden.sh` gates | Not added | Lines ~737: `wlog_q1`, `wlog_q2`, `wlog_q3`, `wlog_q4`, `wlog_u` |
| `LANG-DB-DESIGN.md §10` | Not written | W data model + claim table (defensible vs never-claimed) |
| `ROADMAP.md` TG-DONE 7 | Not updated | Store row becomes "W measured" |

---

## Design Decisions Needed (Before Implementation)

1. **B4 L1 Merge Strategy** — Step 2 refuted; step 2''' (tail delta only) avoids L1 merge during phase. But phase `u` may need L1 merge eventually. Decision: **defer L1 merge to compaction** (B4 step 3 / `st_compact`), keep L1 static during W run. Confirm with B4 owner.

2. **B5 Partition Count** — B8 says "partitioned by order range". How many partitions? B5 defaults P=3 (A78 pairs). W's order count (1M) / 3 = ~333k orders/partition. Confirm partition count matches B5.

3. **B7 Query Shape for q2/q4** — q2 = "orders in state s at time t" (EvByT bucket + prefix fold). q4 = "revenue by state" (money op_add over State-masked amounts). Are these single DSL queries or fused kernels? B7 templates: scan-filter-agg (Q6), scan-group-agg (Q1). q2 ≈ Q6 shape; q4 ≈ Q1 shape. Confirm planner picks correct template.

4. **Crash Phase `c` Injection Point** — B1 `scrash_torn` harness kills at random commit. B8 phase `c` = "crash during u". Need to synchronise: run phase `u` for N updates, then SIGKILL at a specific point (e.g., after 50% of events). Harness modification needed.

5. **Performance Targets Freeze** — B8 §7: "operator freezes a/b/c BEFORE the run (D11-I)". Must agree on ingest/q1/q2/u/r/p targets before measuring. Current targets in blueprint are "report, not gates until D11-I".

---

## Implementation Order (Chain-Gated Commits)

Per B8 §5 and ROADMP critical path:

```
1. A7 (raw ingest + str)          → 2-3 chain commits
2. B4 step 1 (tail + L0 promote)  → 1 commit (stall ≤ 10 ms)
3. B4 step 2''' (tail delta)      → 1 commit (folds unchanged)
4. B1 recovery follow-up card     → 1 commit (st_verify_range)
5. B5 step 1 (PartTab, P=1)       → 1 commit (degenerate case, gates unchanged)
6. B5 step 2 (P writers, G10)     → 1 commit (throughput ≥ 2x)
7. B5 step 3 (2PC, crash)         → 1 commit (atomic cross-partition)
8. B3 step 4 (pool + PreJIT)      → 1 commit (--codegen for builtin words)
9. B6 step 1 (scan par_run)       → 1 commit (nn4 fold unchanged)
10. B6 step 2 (mxv/vxm/BFS par)   → 1 commit
11. B6 step 3 (mxm + B5 concurrency) → 1 commit
12. B7 step 1 (DSL parser + AST)  → 1 commit (c70_qdsl construct)
13. B7 step 2 (planner + Q6/Q1)   → 1 commit (folds == sqlite == oracle)
14. B7 step 3 (join + rank-3)     → 1 commit (latency rows)
15. B8 step 1 (wlog.bp i + q1/q3) → 1 commit (correctness core)
16. B8 step 2 (q2/q4 + u)         → 1 commit
17. B8 step 3 (c/r/p + RESULT)    → 1 commit
```

**Estimated**: 17 chain-gated commits minimum. At ~1 commit/day = 3-4 weeks.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| A7 not landed before B8 start | HIGH | BLOCKS entirely | A7 is on critical path (ROADMAP row 94), must complete first |
| B4 L1 merge still too slow | HIGH | Phase `u` stalls > 10 ms | B4 step 2''' avoids L1 merge; validate with `sgraph2.sh` phase `u` row |
| B5 2PC complexity | MEDIUM | Cross-partition commit bugs | B5 step 1 (P=1) validates degenerate case first; G6 unchanged |
| B7 planner chooses wrong access path | MEDIUM | q2/q4 slower than sqlite | `explain` prints plan; B7 §8 probe: "plan choosing scan where bucket exists" |
| Pool kernel ABI drift (B3) | LOW (fixed) | Trap 82 on kernel dispatch | `gb_pool_reuse` gate validates cross-compiler pool read |
| Crash harness `c` phase not representative | MEDIUM | Invalid reopens not caught | Modify `scrash_torn` to kill during phase `u` specifically |

---

## Verdict

**STILL BLOCKED (refreshed 2026-09-11)** — but the blocker set shrank: **A7 steps 1-3 and B5 step 1 and B7 step 1 have landed since this analysis was written.** B8 cannot start until **B4 step 1 (stall ≤ 10 ms), B5 steps 2-3, B3 step 4 verification, B6, B7 steps 2-3** are complete and green. B8 step 1 (wlog.bp phases `i`+q1/q3) is now unblocked on the ingest side and could start in parallel once B4 step 1 lands. The critical path is:

```
A7 → B4 step 1 → B5 step 1 → B3 step 4 → B6 → B7 → B8
```

**Earliest B8 start**: After A7 lands (ROADMAP shows A7 depends on A6, which is COMPLETE 2026-09-09). A7 is 2-3 chain commits. Then B4 step 1 (1 commit), B5 step 1 (1 commit), B3 step 4 (1 commit), B6 step 1 (1 commit), B7 step 1 (1 commit). **~6-8 commits before B8 step 1 can begin.**

---

## Files to Create (B8 Scope)

| File | Lines | Template |
|------|-------|----------|
| `selfhost/std/wlog.bp` | ~400 | `sgraph2.bp` phase structure |
| `bench/oracles/wlog.py` | ~200 | `bench/oracles/ordfsm.py` + `money.py` pattern |
| `bench/tq_sqlite/wlog_sqlite.py` | ~300 | `bench/tq_sqlite/sgraph_sqlite.py` pattern |
| `bench/vs_rust/wlog.sh` | ~150 | `bench/vs_rust/sgraph2.sh` pattern |
| `bench/vs_rust/std_golden.sh` | +5 gates | Add at ~line 737 |
| `docs/LANG-DB-DESIGN.md` | +§10 | Append after §9.5 |

---

## Next Actions

1. **Complete A7** (raw-byte ingest + str handles) — unblocks ingestion path
2. **Land B4 step 1** (tail + bounded L0 promotion) — unblocks phase `u` stall target
3. **Land B5 step 1** (PartTab, P=1 degenerate case) — unblocks multi-writer foundation
4. **Verify B3 step 4** (pool + PreJIT) — `gb_pool_reuse` gate must pass
5. **Freeze B8 performance targets** with operator (D11-I) before measurement runs