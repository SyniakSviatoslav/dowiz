Status: 2026-09-08 -- read-only research by the session's analyst agent (Fable) over /root/dowiz/bebop-lang at the working tree of today (bebop.bp 6909 lines, bebop.bin 159,196 bytes and `ldd` says "not a dynamic executable", seed/seed.S 159 lines, store.bp 416 lines). Seven operator proposals for the tensor-graph DB evaluated against what the tree already has. Every repo claim carries file:line; every outside claim carries a URL; every "this box" number is either measured today by a read-only probe (`/proc/mounts`, `/proc/cpuinfo`, `ls /dev`, `wc`, `ldd`) or attributed to the committed row that measured it (ROADMAP.md Measured table, bench/vs_rust/RESULT-*.md, bench/substrate_spike/RESULT*.md). No compiler, gate, benchmark or `git` command was run (BOX.md: one heavy slot, three other agents live). Web sources were read through a search fetcher; where a claim rests on memory of a paper rather than fetched text it is marked (M). This is a PROPOSAL pending operator decisions.

# Seven architectural proposals for the store: what already exists, what is wrong, what to adopt

## 0. Executive summary (10 lines)

1. **Four of the seven are, in their load-bearing half, already in the tree under other names.** "Bring your own kernel" is the status quo: there is no query language, no planner and no engine process to ship to (LANG-DB-DESIGN.md:261-262 "No planner"; :303-304 "Not needed: ... a WAL, a planner"); a running program already compiles bebop source in-process (gb_run.bp:379-401) and runs the resulting machine words over a read-only mapping of the store in a forked child (gb_run.bp:241-271). "Memory as Git" is the store's design sentence (store.bp:1-14: append-only bump arena, two superblocks, root swap; mvcc.bp:3-4 "never in place", `prev` edge :53; compaction = Cheney copy into a new file + rename, store.bp:355-416) plus B4's `prev`/time-travel row (B4:22-23, :37-38, :61). ECS is the SoA column model the graph library already uses (`arr i64` per column, RESEARCH-GRAPHBLAS:82-83; GbMatrix = separate rp/ci/vv arrays, gb.bp:8-10) and its "free schema evolution" is LANG-DB §4d's Cap'n Proto rule (:220). OSR's cheap form -- switch execution strategy when the data's character changes -- is the Beamer push/pull switch at alpha 14 that runs mid-BFS today (sgraph2 frontier row, ROADMAP.md:188).
2. **Proposal 4's premise is false: there is no WAL in this store and the 44.7 ms recovery row is not a logging cost.** `st_reopen_verify` (store.bp:131-139) crc-scans the whole object arena from cell 1024 to the live cursor on every non-fresh open (store.bp:103-117); the fix -- an anchor cell in the superblock so only the last commit's pages are verified -- is designed in docs/blueprints/B1-durability-torn-write.md:117-188 and needs no new hardware. What remains of the proposal after the correction is one builtin (`dc cvap` + `dsb`, the AArch64 point-of-persistence clean; `/proc/cpuinfo` shows `dcpop` on all 8 cores today) replacing `sys_msync` in `st_commit_sync` (store.bp:235-241) IF a DAX-mapped device ever exists. None exists here (`ls /dev/pmem*`: none; no ACPI NFIT), Optane was cancelled in July 2022 and the target nodes are phones. "Recovery = 0" is wrong on real PMEM too: ADR platforms do not flush CPU caches, so a torn multi-word commit is exactly as possible as a torn page, and PMDK's own store replays its undo/redo logs at open.
3. **Proposal 1's real cost is the crux and it is not small: a WASM validator+interpreter or an eBPF-class verifier is a second compiler-sized, safety-critical artifact.** wasm3, the smallest serious WASM interpreter, is 64 KB of flash code (its README) -- 40 % of bebop.bin, and it interprets; the Linux BPF verifier is ~30,000 lines of C, larger than this whole compiler (6909 lines) by 4x, and kernel eBPF is unreachable from an unprivileged Android app anyway. The 159-line seed.S trust root does not survive either. The narrow form that is worth building is a **checked kernel dialect** of bebop itself: bounds checks (2 words per access, RESEARCH-NOPOINTERS-SQL:84-86), no `sys_*` builtins, `ref`-typed store access -- the compiler is the verifier, at native speed, ~300 lines, zero dependencies.
4. **Proposal 3 (race three plans) solves an estimation problem this design does not have** -- cardinalities are exact from `rp[k+1]-rp[k]` (B7:15, :34) -- and pays for it on a box where ONE A78 already saturates DRAM (ROADMAP.md:207: ~12 GB/s for one core and for three) and where every extra process counts against a cap of 32 (BOX.md). Racing three memory-bound kernels makes each ~3x slower. Its 1993 ancestor (Antoshenkov, Rdb/VMS) and its modern heirs (Vectorwise micro-adaptivity, SkinnerDB) all restrict the race to cheap variants of ONE operator, not whole plans.
5. **Proposal 5 (reactive materialisation as THE execution model) was measured and rejected in this tree already**: the sweep/dataflow engine is 41x slower than linear code on dense work (bench/substrate_spike/RESULT.md; ROADMAP.md:193-197) and the incremental curve crosses over at k/N = 0.39 % (RESULT-incr.md; ROADMAP.md:199-202); LANG-DB §7 item 2 (:392) records "whole-graph incremental recomputation as the default engine" as the thing Eve died of. The survivable form is a declared set of materialised cells maintained from B4's tail delta, gated on a measured crossover.
6. **Proposal 6 (mid-loop OSR) cannot be built as stated on this codebase today**: `sys_run` must execute in a forked child because entering another image's entry stub reconfigures the caller's arena registers (gb_run.bp:241-251, :566-568), so there is no in-process "stop here, resume at the same byte" path; hardware cache-miss counters are blocked (swpmu.bp:1-4: perf_event_open EACCES under Android seccomp); and the compiler emits no frame-layout side table to map loop state between two kernels (the thing V8's OSR is made of). The sound, cheap form is what HyPer/Umbra do: switch tiers at CHUNK boundaries -- and B3's tier-0/specialised switch already does it at query granularity.
7. **Proposal 7 is ALREADY, with three small gaps**: no commit object chain (the superblock keeps gen g and g-1 only, store.bp:9-12; older roots survive only through a type's own `prev`), no named branches/heads (one `root` cell), and no `st_open_at(gen)`; ~60-100 lines in store.bp. Its motivation ("MVCC's undo logs and locks wreck graph performance") does not apply: this store has no undo log, no redo log and no reader table (RESEARCH-NOPOINTERS-SQL:177-179; LANG-DB:200-206). Its cost is already on the books: the file grows by every update until compaction (LANG-DB:207-210; sbench: 85.2 MB after 10^5 updates vs sqlite 34.1 MB, ROADMAP.md:181).
8. **Proposal 2 (ECS) has one measurable win nobody has claimed for it here -- header amortisation.** Every store object pays a 16-byte header (store.bp:3-5); a 3-field record is 40 B and the G7 size row LOSES 2.5x/2.1x to sqlite (ROADMAP.md:181). A column stores one header per column, not per row. That attaches to A8's gate (G7 file size <= 1.2x sqlite, ROADMAP.md:91). Its loss is equally measurable: a point read of an entity with k components is k random lines instead of 1 (LANG-DB:431-432 already notes AoS "{u,v} pairs: -8 lines" for the window query). So: NARROW to "columns for scan-shaped tables, objects for entities", decided per type, not "no records".
9. **Against D0 and zero-dependencies**: no proposal adds a dependency except 1 (a runtime/verifier), which is the only one that also touches the trust argument (seed.S:1-4 "Zero C"; MANIFESTO C10). Proposal 7's Git shape is the one that would matter on a MESH (branches per node + merge), and there it becomes Automerge-class CRDT work that LANG-DB §1 [w23] scopes out of a single-writer store. Proposal 5's "background propagation" is a resident process per store on a box that SIGKILLs the 33rd process (BOX.md). Proposal 4 optimises for hardware that no courier's phone has.
10. **Net**: 0 ADOPT-as-stated; 3 ALREADY (1, 7, and the SoA half of 2); 3 NARROW (2, 5, 6); 2 REFUTE (3, 4). The two additions worth a ROADMAP row are the checked kernel dialect (from 1) and per-type column storage measured on the G7 size row (from 2); the commit-chain cells (from 7) are a B4 step-3 footnote.

---

## 1. Method and the box today

Read: docs/LANG-DB-DESIGN.md (all), ROADMAP.md, /root/dowiz/MANIFESTO.md, /root/dowiz/DECISIONS.md D0 (:6-12), docs/RESEARCH-GRAPHBLAS-2026-09-06.md, docs/RESEARCH-NOPOINTERS-SQL-2026-09-06.md, docs/blueprints/B1-B5, B7, selfhost/prelude/store.bp, selfhost/std/mvcc.bp, selfhost/std/gb_run.bp, selfhost/std/pool_compile.bp, selfhost/std/stm.bp, selfhost/std/swpmu.bp headers, bebop.bp:6826-6878 (`emit_sys_run`), seed/seed.S:1-30, bench/substrate_spike/RESULT*.md, bench/vs_rust/RESULT-sbench.md:17, docs/BOX.md.

Measured or observed today (read-only):

| fact | value | how |
|---|---|---|
| f2fs mount hosting the tree | `fsync_mode=nobarrier` on `/data` (also `barrier`, `flush_merge`); `/metadata` is `fsync_mode=posix` | `grep f2fs /proc/mounts` |
| persistent-memory device | none: `/dev/pmem*` absent, `/sys/firmware/acpi/tables/NFIT` absent, `/sys/bus/nd` permission denied (cannot be enumerated from this uid) | `ls` |
| CPU persistence-clean instruction | `dcpop` present on all 8 cores (ARMv8.2 `DC CVAP`, clean to point of persistence) | `grep -o dcpop /proc/cpuinfo` = 8 |
| perf counters | `/proc/sys/kernel/perf_event_paranoid` reads -1 under proot, but swpmu.bp:1-4 records `perf_event_open` (syscall 241) returning EACCES under Android seccomp; not re-tested today (no probe budget) | file read + code comment |
| compiler artifact | bebop.bin 159,196 bytes, statically linked, no libc | `ls -l`, `ldd` |
| compiler source | 6909 lines; store 416; gb_run 575; gen_gb 475; gb_compile1 192; seed.S 159 | `wc -l` |
| WASM in the tree | none: T94 "WASM direct binary emitter + wasmi.bp interpreter" is OPEN in TASKS.md:103 and part of the superseded 2026-08 vision (HISTORY.md:2309-2320); `find -name 'wasm*.bp'` returns nothing | find/grep |
| WASM in the parent project | dowiz-kernel runs under Wasmtime with fuel metering (DECISIONS.md:387-388, D-fuel) and MANIFESTO C1 says "deterministic Rust/WASM only" (MANIFESTO.md:14) -- a Rust dependency of the kernel crate, not of bebop-lang | grep |

Numbers reused from committed rows (not re-measured): ROADMAP.md:141-212 Measured table; RESULT-sbench.md:17 (`recover` 44,713 us vs sqlite 3,299 us); B3 row ROADMAP.md:107 (tier-0 1 ms; compile-on-miss 95-98 ms vs the 50 ms gate; pool hit 1.58 ms vs a 1.04 ms fork floor; generated frontier BFS 26 ns/edge vs 23 in-process); substrate spike (41x; crossover k = 256 of 65,536).

Verdict vocabulary (as asked): ALREADY / ADOPT / NARROW / REFUTE.

---

## 2. Proposal 1 -- Bring Your Own Kernel (WASM / eBPF replaces the query language)

### 2.1 What exists, precisely

| proposal element | in the tree | file:line |
|---|---|---|
| "the DB is blind to queries; the app ships selection logic" | there is no query language and no planner; a query is a `.bp` fn over `ref T`/CSR compiled by bebop.bin | LANG-DB:254-268 (§4f), :261-262 "No planner: the plan is the index chosen by the programmer, exactly as in MUMPS"; RESEARCH-NOPOINTERS-SQL:183-185 |
| "kills Query -> AST -> Plan -> Execute" | that layer does not exist yet; B7 would ADD it (~450 lines: parser 200, planner 200, glue 250) | B7:11, :45-52; ROADMAP.md:111 |
| "app and DB are one logical address space" | the store is a library `use`d by the program (31 `st_open(` call sites across selfhost/std and std_tests; sgraph2.bp:1-2 `use "selfhost/prelude/store.bp"`); there is no engine process | store.bp:140-151 `st_open` maps the file into the caller |
| "compile the kernel and ship it to the engine" | a program generates kernel SOURCE, compiles it in-process with the resident compiler (`emit_words_offs`, `entry_stub`), packs the image | gb_run.bp:379-401 `gb_bg_compile_noexpand`; pool_compile.bp:29-66 `par_compile` (W workers each `emit_words` after `sys_clone`) |
| "engine runs it directly over the data arena" | the child mmaps the kernel image PROT_READ\|EXEC and the store PROT_READ, then `sys_run` = `blr` into the seed entry contract | gb_run.bp:252-271; bebop.bp:6826-6849 (entry_off = LE64 at base[size-8], x0/x1 = argc/argv, `blr`) |
| "physical isolation preserved" | fork: `sys_clone(17, ...)` = SIGCHLD only, no CLONE_VM; result returned through an anonymous MAP_SHARED cell | gb_run.bp:184-186, :519-541 |
| kernels cached by digest, ABI-checked | Pool/Kernel store objects; a Kernel whose `compiler_digest` mismatches is a MISS | gb_run.bp:9-13, :78-94; open defect on the file-backed path, ROADMAP.md:107 (B3 "OPEN DEFECT CANDIDATE") |

So the proposal's headline is the status quo with bebop as the bytecode. The instruction words ARE position-independent AArch64 (seed.S:1-4; a `.bin` is words + literal cells + an LE64 entry offset), and the store is position-independent (LANG-DB:178-180). What the proposal adds is only the two words "WASM/eBPF" -- i.e. a DIFFERENT bytecode with a verifier.

### 2.2 What is actually missing today, and it is not the bytecode

**Memory safety.** A bebop kernel has no bounds checks (LANGUAGE.md:68 "no bounds check: reading past the end is UNDEFINED"; :129 lists bounds checks under "What is NOT"). A kernel can read any address in its process. Under the fork model that is the child's address space, which is a COPY of the parent's, including every MAP_SHARED mapping the parent held at fork time. In the B3 gate the parent maps the data store read-only (gb_pool.bp:229 `st_map_ro`) so the child inherits PROT_READ; but a WRITER program that dispatches a kernel while holding its own `st_open` RW MAP_SHARED mapping (store.bp:146: prot 3, flags 1) hands the child write access to the file through the inherited mapping. "Physical isolation" today is therefore conditional on the parent's mapping mode at fork, and a hostile kernel from anyone but the local app author can corrupt the store. The `run`-inside-fork rule was chosen for the parent's REGISTER state (gb_run.bp:241-251), not as a security boundary.

**Trust of the kernel source.** Kernels in the tree are GENERATED from templates by the store's own program (gen_gb.bp), i.e. same trust domain as the app. LANG-DB:225 already rules that a store file "cannot inject code": code bytes come only from the local `.bcas/<sha256>.bin`. For a kernel that arrives from a PEER (the mesh case) MANIFESTO §3.4 requires ML-DSA-signed code blobs verified against a pinned root -- a signature, not a sandbox.

### 2.3 What WASM or eBPF would buy, and what each costs against the invariants

| | WASM (validator + interpreter or baseline JIT) | eBPF (verifier + JIT) | checked bebop dialect (proposed narrow form) |
|---|---|---|---|
| memory safety mechanism | linear memory, every load/store bounds-checked against the memory size; validation of the type stack | static verifier: bounded loops (since Linux 5.3), pointer-type tracking, 1M-insn limit, no unbounded loops over 10M edges without helpers | bounds check `cmp xidx,xlen ; b.hs trap` (2 words), `tbnz` on power-of-two tables (1 word); `ref T` typed access; no `sys_*` in the dialect (compile-time reject) |
| size of the safety artifact | wasm3: "64KB Flash and 10KB RAM" (README, https://github.com/wasm3/wasm3) -- an interpreter, ~40 % of bebop.bin's 159 KB; a Liftoff-class baseline compiler is a second code generator of the register model's size (D14 row 7 cites wasmtime's own baseline tier at 1.1-1.5x of optimising, docs/DECISIONS-RESEARCH-2026-09-06.md:82) | kernel/bpf/verifier.c "~30,000 lines" (https://kernel-internals.org/bpf/bpf-verifier/ ; complexity history https://pchaigno.github.io/ebpf/2019/07/02/bpf-verifier-complexity.html ; spec https://docs.kernel.org/bpf/verifier.html) = 4.3x bebop.bp; userspace uBPF has no verifier of that strength | ~300 lines in bebop.bp (emitter for checked `[]` on typed tables + a dialect flag + a builtin allowlist), 2 constructs, bpref mirror |
| speed | interpreter: wasm3 is an interpreter; the OOPSLA'22 in-place interpreter reports interpreters at roughly one order below JIT (https://arxiv.org/pdf/2205.01183) -- the K6 scan at 18 ns/row would become ~100+ ns/row, back at sqlite's 158 | JIT'd eBPF is native-class but the program shape is restricted | native; RESEARCH-NOPOINTERS-SQL:84-86: +15 % on codegen-bound scans, ~0 at the DRAM ceiling |
| zero dependencies | a WASM interpreter written in bebop is the T94 task, OPEN and parked with the 2026-08 vision (HISTORY.md:2309-2320; TASKS.md:103); writing one is a multi-thousand-line project whose correctness IS the safety story -- a bug is a sandbox escape | kernel eBPF needs CAP_BPF: this proot is `untrusted_app_27` uid 10546 (BOX.md), so the kernel verifier is unreachable; a userspace verifier is a 30k-line port | none added; the verifier is the compiler that is already fixpoint-tested and fuzzed (TG-DONE 4/8) |
| trust root | seed.S:1-4 "FROZEN ... Zero C" stays, but the sandbox's correctness is a new root of equal weight | same | unchanged |
| D0: decentralised / mesh | portable bytecode is the one genuine mesh argument: a kernel from a peer runs on any node's CPU | eBPF is Linux-kernel-shaped, not a mesh format | AArch64-only, like everything here (ROADMAP.md:10); a peer kernel is SIGNED source compiled locally (MANIFESTO §3.4), which is the D0-consistent answer |
| D0: reliability > latency | an interpreter is slower but deterministic; fuel metering (D-fuel, DECISIONS.md:387) is what the parent kernel already uses to bound compute | verifier-bounded compute | a kernel dialect can carry the same step budget (swpmu.bp's software step counter, :4-7) |

Prior art and what happened to it: SingleStore's Code Engine runs Wasm UDFs under wasmtime in a 16 MB per-function in-process sandbox (https://docs.singlestore.com/cloud/reference/code-engine-powered-by-wasm/) -- as FUNCTIONS INSIDE SQL: the query language stayed, WASM became the UDF language. ScyllaDB added Wasm UDFs on wasmtime as an experimental feature (https://www.scylladb.com/2022/04/14/wasmtime/ ; https://docs.scylladb.com/manual/stable/cql/wasm.html), again inside CQL. XRP (OSDI'22, https://www.usenix.org/system/files/osdi22-zhong_1.pdf) runs eBPF "storage functions" (B+-tree lookups, aggregations) from the NVMe driver hook, integrated with WiredTiger; it removes kernel storage-stack overhead on NVMe (a cost this mmap store does not pay: the page cache is the buffer pool, LANG-DB §6) and it keeps the DB's query layer. No shipped system replaced its query language with BYOK; the ones that adopted a sandbox adopted it for UNTRUSTED code inside a trusted engine. Here the app and the engine are one trust domain and one process.

### 2.4 Verdict: ALREADY (bebop is the bytecode); NARROW-ADOPT the checked kernel dialect

- ALREADY: the pipeline "generate kernel -> compile in-process -> run over the mapped store in an isolated child -> cache by digest" is gb_run.bp + gb_compile1.bp + gen_gb.bp (1,242 lines) and is gated (gb_pool, gb_pool_abi, gb_bfs_gen, ROADMAP.md:107). B7's DSL is optional under this reading: its gates (Q6 >= 10x, Q1 >= 5x) are kernel gates and hold for hand-written kernels; the operator should decide whether B7's surface is wanted at all.
- NARROW-ADOPT: a **checked kernel dialect** (`kernel fn`): (i) `[T]`/`ref T` access emits the 2-word bounds check (1 word on power-of-two tables) -- RESEARCH-NOPOINTERS-SQL:84-97 already priced it; (ii) `sys_*` builtins rejected at parse time (like `scan` is reserved, ROADMAP.md:92); (iii) a software step budget from swpmu.bp; (iv) the dispatching parent drops RW mappings before `sys_clone` (mprotect to PROT_READ, or the child does it first). ROADMAP row: attaches to A8 (typed tables, the `[T]`/`ref T` types the checks need) and to B3's OPEN DEFECT row (the ABI-mismatch dispatch that trapped 82 is exactly a kernel running with no fence). Gate (falsifiable): a fuzz corpus of 10^4 generated kernels with out-of-range indices and forged refs -> 0 SIGSEGV (TRAP-82), 100 % loud trap; K6 ns/row with checks on <= 1.2x checks off; sgraph2 frontier fold unchanged. Cheapest refutation: hand-instrument the frontier BFS inner loop with the 2-word check and measure ns/edge-slot on the promoted binary -- if it exceeds 1.2x (45 -> 54 ns), the dialect is off by default for kernels and on for store code, which NOPOINTERS §1.2(c) already anticipated.
- REFUTE the WASM/eBPF form for THIS project: it adds the only kind of dependency the project cannot audit (a safety artifact larger than the compiler), interprets or re-implements codegen, and answers a question (untrusted code inside a trusted engine) that the local-first, single-process, signed-code design does not ask. Reopening trigger: a mesh use case that must run UNSIGNED peer kernels -- and D0 (crypto, post-quantum code-sign) says that use case must not exist.

---

## 3. Proposal 2 -- ECS instead of tables

### 3.1 What exists

- Columns as separate dense `arr i64` store objects: GbMatrix is {rp, ci, vv, mask} as four refs to arrays (gb.bp:8-10); tables in the bench are SoA already (nn.bp columns, RESEARCH-GRAPHBLAS:82-83 "columns = SoA vectors (in the store: `arr i64` per column -- nn.bp already does it)"); the T100 window query reads four arrays rp/ci/us/vs (LANG-DB:427-429).
- Entities as bare i64 ids: a row id is an index into the column arrays; CSR `ci` holds row ids (B2:27-28); the edge log stores (src, dst, rel) triples, not records (LANG-DB:593).
- "A new field is just a new array, old data ignores it": for OBJECTS this is §4d's append-a-field rule at zero cost (LANG-DB:220 -- shorter old objects read as 0/default); for COLUMNS it is trivially true.
- "Raw bytes" components: A7 (byte arena + `str` as `(off<<32|len)`, ROADMAP.md:90) is the byte-column mechanism; today every cell is 8 B (LANG-DB:154 "Scalars: i64 only (doctrine)").

So "ECS" here is a vocabulary for SoA + row ids, which the graph side of the store already is. The proposal's novelty is the word "instead": no records at all.

### 3.2 What "no records" costs and buys, with numbers from the rows

| effect | AoS objects (today's §4a) | pure ECS / SoA columns | source |
|---|---|---|---|
| per-row overhead | 16 B header (h0, h1) per object: a 3-field record = 40 B; G7 size row 2.5x/2.1x LOSS vs sqlite (85.2/72.4 MB vs 34.1 MB) | one header per COLUMN; the same record = 24 B; with A8's u32 cells 12 B | store.bp:3-5; ROADMAP.md:181, :91 (A8 gate: file size <= 1.2x sqlite) |
| point read of one entity with k fields | 1 line (fields adjacent) | k random lines (one per column array); LANG-DB §8 already computed the direction: "records as {u,v} pairs: -8 lines" on the 35-line window query | LANG-DB:427-432 |
| scan over 1 of k fields | reads 40 B/row to use 8 | reads 8 B/row: the projection-is-free argument | RESEARCH-GRAPHBLAS:88 "projection = zero bytes (SoA)" |
| per-object crc32 | h1 carries crc of the payload; torn-object detection at reopen is per object | crc per column block (A7's "crc32x per page", ROADMAP.md:90); `st_verify` must change shape | store.bp:103-117 |
| MVCC per row | supersede = re-append the object (mvcc.bp:48) | an update to one entity CoWs one 4 KB block PER COMPONENT touched (LANG-DB:629-633: "update under MVCC = 4 KB block CoW, not one store") -- k components = k blocks | LANG-DB §9.4 |
| schema evolution | append free, rename free, type change = migration at compaction (LANG-DB §4d) | append free; DELETE of a component is free (drop the array); type change of a column = rewrite the column | LANG-DB:216-231 |
| the thesis sentence | "A Bebop program's persisted objects are its in-memory objects (same layout ...)" | violated for entities: an in-memory struct would have to be materialised from k columns | ROADMAP.md:13-15 |
| ECS archetypes | -- | archetype ECS (Flecs) groups entities with the same component SET into one table with columnar storage -- i.e. it re-invents tables (https://ajmmertens.medium.com/building-an-ecs-storage-in-pictures-642b8bfd6e04 ; https://www.flecs.dev/ecs-faq/); sparse-set ECS keeps one array per component plus an id->index map = a CSR/bitmap-indexed column | prior art |

Prior art fate: ECS won in game engines (Unity DOTS, Bevy, EnTT, Flecs), where the workload is "every frame, scan every entity that has components A and B" -- a pure scan workload with no durability, no point lookups by key, no MVCC. Archetype ECS exists precisely because pure per-component arrays fragment iteration; the AFIT thesis on ECS storage (https://scholar.afit.edu/cgi/viewcontent.cgi?article=6355&context=etd) and "The Essence of ECS" (https://arxiv.org/pdf/2606.14919) both frame the archetype/sparse-set trade as scan locality vs structural-change cost, which is the AoS/SoA trade above under a new name. No ECS became a durable database; column stores (the DB-world name for the same idea) won ANALYTICS and lost OLTP point-access -- exactly the split RESEARCH-GRAPHBLAS:191-202 already draws for this project.

### 3.3 Verdict: NARROW -- "columns per type, not instead of types"

- What to adopt: a per-type storage choice `column` vs `object` in the layout string (LANG-DB §4d's "user's choice per type" pattern, :224): scan-shaped tables (W's order-event log, lineitem in B7) as columns with one header per column; entity-shaped types (the FSM state per order, GbMatrix headers) as objects. The compiler already digests layouts (store.bp:280-285); a column table is `{n, ref col_0, ..., ref col_k}` -- a GbMatrix-shaped header, which gb.bp:8-10 IS.
- ROADMAP row: A8 (typed tables, u32 cells; gate G7 file size <= 1.2x sqlite). The size row is the falsifiable gate: 1M 3-field records as columns must land <= 1.2x sqlite's 34.1 MB (i.e. <= 41 MB) where objects land at 72.4 MB after compaction. The point-lookup row must not regress past 2x (450 ns today, ROADMAP.md:177) -- if it does, the entity type stays an object.
- Cheapest refutation: the T100 window query has both layouts in the tree already (SoA arrays in nnidx.bp vs the §8 estimate of a {u,v} AoS record); write the AoS variant of nnidx.bp (~30 lines), measure the 4.0 us row. If AoS is not faster on the point query, the "k lines" cost is hidden by OoO overlap and pure columns are safe for this workload too.
- D0 cost: none. Zero-dep cost: none.
- What to REFUTE inside the proposal: "raw bytes" components -- the integer-only doctrine and A7's `(off,len)` handle already give bytes a home without breaking the 8-byte cell; and "no records" as a doctrine, because the thesis sentence and the point-lookup row both depend on records.

---

## 4. Proposal 3 -- Speculative plan execution ("quantum optimizer")

### 4.1 The problem it solves does not exist here as stated

- A cost-based optimiser is needed when cardinalities are ESTIMATED. B7's planner uses exact counts: `rp[k+1]-rp[k]` for a bucket, zone maps for a range, `rp` counts for group-by cardinality (B7:15, :33-37; RESEARCH-NOPOINTERS-SQL:189-192 "exact cardinality of the bucket is known, not an estimate"). For k <= 4 tables it enumerates k! <= 24 orders with a 3-row cost table (B7:34). There is no estimation error to race against on single-table access paths; on joins the enumeration is exact in the inputs and the residual error is the output multiplicity, which SpGEMM computes from `rp` before running (RESEARCH-GRAPHBLAS:112-114).
- The skew case the proposal names ("skewed data") is in the tree as a measured-first twin: B2 (i) runs uniform AND Zipf(1.1, 1 % keys carry 30 % of rows) and gates on both (B2:23, :66). If the exact-count planner picks wrong on Zipf by > 2x, THAT row says so, and the fix is a 1-pass zone map or a per-bucket count check -- a probe, not a race.

### 4.2 What the race costs on this box

| resource | fact | consequence |
|---|---|---|
| memory bandwidth | ~12 GB/s for ONE A78 and ~12 GB/s for three (ROADMAP.md:207) | three concurrent memory-bound kernels each get ~1/3: the "first 1000 rows" phase runs every candidate ~3x slower, and a scan that would finish in 18 ms alone finishes no sooner by racing |
| cores | 3 usable A78 (ROADMAP.md:206); nn4 3-core scaling 2.21x on a bucketed scan (ROADMAP.md:172) | a race consumes the entire parallel budget that B6 wants for row-range partitioning of ONE plan |
| process cap | 32 phantom processes; `sys_run` kernels run in forked children (gb_run.bp:241-251) | three racers = three children per query; the fork floor is 1.04 ms each (ROADMAP.md:107) -- more than a 1000-row sample costs (1000 x 18 ns = 18 us) by 50x |
| JIT cost | a specialised kernel compiles in 95-98 ms today, gate 50 ms UNMET (ROADMAP.md:107); tier-0 generic is 1 ms | "JIT all three" costs ~300 ms before the race starts; tier-0 variants exist without compile but at 2-5x the specialised speed |
| kill the losers | a loser writing output into the store leaves garbage above its cursor (harmless, LANG-DB:274-276) but a loser running in the parent's RW mapping is the isolation problem of §2.2 | same fix as §2 |

### 4.3 Prior art and its fate

- Antoshenkov, "Dynamic query optimization in Rdb/VMS" (ICDE 1993) and the Oracle Rdb line (https://www.semanticscholar.org/paper/Query-processing-and-optimization-in-Oracle-Rdb-Antoshenkov-Ziauddin/2a5ef508107647f5755a900bb1ad289709f0e3c2): Rdb ran COMPETING retrieval strategies for the same operator and dropped the losers -- the proposal, 33 years ago, restricted to one operator's access methods, on a machine where the competition was I/O-bound (M).
- Kabra & DeWitt, mid-query re-optimisation (SIGMOD 1998, https://dl.acm.org/doi/10.1145/276304.276315): collect statistics at materialisation points, re-plan the rest -- sequential, not concurrent.
- Vectorwise micro-adaptivity (Raducanu/Boncz/Zukowski, SIGMOD 2013, https://15721.courses.cs.cmu.edu/spring2018/papers/03-compilation/p1231-raducanu.pdf): keep several compiled FLAVOURS of one primitive, pick per call with an epsilon-greedy bandit "guided by the actual costs observed so far" -- the cheap, sound race: sequential trials of tiny units, no parallel duplication, consistent gains on TPC-H.
- SkinnerDB (Trummer et al., TODS 46(3) 2021): join order chosen per time slice by a regret-bounded learner -- the "abandon the CBO" thesis in its strongest published form; also sequential, and its cost is bounded by construction, not by racing.
- The survey of robust optimisation (SIGMOD Record 44(3), https://dl.acm.org/doi/10.1145/2854006.2854012) classifies parallel plan execution as the most expensive family precisely because it multiplies resource use.

### 4.4 Verdict: REFUTE as a planner replacement; the micro-adaptive form is a footnote to B3

- REFUTE: on exact cardinalities the race has nothing to learn, and on this box it costs 3x bandwidth, the whole core budget, 3 forks and ~300 ms of JIT per query.
- Footnote worth one line in B3/B7: Vectorwise-style flavour selection among ALREADY-COMPILED pool kernels (e.g. push vs pull mxv, or tier-0 vs specialised) per chunk, sequentially, keyed by measured ns/row of the previous chunk. That is B3's tier switch made per-chunk (see §7 below), not a race. Gate if ever built: the B2 Zipf join row must show the planner's choice > 2x off the best kernel; otherwise nothing is built.
- D0 cost of the proposal: reliability-over-latency is inverted -- the race spends 3x the energy of one plan (ROADMAP's per-commit energy proxy, docs/PERF.md) on a battery-powered node to shave latency.

---

## 5. Proposal 4 -- Abolish the WAL for persistent memory

### 5.1 The premise, checked

| claim in the proposal | the tree |
|---|---|
| "the 44.7 ms recovery exists only because disk-style safety is imitated in RAM via logging" | there is no log. Commit = write the OTHER superblock (store.bp:197-203, :201 `512 - sb`); the arena is the after-image (LANG-DB:194-199 "No rename, no page CoW, no journal"); §4i lists "a WAL" under "Not needed" (:303-304); RESEARCH-NOPOINTERS-SQL:177-179: append-only + root swap "needs neither UNDO nor REDO". The 2026-08 vision's "no WAL locks" (HISTORY.md:61) was never a mechanism to remove |
| where the 44.7 ms goes | `st_open` -> `st_reopen_verify` (store.bp:149, :131-139) -> `st_verify(base, base[sb+4])` crc-scans EVERY object from cell 1024 to the live cursor (store.bp:103-117). The scan is O(store), the damage a crash can do is O(last commit) -- B1 follow-up card, docs/blueprints/B1-durability-torn-write.md:117-123 |
| the fix | anchor cell 9 of the superblock = first object touching the page that holds the cursor; at reopen verify [P.anc, G.cursor) only (B1:145-172); acceptance = 1000 torn trials 0 invalid + a NEW negative test tearing the shared page + the `recover` row reported as a ratio (B1:174-184). Pure software, one signature change |
| sqlite's 3.3 ms | is not zero either: it replays WAL frames at open (RESULT-sbench.md:17 "open with a non-empty -wal after no checkpoint") |

So the proposal diagnoses a logging cost that does not exist and prescribes hardware for a software choice that is already scheduled to change.

### 5.2 What is left after the correction

The genuine content of "persistent memory" for THIS store is one substitution: byte-addressable NVM turns `sys_msync(range)` (~100 us per page, LANG-DB:668; durable commit 506 us, ROADMAP.md:183) into cache-line writebacks + a fence (~tens of ns per line). Because the store already writes "straight into the final arena structure" (bump append, store.bp:171-178; no page CoW; root swap), the DAX form of the store is the SAME file format and the SAME code with two call sites changed:

- `st_commit_sync` (store.bp:235-241): `sys_msync(appended range)` -> `dc cvap` over the appended lines + `dsb ish`; `st_sync(base, 8192)` -> the same over the superblock's 16 cells.
- One builtin `persist(cells, n)` under the L1 discipline (asm -> objdump -> words, AGENTS.md:181): `DC CVAP` is present on this CPU (`dcpop` in /proc/cpuinfo today), so the WORDS can be derived here; their EFFECT cannot be observed here.

What does NOT go away on real PMEM:

- Ordering. On ADR platforms "data in the CPU caches must be flushed by the application using CLWB ... and an SFENCE" (Intel, https://software.intel.com/content/www/us/en/develop/articles/eadr-new-opportunities-for-persistent-memory-applications.html); only eADR (3rd-gen Xeon) puts caches in the persistence domain, and even then "an SFENCE operation to maintain write order" is required. AArch64's equivalent is `DC CVAP` + `DSB`. The two-superblock protocol's "payload durable BEFORE the root" (store.bp:235-239) is exactly the ordering constraint and it stays.
- Torn commits. 8-byte stores are atomic; a 6-cell object is not. The crc in h1 (store.bp:4-5) and the verify-last-commit step remain necessary. Recovery is O(last commit), not 0 -- on PMEM as on f2fs. libpmemobj, the reference PMEM object store, keeps redo/undo logs and processes them at open (https://link.springer.com/chapter/10.1007/978-1-4842-4932-1_7); the crash-consistency bug hunts found 49 new bugs across PMDK's own Array and 15 published PMEM indexes (Witcher, SOSP'21, https://www3.cs.stonybrook.edu/~dongyoon/papers/SOSP-21-Witcher.pdf ; Agamotto OSDI'20, https://www.usenix.org/system/files/osdi20-neal.pdf) -- the ordering discipline is HARDER to get right at cache-line granularity than at page granularity, because the failure window is every store.
- Failure-atomic msync (Park/Kelly/Shen, EuroSys 2013, https://web.eecs.umich.edu/~tpkelly/papers/Failure_atomic_msync_EuroSys_2013.pdf) and its CXL-era successor Snapshot (https://arxiv.org/pdf/2310.16300) show the OTHER direction is where the field went: keep msync as the commit primitive and make it atomic, rather than abolish it.

### 5.3 Measurability and the invariants

- Not measurable here even in principle: no PMEM/CXL device; f2fs `fsync_mode=nobarrier` (confirmed today) means power-loss durability is not provable on this box for ANY design (LANG-DB:279-281, §0.4); the B1 sector model is the proof (B1:18, :22-33).
- Hardware trajectory: Intel wound down Optane in July 2022 with a $559 M write-off (https://www.datacenterdynamics.com/en/news/intel-kills-off-optane-memory-writes-off-559-million-inventory/ ; https://www.theregister.com/on-prem/2022/07/29/why-intel-killed-its-optane-memory-business/), PMDK moved to security-fix-only maintenance (https://pmem.io/blog/2022/11/update-on-pmdk-and-our-long-term-support-strategy/), final 200-series shipments end late 2025 (https://www.tomshardware.com/pc-components/ssds/intel-schedules-the-end-of-its-200-series-optane-memory-dimms-shipments-to-draw-to-an-end-in-late-2025). CXL memory that ships is overwhelmingly volatile DRAM expansion; the CXL consortium's persistent-memory support deck is a 2021 specification document (https://computeexpresslink.org/wp-content/uploads/2023/12/CXL-2.0-Presentation-Persistent-Memory-20210615_FINAL.pdf), not a product.
- D0: local-first on courier phones with UFS flash; "optimise exclusively for byte-addressable NVM" optimises for hardware no node in the mesh has, and would drop the msync path those nodes need. Reliability-over-latency says the 506 us durable commit (or 7 us/commit in batches of 100, ROADMAP.md:105) is the number to keep honest, not to abolish.

### 5.4 Verdict: REFUTE the premise and the exclusive strategy; the design is already DAX-ready

- REFUTE: no WAL exists; the recovery row is a verify-scope bug with a written fix (B1 card); "recovery = 0" is false on PMEM; the hardware is cancelled and absent from every target node.
- What survives as a note in LANG-DB §4g: the store's on-disk format needs no change for DAX; the port is one `persist` builtin at two call sites; forward-port only when a DAX box exists. No ROADMAP row.
- The B1 follow-up card IS the action, and it is the only recovery work that has a gate.

---

## 6. Proposal 5 -- Reactive tensor materialisation (dataflow instead of pull)

### 6.1 The measured record in this tree

The proposal's execution model -- every mutation propagates as a wave through the tensor graph, updating terminal cells in the background; reads compute nothing -- has been built twice here and measured both times:

| experiment | number | source |
|---|---|---|
| T55 spike: the same 12-op function as a cell substrate (activity-wave sweeps to quiescence) vs linear code | 738 ms vs 18 ms = **41x slower**; the Rust twin of the same sweep ENGINE is 39x slower than linear Rust -- the model, not the codegen | bench/substrate_spike/RESULT.md; ROADMAP.md:193-197 |
| T107 incremental curve, 2^16-cell DAG, k changed inputs per rep | sweep beats full recompute only for k < 256 = **0.39 % of N**; at k = 4096 the sweep is 4.8x (bebop) / 10.7x (Rust) slower than full | bench/substrate_spike/RESULT-incr.md; ROADMAP.md:199-202 |
| the roadmap's standing decision | "cores only for parallel scans (T106), the sweep engine only where activity is sparse (T107); no runtime cells for ordinary code (T55 spike: 41x/740x against)" | ROADMAP.md:29-30 |
| Eve, the nearest prior attempt at "everything reactive" | died 2018; item 2 of what Bebop must NOT take: "whole-graph incremental recomputation as the default engine ... incremental only for sparse deltas, full recompute otherwise -- the compiler decides per site with a measured crossover, never a global fixpoint" | LANG-DB:362-398, :392 |

"The DB runs like a neural net forward pass" is a full recompute -- a scan at 18 ns/row today, ~2 ns/row at the DRAM ceiling (RESEARCH-GRAPHBLAS:99) -- and a scan is what the pull query already is. "A wave through the tensor graph" from the changed cells is SpMSpV from a frontier of size |delta|: cost |delta| x fan-out per level, which is the incremental branch of the curve above and wins only below the crossover.

### 6.2 What is genuinely there and what is missing

- The delta already exists: B4's tail COO is the per-transaction delta matrix, merged by eWiseAdd at promotion (B4:24, :31-33; RESEARCH-GRAPHBLAS:157-160 "batches = delta matrices = log-structured tensor").
- Views with staleness tracking already exist in design: an index carries the table generation and is "stale when table generation != idx.generation (loud trap or rebuild)" (LANG-DB:265-267); Eve's `bind` maps to exactly this (LANG-DB §7 table, "stale when table.generation != view.generation (rebuild or incremental)").
- Missing: a declared set of MATERIALISED cells (the "terminal nodes") with an update rule per cell, the incremental kernel that applies the tail delta to them (for W's folds: count/sum per key = `reduce` over the delta rows, O(|delta|)), and the crossover switch (recompute the view fully when |delta| / |view input| > c, c measured, T107 says ~0.4 % for a DAG sweep; for a linear reduce it will be much higher, which is the point of measuring).
- "The client reads a single i64 cell kept current in the background": the background writer is a PROCESS (a forked child as in gb_bg_launch, gb_run.bp:457-461) or a thread; on this box every process counts toward 32 (BOX.md) and B5's single-writer-per-partition rule (B5:24-31) means the propagator is THE writer of that partition, i.e. the view must be updated inside the commit that changes its inputs -- synchronous, in the writer, not "in the background". That is write amplification per commit, which is fine when the views are few and O(|delta|), and is exactly Noria's problem when they are many: Noria (OSDI'18, https://www.usenix.org/conference/osdi18/presentation/gjengset) had to invent partial state and eviction because "its memory footprint would explode with many queries"; Materialize keeps every arrangement resident (LANG-DB §7 table); DBSP/Feldera (https://docs.feldera.com/vldb23.pdf) gives the mechanical incrementalisation of any relational query, with cost O(|delta|) for linear operators and O(|delta| x indexed other side) for joins -- the theory that says which views are cheap.

### 6.3 Verdict: NARROW -- declared materialised cells over B4's delta, gated on a crossover

- ADOPT (narrow): `view` objects in the store = {digest of the fold kernel, input table ref, generation, ref cells}; the writer applies the fold's INCREMENTAL kernel to the tail delta at commit (B4 step 1's promotion point) when |delta| <= c x |input|, else marks the view stale and the reader recomputes on demand (today's pull). Reads of a fresh view are one `ldr`. ROADMAP row: B4 step 3 (it needs `prev`/gen on matrices) feeding B8 (W's folds per merchant/courier are the views). Falsifiable gate: on W's order log, 10^5 events appended in batches of 100: (a) incremental view maintenance cost per event <= 0.5 us amortised (the G9c number, B4:7) on top of the update; (b) the crossover c measured for reduce and for a 2-way join view and written into the blueprint; (c) fold of every view == full recompute == python oracle after every 10^4 events. Cheapest refutation: implement ONE view (count per key) by hand over sgraph2's edge log (~40 lines) and measure ns/event; if incremental maintenance costs more than the scan it replaces at the batch sizes W has, the row is dropped and pull stays.
- REFUTE the paradigm claim ("invert the paradigm", "the client computes nothing"): the numbers above are the refutation; and LANG-DB §7's Eve section already spent the argument.
- D0: local-first is unaffected; reliability-over-latency FAVOURS pull (a fresh fold is never stale) unless the view is maintained in the commit; a reactive store that answers from a stale cell is a latency optimisation wearing a reliability costume. Zero-dep: none added.

---

## 7. Proposal 6 -- On-stack replacement of a running kernel

### 7.1 What the proposal needs and what the tree has

| OSR ingredient (V8) | in the tree | file:line |
|---|---|---|
| a trigger signal (V8: type feedback / tier-up counters; here: cache misses above a threshold) | hardware counters blocked: perf_event_open EACCES under Android seccomp; software step counters exist | swpmu.bp:1-9 |
| stop mid-loop and resume the same iteration in new code (needs a side table mapping every live value's location in frame A to frame B) | none: the register model's frames are `80 + 8*marks + 8*slots` with no metadata (A6, ROADMAP.md:89); "Sparkplug uses Ignition's frame layout, making OSR trivial, while Maglev and TurboFan use different layouts, requiring full frame translation" (https://www.thenodebook.com/node-arch/v8-engine-intro; V8 OSR at loop headers https://wingolog.org/archives/2011/06/20/on-stack-replacement-in-v8 ; https://v8.dev/blog/maglev) | -- |
| enter new code in the same process | `sys_run` is fork-only: "entering another compiled image's entry stub reconfigures the caller's own arena/signal-stack registers ... an in-process sys_run is explicitly ruled out" | gb_run.bp:241-251, :566-568 |
| compile in the background | exists: gb_bg_launch forks a compiling child, parent continues on tier-0 | gb_run.bp:457-472; ROADMAP.md:107 (c) |
| a specialised kernel arrives while the generic one runs | exists at QUERY granularity: tier-0 answers now, the specialised kernel serves the NEXT call | B3:33-36; gb_run.bp:462-472 |
| "the data's character changes mid-scan" -> switch strategy | exists at ALGORITHM granularity: Beamer push/pull switch at alpha 14 inside one BFS, 4.3x over queue BFS | ROADMAP.md:188; sgraph2 phases; LANG-DB:555 |

### 7.2 The sound form: chunk-boundary switching, not byte-level resume

- Kohn/Leis/Neumann, "Adaptive Execution of Compiled Queries" (ICDE 2018, best paper, https://dblp.org/rec/conf/icde/KohnL018.html): start on an interpreter of the query's IR while LLVM compiles in the background, then switch to compiled code -- the switch happens at MORSEL (chunk) boundaries, where the only live state is "which rows are done" (M for the granularity detail; Leis/Neumann's 2025 retrospective https://www.hytradboi.com/2025/slides/leis-neumann-compilation.pdf ; Schmidt's dynamic blocks https://db.in.tum.de/~schmidt/papers/dynamic-blocks.pdf). That is OSR with the side table reduced to one integer, and it is exactly what B6's row-range partitioning gives for free (B6: work queues over row ranges).
- "Resume at the same byte" is a V8 requirement because JavaScript loops have arbitrary live state; a scan/mxv kernel over CSR has live state (row, acc) that the chunk boundary serialises anyway.
- Micro-adaptivity (§4.3) chooses per chunk among precompiled flavours by the previous chunk's measured cost -- the cheapest trigger that works without PMU access: `clock_ms` or the swpmu step counter per chunk.

### 7.3 Verdict: NARROW -- tier/flavour switch at chunk boundaries inside one query

- What to adopt: B3's tier switch made intra-query: the dispatcher runs the query as a sequence of row-range chunks (B6's unit); between chunks it (a) checks whether the background-compiled specialised kernel has landed (pool hit) and switches, (b) optionally switches flavour (push/pull, tier-0/specialised) on the last chunk's ns/row. State carried across the switch: the chunk index and the partial reduce -- values that already cross the fork boundary through the MAP_SHARED result cell (gb_run.bp:519-541). ROADMAP row: B3 step 4 follow-up + B6. Falsifiable gate: on a 1M-row scan whose specialised kernel compiles in ~95 ms while tier-0 runs at 2-5x the specialised cost, wall time with the mid-query switch <= max(tier-0 alone, compile + specialised alone) x 0.8; frontier BFS fold unchanged. Cheapest refutation: the B3 latency rows already have tier0 (1 ms) and specialised compile (95-98 ms); if a whole query at tier-0 takes < 95 ms there is nothing to switch to -- which is today's case for every gate in the tree, so the row only opens when B8's W has a query longer than the compile.
- REFUTE the byte-level form: no in-process entry, no counters, no frame map; building the three is the register model's size again for a case (a scan whose character changes mid-way) that the chunk switch covers.
- D0/zero-dep: none.

---

## 8. Proposal 7 -- Memory as Git (topological versioning)

### 8.1 What "memory as Git" already is, mechanism by mechanism

| Git-shaped property | in the store today | file:line |
|---|---|---|
| immutable, append-only object arena | `st_alloc` bump-allocates, nothing is rewritten except the alternate superblock | store.bp:171-178; LANG-DB:170-172, :272-276 |
| every transaction creates a new consistent world; commit = root swap | `st_commit_m` writes the OTHER superblock with gen+1 and the new root; readers pick the higher valid gen | store.bp:197-203, :77-81 |
| a reader holds a "commit" and sees a whole consistent world with no locking | the mapping is the token: "objects reachable from any older root are never overwritten ... a reader is consistent for as long as its mapping lives -- no reader table, no lock, no token integers" | LANG-DB:200-206; `st_map_ro` store.bp:153-158; G6 (4 writers x 10^4, 4 readers, 0 lost/torn: LANG-DB:317; green per ROADMAP.md:52) |
| new version = new object with a `prev` edge, never in place | mvcc.bp:3-4, :48-65 (`prev[nv] = old`, :53); B4's GbMatrix carries `ref prev` (gb.bp:8-13; B4:23, :37 "time-travel: follow prev while gen > wanted") | |
| exact accounting of superseded space | tx[3]/tx[4] -> superblock cells 7 (live) and 8 (superseded) | store.bp:13-14, :189-192, :199-200 |
| GC/compaction only when no reader needs the old version, off the hot path | `st_compact` = Cheney copy into `<store>.tmp` + rename + directory fsync; the OLD INODE stays alive exactly while a mapping exists -- the kernel's refcount is the "every reader has left" test | store.bp:355-416; LANG-DB:203-206, :246-247 |
| "repacking csr-buckets in the background" | B4's tail -> L0 -> L1 promotions (bounded, never in place) + compaction as a forked child (the gb_bg_launch pattern) | B4:31-38; gb_run.bp:457-461 |
| snapshot isolation for readers, serialisable single writer | RESEARCH-NOPOINTERS-SQL:157-160; B5 keeps single-writer as the degenerate case of partitioned writers | B5:11 |

The proposal's motivation -- "classic MVCC uses undo logs and locks, which wreck graph performance" -- is true of InnoDB and false of this store: "append-only + root swap needs neither UNDO (nothing is overwritten) nor REDO" (RESEARCH-NOPOINTERS-SQL:177-179), and LMDB is the industrial proof that the same shape works without either (symas.com/lmdb quoted at :316-318).

### 8.2 What is missing, exactly

1. **A commit chain.** The superblock keeps ONE root and ONE generation (cells 2-3, store.bp:10-11); the other superblock is g-1. Older worlds are reachable only through a TYPE's `prev` (LANG-DB:224 "user's choice per type"), not through the store. Git has a parent pointer per commit. Fix: a `Commit {gen, root, ref prev_commit, live, superseded, mig}` object appended by `st_commit_m` (6-7 cells), its offset in a superblock cell (cells 9..14 are zero today, :64-69; the B1 card takes cell 9 for `anc`, so cell 10). ~30 lines. Compaction keeps the chain only as far as the type-level `prev` policy says (else the chain pins every generation forever -- the file-growth cost below).
2. **Named heads / branches.** One `root` cell = one branch. A root table object {n, (name digest, ref commit) x n} makes `root` a table of heads (B5 already plans `PartTab` as a root table, B5:21-22; the same shape). ~40 lines. Merge across branches is NOT in scope: it is the CRDT problem (Automerge, LANG-DB §1 [w23] "out of scope for a single-writer store").
3. **`st_open_at(gen)` / `st_snapshot_at(commit)`.** Today a reader can only take the live superblock (`st_snapshot`, store.bp:209-226). With the chain, a reader walks `prev_commit` to the wanted generation. ~20 lines.
4. **Reader registration** -- deliberately absent and NOT needed: LMDB needs its reader table because it REUSES pages (LANG-DB:47, :206); this store never reuses, and compaction-by-rename lets the kernel do the accounting. The proposal's "GC runs only when every reader has left an old version" is therefore already true, by inode refcount, for FILE-level compaction. It would be needed only for in-file space reuse, which the design rejects on purpose (LANG-DB:355-358 "reintroducing a second commit mechanism ... two truths").

### 8.3 What it costs, already measured

- The file grows by every update until compaction: logical size 85.2 MB after 10^5 updates of 1M records vs sqlite 34.1 MB (2.5x), 72.4 MB after compaction (2.1x); compaction 747 ms vs VACUUM 544 ms (ROADMAP.md:181-182); the edge log's 100 L0 rebuilds + 5 compactions cost 30 us/edge with a 747 ms max stall (ROADMAP.md:190), which B4 exists to cut to <= 0.5 us / <= 10 ms (B4:7). A commit chain that pins history makes every one of these numbers worse in proportion to the retained depth -- so the chain must be a per-type policy (as §4d says) and compaction must be allowed to cut it.
- Prior art fate: Noms (Attic Labs) is dormant since 2018 (https://github.com/attic-labs/noms); Dolt, its fork, is alive as versioned SQL on prolly trees with a Git-style commit graph (https://docs.dolthub.com/architecture/storage-engine) -- and pays "(1+k/w) log_k(n)" 4 KB chunks rewritten per edit (LANG-DB [w21]); Irmin (https://github.com/mirage/irmin) started as irmin-git for Tezos in 2017 and moved to irmin-lmdb, then leveldb, then its own irmin-pack -- the Git-SHAPED store had to stop being Git-IMPLEMENTED to perform; Datomic is the single-transactor, immutable-segment form and the one LANG-DB already copies (:49). Aspen (PLDI'19, LANG-DB:585) is the purely-functional GRAPH store: snapshots by construction, traversal slower than packed CSR -- the trade B4's L1 blocks make.

### 8.4 Verdict: ALREADY; three small gaps go into B4 step 3

- ALREADY: store.bp + mvcc semantics + Cheney compaction + B4's `prev` are the proposal.
- Add to B4 step 3 (the `prev` + time-travel step, B4:61): the Commit object chain (cell 10), the heads table, `st_open_at(gen)`; gate: phase `h` "fold as of gen g" (B4:61, :66) must succeed for g older than g-1 without any type declaring `prev`, and the size-after-compaction row must be reported with the retained depth (B4:87, :97 "prev depth").
- REFUTE inside the proposal: "MVCC wrecks graph performance" (not this MVCC), "reader registration" (not needed), and "immutability is free" (it costs 2.1-2.5x file size and a compaction pass, measured).
- D0: Git's actual purpose is DISTRIBUTED merge. On the mesh, per-node branches + merge is the local-first sync problem (MANIFESTO C4, C11 store-and-forward) and it is CRDT/event-log work (C3: "state = fold(events)") that lives above the store, not in it. The store's contribution is that every generation is an immutable, position-independent, crc'd image -- which is what a sync layer wants to ship. Zero-dep: none.

---

## 9. Verdict table and what each costs against D0 / zero dependencies

| # | proposal | verdict | what exists (file:line) | what to build | ROADMAP row | gate (the number) | cost vs D0 / zero-dep |
|---|---|---|---|---|---|---|---|
| 1 | BYOK via WASM/eBPF | ALREADY (bebop as bytecode); NARROW-ADOPT checked kernel dialect; REFUTE the WASM/eBPF runtime | gb_run.bp:379-401 in-process compile, :252-271 fork+`sys_run`, :78-94 digest/ABI check; bebop.bp:6826-6878 | bounds-checked, `sys_*`-free `kernel fn` dialect; parent drops RW mappings before fork | A8 (types) + B3 defect row | 10^4 hostile kernels -> 0 TRAP-82; K6 with checks <= 1.2x | runtime/verifier = a safety dependency 0.4-4x the compiler's size; the dialect adds none |
| 2 | ECS | NARROW: columns per type, not "no records" | gb.bp:8-10 SoA arrays; LANG-DB:220 free field append | `column` layout kind; AoS twin of nnidx.bp | A8 (G7 size) | 1M 3-field records <= 1.2x sqlite (<= 41 MB); PK lookup <= 2x today's 450 ns | none |
| 3 | race three plans | REFUTE | B7:15,:34 exact cardinalities; B2:23 Zipf twin | nothing; footnote: per-chunk flavour pick (Vectorwise) | -- | opens only if B2 Zipf shows the planner > 2x off | 3x bandwidth + 3 forks + 300 ms JIT per query; energy vs reliability-over-latency |
| 4 | abolish the WAL for PMEM | REFUTE (false premise: no WAL; recovery = verify scope) | store.bp:103-139; B1:117-188 fix card | nothing now; `persist` builtin at 2 call sites when a DAX box exists | B1 follow-up card (already written) | B1 card acceptance 1-3 | hardware absent on every node; Optane cancelled 2022 |
| 5 | reactive materialisation | NARROW: declared views over B4's delta with a measured crossover | RESULT.md 41x; RESULT-incr.md 0.39 %; LANG-DB:392 | `view` objects + incremental reduce/join kernels + stale flag | B4 step 3 -> B8 | <= 0.5 us/event maintenance; crossover c written; folds == oracle | a background propagator is a process (cap 32); stale reads vs reliability |
| 6 | mid-loop OSR | NARROW: chunk-boundary tier/flavour switch | gb_run.bp:241-251 fork-only run; swpmu.bp:1-4 no PMU; B3 tiers; Beamer switch | intra-query pool-hit switch between row-range chunks | B3 step 4 + B6 | wall <= 0.8 x min(tier-0 alone, compile+specialised) on a > 95 ms query | none |
| 7 | memory as Git | ALREADY; 3 gaps into B4 step 3 | store.bp:171-178, :197-203, :355-416; mvcc.bp:48-65; LANG-DB:200-206 | Commit chain (cell 10), heads table, `st_open_at` (~90 lines) | B4 step 3 | as-of fold for g < g-1 without type `prev`; size row with depth | merge = CRDT above the store; file growth 2.1-2.5x measured |

Order of the two real additions: the checked kernel dialect (row 1) BEFORE any peer-supplied or generated kernel runs against a writer's mapping -- it is a correctness/safety row, and B3's open defect (a stale-ABI kernel dispatched, TRAP-82) is its first test case; the column layout (row 2) with A8, where its gate already lives.

---

## 10. Prior-art matrix

| proposal | system | what it did | what happened | what Bebop takes | URL |
|---|---|---|---|---|---|
| 1 | SingleStore Code Engine | Wasm UDF/UDAF/TVF under wasmtime, in-process 16 MB sandboxes | shipped, INSIDE SQL; the query language stayed | the sandbox is for untrusted code inside a trusted engine -- a different trust topology | https://docs.singlestore.com/cloud/reference/code-engine-powered-by-wasm/ |
| 1 | ScyllaDB Wasm UDFs | wasmtime-based UDFs in CQL | experimental; renamed xwasm -> wasm in 5.4 | same | https://www.scylladb.com/2022/04/14/wasmtime/ ; https://docs.scylladb.com/manual/stable/cql/wasm.html |
| 1 | XRP (OSDI'22) | eBPF storage functions at the NVMe driver hook; BPF-KV, WiredTiger | research + one integration; bypasses the kernel storage stack, which an mmap store does not pay | nothing: the cost it removes is absent here | https://www.usenix.org/system/files/osdi22-zhong_1.pdf |
| 1 | wasm3 | smallest fast WASM interpreter: 64 KB flash, 10 KB RAM | alive; embedded targets | the size floor of a WASM sandbox: 40 % of bebop.bin, interpreting | https://github.com/wasm3/wasm3 ; https://arxiv.org/pdf/2205.01183 |
| 1 | Linux BPF verifier | static verification of bytecode, ~30,000 lines | needs CAP_BPF; unreachable from an unprivileged Android app | the size floor of a verifier: 4x bebop.bp | https://kernel-internals.org/bpf/bpf-verifier/ ; https://docs.kernel.org/bpf/verifier.html ; https://pchaigno.github.io/ebpf/2019/07/02/bpf-verifier-complexity.html |
| 2 | Flecs / archetype ECS | entities with the same component set in one columnar table | archetypes re-invent tables to fix per-component fragmentation | columns per type; AoS for entities | https://ajmmertens.medium.com/building-an-ecs-storage-in-pictures-642b8bfd6e04 ; https://www.flecs.dev/ecs-faq/ |
| 2 | ECS storage studies | archetype vs sparse-set = scan locality vs structural-change cost | the AoS/SoA trade under another name | the G7 size row decides | https://scholar.afit.edu/cgi/viewcontent.cgi?article=6355&context=etd ; https://arxiv.org/pdf/2606.14919 |
| 3 | Rdb/VMS (Antoshenkov 1993) | competing retrieval strategies for one operator, losers dropped | product feature of Oracle Rdb; per-operator, I/O-bound era | the idea is 33 years old and was never whole-plan | https://www.semanticscholar.org/paper/Query-processing-and-optimization-in-Oracle-Rdb-Antoshenkov-Ziauddin/2a5ef508107647f5755a900bb1ad289709f0e3c2 |
| 3 | Kabra & DeWitt 1998 | mid-query re-optimisation at materialisation points | the sequential form; widely copied | re-plan between chunks, never in parallel | https://dl.acm.org/doi/10.1145/276304.276315 |
| 3, 6 | Vectorwise micro-adaptivity | per-call choice among compiled primitive flavours by an epsilon-greedy bandit on measured cost | shipped in Vectorwise; consistent TPC-H gains | the cheap runtime switch: per chunk, sequential, measured | https://15721.courses.cs.cmu.edu/spring2018/papers/03-compilation/p1231-raducanu.pdf |
| 3 | SkinnerDB (TODS 2021) | join order re-chosen per time slice by a regret-bounded learner | research; the strongest "no CBO" form, still sequential | nothing while cardinalities are exact | (survey) https://dl.acm.org/doi/10.1145/2854006.2854012 |
| 4 | Intel Optane / PMDK | byte-addressable NVM DIMMs; libpmemobj with undo/redo logs replayed at open | wound down July 2022 ($559 M write-off); PMDK maintenance-only; last shipments late 2025 | nothing; the store's format is already DAX-shaped | https://www.theregister.com/on-prem/2022/07/29/why-intel-killed-its-optane-memory-business/ ; https://www.datacenterdynamics.com/en/news/intel-kills-off-optane-memory-writes-off-559-million-inventory/ ; https://pmem.io/blog/2022/11/update-on-pmdk-and-our-long-term-support-strategy/ ; https://link.springer.com/chapter/10.1007/978-1-4842-4932-1_7 |
| 4 | ADR / eADR | ADR does not flush CPU caches; eADR does, SFENCE still required | ordering discipline survives the hardware | `DC CVAP` + `DSB` = the AArch64 form; two call sites | https://software.intel.com/content/www/us/en/develop/articles/eadr-new-opportunities-for-persistent-memory-applications.html |
| 4 | Agamotto / Witcher | crash-consistency bug hunting in PMEM software; Witcher: 49 new bugs incl. 6 in PMDK | "recovery = 0" is where the bugs live | keep crc per object and verify-last-commit | https://www.usenix.org/system/files/osdi20-neal.pdf ; https://www3.cs.stonybrook.edu/~dongyoon/papers/SOSP-21-Witcher.pdf |
| 4 | Failure-atomic msync / Snapshot | make msync the atomic commit primitive (disk 2013; CXL/PM 2023) | the field kept msync and made it atomic | the direction the store already takes | https://web.eecs.umich.edu/~tpkelly/papers/Failure_atomic_msync_EuroSys_2013.pdf ; https://arxiv.org/pdf/2310.16300 |
| 5 | Eve | everything reactive, Datalog fixpoint per event | shut down Jan 2018 | LANG-DB §7: incremental only for sparse deltas | (LANG-DB §7 sources) https://witheve.com/deepdives/whateveis.html |
| 5 | Noria (OSDI'18) | dataflow materialised views with PARTIAL state and eviction | research; partial state needed because full state "would explode with many queries" | views are few, declared, and evictable to "stale" | https://www.usenix.org/conference/osdi18/presentation/gjengset |
| 5 | DBSP / Feldera | mechanical incrementalisation of any relational query over Z-sets; linear ops O(delta) | shipping (Feldera) | the cost model for which views are cheap | https://docs.feldera.com/vldb23.pdf |
| 5 | Materialize / differential dataflow | arrangements resident; work per update prop. to delta x matched slice | shipping; memory = all arrangements | why views must be declared, not implicit | https://materializedview.io/p/everything-to-know-incremental-view-maintenance |
| 6 | V8 OSR (Sparkplug/Maglev/TurboFan) | OSR at loop headers; frame translation needed when layouts differ; deopt 2-20x for that invocation | shipping; the frame map is the artifact | the map is what Bebop lacks; chunk boundaries remove the need | https://wingolog.org/archives/2011/06/20/on-stack-replacement-in-v8 ; https://v8.dev/blog/maglev ; https://arxiv.org/pdf/1708.02512 |
| 6 | HyPer/Umbra adaptive execution (ICDE'18 best paper) | interpret the IR while LLVM compiles, switch at morsel boundaries | shipped in Umbra; the DB-world OSR | B3's tiers made intra-query | https://dblp.org/rec/conf/icde/KohnL018.html ; https://www.hytradboi.com/2025/slides/leis-neumann-compilation.pdf ; https://db.in.tum.de/~schmidt/papers/dynamic-blocks.pdf |
| 7 | Noms -> Dolt | Git-style commit graph over prolly trees | Noms dormant since 2018; Dolt alive as versioned SQL | the commit-chain shape; not per-record hashing | https://github.com/attic-labs/noms ; https://docs.dolthub.com/architecture/storage-engine |
| 7 | Irmin (Tezos) | Git-principled branchable store | moved irmin-git -> lmdb -> leveldb -> irmin-pack for performance | Git-shaped, not Git-implemented | https://github.com/mirage/irmin |
| 7 | Datomic; LMDB; Aspen | immutable segments + one transactor; CoW + two meta pages + reader table only because pages are reused; purely functional graph snapshots | all alive; already the store's cited ancestors | already copied (LANG-DB §1) | https://docs.datomic.com/indexes/index-model.html ; https://raw.githubusercontent.com/LMDB/lmdb/mdb.master/libraries/liblmdb/lmdb.h ; https://arxiv.org/abs/1904.08380 |

---

## 11. What could not be verified

- Whether a forked kernel child can actually write through an inherited RW MAP_SHARED store mapping was reasoned from the clone flags (gb_run.bp:184-186: flags 17, no CLONE_VM) and store.bp:146 (prot 3, flags 1); no probe was run. The B3 gate's parent maps read-only (gb_pool.bp:229), so the gate would not show it.
- `perf_event_open` blocking: swpmu.bp:1-4 records EACCES; `/proc/sys/kernel/perf_event_paranoid` reads -1 under proot today (proot may present a synthetic value). Not re-tested.
- The exact granularity of HyPer's adaptive switch (morsel boundaries) is from memory of the ICDE'18 paper (M); the dblp record and the authors' 2025 slides were fetched, the PDF was not.
- The BPF verifier's "~30,000 lines" is the kernel-internals.org figure for a recent kernel, not a count run here.
- No number in this document was produced by running a compiler, gate or benchmark today.

VERDICT: 0 ADOPT-as-stated; ALREADY 1 (bebop is the bytecode: gb_run.bp + gb_compile1.bp + gen_gb.bp), 7 (store.bp + mvcc + Cheney + B4 `prev`; 3 gaps ~90 lines into B4 step 3) and the SoA half of 2; NARROW 2 (columns per type, gate = G7 size <= 1.2x sqlite under A8), 5 (declared views over B4's delta, gate <= 0.5 us/event + measured crossover), 6 (chunk-boundary tier switch under B3/B6); REFUTE 3 (exact cardinalities leave nothing to race; 3x bandwidth on a one-core-saturates-DRAM box) and 4 (no WAL exists; 44.7 ms is `st_verify`'s scope, fixed by the B1 card; PMEM absent, cancelled, and not recovery-free). The one addition that is a safety row, not a speed row: the checked kernel dialect -- bounds checks + no `sys_*` + a step budget -- which gives WASM-class memory safety at native speed with zero dependencies and makes B3's open TRAP-82 defect a loud trap.

---

## Main-session verification of this document's three flagged-unverified claims (2026-09-08)

The research agent marked three claims as reasoned-but-not-probed. Two are now confirmed and one
is **contradicted**, which changes what proposal 6 is allowed to assume.

**1. The fork-inherits-a-writable-mapping hazard: CONFIRMED, and it is real.**

- `selfhost/std/gb_run.bp:246` dispatches through `sys_clone(17, ...)`. Flag 17 is `SIGCHLD`
  alone — no `CLONE_VM` (0x100) — so this is a plain fork.
- A fork keeps `MAP_SHARED` mappings shared. Private pages go copy-on-write; shared ones do not.
- `selfhost/prelude/store.bp:146` is `sys_mmap(0, size, 3, 1, fd, 0)`: `prot = 3`
  (`PROT_READ|PROT_WRITE`), `flags = 1` (`MAP_SHARED`). Every writer that has called `st_open`
  therefore holds a writable shared mapping of the store file.
- The B3 gate is safe only because it maps read-only for the dispatch:
  `bench/vs_rust/std_tests/gb_pool.bp:229` uses `st_map_ro` (`store.bp:155`, `prot = 1`).

So a dispatched kernel forked from a process that holds `st_open`'s mapping can write the store
file, and nothing in the current design stops it. "Physical isolation" holds for the gate's
shape and not for the writer's. That makes the checked-kernel-dialect row a SAFETY row, not a
performance one, and it should say so.

**2. `perf_event_paranoid`: CONTRADICTED. It reads `-1` on this box today**, not the EACCES
`selfhost/std/swpmu.bp:1-4` recorded. `-1` is the most permissive setting, so `perf_event_open`
is not blocked by the paranoid level any more (whether the proot seccomp filter lets the syscall
through is a separate question and was NOT tested). Proposal 6's cache-miss trigger may therefore
be measurable after all — but swpmu.bp's comment is stale and must not be cited as the reason it
cannot be. Re-probe before either building on it or ruling it out.

**3. The BPF verifier line count and HyPer's switch granularity** stay as the document has them:
an external figure and a recollection, both marked as such in the text. Neither is load-bearing
for a verdict.
