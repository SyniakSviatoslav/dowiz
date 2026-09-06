Status: 2026-09-06, owner main session (Fable fork), grounded at HEAD b1c0175; depends on B3 (GbMatrix objects, eWiseAdd template) and B1 (G5 harness for kill -9 during merge). Replaces sgraph2 stage 2's O(N) L0 rebuild (stall 747 ms, 30 us/edge amortised -- verified HISTORY STORE PULL T117 and sgraph2.bp:108-182 `phase_log`).

# B4 purely functional tensor updates -- tail COO + L0 + L1 row-block CoW, versions, time-travel

## 0. Goal

A matrix update never touches an old version: `assign` returns a new GbMatrix sharing every unchanged block. Gates G9c: 1M single-row updates amortised <= 0.5 us, max stall <= 10 ms, folds == oracle after every 10^4 updates, kill -9 during an L0->L1 merge leaves gen k or k-1 valid; update twin vs sqlite WAL `UPDATE` (RESULT-sbench row).

## 1. Scope

In: the three-tier matrix (tail, L0, L1), RowBlock CoW with a 2-level blocktab, eWiseAdd merge at promotion, `prev` for time-travel, tombstones inside a version (existing TB bitmap, sgraph2.bp:183 `tomb`), compaction of unreachable blocks (existing `st_compact`, store.bp:247 -- unreachable = not copied), reads that apply the tail as a delta. Out: multi-writer (B5), the DSL (B7). Fixed points: superblock/root swap semantics (`st_commit`:134), `csr_build` (sgraph2.bp:24), sgraph2's folds (the phase `n`/`f` oracles bench/oracles/sgraph2.py stay the oracle for the new structure).

## 2. Preconditions

- Today's tiered shape (verified sgraph2.bp:9-10, :108-182): L1 = RP/CI built once; LOG = chunks E of 10k pairs; L0 = RP0/CI0 rebuilt over ALL logged edges every batch (the O(N) stall); TB tombstones; reads = L1 slice + L0 slice minus TB (`nbr_fold` :187).
- LANG-DB §9.3 (tiered CSR pick) and §9.4 (block-CoW 4 KB, tombstones) -- verified docs/LANG-DB-DESIGN.md:575, :608.
- B3's GbMatrix header carries `ref prev` and `gen`.

## 3. Design

Layout (cells; every ref object-relative, `st_link`/`st_ref` store.bp:197-198):
```
GbMatrix v_g  {n, m, nnz, fmt=5 (tiered), ref tail, ref L0, ref L1, ref tb, gen, ref prev}
tail          COO chunk {cnt, (row, col, val) x cnt}                     cap 4096 triples, append-only within a version
L0            GbMatrix fmt=1 (plain CSR) over <= 2^18 edges              rebuilt WHOLE at promotion (bounded: 3 passes over <= 2 MB)
L1            {nblk, ref root}  root: page refs (fan-out 512) -> RowBlock refs
RowBlock      {first_row, rp_local[65], ci[...], vv[...]}                64 rows; ~650 cells at mean degree 10
```
Operations:
```
assign(A, i, j, v):      tail' = tail + (i,j,v) (copy chunk if it is shared, i.e. gen != tx gen) ; header' ; O(4096)
promote_tail(A):         when cnt == 4096: L0' = csr_build(L0 ∪ tail)  (<= 2^18 edges, else promote_L0 first) ; tail' = empty
promote_L0(A):           when |L0| >= 2^18: for each RowBlock touched by L0 rows: RowBlock' = merge(RowBlock, L0 rows)  (two-pointer, sorted ci)
                         page' for each touched page, root' ; L1' ; L0' = empty        (append T*~5 KB + pages; never in place)
read_row(A, i):          L1 block row  ⊕  L0 row  ⊕  tail entries for i  (tail scanned linearly, <= 32 KB)  minus tb
mxv/scan (B3 kernels):   iterate L1 then L0, apply tail as a delta vector before the reduce (template parameter `tiered`)
snapshot:                gen ; time-travel: follow prev while gen > wanted
compaction:              st_compact copies only reachable objects -> old blocks/pages vanish unless reachable through prev
```
Invariants: (1) an object is written only inside the transaction that allocated it (`st_alloc`:112 in the current tx); a shared object (gen < tx gen) is copied before modification; (2) `nnz` of the header = nnz(L1) + nnz(L0) + cnt(tail) - tombstoned; (3) every RowBlock's `first_row` is a multiple of 64 and ci is sorted within each local row; (4) promotion order: tail -> L0 -> L1, never skipping a tier.

Costs (RESEARCH-GRAPHBLAS §2.2): single update ~10 KB append when it touches L1 directly -- avoided by the tail; amortised L1 write ~300 B/edge at promotion; read amplification 3 slices as today (1.1 us measured, sgraph2 phase `n`).

Failure modes: promotion crash mid-merge -> the old root still names the old L1/L0/tail (nothing in place), reopen at gen k-1 (G5); tail chunk shared across versions and appended in place -> corrupts the old version: guarded by invariant (1) with an `st_check`-style assertion in debug builds; a RowBlock exceeding 4 KB -> split into two blocks with the same `first_row` range? No -- blocks are 64 rows fixed; a dense block simply is larger (append is variable-length), only the page fan-out is fixed.

## 4. Files and functions touched

| file | anchor | change |
|---|---|---|
| selfhost/prelude/gb.bp | B3 | fmt=5 tiered matrix: assign, promote_tail, promote_L0, read_row, block/page CoW helpers (~300 lines) |
| selfhost/std/gen_gb.bp | B3 templates | `tiered` template parameter: L1+L0 iteration + tail delta |
| selfhost/std/sgraph2.bp | :108-182 `phase_log`, :187 `nbr_fold`, :214, :256, :399 | phase `l` uses assign/promote; reads via read_row; folds unchanged (oracle bench/oracles/sgraph2.py) |
| bench/vs_rust/sgraph2.sh | :16-20 (log rows) | rows: max stall, ns/edge amortised, promotion count, prev-chain depth; new phase `u`: 1M single-row updates |
| bench/tq_sqlite/sgraph_sqlite.py | twin | `UPDATE` 1M single rows in WAL mode, ms and us/row |
| bench/vs_rust/scrash.sh | :12-30 | variant: SIGKILL during phase `l` promotions (G5 harness reused with the sgraph2 store) |

## 5. Steps

1. tail + L0 promotion (no L1 change yet): phase `l` becomes append-to-tail + bounded L0 rebuild; stall row must drop from 747 ms to <= 10 ms; folds unchanged.
2. L1 as RowBlocks under a blocktab + L0->L1 merge promotion; phase `b` builds L1 in block form (csr_build output split into blocks); folds unchanged; compaction row.
3. `prev` + time-travel read (phase `h`: fold "as of gen g"), tombstones inside a version, the update twin phase `u`, the kill -9 variant.
Each step one chain-gated commit (no codegen change: battery + sgraph2.sh rows).

## 6. Constructs, oracles, twins

- Oracle: bench/oracles/sgraph2.py extended with `u` (1M updates, deterministic LCG rows) and `h` (as-of folds).
- Gates: std_golden `sgraph2` (existing fold) + `sgraph2_u` + `sgraph2_h`; scrash-variant `scrash_gb` (TRIALS 50 in the battery).
- Twin: sqlite WAL `UPDATE` per row (python ctypes, prepared) -- sgraph_sqlite.py.

## 7. Gates

```
BEBOP_TMP=$OUT bash bench/vs_rust/sgraph2.sh      # rows: log stall <= 10 ms (was 747), ns/edge <= 500 (was 30000), nbr fold equal, bfs fold equal
                                                  # phase u: 1M updates, amortised us/row <= 0.5, max stall <= 10 ms, fold == oracle every 10^4
TRIALS=50 bash bench/vs_rust/scrash.sh --gb       # 0 failures, gen in {k, k-1}
tools/battery.sh ./bebop.bin $OUT/bat SRC=bebop.bp  # GREEN
```
RED: stall > 10 ms (promotion not bounded), a fold drift after promotion (merge bug: duplicate or dropped edge), or an old version's fold changing after an update (invariant 1 broken).

## 8. Risks and probes

| risk | probe |
|---|---|
| tail scan cost dominates point reads | measure `n` phase with tail at 0/2048/4096; cap tunable |
| L0 rebuild at 2^18 edges > 10 ms on the register-model binary | B2 row (iii) gives ns/edge for csr_build; lower the L0 cap if needed |
| page CoW write amplification with random rows | `u` phase with uniform rows is the worst case; report bytes appended per update |
| prev chains grow unbounded | compaction drops prev when the type does not declare history (LANG-DB §4d) -- row: file size after compaction |

## 9. VERDICT format

```
VERDICT: GREEN|RED
log: stall_ms <v> ; ns_per_edge <v> ; promotions <n>
updates: us_per_row <v> ; max_stall_ms <v> ; bytes_per_update <v> ; sqlite_us_per_row <v>
folds: nbr <equal> bfs <equal> asof <equal>
crash: trials <n> failures <k>
size: after_compaction <bytes> (prev depth <n>)
journal: <line>
open: <deviations>
```

## 10. Worker prompt skeleton

`<context>` this blueprint; sgraph2.bp:108-182 (what is replaced) and :187 (reads); store.bp tx API; oracles; $OUT; harness rules; `<constraints>` never modify a shared object in place (invariant 1), folds first, zero deps; `<output_format>` §9; `<task>` steps 1-3.
