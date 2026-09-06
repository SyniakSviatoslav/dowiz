Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on A1 (register model landed: the twins must run on the promoted register-model bebop.bin, otherwise the rows measure the stack machine). Decides whether B3-B7 are built as specified or narrowed.

# B2 decisive twins -- SpGEMM join, specialise-then-run scan, CSR-build profile

## 0. Goal

Three measured-first rows before any library code: (i) 2-way join as SpGEMM, gate `>= 10x sqlite native AND >= 0.7x best Rust` on uniform and Zipf keys; (ii) specialise-then-run scan, rows with and without the 50 ms compile; (iii) the sgraph phase-b CSR build profile (45-90 s, HISTORY STORE PULL "to be profiled"). If (i) fails the Rust condition by more than 2x, the thesis narrows to "graph + scan DB": B7's join path stays csr-bucket only and ROADMAP 6b is amended by the main session.

## 1. Scope

In: hand-written kernels only (~60 lines each), the sqlite twins (python ctypes, LANG-DB §8 ctypes-floor rule, verified docs/LANG-DB-DESIGN.md:400), Rust twins in bench/vs_rust/rust_once/ (verified: k0..k8h exist, built by honest.sh with `rustc -O`), one report file bench/vs_rust/RESULT-twins.md. Out: gb.bp, generators, DSL (B3/B7); any store change.

## 2. Preconditions

- csr_build in selfhost/std/sgraph2.bp:24 (counting sort over (src,dst) into rp/ci store objects) -- the join's build side reuses it verbatim.
- csr_from_edges in selfhost/std/csr.bp:20 is the 4096-edge selection-sort twin of Rust csr.rs -- NOT for 1M rows (m <= 4096, verified csr.bp:19).
- honest.sh method (verified bench/vs_rust/honest.sh:1-30): REPS=100 in-process, R=11 medians, pinned A78 core, `rustc -O`; the twin rows follow it.
- sbench_sqlite.py opens sqlite through ctypes with prepared statements (verified bench/tq_sqlite/sbench_sqlite.py:20-24); the join twin extends it.
- Register-model binary promoted (A1 VERDICT GREEN).

## 3. Design

(i) Join. Data: `R(k,a)`, `S(k,b)`, 1M rows each, k in [0, 1M): uniform (LCG 4711) and Zipf(1.1) with 1 % of keys carrying 30 % of the rows (deterministic generator in both bebop and python so folds agree). Output: `count` of matching pairs and `checksum = sum((a*b) mod 2^61)` over pairs -- exact in wraparound i64, equal across engines.

```
bebop join_spgemm(R, S):
  rp, ci = csr_build(S by k)                 # sgraph2.bp:24; ci holds row ids of S, vv holds b
  for r in 0..|R|: k = Rk[r]; for j in rp[k]..rp[k+1]: cnt += 1; sum += Ra[r] * Sb[ci[j]]
  = Gustavson row of R^T . S over plus-times restricted to the count/checksum reduce
```
Rows: `csr_build` ms (separately), probe ms, total ms; Rust: `HashMap<u64, Vec<u32>>` build + probe, and sort-merge (sort both by k, merge); sqlite: `SELECT count(*), sum((a*b)%...) FROM R JOIN S USING(k)` with an index on S.k (indexed nested loop) and without (sqlite's automatic index / hash plan), `EXPLAIN QUERY PLAN` recorded, ctypes floor subtracted per the §8 rule.

(ii) Specialise-then-run. Schema fixed by the twin: 3 i64 columns, predicate `c0 in [lo,hi) and c1 < q`, aggregate `sum(c2)`. bebop row A: generate `scan_<digest>.bp` from a template (string building in .bp, ~40 lines), `bebop.bin compile` it (wall ms measured by the driver), run; row B: run only (memo hit). Rust twin: a generic scan taking the schema (column count, predicate columns, bounds) at runtime through `&[i64]` and a small interpreter of the predicate (the honest "runtime schema" form) -- and a second Rust row with the schema as `const` generics (the best form, = what bebop generates). Gate: latency-to-result incl. compile <= 1/5 of Rust generic at 1M rows; scan-only ~1x of Rust const.

(iii) CSR-build profile. sgraph phase b (verified sgraph2.bp:70 `phase_build`, :24 `csr_build`): time each phase by `clock_ms` deltas printed by the program (differential timing; `pcprof.sh` is unreliable under proot -- memory pitfall): pair generation, counting pass, prefix sum, fill pass, `st_seal`, commit. Report the top phase and its ns per edge with the register-model binary; this row decides whether B4's L0 rebuild budget (<= 2 ms for 2^18 edges) is realistic.

## 4. Files and functions touched

| file | change |
|---|---|
| bench/vs_rust/std_tests/join_twin.bp | new: generator, csr_build call, probe, folds (~120 lines incl. both distributions) |
| bench/vs_rust/std_tests/scan_gen.bp | new: template writer + runner for (ii) (~80 lines) |
| bench/vs_rust/rust_once/join_hash.rs, join_merge.rs, scan_generic.rs, scan_const.rs | new twins (~40 lines each) |
| bench/tq_sqlite/join_sqlite.py | new twin (ctypes, indexed / no-index plans) |
| bench/vs_rust/twins.sh | new driver (honest.sh style: PIN, R=11, REPS) writing RESULT-twins.md |
| selfhost/std/sgraph2.bp `phase_build` :70 | add clock_ms prints per sub-phase under a `t` flag (no fold change) |

## 5. Steps

1. join_twin.bp + python oracle fold + `join_sqlite.py` + Rust twins + twins.sh (rows i); folds equal across 4 engines before any timing row is trusted.
2. scan_gen.bp + Rust scan twins (rows ii).
3. phase_build sub-phase timing (row iii) -- sgraph2 fold unchanged (std_golden gate `sgraph2` stays green).
Each step: battery GREEN (no codegen change -> chain without --codegen only if bebop.bp untouched; here bebop.bp is untouched, so `tools/battery.sh` suffices), one journal line, RESULT-twins.md appended.

## 6. Constructs, oracles, twins

- Oracle: bench/oracles/join_twin.py (stdlib) producing (count, checksum) for both distributions; registered as std_golden gate `join_twin`.
- Twins: as in §4; sqlite plan text stored next to the row.
- No parity construct (no codegen change).

## 7. Gates

```
BEBOP_BIN=./bebop.bin BEBOP_TMP=$OUT R=11 bash bench/vs_rust/twins.sh
# rows: join uniform/zipf: bebop ms, rust_hash ms, rust_merge ms, sqlite_idx ms, sqlite_noidx ms
# gate (i): bebop <= sqlite_best/10 AND bebop <= rust_best/0.7 on BOTH distributions
# gate (ii): bebop_total(compile+run) <= rust_generic/5 ; bebop_run <= 1.5 x rust_const
# row (iii): phase table, top phase named
```
RED = a fold mismatch (stop, it is a bug, not a number), or the Rust condition failing by > 2x (report; the main session amends ROADMAP 6b/B7).

## 8. Risks and probes

| risk | probe |
|---|---|
| sqlite plan choice hides a hash join | run both plans, keep the faster as "sqlite native" |
| Zipf heavy keys blow the output (Σ m_r·m_s) equally for all engines | cap heavy-key multiplicity so the output is ~10M pairs; state it |
| ctypes floor dominates sqlite rows | measure the floor with an empty statement as §8 prescribes and subtract |
| the generic Rust scan is strawmanned | its predicate interpreter is a 3-branch match over an enum, no allocation; publish the source |

## 9. VERDICT format

```
VERDICT: GREEN|RED|NARROW
join_uniform: bebop <ms> rust_hash <ms> rust_merge <ms> sqlite_idx <ms> sqlite_noidx <ms> -> ratios
join_zipf: same
scan: bebop compile <ms> + run <ms> ; rust_generic <ms> ; rust_const <ms>
csr_build_profile: <phase: ns/edge ...> top=<phase>
folds: equal|MISMATCH <which>
journal: <line>
open: <deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint + honest.sh/sbench.sh as templates + the §8 ctypes rule + $OUT + reap/proc-cap rules; `<constraints>` no library code, kernels only, folds before timings, no bebop.bp change; `<output_format>` §9; `<task>` steps 1-3.
