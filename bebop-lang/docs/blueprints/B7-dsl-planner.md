Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on B3 (kernels + pool), B2 (join verdict: SpGEMM join allowed or csr-bucket only), B4 (tiered reads), B6 (par). Feeds B8.

# B7 associative-array DSL + planner -- compiled queries over the store

## 0. Goal

A query surface over associative arrays (not SQL) whose every query becomes a generated kernel: `q { from T where p group by k agg s join U on k order by e limit n }` -> AST -> planner (access path, join order <= 4, mode choice for rank-n) -> gen_gb -> pool/tier-0/compile -> result. Gates: Q6 >= 10x and Q1 >= 5x sqlite native on lineitem SF 0.1 in the store; first-query (tier 0) and repeat (pool) latency rows; a rank-3 construct (time x order x state as two CSRs) with folds == oracle.

## 1. Scope

In: the DSL parser (~200 lines, .bp), AST as store cells, the planner (~200), the generator glue to gen_gb (~250: Q6/Q1/join/order-by templates), the twins. Out: SQL text, NULLs, strings beyond handles (A7), n-way joins > 4 (report "unsupported", not a heuristic), a cost model with histograms (exact `rp` cardinalities only). Fixed points: everything runs through B3's kernel tiers; no interpreter of query plans at runtime except tier 0.

## 2. Preconditions

- Exact cardinalities: `rp[k+1]-rp[k]` (CSR contract csr.bp:5), zone-maps and tombstone masks (LANG-DB §9.4-9.5, verified :608, :638), the nnidx bucket index (bench/tq_sqlite/nnidx.bp; T100 4.0 us window).
- sqlite twin infrastructure: bench/tq_sqlite/sbench_sqlite.py (ctypes, prepared; :20-24), the §8 ctypes-floor rule (LANG-DB :400).
- B2 verdict recorded in RESULT-twins.md (join path).
- DuckDB: not installable here (no apt/pip, verified in RESEARCH-NOPOINTERS-SQL §2.2) -- published numbers only, marked "not here".

## 3. Design

Grammar (bpref mirror required, as for every surface addition):
```
query   := 'q' '{' 'from' ident (join)* ('where' pred)? ('group' 'by' keylist ('agg' agglist)?)? ('order' 'by' expr ('desc')?)? ('limit' int)? '}'
join    := 'join' ident 'on' ident ('=' ident)?          -- equi-join on a key column (index = CSR by that key)
pred    := cmp ('and' cmp)*                              -- conjunctions only; 'or' = union of two queries (v2)
agglist := (sum|count|min|max|avg) '(' ident ')' (',' ...)*
```
AST = store objects `{kind, ref children[], attr}` (so a plan can be memoised by digest like a kernel).

Planner (exact, tiny):
```
access(T, pred):  if pred has k = c on an indexed key -> bucket(k) [rp exact]; elif zone-map on a range column -> block skip; else scan
join order:       for k <= 4 tables enumerate k! orders; cost(order) = Σ (input rows exact from rp/bucket counts x access cost from a 3-row table: scan 1, bucket 20, spgemm-probe 15 [ns/row, measured in B2])
group-by:         if |keys| <= 4096 known (rp count) -> dense accumulators; else counting-sort buckets (csr_build)
order-by/limit:   radix sort i64 (4 x 16-bit passes, ~120 lines, replaces csr.bp's selection sort) ; limit n -> heap top-k when n <= 1024
mode choice:      rank-n data = one CSR per mode ordering present in the store; pick the CSR whose leading mode is the most selective predicate
```
Generation: the plan tree maps to ONE fused kernel template when it is scan-filter-agg (Q6 shape) or scan-group-agg (Q1 shape), and to a 2-kernel pipeline (build CSR of the smaller side = transpose kernel; probe = mxm any-pair/plus-second) for joins. The digest of (plan shape, semiring, schema) keys the pool (B3): the first Q6 shape compiles once, every later Q6 over any constants is a pool hit (constants are kernel ARGUMENTS, not template text -- this is what makes "repeat = 0 ms" true across different constants).

Rank-3 construct: events (order, state, t) stored twice: CSR by order (rows = orders, ci = event ids) and CSR by t-bucket; the query `q { from events where t in [a,b) and order = o }` picks the t-CSR when the bucket count is smaller than the order's degree -- the construct checks the planner's choice against a brute-force fold.

Failure modes: a plan shape without a template -> tier 0 generic pipeline (interpreted operators over kernels; row reported) -- never a silent fallback to scanning everything; join order enumeration for k > 4 -> error 92 "unsupported"; a predicate on a non-indexed column with no zone-map -> scan (reported by `explain`).

## 4. Files and functions touched

| file | anchor | change |
|---|---|---|
| selfhost/std/qdsl.bp | new | parser + AST (~200) ; `explain` printing the plan |
| selfhost/std/qplan.bp | new | planner (~200) |
| selfhost/std/gen_gb.bp | B3 | query templates: scan-filter-agg, scan-group-agg, join pipeline, order-by/limit (~250) |
| selfhost/std/csr.bp | :20 selection sort | radix sort helper (~120) shared with order-by |
| tools/bpref.py | grammar | DSL mirror (parse + evaluate over python lists) |
| bench/vs_rust/std_tests/tpch_lineitem.bp | new | SF 0.1 generator (deterministic), Q6, Q1, join with `orders`, rank-3 construct |
| bench/tq_sqlite/tpch_sqlite.py | new | same data, prepared Q6/Q1/join, EXPLAIN QUERY PLAN recorded |
| bench/vs_rust/twins.sh | B2 | rows q6, q1, join, first/repeat latency |

## 5. Steps

1. DSL parser + AST + bpref mirror + `explain`; construct `c70_qdsl` (parse -> explain string fold).
2. Planner + scan-filter-agg / scan-group-agg templates; Q6/Q1 folds == sqlite == python oracle; rows.
3. Join pipeline (per B2 verdict) + order-by/limit (radix) + rank-3 construct; rows; latency rows first/repeat.
Each step one chain-gated commit (`--codegen` only if bebop.bp's parser changes for the DSL -- prefer implementing the DSL in .bp over a compiler change).

## 6. Constructs, oracles, twins

- Oracle: bench/oracles/tpch.py (stdlib) for Q6/Q1/join folds and the rank-3 fold.
- Gates: `c70_qdsl` (construct), std_golden `tpch_q6`, `tpch_q1`, `tpch_join`, `rank3`.
- Twins: sqlite native (ctypes floor subtracted), DuckDB published (marked), Rust hand-written Q6/Q1 (rust_once/q6.rs, q1.rs) for the "vs best Rust" column.

## 7. Gates

```
tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp     # GREEN incl. the four gates
BEBOP_TMP=$OUT bash bench/vs_rust/twins.sh                 # q6: bebop <= sqlite/10 ; q1: bebop <= sqlite/5 ; join per B2 ; first-query <= 1 ms tier0, repeat <= 0.1 ms
```
RED: fold mismatch; a plan choosing scan where a bucket exists (explain shows it); Q6 slower than 10x sqlite with the register model (template not fused: check that the generated .bp has one loop).

## 8. Risks and probes

| risk | probe |
|---|---|
| the DSL grows toward SQL | the grammar above is frozen for B7; extensions need an operator decision |
| constants baked into templates (kills the memo) | `explain` prints the digest; two Q6 runs with different constants must show one digest |
| planner cost table wrong for the DRAM regime | costs are measured rows from B2/B6, re-measured per bebop.bin md5 |
| rank-3 doubles storage | it is a choice per type (LANG-DB §4d style), reported in the size row |

## 9. VERDICT format

```
VERDICT: GREEN|RED
q6: bebop <ms> sqlite <ms> rust <ms> -> <x> ; q1: ... ; join: ...
latency: first_ms <v> repeat_ms <v> (digest <hex> stable across constants: yes|no)
rank3: fold <equal> plan_choice <t-csr|order-csr> correct <yes>
gates: c70_qdsl tpch_q6 tpch_q1 tpch_join rank3
journal: <line>
open: <shapes without templates, deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint; B3's generator API; csr.bp contract; sbench_sqlite.py as the twin template; $OUT; rules; `<constraints>` DSL in .bp (no compiler grammar change unless unavoidable), constants as kernel arguments, folds before timings, zero deps; `<output_format>` §9; `<task>` steps 1-3.
