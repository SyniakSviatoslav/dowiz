Status: 2026-09-08 -- read-only analysis by the session's FOURTH analyst agent (Fable) of the operator's "win everywhere against SQLite" position on three dimensions (storage compactness under updates, schema agility, in-place OLTP mutability), over /root/dowiz/bebop-lang at today's tree (store.bp 416 lines, sbench.bp, sbench_sqlite.py, RESULT-sbench.md of 2026-09-07 on e9159318). Rests on: ROADMAP.md (A7/A8, B1/B4/B5/B6, C2/C3, D1/D4, E3, the Measured table), docs/LANG-DB-DESIGN.md (§0, §1, §3, §4c-4e, §9.4), docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md, the three other 2026-09-08 research documents, docs/blueprints/B1 (follow-up card), B4, B5, C2, C3, D1, selfhost/prelude/store.bp, selfhost/std/mvcc.bp, docs/BOX.md, docs/ANDROID.md, and today's journal lines 851-853 (R1, the DRAM row, the vmstat calibration). Every repo claim carries file:line verified today by reading; every outside claim carries a URL; every box number is attributed to the committed row or journal line that measured it. No compiler, gate, benchmark, `git` or heavy job was run (three workers live; BOX.md's 32-process cap). This is a PROPOSAL pending operator decisions; nothing here has been measured by this document.

# Beating SQLite on its own three axes: where the line is, what it costs to move it, and which of today's rows measure the harness

## 0. Executive summary (10 lines, leading with what is NOT winnable)

1. **Not winnable, and not worth winning: the ergonomic half of schema agility.** SQLite's `ALTER TABLE ADD COLUMN` is a ~1 ms metadata write with no compile step (https://www.sqlite.org/lang_altertable.html); bebop's equivalent is already free at the data layer (LANG-DB §4d, `docs/LANG-DB-DESIGN.md:216-231`) but costs a recompile of every program that touches the type -- 0.07 s cache hit, 0.3 s after A10, 1.5 s cold (ROADMAP Measured). That gap is the price of zero decoding and no dialect change removes it. Say so and stop.
2. **Not winnable as stated: zero growth per single-row commit.** Under snapshot isolation an 8-byte change appends a block (LANG-DB §9.4, 4 KB) plus a path through the blocktab; SQLite rewrites its page in place and its file does not grow. This is structural to append-only (Postgres has paid it for 30 years -- dead tuples + autovacuum, https://www.postgresql.org/docs/current/routine-vacuuming.html). What IS winnable is the block size (§2.3: 64-row u32 blocks make it ~1-2 KB per commit against SQLite's 4 KB WAL frame + 4 KB checkpoint page), and the peak-at-compaction (2x live) is shared with `VACUUM` (https://www.sqlite.org/lang_vacuum.html) and `mdb_copy -c`.
3. **Winnable, decisively, and already on the roadmap: steady-state compacted size.** The 72.4 MB after compaction decomposes EXACTLY into 48.0 MB records (1M x 6 cells: a 16 B header per 32 B payload) + 8.0 MB ref-table `T` + 8.4 MB `RP` + 8.0 MB `CI` (§2.1). Under C2 (columns: one header per column, and a row's identity is its index, so `T` disappears) plus A8 (u32 cells) the same data is ~20-24 MB -- **0.6-0.7x SQLite's 34.1 MB**, not the 1.2x C2's gate asks for. The 12.8 MB the update ADDED is 8.0 MB of whole-table CoW (`sbench.bp:159` reallocates `T` every transaction) plus 4.8 MB of versions; B4's row-block CoW removes the first term. So point 1's "2.5x loss" is 2.1x of design already scheduled to change and 0.4x of a harness shape B4 replaces.
4. **Point 3 is not a loss; the row pairs an msync'd commit against an un-synced one.** `sbench_sqlite.py:70` runs the "durable" twin at `synchronous=NORMAL`, which by SQLite's own definition does no sync per commit in WAL mode (https://www.sqlite.org/pragma.html#pragma_synchronous); the store side (`store.bp:235-241`) msyncs twice. Class-matched rows in the SAME table already show the store ahead: 1.2x against `FULL`, 1.5x/1.6x on group commit. The class-matched NO-sync row (store `st_commit` = 16 cell stores + a 15-cell crc, zero syscalls, against SQLite `synchronous=OFF` = at least one `write()` per commit) has never been measured and should be a 10-100x win. **The concession is unnecessary.** What changes if the 3.7x is a proot artefact: only the absolute floor -- an msync'd commit can never beat an un-synced write, so the ordering survives native re-measurement and so does the category error.
5. **Both sides of the durable row are mis-measured, not just the pairing.** The store's `phase_durable` (`sbench.bp:196-217`) appends a record and a root that still points at the OLD table: the new version is unreachable, so the store never pays the table-block CoW a real update pays, while SQLite's `UPDATE ... WHERE id=?` is a real update. A fair row must link the record (4 KB block CoW under B4, 8 MB under today's monolithic `T`).
6. **"Batches build new generations, eliminating row-level lock conflicts" is a restatement of the single-writer constraint, and it contradicts B5.** SQLite is single-writer too ("there can only be one writer at a time", https://www.sqlite.org/wal.html), so against SQLite there are no row locks to eliminate on either side; the axis that decides OLTP throughput is the length of the writer's critical section (~0.1 us here vs ~60 us there under proot). Meanwhile ROADMAP B5 (mandatory) REINTRODUCES conflicts as partition locks + STM read-set validation + abort/retry <= 8 then error 91 (`docs/blueprints/B5-multi-writer.md §3`). Either B5's gate (>= 2x a single writer on 3 A78) is a micro-transaction gate the operator says is pointless, or the concession is withdrawn. Both cannot stand.
7. **Background compaction on the A55s is CPU-free, not memory-free, and "only while free of readers" is a misreading of the store.** The same measurement that showed 28/28/28 ms on three A78s showed 28/29/**43** ms when the four A55s also ran (journal 852): one A78 lost 1.5x. Readers NEVER block compaction (the old inode stays alive under their mapping, LANG-DB §4c, `store.bp:355-416` renames a new file); the constraint that exists is the WRITER, which must not commit during a whole-file Cheney copy or must replay afterwards -- and that is why compaction has to be tier-local under B4 (immutable L1 compacts in the background, the tail keeps committing), not whole-file.
8. **A dynamic heap for unstructured tails poisons the scan by WIDTH, not by pointer chasing.** Under today's AoS objects an `(off,len)` tail handle adds 8 B to a 48 B record: +17 % bytes on every full scan even when the tail is never read. Under C2 columns it costs the scan nothing. The survivable form is the industry's VARIANT column with shredding at compaction (Parquet variant shredding https://github.com/apache/parquet-format/blob/master/VariantShredding.md ; ClickHouse JSON dynamic paths https://clickhouse.com/docs/sql-reference/data-types/newjson ; Snowflake VARIANT https://docs.snowflake.com/en/sql-reference/data-types-semistructured), and the store already has the pass that shreds: compaction visits every live object and applies migrations (`LANG-DB-DESIGN.md:222`). Against SQLite's `json_extract` (text re-parsed on every access, https://www.sqlite.org/json1.html ; JSONB is still walked per access, https://sqlite.org/jsonb.html) a tagged-cell tail read is the same design at 8 B per step.
9. **The MAP_SHARED write tax (22x, journal 851) is a bulk-scatter tax, not an OLTP tax.** A durable commit re-dirties ~2 pages that msync just cleaned, so it pays the write-protect-and-refault cycle ~2 x ~5 us = ~10 us of its 229 (4 %); the update row (10^5 updates in 73 ms = 0.73 us each, `RESULT-sbench.md`) shows no per-store tax at all because its pages stay dirty. Consequence: D1 as written (build bulk structures privately, publish sequentially) is right, and the wild-ideas 3.1 form (never map writable; `pwrite` the tail) would ADD one syscall (~10-30 us under proot) to every commit and make point 3 worse. Do not adopt 3.1 for the commit path.
10. **Net: 0 adopt-as-stated.** Point 1: NARROW -- tier-local background compaction under B4 with a contention gate, and the size row re-derived under C2+A8 where it flips to a win. Point 2: NARROW -- a `variant` tail column with shredding at compaction under A7+C2, and the ergonomic gap conceded in writing. Point 3: REFUTE the concession -- the row is a category error on both sides; add the class-matched rows (the single cheapest experiment in this document, ~25 lines in files that already exist), land B1's card, and replace two msyncs with one. Where SQLite is simply better, in one sentence each: any-language multi-process access with fcntl locks and no toolchain; zero growth under sustained random single-row commits on a tight disk budget; schema-on-read with no compile step.

---

## 1. Method, and the two pieces of arithmetic everything below rests on

Read in full: ROADMAP.md (all phases and the Measured table), LANG-DB §0-§4, §6, §9.3-9.5, RESEARCH-BYOK-ECS-DATAFLOW (all), RESEARCH-LITERATURE §2, §7, §9, RESEARCH-CORPUS-IDEAS §5.2, §7.4-7.5, RESEARCH-WILD-IDEAS §3, §5.2, blueprints B1 (with the follow-up card), B4, B5, C2, C3, D1, store.bp, mvcc.bp, BOX.md, ANDROID.md §Status, `bench/vs_rust/RESULT-sbench.md`, `bench/vs_rust/std_tests/sbench.bp`, `bench/tq_sqlite/sbench_sqlite.py`, `bench/vs_rust/sbench.sh`, journal lines 825, 851-853. Verdict vocabulary as in the Phase C review: ALREADY / ADOPT / NARROW / REFUTE.

### 1.1 The size row, decomposed to the byte

`sbench.bp` stores records `P{id,u,v,cell}` (4 payload cells + 2 header cells = 48 B), a table `T{arr ref P}` of n+1 cells, an index `RP` of k+2 cells with k = 1024 x 1024, and `CI` of n+1 cells (`sbench.bp:1-12`, `:27-95`). At n = 10^6:

| object | cells | bytes | share of 72.4 MB |
|---|---|---|---|
| P x 10^6 (16 B header + 32 B payload) | 6 x 10^6 | 48.0 MB | 66 % -- of which 16.0 MB is HEADER |
| T (ref per row) | 10^6 + 3 | 8.0 MB | 11 % -- exists only because rows are objects reached through refs |
| RP (k + 2, k = 2^20) | 1,048,578 + 2 | 8.4 MB | 12 % |
| CI (n + 1) | 10^6 + 3 | 8.0 MB | 11 % |
| sum | | **72.4 MB** | matches the row's 72,396,960 B to within the two headers |

The pre-compaction 85.2 MB is this plus exactly what `phase_update` appends: a whole new `T` (`sbench.bp:159` `st_alloc(base, tx, n + 1, dt)`, 8.0 MB) and 10^5 new 48 B records (4.8 MB) = 12.8 MB = 85,197,016 - 72,396,960 to the byte. So of the 2.5x: 2.1x is the object layout (header + i64 cells + a ref table) and 0.4x is one transaction copying a monolithic array that B4's blocktab (`docs/blueprints/B4-functional-tensor-updates.md §3`) exists to chunk.

SQLite's 34.1 MB is 1M rows of `(rowid, u, v, cell)` as varint-typed records (~15-20 B per row with page overhead, https://www.sqlite.org/fileformat.html) plus a B-tree index on `cell` (~10-12 MB). Note the twin measures with `journal_mode=OFF` (`sbench_sqlite.py:22`), so no `-wal`/`-journal` file is counted.

### 1.2 The durable-commit row, decomposed

Store side (`store.bp:235-241` `st_commit_sync`): `sys_msync` of the appended page range, superblock toggle (`st_commit_m`, `:197-203`: 16 cell stores + crc32x of 15 cells), `sys_msync` of 8192 bytes. LANG-DB §3 measured one-page msync at ~100 us under proot; two of them plus ~30 us of work is the 229 us in `RESULT-sbench.md`, to within noise. SQLite side, row 1: `PRAGMA journal_mode=WAL; synchronous=NORMAL` (`sbench_sqlite.py:70`) -- per SQLite's documentation, in WAL mode NORMAL syncs the WAL only before a checkpoint and "no sync operations occur during most transactions" (https://www.sqlite.org/pragma.html#pragma_synchronous); a commit is a `write()` of one 4 KB WAL frame plus shm bookkeeping: 62.6 us. Row 2: `synchronous=FULL` = one fsync per commit: 277.5 us. The store beats the row with the same durability class by 1.2x and loses to the row with a weaker class by 3.7x. That is the whole of point 3's measured "loss".

What neither row measures: on the store side the appended record is never linked into `T` (`sbench.bp:209-211` links `h2` to the OLD `st_ref(base, h, 0)`), so no table block is copied; a real single-row update under snapshot isolation pays LANG-DB §9.4's 4 KB block CoW (or, with today's monolithic `T`, an 8 MB copy per transaction, `sbench.bp:159`). SQLite's `UPDATE p SET u=u+1 WHERE id=?` is a real update. The store's 229 us is therefore a LOWER bound on its real single-row durable update, and the "1.2x win over FULL" is not yet earned either.

---

## 2. Dimension 1 -- storage compactness under heavy updates

### 2.1 Where the line actually is

Three different quantities hide under "34.1 MB stable vs 85.2 MB":

| quantity | SQLite | store today | store after roadmap rows | winnable? |
|---|---|---|---|---|
| (a) compacted / steady-state mean size | 34.1 MB | 72.4 MB (2.1x) | C2 (one header per column; row id = index, `T` gone) + A8 (u32 cells): `u`,`v`,`cell` columns 12 MB + `RP` 4.2 MB + `CI` 4.0 MB = **~20 MB (0.6x)**; at i64 columns ~24 MB (0.7x); E4/D4 bit-packing below that | **YES, flips to a win** -- with rows already on the roadmap (C2 `ROADMAP.md:125`, A8 `:91`); C2's own gate (<= 41 MB) is conservative by 2x |
| (b) growth per committing transaction between compactions | 0 (in-place page rewrite; freelist reuse) | 8 MB (`T` copy) + 48 B per version | B4 row-block CoW: one block per column touched + blocktab path; ~4.5 KB at 4 KB blocks with a 512-fan-out blocktab; **~1-2 KB with 64-row u32 blocks and a 64-fan-out blocktab** (256 B block + 3 x 512 B path) | NO as "zero"; YES as "fewer bytes appended per commit than SQLite writes per commit" (WAL frame 4 KB + checkpoint page 4 KB = 8 KB, https://www.sqlite.org/wal.html) -- the file still grows where SQLite's does not |
| (c) peak size during reclamation | 2x during `VACUUM` (https://www.sqlite.org/lang_vacuum.html: "up to twice the size of the original database file") | 2x during `st_compact` (new file + old inode, `store.bp:355-416`) | tier-local compaction (§2.3): peak = live + the tier being compacted | tie today; a partial win only if compaction is per tier |
| (d) bytes written to flash per 8-byte update (write amplification) | ~8 KB (WAL frame + checkpoint page), plus f2fs's own log-structured GC | 48 B version + block CoW + amortised compaction copy (72 MB per compaction) | at 1-2 KB per commit and k = 1 trigger: ~2-4 KB per commit all-in | roughly a tie; the store's writes are sequential, which f2fs (itself log-structured, https://docs.kernel.org/filesystems/f2fs.html) prefers -- SQLite's "in-place" is a LOGICAL property on this filesystem; physically both are appended segments |

The honest sentence for point 1: **the mean size is winnable and the growth-between-compactions is a block-size knob; "zero growth" and "no 2x peak" are not reachable by an append-only store and are not reached by SQLite's own reclamation either.**

### 2.2 Prior art on exactly this trade, and what each paid

| system | mechanism | what it cost | source |
|---|---|---|---|
| Postgres heap | new tuple version appended in place of the page's free space, dead tuples reclaimed by autovacuum in the background; `VACUUM FULL` needs 2x disk | bloat when vacuum cannot keep up, long transactions pin dead tuples, xid wraparound -- 30 years of operational lore; HOT updates and `fillfactor` were added to keep versions on the same page | https://www.postgresql.org/docs/current/routine-vacuuming.html |
| LMDB | CoW B+tree; freed pages go to a freelist keyed by txn id and are reused once no reader needs them -- so steady-state random updates do NOT grow the file, but "stale reader transactions ... cause further writes to grow the database quickly"; compaction is `mdb_copy -c` into a new file (2x) | write amplification of a path copy (3-4 pages) per commit; no in-place compaction | https://raw.githubusercontent.com/LMDB/lmdb/mdb.master/libraries/liblmdb/lmdb.h (LANG-DB [w16]) |
| RocksDB (LSM) | leveled compaction: space amplification ~10 % (live x 1.1) at write amplification 10-30x; universal compaction: space up to 2x at lower WA | the WA/space frontier is a dial; nobody gets both | https://github.com/facebook/rocksdb/wiki/Leveled-Compaction ; https://github.com/facebook/rocksdb/wiki/Universal-Compaction |
| DuckDB | single file, block-based; deleted rows' blocks are reused at checkpoint but **the file never shrinks** without a copy | the same "peak = copy" rule | https://duckdb.org/docs/stable/operations_manual/footprint_of_duckdb/reclaiming_space |
| Datomic | immutable segments; a background indexing job merges the log into new index segments; old segments become garbage | storage grows until GC; one transactor | https://docs.datomic.com/indexes/index-model.html (LANG-DB [w20]) |
| SQLite | in-place page rewrite through the WAL; freelist pages reused; `VACUUM` = full rewrite at 2x | zero growth for updates, but deletes leave freelist pages and the file never shrinks without `VACUUM` | https://www.sqlite.org/lang_vacuum.html ; https://www.sqlite.org/fileformat.html |

The pattern: every append-only or CoW survivor has a background reclaimer and a 2x peak; the one that has neither (SQLite) rewrites in place and pays with a page-sized write per commit. The store is in the first family by design (LANG-DB §4e), and the operator's "do not chase in-place rewriting" is the right call. The question is only what the reclaimer costs and where it runs.

### 2.3 The operator's direction, attacked: does background compaction solve bloat or move it?

**It moves it, and the move is a good one only under three conditions the proposal does not state.**

1. **The "free of readers" condition is wrong for this store.** Readers hold a mapping; `st_compact` writes `<store>.tmp` and `sys_rename`s it (`store.bp:355-416`); the old inode stays alive under any live mapping (LANG-DB §4c, "the kernel's inode refcount is the product that reaches zero"). Readers never block compaction and compaction never blocks readers. Waiting for a reader-free window would, on a busy node, mean never compacting. The condition that DOES exist is on the writer: whole-file Cheney copies generation g, and any commit the writer makes during the copy is lost at the rename unless replayed. Today the writer stalls for 747 ms (the G8 "max stall 747 ms" row, `ROADMAP.md:260`). B4 dissolves this: L1 is immutable between promotions, so compacting L1 in a forked child while the writer appends to the tail is safe by construction, and the writer's catch-up is "swap the L1 ref at the next promotion". That is the mechanism; it attaches to B4 step 2/3 and to B6 (the A55 cores), not to a new row.

2. **"Genuinely free" on the A55s is true for CPU and false for memory.** Journal 852: seven concurrent sequential sums gave 28/29/**43** ms on cores 4-6 and 82/83/84/117 ms on cores 0-3; solo was 28. So with the little cores streaming, one big core ran 1.5x slower -- the aggregate ~12 GB/s is shared, and a compaction streaming 72 MB through the shared L3 evicts the analytical working set. Cheney compaction today runs at 72 MB / 747 ms = ~100 MB/s on an A78 (it is pointer-chasing plus `st_refmask`'s linear scan of the layout table per object, `store.bp:294-303`, plus a crc per object), so on an A55 it is ~2 s per 72 MB and ~50 MB/s of traffic -- well under the A55's own 0.7-1.0 GB/s, hence probably invisible to the A78s. But that is a prediction, and the gate below measures it.

3. **The trigger must be sized to the update rate, and it is a real number here.** At 1000 single-row commits/s with 4 KB blocks the arena gains 4 MB/s; with k = 1 (`LANG-DB-DESIGN.md:249-250`) a 72 MB live set compacts every ~18 s, i.e. ~10 % of one A55 and 72 MB of flash writes every 18 s (4 MB/s of compaction traffic on top of 4 MB/s of appends -- write amplification ~2 on top of the block CoW). With 256 B blocks the same rate is 0.25 MB/s and compaction every ~5 min. **Block size is the lever; compaction is the consequence.** A phone with 200 MB free cannot host a 72 MB store that peaks at 144 MB + a copy; tier-local compaction (peak = live + one tier) is what makes the design deployable, and it is only reachable through B4.

**What the direction does NOT solve:** the transient 2x, the growth-per-commit floor of one block + one blocktab path, and the interaction with C3's commit chain (a pinned history makes every number above worse in proportion to retained depth -- `RESEARCH-BYOK-ECS-DATAFLOW:279` already says so; the chain must be per-type policy and compaction must cut it).

### 2.4 Rows, gates, kill experiments

| row | mechanism | attaches to / displaces | gate (the number) | cheapest experiment that kills it |
|---|---|---|---|---|
| **S1 -- size row re-derived under columns** | store `P` as three columns (id positional), `RP`/`CI` at u32; measure the G7 size row both ways | C2 (`ROADMAP.md:125`) -- raises its gate; A8 | 1M records + index **<= 0.8x SQLite's 34.1 MB (<= 27 MB)** after compaction, folds identical; PK lookup <= 2x of today's 450 ns (C2's regression guard) | C2 §7's 30-line AoS-vs-SoA twin of `nnidx.bp`; if the column form of `sbench` lands above 41 MB the arithmetic in §1.1 is wrong |
| **S2 -- bytes appended per single-row commit** | B4 blocktab with the block size and fan-out chosen for write amplification, not only for CSR traversal | B4 step 2 (`ROADMAP.md:108`) -- adds a gate B4 lacks (RESEARCH-LITERATURE §9 already asked for a bytes/edge gate) | 10^5 single-row commits on random rows of a 1M-row column table: **bytes appended per commit <= 2 KB** (SQLite writes 8 KB per commit: WAL frame + checkpoint page); peak file <= 1.6x live with the k = 1 trigger | analytic first (zero code): if 64-row blocks with a 64-fan-out table give > 2 KB at mean degree 10 for the CSR case, the CSR block size and the column block size must differ and B4 gains a per-type block size |
| **S3 -- tier-local compaction in a forked child on an A55** | compact L1 (immutable between promotions) in a `sys_clone(17,...)` child pinned to cores 0-3 while the writer keeps committing to tail/L0; swap the L1 ref at the next promotion; whole-file Cheney stays for the offline case | B4 step 2/3 + B6 (`ROADMAP.md:110`); DISPLACES "only while free of readers" | writer max stall during compaction **<= 10 ms** (B4's number); three A78 scans concurrent with the compaction **<= 1.2x their solo time**; peak file <= live + L1; fold identical before/after | run the existing sequential-sum probe on cores 4-6 while `st_compact` of the 72 MB sbench store runs on core 0: if any A78 exceeds 1.3x solo, "free" is false and compaction must be throttled or scheduled; one slot run, zero new code |
| **S4 -- REFUTE "compaction only while the pipeline is free of readers"** | -- | LANG-DB §4c, `store.bp:355-416` | none: it is a design fact | none needed |

---

## 3. Dimension 2 -- schema agility

### 3.1 What SQLite has, feature by feature, against what the store already has

| SQLite | mechanism there | store today | cost to match | verdict |
|---|---|---|---|---|
| `ADD COLUMN` | metadata write, rows read missing columns as default; ~1 ms; no rewrite | LANG-DB §4d: append a field = new layout digest; shorter old objects read 0 (Cap'n Proto rule); gate G3 `sevolve` | 0 at the data layer; **a recompile of the accessing programs** (0.07 s memo hit, 0.3 s A10 target, 1.5 s cold) | ALREADY at the data layer; the latency is LOST by 100-1000x and cannot be won without an interpreter -- concede in writing |
| rename | rewrite of the schema text | digest is types-only, positional: free | 0 | ALREADY |
| `DROP COLUMN` | table rewrite (since 3.35) | tombstone in the layout table; cell reclaimed at compaction | 0 until compaction | ALREADY, and cheaper |
| type change | manual rebuild (`CREATE TABLE ... AS SELECT`) | migration fn by sha256 applied at compaction (`LANG-DB-DESIGN.md:222`) | one pass per object at compaction, zero on reads | ALREADY |
| dynamic typing per cell | every record cell carries its type in a varint header; every read decodes | none: i64 cells, layout by digest | this is the tax bebop refuses (M1: ~180 ns/row of VDBE + decode) | correctly refused |
| `json_extract` over TEXT / JSONB | parse per access (text) or walk per access (JSONB) | A7's `(off<<32 \| len)` byte handle (`ROADMAP.md:90`) gives bytes a home; no tagged-value encoding exists yet | a tagged-cell encoding over A7 bytes + a reader: ~100 lines of store library | NARROW (§3.2) |
| schema-on-read of an unknown schema | ingest anything, query later, slowly | no: a query is a compiled fn over a known layout | a generic tagged-value scan (the same speed class as `json_extract`) | winnable in THROUGHPUT, not in ergonomics |

### 3.2 The operator's direction, attacked: does a dynamic heap for tails poison the scan path?

**Yes under objects, no under columns, and the reason is width, not chasing.**

- Under today's AoS objects a tail handle is one more 8 B cell in every record: a 48 B record becomes 56 B, and every full scan reads +17 % bytes whether or not the tail is touched. That is the poison, and it is exactly the AoS-vs-SoA argument C2 already makes (`docs/blueprints/C2-column-storage.md §2`). Under C2 the tail is one more column that a scan over other columns never reads: zero cost.
- Pointer chasing on a tail READ is one hop (record -> bytes), and because the bytes are appended in the same transaction as the record they are usually in the same or the next page -- the same locality Cap'n Proto's segments and SQLite's overflow pages rely on. It is not the k-random-lines cost of a columnar point read; it is one extra line.
- What the industry converged on for exactly this workload is a **variant column with shredding**: keep a self-describing tail per row, and when a key appears in enough rows promote it to a real column. Parquet's variant shredding spec (https://github.com/apache/parquet-format/blob/master/VariantShredding.md) and ClickHouse's JSON type (`max_dynamic_paths` shredded, the rest in a shared "unshredded" sub-column, https://clickhouse.com/docs/sql-reference/data-types/newjson) are the two written-down designs; Snowflake's VARIANT does it transparently (https://docs.snowflake.com/en/sql-reference/data-types-semistructured). The store's version costs almost nothing new: the tail is A7 bytes in a tagged-cell format (tag cell, value cell -- no varints, no text), the "promote" step is a migration fn (`LANG-DB-DESIGN.md:222`) generated from a key-frequency count, and it runs in the pass that already visits every live object -- compaction. C2 provides the column it shreds into.
- **"Moving schema evolution up into the compiler of the dialect" is ALREADY the design** (§4d: layout digests, the migration table, `.bcas` by sha256). The only thing the compiler does not do today is emit the migration for a shred automatically; that is a generator, not a language change.

Where SQLite is simply better here: when the schema is unknown at WRITE time and the program that will query it does not exist yet. The store can ingest that (A7 bytes) and scan it at `json_extract` class speed, but every question against it that wants column speed needs a compile. That is the correct place for the line, and the operator's compromise puts it there.

### 3.3 Rows, gates, kill experiments

| row | mechanism | attaches to | gate (the number) | cheapest experiment that kills it |
|---|---|---|---|---|
| **V1 -- `variant` tail column + shred at compaction** | per-row tail as A7 bytes in a tagged-cell encoding, stored as a C2 column; key-frequency count during compaction; keys above a threshold become real columns through a generated migration fn | A7 (`ROADMAP.md:90`), C2 (`:125`), LANG-DB §4d | (i) K6-class scan over declared columns with a 24 B tail present on every row **<= 1.05x** without the tail; (ii) tail key read per row **<= 1.0x SQLite's `json_extract` native** on the same 3-key object (SQLite's number to be MEASURED in the twin -- it is not in any table today); (iii) after one compaction with the key above threshold, a scan over that key runs at column speed (<= 1.1x a declared column) | emulate the tail with a cells array in today's store (no A7): if the declared-column scan slows > 5 % with the tail as a separate object, locality is not what §3.2 says and the tail needs its own page class |
| **V2 -- concede the schema-change LATENCY row in writing** | none | LANG-DB §4d; A10 | report `ADD COLUMN` + first query as a ratio (expected 100-1000x LOSS: compile vs metadata write) with bytes rewritten (expected 0 on both sides) | none: it is measured by definition |
| **V3 -- REFUTE "a dynamic heap brings back pointer chasing" as the cost** | -- | C2 §2 | the cost is scan width under AoS; zero under columns | the 30-line C2 §7 twin already decides it |

---

## 4. Dimension 3 -- in-place OLTP mutability

### 4.1 The row is a category error on both sides (§1.2 has the arithmetic)

- Pairing: msync'd commit (store) vs no-sync commit (SQLite `synchronous=NORMAL`). Same table, class-matched: 1.2x (FULL), 1.5x/1.6x (batch 10/100). SQLite's own documentation is the authority on what NORMAL does not do (https://www.sqlite.org/pragma.html#pragma_synchronous).
- Store side: `phase_durable` never links the new record (`sbench.bp:209-211`); the durable "update" is an append. A real one pays a block CoW.
- Unmeasured entirely: the **no-sync** class on the store side. `st_commit` (`store.bp:197-203`) is 16 cell stores plus `crc32x` over 15 cells and NO syscall; `st_begin` (`:160-168`) is two 15-cell crcs. That is ~0.1 us. SQLite with `journal_mode=OFF; synchronous=OFF` (the twin's default, `sbench_sqlite.py:22`) still writes dirty pages through `write()` at every COMMIT -- at least one syscall, ~10-30 us under proot (docs/RESEARCH-DEPS-2026-09-06.md, cited by RESEARCH-WILD-IDEAS §5.2), ~2-5 us native. The update row already shows the direction inside one transaction: 0.73 us per update vs 6.15 us (73 ms vs 615 ms for 10^5).

### 4.2 What it would cost to WIN the durable single-commit row outright, and the cheapest form

| form | mechanism | expected (proot) | cost |
|---|---|---|---|
| today | msync(tail range) ; toggle ; msync(8 KB) | 229 us | -- |
| **one sync instead of two** | write tail and superblock, then ONE `fdatasync(fd)` (syscall 83, one word after the `sys_fsync` pattern of B1 step 1) -- sound because consistency does not depend on ordering once B1's follow-up card verifies `[P.anc, G.cursor)` at reopen: a superblock whose payload did not land fails the crc walk and the other copy is taken (`store.bp:131-139`); the previous generation's payload was made durable by the previous commit's own sync | ~90-130 us (LANG-DB §3: write+fdatasync 4 KB = 90 us; msync 1 page = 100 us) -- **~2.3x ahead of FULL, ~0.5x of NORMAL** | ~10 lines in store.bp + the B1 card landing FIRST (it is the correctness argument); one new builtin word |
| group commit | already `st_commit_batch` (`store.bp:256-260`) | 7 us at N = 100 | 0 -- and RESEARCH-LITERATURE §7 says: no timer; "sync when the previous sync returned" |
| beat NORMAL on latency | impossible by definition: NORMAL does not wait for the device | -- | not a target |
| native re-measure (Termux, no proot) | `bebop.bin` is static and libc-free (`docs/ANDROID.md:5-10`); RESEARCH-WILD-IDEAS §5.2 gives the 20-minute recipe | both sides shrink; msync natively on f2fs `nobarrier` is a page writeback without a device flush, so the store's floor is probably 30-60 us and SQLite NORMAL's 10-20 us | operator's minutes |

**If the 3.7x does not survive native measurement:** nothing in this section changes except the absolute floor. An msync'd commit cannot beat an un-synced `write()`; the ratio to FULL is what carries information, and the class-matched rows are the ones to publish. If it DOES survive (i.e. msync natively is still ~100 us), then the one-sync form above is the lever and the proot tax was never the story.

**The MAP_SHARED tax reaches point 3 through one mechanism only:** a durable commit msyncs ~2 pages clean and the next commit's first store to each re-faults them (the write-protect-after-clean cycle RESEARCH-LITERATURE §2.2 lists first). At LANG-DB §3's 3.5-7 us per fault that is ~10 us of the 229, and it scales with pages per commit, not with stores -- which is why the update row (pages stay dirty for the whole transaction) shows no per-store tax at 0.73 us. D1 as written leaves the commit path alone; the wild-ideas 3.1 form (MAP_PRIVATE view + `pwrite` of the tail, LMDB's default without `MDB_WRITEMAP`) would turn the store's zero-syscall no-sync commit into a one-syscall commit and cost 10-30 us per commit under proot. **3.1 is a bulk-build discipline, not a commit-path design**, and should be scoped that way in D1 §1b.

### 4.3 "Batches build new generations, eliminating row-level lock conflicts" -- attacked

- Against SQLite there are no row locks to eliminate: SQLite is one-writer-at-a-time in every journal mode (https://www.sqlite.org/wal.html). Row-level locking is Postgres/InnoDB's problem. So the sentence compares against an opponent that is not in the benchmark.
- The axis that DOES decide single-writer OLTP throughput is the length of the writer's critical section: ~0.1 us here (a superblock write) against ~60 us there (a WAL frame `write()` under proot). In-process, that is a queue draining at millions of commits per second versus tens of thousands. That is a win, and it is unmeasured.
- Cross-process, the store's writer lock is `O_CREAT|O_EXCL` on a lock file (LANG-DB §4c) -- a path syscall at ~270 us under proot (LANG-DB §3's `rename` class), worse than SQLite's `fcntl` lock. A futex word in the MAP_SHARED superblock page is cross-process by default (a non-PRIVATE futex on shared memory) and `pool.bp:13-15` already has the builtins; that is a ~10-line change and an experiment, not a row.
- **B5 contradicts the concession.** B5 (`ROADMAP.md:109`, mandatory) has partition futex locks, a global toggle lock, STM read-set validation, abort + retry <= 8 then error 91, and a gate of >= 2x single-writer throughput on 3 A78s (`docs/blueprints/B5-multi-writer.md §0, §3`). That is a concurrent-writer OLTP gate. If micro-transactions are pointless, B5's gate is pointless; if B5 is mandatory, the position "changes arrive in batches" is not the design. The consistent reading: B5 is partition-level parallelism for INGEST batches (its real use in B8's order log), and the words "row-level" and "eliminates conflicts" should leave the position statement.

### 4.4 Where SQLite is simply better on this axis, stated plainly

1. Many processes in many languages committing short transactions against one file with `fcntl` locks and no toolchain -- an ecosystem property; bebop should not chase it.
2. Sustained random single-row commits with zero growth on a tight disk budget (§2.1 (b), (c)).
3. Today, recovery: 3.3 ms vs 44.7 ms -- but only because `st_reopen_verify` scans the whole arena (`store.bp:103-139`) while SQLite replays a 1000-row WAL; after the B1 card the store verifies O(last commit) and the fair row (same number of un-checkpointed commits on both sides) should flip.

Not better, despite the framing: point lookups (store 4-34x ahead), in-process commit rate (unmeasured, store ahead by construction), durable commit at matched class (store 1.2-1.6x ahead).

### 4.5 Rows, gates, kill experiments

| row | mechanism | attaches to | gate (the number) | cheapest experiment that kills it |
|---|---|---|---|---|
| **O1 -- durability-class-matched commit rows (THE cheapest experiment in this document)** | add to `sbench.bp` a phase `n`: 10^5 transactions of one REAL single-row update each (link the record: today via a fresh `T` is unfair -- use 10^3 transactions until B4, or update through a 64-row chunked `T` built for the phase), committed with plain `st_commit`; add to `sbench_sqlite.py` a `commit_nosync` phase: 10^3 transactions of one `UPDATE` each at `journal_mode=OFF; synchronous=OFF` and a `commit_normal` at `WAL; NORMAL`; report four rows: nosync / NORMAL-vs-batch100 / FULL / recover-after-N-commits | `bench/vs_rust/sbench.sh` rows 77-81; RESULT-sbench.md | store no-sync per commit **<= 1 us** and **>= 20x** SQLite `synchronous=OFF`; store batch-100 (7 us) vs SQLite NORMAL single (62.6 us) reported as the like-for-like "lose the last N commits" class; durable single vs FULL >= 1.0x AFTER the record is linked | ~25 lines across two files that exist; one slot run. If the store's no-sync commit exceeds 5 us, something in `st_begin`/`st_commit` pays a syscall it should not, and the whole §4 argument is wrong |
| **O2 -- one-sync commit** | `fdatasync` once after tail + superblock; correctness from the B1 follow-up card's anchored verify | B1 follow-up card (`docs/blueprints/B1-durability-torn-write.md:117-188`) FIRST, then ~10 lines in `st_commit_sync` | durable single **<= 130 us** under proot (from 229), scrash_torn TRIALS=1000 still 0 invalid, plus the card's negative test | if `fdatasync` alone under proot costs > 180 us the two-msync form is already the floor here and the row waits for native |
| **O3 -- recover row with matched work** | after the B1 card: reopen after N = 1000 un-synced commits on both sides | B1 card acceptance 3 | store recover **<= 1.0x SQLite** on the matched row | none beyond the card's own acceptance |
| **O4 -- cross-process writer lock as a shared futex** | futex word in superblock page 0 instead of `O_EXCL` file | LANG-DB §4c; `pool.bp:13-15` | lock/unlock round trip between two bebop processes **<= 20 us** under proot | measure the futex round trip first (RESEARCH-LITERATURE §8 asks for the same number for B3); if > 100 us the O_EXCL file is no worse |
| **O5 -- REFUTE the concession as written; restate** | -- | ROADMAP thesis, B5 | replace "changes arrive in batches ... eliminates row-level lock conflicts" with "single writer per partition; the in-process commit critical section is ~0.1 us; multi-process, multi-language OLTP is SQLite's and not chased" | O1's numbers |

---

## 5. Proposed rows with gates (one table)

| # | row | verdict on the operator's direction | attaches to | gate | kill cost |
|---|---|---|---|---|---|
| S1 | size row under columns + u32 | NARROW (the loss is 2.1x layout already scheduled to change + 0.4x harness) | C2, A8 | <= 0.8x SQLite (<= 27 MB) after compaction; PK lookup <= 2x today | 30 lines (C2 §7) |
| S2 | bytes per single-row commit | NARROW (block size is the lever, not compaction) | B4 step 2 | <= 2 KB per commit; peak <= 1.6x live | analytic, then B4's `u` phase |
| S3 | tier-local compaction on an A55 | NARROW ("free of readers" refuted; CPU-free yes, memory-free to be gated) | B4 step 2/3, B6 | writer stall <= 10 ms; A78 scans <= 1.2x solo during compaction; peak <= live + L1 | one slot run, zero code |
| V1 | `variant` tail column + shred at compaction | NARROW (poison is width under AoS; zero under columns) | A7, C2, §4d | scan <= 1.05x with tail; key read <= 1.0x `json_extract`; shredded key at column speed | emulated tail in today's store |
| V2 | concede ADD-COLUMN latency in writing | ADOPT the concession (the only one in this document) | -- | reported ratio, bytes rewritten 0/0 | none |
| O1 | class-matched commit rows | REFUTE the concession | sbench | no-sync <= 1 us and >= 20x; FULL >= 1.0x with the record linked | ~25 lines, one slot run |
| O2 | one-sync commit | -- | B1 card first | <= 130 us proot; torn trials 0 | if fdatasync alone > 180 us |
| O3 | matched recover row | -- | B1 card | <= 1.0x SQLite | card acceptance |
| O4 | shared-futex writer lock | -- | LANG-DB §4c | <= 20 us cross-process | futex round-trip probe |
| O5 | restate the position (drop "row-level", reconcile with B5) | REFUTE as written | ROADMAP thesis, B5 | -- | -- |

Order: **O1 first** (it decides whether point 3 is conceded or won, costs one slot run, and its numbers are needed by O5 and by the size/commit interaction in S2); then S3's zero-code contention probe; then the B1 card (it unblocks O2 and O3 and is already specified); S1 and V1 wait for C2/A8/A7 as scheduled.

---

## 6. What a fair benchmark of each dimension would have to measure

Two of the three current rows measure the harness. The rows that would measure the design:

**Dimension 1 (size).**
- Split the size row into entropy / derived / header / garbage bytes (RESEARCH-WILD-IDEAS §4.2 asked for the same split): today that is 32 / 24.4 / 16 / 12.8 MB, and it says C2's header amortisation and the `T` removal are the levers before any compression is credited.
- Measure at STEADY STATE under a sustained commit rate with the compaction trigger active: mean file size, peak file size, and bytes written to flash per commit (`/proc/self/io` `write_bytes` if proot exposes it -- `/proc/vmstat` is frozen here, journal 853, so verify the counter moves before believing it). SQLite's row must count `db + -wal + -shm` in WAL mode, not `journal_mode=OFF`'s single file.
- Report the store's ALLOCATED blocks (`stat -c %b`, already in `sbench.sh:21`) as the primary size, not `arena_used*8`.

**Dimension 2 (schema).**
- `ADD COLUMN` then first query: wall time (compile included on the store side) and bytes rewritten -- report the loss.
- A 3-key tail on every row: declared-column scan with/without the tail; tail key read per row vs `json_extract` and vs JSONB (`https://sqlite.org/jsonb.html`); the same key after shredding vs a declared column.
- Type change: migration at compaction vs `CREATE TABLE ... AS SELECT`, bytes written and wall.

**Dimension 3 (OLTP).**
- Four durability classes, each side in its own class: no sync (store `st_commit` vs SQLite `synchronous=OFF`), lose-last-N (store group commit vs SQLite `WAL; NORMAL`), sync per commit (store one/two msync vs `FULL`), group commit at N (already there). Never pair across classes.
- The store's update must LINK the record (a real update), and the row must state the block CoW bytes it paid.
- Recovery with matched un-synced work on both sides.
- Cross-process: two writers alternating commits through the file, per-commit latency including the lock.
- Every row re-taken once outside proot (Termux native) before any ratio is called a design property; the ctypes floor (~8 us per op, `RESULT-sbench.md` note) subtracted from SQLite's per-op rows as G7 already requires.

---

## 7. Prior-art matrix for the three axes

| axis | system | what it chose | what it cost them | URL |
|---|---|---|---|---|
| 1 | SQLite | in-place page rewrite through WAL; freelist; `VACUUM` at 2x | 8 KB written per 8 B change; deletes never shrink the file | https://www.sqlite.org/wal.html ; https://www.sqlite.org/lang_vacuum.html |
| 1 | Postgres | append new version + background autovacuum | bloat when vacuum lags; long readers pin garbage; HOT/fillfactor as mitigations | https://www.postgresql.org/docs/current/routine-vacuuming.html |
| 1 | LMDB | CoW B+tree + txn-keyed freelist, no growth at steady state; stale readers grow the file; compaction = copy | path copy per commit; no in-place compaction | https://raw.githubusercontent.com/LMDB/lmdb/mdb.master/libraries/liblmdb/lmdb.h |
| 1 | RocksDB | leveled (space 1.1x, WA 10-30x) or universal (space <= 2x, lower WA) | the dial, never both | https://github.com/facebook/rocksdb/wiki/Leveled-Compaction ; https://github.com/facebook/rocksdb/wiki/Universal-Compaction |
| 1 | DuckDB | block reuse at checkpoint; file never shrinks without a copy | same 2x rule | https://duckdb.org/docs/stable/operations_manual/footprint_of_duckdb/reclaiming_space |
| 1 | Datomic | immutable segments + background indexing job | growth until GC; one transactor | https://docs.datomic.com/indexes/index-model.html |
| 1 | f2fs | the filesystem under all of them here is log-structured with its own GC | "in-place" is logical on this box | https://docs.kernel.org/filesystems/f2fs.html |
| 2 | SQLite | `ADD COLUMN` O(1), `DROP COLUMN` rewrite, dynamic typing per cell, `json_extract`/JSONB walked per access | per-row decode on every read (M1's 180 ns/row) | https://www.sqlite.org/lang_altertable.html ; https://www.sqlite.org/json1.html ; https://sqlite.org/jsonb.html ; https://www.sqlite.org/fileformat.html |
| 2 | Parquet variant shredding | typed shredded columns + an unshredded variant remainder | a spec, 2025; readers must merge both | https://github.com/apache/parquet-format/blob/master/VariantShredding.md |
| 2 | ClickHouse JSON | `max_dynamic_paths` shredded sub-columns, the rest in shared data | scans over shredded paths are column-speed, the remainder is slow | https://clickhouse.com/docs/sql-reference/data-types/newjson |
| 2 | Snowflake VARIANT | automatic shredding of stable keys | opaque to the user | https://docs.snowflake.com/en/sql-reference/data-types-semistructured |
| 2 | Cap'n Proto / FlatBuffers | append-only fields, zero default, deprecate-never-delete | no type change without a new struct | https://capnproto.org/encoding.html ; https://flatbuffers.dev/evolution/ |
| 2 | PS-algol / Napier88 / PJama | closed world, evolution "invariably problematic" | dead (LANG-DB §1 [w3][w4]) | https://archive.cs.st-andrews.ac.uk/papers/download/DKM09a.pdf |
| 3 | SQLite WAL | one writer; NORMAL = no sync per commit, FULL = fsync per commit | 4 KB frame per commit; checkpoint | https://www.sqlite.org/wal.html ; https://www.sqlite.org/pragma.html#pragma_synchronous ; https://www.sqlite.org/atomiccommit.html |
| 3 | LMDB | one writer, readers lock-free; default writes via `pwrite`, `MDB_WRITEMAP` for a writable map | a syscall per commit in the default mode; wild-pointer risk in WRITEMAP | https://raw.githubusercontent.com/LMDB/lmdb/mdb.master/libraries/liblmdb/lmdb.h |
| 3 | Crotty/Leis/Pavlo CIDR'22 | "the OS may flush any dirty page anytime" -- out-of-place writes are mandatory for transactions over mmap | the reason the store appends | https://db.cs.cmu.edu/papers/2022/cidr2022-p13-crotty.pdf |
| 3 | Snapshot (ICCD'23) | keep the working copy in DRAM, sync deltas at msync; msync's page-level dirty tracking amplifies writes | the D1 discipline, from the PM side | https://arxiv.org/pdf/2310.16300 |

---

## 8. What could not be verified, and what would change

- No number in this document was produced today. The size decomposition in §1.1 is arithmetic on `sbench.bp`'s object sizes and matches the recorded bytes; the predicted 20-24 MB under C2+A8 assumes positional ids, u32 for `u`/`v`/`cell` (their ranges fit) and no per-block overhead beyond a blocktab.
- The no-sync per-commit cost of `st_commit` (~0.1 us) is read from the code (`store.bp:197-203`), not timed; O1 times it.
- SQLite `synchronous=OFF` per-commit cost under proot is inferred from "at least one `write()` per commit" and the DEPS document's syscall estimate; O1 times it.
- Whether a non-PRIVATE futex on the MAP_SHARED superblock page works across two bebop processes under proot: reasoned from the futex API, not probed (O4).
- Whether `/proc/self/io` moves under proot: `/proc/vmstat` does not (journal 853), so it must be calibrated first.
- The one-sync commit's soundness rests on B1's follow-up card being landed and its negative test (tear the shared page) passing; without the anchored verify, dropping the payload-before-superblock msync ordering is NOT sound and O2 must not be built.
- The A55 compaction contention number is a prediction from the 28/29/43 datum; S3's probe replaces it.

VERDICT: 0 ADOPT-as-stated. Point 1 NARROW -- steady-state size flips to ~0.6-0.7x SQLite under C2+A8 (the 2.5x is 2.1x scheduled layout + 0.4x a monolithic-table harness shape), growth per commit is a B4 block-size knob with a floor, "background compaction" is right only as tier-local compaction under B4/B6 with a memory-contention gate, and "only while free of readers" is refuted by the store's own inode-refcount design. Point 2 NARROW -- the data layer is ALREADY as agile as SQLite's; the unwinnable part is the compile step's latency (concede it); the tail heap poisons scans by width under objects and not at all under columns, and its survivable form is a shredded variant column that the compaction pass already knows how to migrate. Point 3 REFUTE the concession -- the losing row pairs an msync'd append against an un-synced update, both sides mis-measured; class-matched rows already win 1.2-1.6x and the unmeasured no-sync row should win by 10-100x; "batches eliminate row-level lock conflicts" is a restatement of single-writer and contradicts B5's mandatory abort/retry gate. The single cheapest experiment across all three is O1: ~25 lines in `sbench.bp` and `sbench_sqlite.py`, one slot run.
