Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on A7 (raw-byte path + str values) and B1-B7. The acceptance of the tensor-graph-DB thesis on the real workload W = the dowiz-core order log (D14 item 8; T66).

# B8 workload W end to end -- the order log as a tensor-graph store, measured against sqlite

## 0. Goal

Run dowiz-core's order lifecycle on the store end to end: ingest events through the raw path, keep `order -> events` as a CSR and the FSM as the 12x12 nilpotent matrix, answer the W queries through B7, apply updates through B4/B5, survive B1's crash model, scale scans over cores (B6) -- and publish the G-rows that say what "replace SQL" means for W and what is never claimed. Gates: every fold == the production Rust oracle (T66: bench/oracles/rust/src/bin/ordfsm.rs calls crates/dowiz-core/src/order_machine.rs -- verified selfhost/std/ordfsm.bp:3-6), plus the sqlite twin rows.

## 1. Scope

In: `selfhost/std/wlog.bp` (the W program: ingest, index, queries, updates, crash phases), the event generator shared with the oracles (LCG sequences as ordfsm.bp:17 B2), the sqlite twin schema (orders, events, one index each), REPORT rows in RESULT-sgraph.md/REPORT-honest.md, the LANG-DB-DESIGN §10 "W" section. Out: money.bp's arithmetic beyond using its ops as the aggregate (verified selfhost/std/money.bp:43-113 `op_*`), any dowiz-core code change.

## 2. Preconditions

- ordfsm.bp (verified :7-22): 12 states, 14 edges, `decide`:33 (0 ok | 1 SameStatus | 2 ScaffoldDisabled | 3 IllegalTransition), `step`:58 (fold freezes on the first error), signature (12, 14, acyclic, mu 4, rho 0, reach 3839, Some(12)); gate `ordfsm` in std_golden.sh:737; money.bp gate :733-735 with oracle bench/oracles/money.py.
- Store objects and CSR (sgraph2.bp), gb.bp (B3), tiered updates (B4), partitions (B5), par (B6), DSL (B7), raw ingest + `str` handles (A7).
- sqlite twin infrastructure (bench/tq_sqlite/*.py).

## 3. Design

Data model in the store (all objects append-only, refs object-relative):
```
Orders    {n, ref meta (per-order: created_t, customer, total_money)}                     -- SoA arrays
Events    {m, ref order_id[], ref from[], ref to[], ref t[], ref amount[]}                 -- append log, time-ordered (t = zone-map)
EvByOrder GbMatrix (B3/B4 tiered): rows = orders, ci = event ids                            -- order -> events
EvByT     GbMatrix: rows = t-buckets (per hour), ci = event ids                             -- the second mode (rank-3 reading, B7)
FSM       12x12 dense adjacency (ordfsm.bp adj masks), A^12 == 0 asserted at open
State     GbVector dense: current state per order (= fold of EvByOrder row through FSM)
```
Phases of wlog.bp (argv phase letter, sgraph2 style):
```
i  ingest N orders x k events from a byte file (A7 raw handles; twin: sqlite executemany in one tx)   rows: ms, MB RSS
q1 current state of every order (mxv over EvByOrder with the FSM fold as ⊗ -> reduce)                  fold: state histogram
q2 orders in state s at time t (EvByT bucket + prefix fold)                                              fold
q3 illegal-transition audit (decide over every consecutive pair; count by code)                          fold = ordfsm codes
q4 revenue by state (money op_add over State-masked amounts)                                             fold = money-exact
u  apply E new events (B4 assign + FSM check; B5 partitions by order range)                              rows: us/event, stall
c  crash during u (B1 harness) ; r  reopen + q1                                                         rows
p  q1/q2 with P = 1/2/3 cores (B6)                                                                       rows
```
"Replace SQL" for W, concretely: the sqlite twin implements the same eight phases with two tables and two indexes; the claim is per row: ingest, q1-q4, u, r, p -- each a ratio with the ctypes floor subtracted. Never claimed (RESEARCH-NOPOINTERS-SQL §2.4): concurrent writers beyond partitions, ad-hoc SQL over W, sub-50 ms first-shape queries not in the pool, datasets beyond RAM.

Failure modes: an illegal transition in the event stream (q3 counts it; u refuses it with code 3 -- the fold is the production oracle's), FSM drift vs order_machine.rs (the signature check at open catches a wrong adjacency), t-bucket skew (report bucket max).

## 4. Files and functions touched

| file | anchor | change |
|---|---|---|
| selfhost/std/wlog.bp | new | phases above (~400 lines), reusing ordfsm.bp decide/step and money.bp ops via `use` |
| bench/oracles/wlog.py | new | folds for q1-q4/u/r via the Rust oracle binary for transitions (bench/oracles/rust/src/bin/ordfsm.rs) + stdlib for aggregates |
| bench/tq_sqlite/wlog_sqlite.py | new | twin phases (ctypes, prepared, WAL) |
| bench/vs_rust/wlog.sh | new | driver (sgraph2.sh style), writes RESULT-wlog.md; TRIALS for phase c |
| bench/vs_rust/std_golden.sh | :737 | gates wlog_q1..q4, wlog_u |
| docs/LANG-DB-DESIGN.md | new §10 | W data model + the claim table |
| ROADMAP.md TG-DONE 7 | row | the store row becomes "W measured" |

## 5. Steps

1. Data model + ingest (phase i) + q1/q3 (state and audit folds == oracle) -- the thesis' correctness core.
2. q2/q4 via B7 DSL + EvByT; u via B4/B5; folds.
3. c/r/p phases; RESULT-wlog.md; LANG-DB §10; TG-DONE 7 row.
Each step one chain-gated commit.

## 6. Constructs, oracles, twins

- Oracles: wlog.py (transitions through the PRODUCTION Rust binary as T66 does; money aggregates byte-exact via money.py rules).
- Gates: wlog_q1, wlog_q2, wlog_q3, wlog_q4, wlog_u (std_golden); crash phase in the battery with TRIALS 20.
- Twin: wlog_sqlite.py rows for every phase.

## 7. Gates

```
tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp      # GREEN incl. wlog_* gates
BEBOP_TMP=$OUT bash bench/vs_rust/wlog.sh                   # RESULT-wlog.md: per phase store / sqlite / ratio; targets (report, not gates
                                                            #  until the operator freezes them, D11-I): ingest >= 10x, q1 >= 20x, q2 >= 10x, u >= 5x, r <= 2 ms, p3/p1 >= 1.4
```
RED: any fold != the production oracle (this is the one non-negotiable gate), a crash trial reopening at a gen other than k/k-1.

## 8. Risks and probes

| risk | probe |
|---|---|
| the generator produces only legal sequences (audit q3 trivial) | inject 1 % illegal steps as ordfsm B2 does; q3 must count them |
| money exactness across bebop/python/Rust | reuse money.py's rules; fold = (h, oks, errs) triple |
| thresholds argued after the fact | the operator freezes a/b/c BEFORE the run (D11-I), recorded in the journal |
| W is too small to show DRAM effects | N = 1M orders x 8 events (64 MB) minimum; row with 100k as the "small" case |

## 9. VERDICT format

```
VERDICT: GREEN|RED
folds: q1 q2 q3 q4 u r -> equal|MISMATCH (oracle = production Rust)
rows: ingest <ms>/<ms> <x> ; q1 ... ; q2 ... ; q3 ... ; q4 ... ; u us/event <v>/<v> ; r <ms>/<ms> ; p1/p3 <ms>/<ms>
crash: trials <n> failures <k>
claims: defensible <list> ; never <list>
journal: <line>
open: <deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint; ordfsm.bp/money.bp and their oracles (T66); sgraph2.bp/sgraph2.sh as the phase-driver template; B3-B7 APIs; $OUT; rules; `<constraints>` folds == production oracle before any timing, thresholds frozen by the operator first, zero deps; `<output_format>` §9; `<task>` steps 1-3.
