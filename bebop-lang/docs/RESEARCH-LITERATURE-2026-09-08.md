Status: 2026-09-08 -- read-only literature analysis by the session's analyst agent (Fable) for /root/dowiz/bebop-lang at today's tree (bebop.bp 6909 lines, bebop.bin 159,196 bytes, A2b closed, A3 and A12 refuted, B2 measured). Corpus: `/root/.cache/bebop/research/shortlist.jsonl`, 2,000 records = the top of 5,909 phrase-matched records out of 85,663 unique arXiv records (cs.DS, cs.PL, cs.DC, cs.AR, cs.DB, all years), plus ~40 keyword sweeps over the full 85,663 (`arxiv.jsonl`, grepped, never loaded whole). **What is wrong with the corpus:** it is arXiv ONLY; the OpenAlex crawl that was meant to supply a citation graph was discarded (its topic ids were never resolved and it turned out to be an ML/AI corpus), so **there is no citation signal in the ranking** -- it is phrase match, breadth across roadmap rows, venue/code availability and recency, and nothing here may be called "highly cited". Papers that were never on arXiv (Crotty et al. CIDR'22 on mmap in DBMSs, the HyPer/Umbra papers, Beamer's DOBFS, Salsa) are absent by construction and are named below only as absences. **No paper was read in full**: every number attributed to a paper is from its abstract and is marked "(abstract)". Every repo claim carries file:line and was verified by grep today; no compiler, gate, benchmark or `git` command was run. This is a PROPOSAL pending operator decisions.

# What the literature changes, and what it does not: bebop's open rows against 85,663 arXiv records

## 0. Executive summary (10 lines)

1. **RULED OUT as gate-movers: more one-pass codegen work.** Three independent single-pass back-ends -- TPDE (2505.22610), copy-and-patch (2011.13127), Deegen's baseline JIT (2411.11469) -- and the survey "Whose baseline compiler is it anyway?" (2305.13241) all land, by their own abstracts, at LLVM -O0-class run time (copy-and-patch "14 % faster than LLVM -O0"; Deegen's baseline "33 % slower than LuaJIT's optimizing JIT"). bebop's honest rows (K8H 1.1x, K1H 1.8x, the join's 1.37x at equal algorithm) are already inside that band. The next 2x is not in the emitter; A2b-class peepholes should be closed as a lane, not extended.
2. **RULED OUT: any optimisation whose input is an IR** -- 598 of the 2,000 shortlisted records match "A2b/codegen" and essentially all of them (register allocation, instruction scheduling, superoptimisers, LLM peephole discovery, verified JITs) assume SSA/CFG. None can be approximated in one text-directed pass better than A12's measured 0-2 %. Lazy basic-block versioning (1411.0352) is the one genuinely one-pass optimisation family in the corpus and it optimises dynamic type tests, which bebop does not have.
3. **RULED OUT: WASM/eBPF/SFI runtimes**, again -- but the corpus adds a number the review lacked: WASM runs 45-55 % slower than native across SPEC, peak 2.5x (1901.09056, abstract). The corpus also adds a shape that helps C1: Lightweight Fault Isolation, a machine-code SUBSET with a small verifier, formally verified (2508.15898). After A5 every data access is `ldr/str [x17, xt, lsl #3]`; one `and` on the index makes the emitted code that subset by construction (section 6).
4. **The decision-changing finding is about B2's 96 %, and it touches A5, B4 and C6.** 18,051 ms in the store's file-backed page path against 747 ms on plain arrays is 1.8 us per `st_put` against 75 ns -- 24x -- while first-touch faults over the 80 MB `ci` object cost at most ~70 ms at the box's measured 3.5 us/CoW-fault. So the cost is per STORE, not per page: the mmap-for-storage literature (2409.10946, 2112.14013, 2310.16300, 2306.05701) names three candidate mechanisms and a three-run experiment separates them (section 2). Until that runs, **A5's "arena image = file" must not be promoted**: if the penalty is a MAP_SHARED property, a file-backed arena would put every `zeros` scatter in every program on the 24x path.
5. **B2's join gap is locality, not codegen or width.** PB-SpGEMM (2002.11302) and MAGNUS (2501.07056) say Gustavson accumulators have poor cache reuse on large matrices and that blocking the intermediate product is worth 20-50 % to an order of magnitude (abstracts); the plain-array CSR build's 747 ms is ~22x above its 12 GB/s bandwidth floor (~33 ms) for the same reason (1008.2849: counting sort with write-combining reaches 88 % of peak bandwidth, abstract). The lever B7's templates need is propagation blocking, and it is an algorithm, not a compiler pass.
6. **A10 is neither confirmed nor refuted by the corpus, but its correctness argument has a name**: forward build systems (2202.05328, LaForge 2108.12469) define correctness as "identical to running the commands in order" -- exactly A10's memo-on == memo-off md5 gate -- and get it by TRACING what each step read rather than enumerating dependencies by hand, which A10's blueprint does with a trailing "...". Section 5 gives the cheap pre-measurement that decides whether the 0.3 s gate is reachable at all.
7. **C4 can be pre-screened statically before any measurement**: the IVM dichotomy over semirings without additive inverse (2606.07795, abstract) -- constant amortised update time and constant-delay enumeration iff the query is alpha-acyclic p-hierarchical -- says which declared views can be cheap in principle. A view over a non-hierarchical join is refuted on paper; only hierarchical ones earn the k/N measurement.
8. **B3's 1.04 ms fork floor is page-table copy** (Async-fork 2301.05861, abstract: copying the page table dominates fork) and it scales with the parent's populated mappings, so it gets WORSE as stores grow; a pre-forked resident runner (one process per store, futex-dispatched) is the shape that reaches the 0.1 ms pool-hit gate. ShareJIT (1810.09555, Android/ART) states the tension the stale-`.gbpool` defect is an instance of: specialisation to context versus shareability of cached code.
9. **B4 has no size gate and needs one**: dynamic graph structures cost 3.3-10.8x (Aspen) and 4.1-8.9x (fine-grained methods) the memory of static CSR (2502.10959, abstract), and the G7 size row already loses 2.5x. The pointer-free alternative to CoW row blocks is the packed memory array (CPMA 2305.05055: "without pointers", 3x batch-insert and 4x range-query over PaC-trees, abstract), which fits "the file is the memory image" better than a blocktab.
10. **Net: 3 findings that change a decision** (the page-path experiment gating A5; blocking as the B2/B7 lever with A8's width demoted; the masked-index SFI for C1 without waiting for A8), **2 gates to add** (B4 bytes/edge; C4 static pre-screen), **1 gate to sharpen** (A2 csel/K6 by selectivity sweep, from 2606.22423), and everything else in the corpus is confirmation or inapplicable.

---

## 1. Method, and the shape of the corpus

The 2,000 shortlisted records split by matched row: A2b/codegen 598, C1 safety 536, B3/B7 sparse kernels 305, graph engines 283, B1 durability 250, C2 storage layout 152, B3/C5 adaptive 142, C3/C4 versioning 56, A5-A7 memory model 53, self-hosting 17; 1,626 matched one row only. By year, 1,006 of 2,000 are 2025-2026 (recency was weighted). The A2b and C1 buckets are heavily polluted by LLM-code-generation and agent papers that matched "code generation"/"verifier"/"sandbox"; those were skipped on title.

Because the shortlist is phrase-ranked, the useful work was the keyword sweeps over the full 85,663: mmap/page-fault/TLB, incremental compilation and build systems, bounds checking and SFI, single-pass and baseline compilers, joins and SpGEMM, ARM/Android/big.LITTLE, crash consistency and msync, append-only/LSM/compaction, MVCC and multiversion GC, IVM, query compilation and adaptive execution, dynamic graph storage, radix/counting sort and write-combining, fork cost, self-hosting and trusting trust. About 130 abstracts were read in full; the rest were judged on title and venue.

Reading discipline applied throughout: a paper's RESULT is quoted from its abstract; its APPLICABILITY is judged against the four constraints (zero dependencies incl. no second code generator; one pass, no IR; i64 cells/arm64/Linux syscalls/512 fns; a phone under proot with one heavy slot and a 32-process cap) and against the two refuted rows (A3: the applicable set was empty; A12: 0-2 % against >= 5 %). Where a paper assumes LLVM, SSA, x86 servers, GPUs or a runtime, it is listed as inapplicable and not stretched.

---

## 2. B2 (iii) -> A5/A6, B4, C6: the store's page path is a per-store cost, and the literature says which three things it could be

### 2.1 The measurement, re-read

`bench/vs_rust/std_tests/csr_build_profile.bp:71-110` fills `ci` with `st_put(base, ci, 1 + s, dst[j])` inside the 10M-slot fill loop; `st_put` is a plain cell store into the mapping (`selfhost/prelude/store.bp:270-273`); the mapping is `sys_mmap(0, size, 3, 1, fd, 0)` = PROT_READ|WRITE, MAP_SHARED over a freshly `sys_ftruncate`d 512 MB f2fs file (`store.bp:145-146`). The identical fold on `zeros()` arrays is the 747 ms arm.

Arithmetic the profile row does not state: 18,051 ms / 10M stores = **1.8 us per `st_put`** against 75 ns on plain memory. The `ci` object is 80 MB = ~19,500 pages; at the box's measured 3.5 us per CoW fault (ROADMAP Measured table) first touch of every page costs ~70 ms, and even at ten times that it does not reach 18 s. **So the penalty is proportional to stores, not to pages**, and "file-backed page path" has to mean something that happens again and again on the same page.

### 2.2 What the corpus offers as mechanisms

Three mechanisms, each from a paper that measured it on Linux (all abstracts):

- **Dirty-page write-protect cycles.** A MAP_SHARED file page is write-protected whenever the kernel cleans it, so the filesystem's `page_mkwrite` runs on the NEXT write; if background writeback (dirty ratio / expiry, and Android tunes these low) keeps cleaning pages of an 80 MB working set during a 20-40 s run, a random scatter hits clean pages continuously and every such store is a minor fault. 2112.14013 (TACO) measures minor faults at "a few 1000's of CPU cycles" each and up to 29 % of run time for lazy-allocation faults alone; 2306.05701 (CAWL) models exactly this interaction -- page cache, background writing, I/O throttling -- and reports naive I/O models are off by 67 % on random writes. 1.8 us per store is the right order for one minor fault plus f2fs bookkeeping.
- **TLB shootdowns from mmap/page-cache eviction cycles.** 2409.10946 shows TLB shootdowns triggered by page-cache eviction and re-mapping of "recycled" pages are "a significant source of bottlenecks, previously misattributed to other components of the Linux kernel" (up to 92 % in micro-benchmarks). Single-core pinning makes this less likely here, but the box swaps to zram at idle (BOX.md), so eviction pressure is real.
- **Page-level dirty tracking amplification** is what Snapshot (2310.16300) exists to remove: FAMS/msync "suffers from the overhead of msync() and the write amplification from page-level dirty data tracking", and its fix is to keep the working copy in DRAM and sync deltas at msync -- 1.2x over PMDK on Optane, 8x on Kyoto Cabinet's msync-based commits (abstract). That is a build-then-publish discipline, not a kernel change.

The canonical paper on this problem (Crotty, Leis, Pavlo, "Are You Sure You Want to Use MMAP in Your DBMS?", CIDR 2022) is not on arXiv and is therefore not in the corpus; it is named here only so nobody concludes the corpus is silent because the question is open.

### 2.3 The experiment that separates them (three runs, one integer each)

All three change one argument of the existing profile driver; `st_compact` already maps with `flags = 2` (MAP_PRIVATE) at `store.bp:357`, so the flag is already in the builtin's vocabulary.

| run | change | if the 18 s vanishes |
|---|---|---|
| R1 | store arm mapped MAP_PRIVATE (flags 1 -> 2) on the same f2fs file | it is the shared-dirty-page / writeback path (mechanism 1 or 3); an anonymous working copy + sequential publish is the fix |
| R2 | store arm mapped MAP_SHARED on a memfd/tmpfs file instead of f2fs | it is f2fs's `page_mkwrite` cost specifically (block reservation, inode locking) |
| R3 | store arm with the whole `ci` range pre-touched sequentially once before the fill (the bebop equivalent of MADV_POPULATE_WRITE; there is no `sys_madvise` builtin -- `grep -c madvise bebop.bp` = 0) | it is first-touch after all, and the 3.5 us/fault number is wrong for this mapping |

Gate for the row this creates (B2 (iii) follow-up, or the first step of B4): **store-arm CSR build <= plain-arm + 100 ms at n = 1M / m = 10M** (today +17,300 ms). Cost against the constraints: zero words in the compiler for R1-R3; the eventual fix is a store-library rule ("build in the arena, publish by one sequential copy"), ~30 lines in `store.bp`/`sgraph2.bp`.

### 2.4 Why this gates A5

The A5 row (`ROADMAP.md:88`) ends "arena image = file", and the blueprint's design (`docs/blueprints/A5-arena-relative-addressing.md §3` "Address space") places file maps INSIDE the reserve so that store bases are indices. If R1 shows the penalty is a MAP_SHARED property, then a file-backed arena would move every `zeros`-array scatter in every program -- the compiler's own tables included -- onto the 24x path, and A5's own gate (K5 <= +8 %) would be missed by an order of magnitude for a reason that has nothing to do with addressing. The dependency edge to add: **A5 step 0 includes R1-R3**, and "arena image = file" is re-specified as "arena is anonymous inside the reserve; file maps are separate slots" unless R1 exonerates MAP_SHARED. A6/A7 inherit the same rule (A7's `sys_mapb` maps files into the reserve -- reads only, so it is unaffected).

Also touched: **B4**'s risk row "L0 rebuild at 2^18 edges > 10 ms" (blueprint §8) -- at 1.8 us/store a 2^18-edge L0 rebuild into the store is ~0.5 s, so B4 step 1 cannot pass while the rebuild targets the mapping; and **C6** -- the 44.7 ms recovery row and this row are both "the mapping is slow to write", and neither is a persistence-hardware problem; C6 stays conditional.

---

## 3. B2 (i) -> A8, B3, B7: the join is latency-bound; blocking is the lever, width is second

### 3.1 Reading the 0.67x

The join twin reads 0.67x of the best Rust row against a >= 0.7x gate; at EQUAL layout and algorithm (`rust_once/join_csr.rs` at 64-bit indices) the gap is 1.37x. So the layout share is ~1.1x, not the ~2x that halving bytes buys a bandwidth-bound kernel. That is the tell: **the twin is not at the DRAM ceiling; it is bound by cache misses in the scatter/gather**, where narrower cells help only insofar as more of the accumulator fits in cache.

The plain-arm CSR build says the same thing: 10M pairs x (src 8 B + dst 8 B + `cur` read/write 16 B + `ci` write 8 B) is ~400 MB of traffic, ~33 ms at the box's 12 GB/s; the measured 747 ms is ~22x above that floor.

### 3.2 What the corpus says (abstracts)

- 2002.11302 PB-SpGEMM: "it is well known that SpGEMM is memory-bound ... yet existing algorithms fail to saturate the memory bandwidth"; outer-product formulation + **propagation blocking** + in-cache sorting/merging "saturates memory bandwidth", 20-50 % faster than heap/hash SpGEMM, "attains performance predicted by the Roofline model, and its performance remains stable with respect to matrix size and sparsity".
- 2501.07056 MAGNUS (ICS'25): "current multithreaded implementations are based on Gustavson's algorithm and often perform poorly on large matrices due to limited cache reuse by the accumulators"; reorders the intermediate product into cache-sized chunks, "often an order of magnitude faster than at least one baseline", "close to the optimal bound ... regardless of the matrix size".
- 1709.07122 PCPM (USENIX ATC'18): partition-centric processing for PageRank/SpMV "drastically reduces the amount of DRAM communication while achieving high sustained memory bandwidth", with "branch avoidance mechanisms to get rid of unpredictable data-dependent branches".
- 1008.2849: counting/radix sort using virtual memory and **write-combining** reaches "at least 88 % of the system's peak memory bandwidth" per pass; "sort-merge joins" named as the motivating operation. The CSR build IS a counting sort by `src`.
- 2312.14874: prefix sums partitioned into cache-sized chunks are up to 3x faster than SIMD+multithreaded library versions -- the prefix pass of the same build.
- 2108.10540: predefined joins through row IDs as pointers, which is what "index = CSR by that key" (B7 §3) is, so B2's choice of `rust_csr` as the adjudicating twin has literature standing.

### 3.3 What it changes

- **B7's Gustavson/SPA template gains a blocked form**: the intermediate product is binned into 2^k destination ranges sized to the A78's private cache, each bin accumulated in-cache, then written out. This is the propagation-blocking pattern, expressible in bebop as two loops and a bin table -- no compiler change, no dependency.
- **The B2 (i) gate should be re-run blocked-vs-blocked.** B2's rule ("do not report an algorithm win as a language win") is right; its consequence is that once the blocked algorithm is what B7 ships, the honest twin is `join_csr.rs` blocked the same way, and the >= 0.7x line is then a codegen question again, at which point the literature ceiling of section 4 applies.
- **A8's u32 gate is demoted from "K6 ns/row -2x" to a measured share**: expect ~1.1-1.3x on the join from width alone (from the 0.67/1.37 arithmetic), 2x only on streaming scans that are already at the ceiling. The G7 size gate (<= 1.2x sqlite) stays the real reason for A8.
- **B4's L0 rebuild budget** (10 ms at 2^18 edges) is reachable only with a blocked build: at today's 75 ns/pair the rebuild is ~20 ms on plain arrays before the store is involved.

Gate proposal, in ROADMAP style: **plain-arm CSR build <= 150 ms for 10M slots (5x from 747)** via binned counting sort, fold identical; **join twin, blocked bebop vs blocked `rust_csr`, >= 0.7x** on uniform and Zipf. Cheapest kill: bin the fill pass at 2^8 and 2^12 bins in `csr_build_plain.bp` (~40 lines); if neither beats the direct scatter by >= 2x at n = 1M, the scatter is not the cost and this section is wrong.

---

## 4. A2b and the register model: the corpus fixes the ceiling of one-pass codegen

Result (abstracts): TPDE compiles LLVM-IR "8-24x faster than LLVM -O0 while being on-par in terms of run-time performance" with one analysis pass plus one combined isel/RA/encoding pass; copy-and-patch's code "runs an order of magnitude faster than interpretation and 14 % faster than LLVM -O0"; Deegen's automatically generated baseline JIT is "only 33 % slower ... than LuaJIT's optimizing JIT"; 2305.13241 places six production Wasm baseline compilers in a two-dimensional compile-speed/run-time space and finds value-tag overheads reducible "to near zero". ROADMAP's D14 item 10 already cites wasmtime's 1.1-1.5x; the corpus agrees from four more directions.

Applicability: bebop is one pass with no IR, and its honest rows (K8H 1.1x, K4 1.6x, K1H 1.8x, join 1.37x at equal algorithm) sit inside the published band for that compiler shape. Two consequences:

- **Do not open another A2b-class row expecting a gate to move.** The remaining word-level ideas in A2b's own closing note (a `tst`-absorbing comparison fold) are worth <= 1 word per loop; 2502.20547 (a negative result on AoT inline-cache optimisation: "reducing the number of memory accesses ... does not shorten execution times on contemporary architectures") is the corpus's reminder that word counts are not time -- which A2b learned when k8h words fell 14 -> 9 and the gate was still a TIME.
- **The IR-assuming 90 % of the codegen bucket is inapplicable and stays so.** Register allocation (2011.05608's "future-active set" is the one allocator "not reliant on properties of SSA form", but it still needs a liveness model A12's probe showed is not worth building), instruction scheduling (1409.7628, 1804.02452), LLM-found peepholes for LLVM (2508.16125, 2603.18477), verified JITs (2212.03129) -- none can be approximated in the emitter's single pass better than the measured 0-2 %.

One inapplicable-but-instructive family: lazy basic-block versioning (1411.0352, 1401.3041, 1511.02956, and CRuby's YJIT lineage 2609.01502) is the only optimisation in the corpus that is genuinely one-pass -- it specialises blocks as it emits them, "without a separate type analysis pass". It targets dynamic type tests, which a statically typed i64 language does not execute. If bebop ever needs a one-pass specialisation mechanism (e.g. versioning a loop body on "bound is a constant"), this is the design to copy, not an IR.

---

## 5. A10 per-fn memo: not refuted, but the blueprint should borrow one idea and measure one floor first

What the corpus has: incremental compilers and build systems, not "does memoisation pay below N functions". 2002.06183 builds an incremental Stratego compiler by reusing the non-incremental one and wiring its stages through an internal incremental build system; its lesson is that "dependency tracking, caching, cache invalidation, and change detection" are the cross-cutting, error-prone part and that "retrofitting incrementality into a compiler is even harder". LaForge (2108.12469) and "Forward Build Systems, Formally" (2202.05328) go further: correctness of an incremental build is DEFINED as behaving identically to running the full commands in order, and the sound way to get it is to TRACE what each step read rather than to declare dependencies.

Applicability to A10, which is exactly a retrofit:

- **A10's gate is the right one**: "memo hit and miss produce byte-identical output: gen3 == gen4 with the memo on AND off" is Rattle's correctness criterion word for word.
- **A10's dependency list is hand-enumerated and admits it** (blueprint §1: text, callee arities, ctor tags, literal ordinals, "...the caller's text did not change!"). The forward-build answer is to have the emitter RECORD what a fn's emission read -- which `fntab` cells, which callee entries, which literal ordinals -- into the reloc/trace zone the memo already needs for `bl` re-linking, and to key the memo on a hash of that trace. A missed dependency then fails closed (a miss), not open (a stale word), and the md5 gate stops being the only line of defence. Cost: ~50 lines in the reloc recording the blueprint already plans; zero runtime words.
- **The floor decides reachability, and it is measurable today without building anything.** The gate is 0.3 s against 2.15 s. Known pieces: trivial-program floor 106 ms and whole-output cache hit 70 ms (ROADMAP Measured; T108). The blueprint estimates hashing + memo read + relink + literal/stub/write at 0.05-0.15 s. What is NOT known is the distribution of per-fn emission time: if the 250 fns are ~6 ms each the arithmetic works; if `emit_call`/`compile_fn_at`-class fns are 50-100 ms each, a one-fn edit to one of THOSE is 100+ ms of emission plus the floor, and the gate is met only for small fns. **Pre-measurement (one scratch build, zero commits): `clock_ms` around each fn's emission in `compile_program_offs`, print the top 10.** If floor + max(fn) > 0.3 s, the gate should be restated as a median over the fns actually edited in the last 30 journal entries, before A10 is built.

Nothing in the corpus says per-function memoisation does not pay at 250 functions; the closest negative is Chordata's "13x speedup with 20x memory overhead" (2603.19560, abstract), which is irrelevant at A10's ~1 MB memo. ShareJIT (1810.09555) is about B3, not A10, and is used there.

---

## 6. C1 checked kernel dialect: A5's addressing form is an SFI subset, and one word buys containment before A8 exists

### 6.1 The corpus on the cost of checks (abstracts)

- 1901.09056: WebAssembly vs native across SPEC CPU, 45 % (Firefox) to 55 % (Chrome) slower, peak 2.5x; "some ... inherent to the WebAssembly platform" (bounds-checked linear memory is one of them). This is the number that was missing from the review's "an interpreter is an order slower" line: even JIT'd, the sandbox model costs tens of percent.
- 1907.04241 CHOP: "80.12 % of dynamic bounds check instructions can be avoided" by profile-guided inference over SoftBound-class checks; 2403.02416 finds "69.8 % of the access patterns consist of uncomplicated traversals" across 3.8 billion JVM array accesses. Both say the same thing for C1's gate (K6 <= 1.2x with checks): **the per-element check is the wrong form; the hoisted check is the right one.** For a CSR loop `while i < rp[k+1] { ... ci[i] ... }` the check `rp[k+1] <= len(ci)` once per row subsumes every element check, and the emitter already has the machinery to see a loop-invariant bound (hoist_scan, A2). That is the difference between meeting 1.2x and not.
- 2508.15898 (FMCAD'25): Lightweight Fault Isolation -- untrusted code must be "written in a subset of the machine language that guarantees it never reads or writes outside of a region", checked by a small verifier that is itself formally verified. Cage (2408.11456) does the same with Arm MTE/PAC at < 5.8 % overhead, but needs MTE, which this proot cannot enable.

### 6.2 The mechanism that falls out of A5

After A5 every array access is `add xt, x<base>, x<idx> ; ldr xd, [x17, xt, lsl #3]` (A5 §3) and the reserve is 2^29 cells. Under the `kernel fn` dialect bit, emit **one more word**: `and xt, xt, #0x1fffffff` (a valid logical immediate) before the load/store. The emitted code is then, by construction, the LFI subset: no data access can leave the reserve, no `svc` exists (C1 step 2 rejects `sys_*` at parse), and the store mapping is read-only in the child after C1's step-1 fence. That is containment of WRITES (the confirmed hazard, `gb_run.bp:246` + `store.bp:146`) and of out-of-process reads, at 1 word per access instead of 2, **with no dependency on A8's typed lengths**. What it does NOT give: per-object bounds (a masked index can still read any cell of the reserve -- the parent's arena copy and the store -- so a hostile kernel's confidentiality against its own dispatcher is unchanged, and a masked index into a PROT_NONE page still SIGSEGVs, i.e. a loud trap-82 rather than silence).

The verifier the LFI paper wants is then tiny here: a word scanner that accepts only `[x17, xt, lsl #3]`-form loads/stores each preceded by the mask, and rejects `svc`/`blr`/unmasked forms. `tools/check_abi.py` already decodes load/store register fields from emitted words (:98-112) and builds the syscall allowlist in `sys_allow` (:120); the same shape, ~100 lines, gives an independent check of the emitter's output -- the diverse-double-compiling flavour of assurance (section 14) applied to kernels. Zero dependencies; it never runs in the binary.

### 6.3 Gate restatement

C1's gate as written ("10^4 hostile kernels -> 0 SIGSEGV/TRAP-82 and 100 % loud traps") mixes two properties. Proposed split: **(a) containment** -- 10^4 hostile kernels, 0 writes reach the store file (compare crc of the file before/after) and 0 accesses outside the reserve (the verifier accepts every emitted kernel); traps allowed -- reachable after A5 + C1 steps 1-2 + the mask; **(b) bounds** -- 100 % loud traps at the object boundary -- after A8, with the HOISTED check form, gate K6 <= 1.2x (per-row, not per-element). Cheapest kill of (b), as the blueprint §7 already says: hand-instrument the sgraph2 frontier inner loop; do it in BOTH forms (per-element and hoisted) so that the 1.2x line is compared against the form that would ship.

Cost against the constraints: +1 word per checked access (mask) and +2 per checked loop (hoisted bound), all under a dialect bit, so the self-compiler's census is unchanged; ~100-line dev-time verifier.

---

## 7. B1 durability and its follow-up card: the corpus confirms the design and removes one knob

- **Group commit needs no timer.** 2606.18187 (abstract): in closed-loop OLTP the parameter-free greedy-pipelined policy ("flush the instant the device is free") is within ~0.1 % of the oracle-tuned timer at every load; the square-root timer collapses onto greedy above a device-set threshold; PostgreSQL `commit_delay=0` is competitive with any tuned value. `st_commit_batch(N)` therefore should not grow a timer or an adaptive batch size; "msync when the previous msync returned" is the policy, and the sbench rows batch10/batch100 already measure it.
- **The syscall return is not the commit boundary; the harness is.** 2603.01384 argues, across ext4 journaling, fsync failure semantics and NVMe flush behaviour, that "no syscall-based persistence primitive can define a commit boundary under failure". With f2fs `fsync_mode=nobarrier` on `/data` (BOX.md), B1's stance -- the torn-write sector model is the proof, the msync return is not -- is the literature's stance.
- **Representative crash states.** Pathfinder (2503.01390, OOPSLA'25) prunes the crash-state space by "update behaviors" and finds 8x more bugs in MMIO-based applications than prior tools. `scrash_torn.sh`'s uniform old/new/torn/zero per page is the exhaustive-random model; a representative-state variant that targets the pages the last commit dirtied is exactly the follow-up card's anchor logic turned into a test generator, and it is the cheapest way to get acceptance item 2 ("tear a byte in the page containing `mark`") as a class rather than one hand-written case.
- **Verify only what the last commit could have torn** is the shadow-paging argument MOD (1908.11850) makes for PM ("out-of-place updates ... with space-reducing structural sharing", 40 % over PMDK's STM, abstract); the anchor-cell design in the B1 card is that argument applied to an append-only arena and needs no paper to justify it.

Nothing here changes a B1 gate. It removes a design degree of freedom (no timer) and gives item 2 of the card a generator.

---

## 8. B3 kernel pool and C5 tier switching: the fork floor, code-cache sharing, and chunk-level kernel choice

- **Fork floor.** Async-fork (2301.05861, abstract): in fork-based snapshotting "copying the page table dominates the execution time of fork". B3's measured 1.04 ms floor is therefore proportional to the parent's POPULATED mappings (the 256 MB arena and the store mapping; A5's 4 GiB PROT_NONE reserve has no page tables and costs nothing). Consequence: the floor rises with store size and the 0.1 ms pool-hit gate cannot be met by any per-call fork. The shape that meets it is a **pre-forked resident runner per store** -- forked once with the store mapped read-only, parked on the pool.bp futex idiom, dispatched `(kernel image, args)` through a MAP_SHARED cell, `blr`ing the kernel in-process and answering through the same cell. Cost: one resident process per open store against the 32-process cap (C4 refused a resident process per store for background propagation; this one replaces N transient forks with one parked process, so the average count falls). Cheapest kill: measure the futex round-trip between two bebop processes with the existing pool.bp primitives; if it is > 100 us on this box, the gate is unreachable by any design and should be restated.
- **The stale-`.gbpool` defect is a code-cache identity problem with a name.** ShareJIT (1810.09555, OOPSLA'18, on ART/Android) shares JIT code caches across processes and states the tradeoff exactly: "increased specialization to a single process' context decreases the extent to which the compiled code can be shared", so it "limits some optimization to increase shareability"; it reports -37 % JIT compile time and -16 % memory (abstract). B3's `compiler_digest` key is that identity; the OPEN defect is the file-backed lookup skipping it. Nothing new to build, but the defect is the whole of the safety story for the pool and should be fixed before any C5 flavour switching adds a second key dimension.
- **Kernel choice per chunk is where adaptivity has literature support.** Seer (2403.17017) selects an SpMV kernel per dataset with a decision tree (2x over the best single kernel across SuiteSparse, abstract); CAKE (2602.04181) selects per morsel with a microsecond-scale bandit (up to 2x end-to-end, abstract); 2511.16455 finds plan-level adaptivity helps DuckDB through cardinality refinement but costs PostgreSQL. All three restrict the choice to ONE operator's variants at chunk granularity -- the review's rejection of racing whole plans stands, and C5's "chunk-boundary switching" is the supported shape. C5's gate (within 1.1x of the better fixed flavour on Zipf) is consistent with these papers' claims.
- **The C5 trigger.** `perf_event_paranoid` reads -1 but `swpmu.bp:1-4` recorded EACCES; the corpus has ARM SPE profiling (2410.01514) which needs a kernel driver this proot will not have. Re-probe syscall 241 before designing any counter-driven switch; a software proxy (bytes touched per chunk / elapsed `clock_ms`) is the fallback and needs no probe.

---

## 9. B4 functional tensor updates: add a size gate, and weigh a pointer-free PMA against CoW row blocks

- **Sizes (abstracts).** 2502.10959 ("Revisiting the Design of In-Memory Dynamic Graph Storage"): "Aspen consumes 3.3-10.8x more memory than CSR, while the optimal fine-grained methods consume 4.1-8.9x more memory than CSR". 2502.13862 compares PetGraph, SNAP, SuiteSparse:GraphBLAS, cuGraph and Aspen on load/clone/update and finds a hand-built representation 3.3x Aspen on loading. B4's blueprint has stall, amortised-update and fold gates and NO bytes-per-edge gate, on a store whose size row already loses 2.5x to sqlite. Proposed gate: **bytes per edge slot after 1M updates and one compaction <= 1.5x static CSR at the same cell width** (16 B at i64, 8 B at u32).
- **The pointer-free alternative.** CPMA (2305.05055): a compressed packed memory array, "without pointers", batch-parallel, "3x faster batch-insert throughput and 4x faster range-query throughput" than compressed PaC-trees and "2x faster on batch inserts" than PaC-trees on dynamic-graph processing (abstract). A PMA is one array with gaps; inserts move O(log^2 n) amortised cells; scans are sequential. For a store whose thesis is "the file is the memory image" and whose refs are offsets, a packed CSR (PMA per row range) is the structure that keeps `ci` one object and keeps scans at CSR speed, against B4's tail + L0 + L1 row-block CoW with a 2-level blocktab. DGAP (2403.02665) is the counter-evidence in favour of B4's current shape: "a single mutable CSR ... with a per-section edge log", 3.2x update and 3.77x analysis over XPGraph/LLAMA/GraphOne on PM (abstract) -- i.e. sgraph2's edge-log + tiered CSR already matches a published design. Decision the operator can make on paper: B4 step 0 = analytic bytes/edge for both shapes at mean degree 10 before either is coded.
- **Versions and GC.** 2108.02775 and 2212.13557 give O(1)-amortised, space-bounded reclamation for version lists; the store's Cheney compaction is the reachability-based equivalent and does not need them. Note only that "compaction = GC of unreachable versions" has the space-blowup failure mode those papers bound: the file grows by every update until compaction, and B4's 1M-update gate should report the peak file size, not just the compacted one.
- **Multi-version B-trees** (2606.09133) and path-copying trees (2212.00521, which "scale in practice" under write-heavy loads contrary to expectation) support CoW as a concurrency strategy for B5's readers; neither changes a B4 number.

---

## 10. B6 multi-core: the DRAM ceiling is confirmed, and the four A55 cores have a documented use

- The measured "~12 GB/s for one A78 and for three" is the whole story for any streaming kernel: B6's stop rule (do not spawn when the single-core kernel is at >= 80 % of 12 GB/s) is what the roofline literature prescribes (2604.06637 gives sparsity-aware rooflines for SpGEMM; 2606.22423's decode-throughput law says when a scan is bandwidth-bound at all). nn4's 2.21x on 3 cores says nn4 is NOT at the ceiling, which is consistent -- it is a bucketed scan with compute.
- **The A55s.** 1701.05478 (Decoupled Access-Execute on ARM big.LITTLE): split loops into a memory-bound Access phase on LITTLE cores and a compute-bound Execute phase on big cores; "IPC improvement of up to 37 % in the Execute phase" and "more than half of the program runtime" shifted to LITTLE (abstract, preliminary). 1506.08988 schedules GEMM micro-kernels across big/LITTLE with cache-aware configuration. The box has cores 0-3 (A55) idle in every bench script (BIG-core pinning). A prefetching helper on an A55 for a latency-bound kernel (the join's scatter, section 3) is the one use of those cores that does not compete for the shared 12 GB/s -- but it is a B6 step 5 at best, after blocking (section 3) has removed the misses it would prefetch.
- Loop scheduling for irregular iteration cost (2007.07977: self-managed chunk size + work stealing within 5.4 % of the best tuned method, abstract) supports B6's "static ranges + one dynamic queue"; no change.

---

## 11. C2 per-type columns, A2's csel, K6: one law and one crossover from a paper with a one-command artifact

2606.22423 ("When Is a Columnar Scan Bandwidth-Bound?", abstract; code released): a decoder's value throughput is independent of bit width, so the bandwidth fraction obeys `f = min(1, T_dec * b / (8 * beta))`, validated on x86/AVX2 and on an **Apple M4/NEON** machine with median error 0.003; two crossovers: "branch-free predicate evaluation beats branchy in a mid-selectivity band (the sigma(1-sigma) misprediction parabola)", and "zone-map skipping is clustering-gated rather than selectivity-gated".

Applicability, all three concrete:

- **A2/K8H's csel win is selectivity-dependent by that law.** K8H's data-dependent `if` measured 1.1x at one selectivity; at very low or very high selectivity the branchy form is as good and csel's extra words are pure cost. The honest.sh row should sweep selectivity (e.g. 1 %, 25 %, 50 %, 75 %, 99 %) once; if csel loses outside the mid band, A2's "pure `if` -> csel" rule gains a cheap static heuristic (constant-comparison shapes stay branches).
- **A8 + C2 at raw u32 cells have T_dec = memory speed**, so the K6 scan is bandwidth-bound at b = 32 by construction and the 2x from halving bytes is real THERE (unlike the join, section 3). This is the right place for A8's "-2x" line.
- **Zone maps (B7, LANG-DB §9.4-9.5) pay off only on clustered columns**; the planner should consult clustering (sortedness of the CSR key), not predicate selectivity, before emitting a skip check.

Also in the corpus for C2: Lance (2504.15247) shows columnar random access is an encoding problem, not an inherent weakness (Parquet "over 60x better random access" when configured), which softens C2's "point read of k components is k cache lines" -- true, but the 2x-of-450 ns regression gate is the right test; Kuzu's columnar graph storage (2103.02284) with "single-indexed edge property pages" and list-based processing is the published version of "columns for edge properties, objects for entities".

---

## 12. C4 declared views and B7: a static dichotomy that kills views on paper

2606.07795 ("The Role of Semirings in Incremental View Maintenance", abstract): for inserts into K-databases over a commutative semiring WITHOUT additive inverse (natural, provenance, tropical -- i.e. exactly the semirings gen_gb runs, where deletions are tombstones, not negative deltas), a conjunctive query without self-joins "can be maintained with amortized constant update time and constant enumeration delay if and only if it is alpha-acyclic p-hierarchical", with conditional lower bounds on the other side. 2605.08397 generalises update cost to a "maintenance width" via heavy-light partitioning; F-IVM (2303.08583) and DBSP (2203.16684) are the engines that realise the upper bounds (F-IVM "orders of magnitude" over first-order IVM, abstract).

Applicability to C4, whose gate is a measured k/N crossover per view: **add a step 0 that classifies each declared view syntactically.** A view whose join graph is not alpha-acyclic p-hierarchical cannot have constant update time under any engineering, so its maintenance cost grows with the data and it is refuted without a measurement; a hierarchical view earns the k/N run. This is a ~50-line check over B7's AST (store objects `{kind, ref children[], attr}`) and it should also inform B7's planner about which aggregates it may promise to keep incrementally. The 2308.04214 finding that differential dataflow handles deletions consistently is the reminder of why C4 ruled out negative deltas: tombstones + recompute at commit is the semiring-without-inverse regime this dichotomy is about.

Nothing here revives whole-graph reactive execution; the measured 41x and the 0.39 % crossover stand, and the dichotomy explains part of why (most interesting views are not hierarchical).

---

## 13. A9 NEON builtins and the parser

- `scan` classes are "vectorized classification" in the literature's terms: 2503.01662 reports a 20-fold improvement in HTML scanning on recent ARM processors with SIMD classification (abstract). The 46-word scalar `scan` landed with K5 -10 % as its gate; a NEON form of class 0/1 (whitespace/ident) over 16 bytes at a time is the follow-up if K5 is ever the bottleneck again, and it is a builtin, so it costs nothing in the one-pass constraint.
- Prefix sums (2312.14874) and `cmp_mask`/`sum64` over u32 after A8: the corpus's ARM SpMV work (SPC5 2307.14774; the A64FX ECM model 2103.03013 finding "CRS is not a good practical choice on this architecture" in favour of SELL-C-sigma) is SVE-specific; the A78 has NEON only, 4 x u32 lanes, and the formats question does not transfer at 128 bits. No change to A9.

---

## 14. Self-hosting, the honesty floor, fuzzing: three cheap things and one parked task

- **Diverse double-compiling** (1004.5534; Camlboot 2202.09231 did it for OCaml in about a person-month using a reference interpreter "implemented in a small subset of the language"): gen3 == gen4 is a FIXPOINT, not a DDC -- it proves stability, not correspondence to source. The tree already has the second implementation DDC needs: `tools/bpref.py` is an interpreter of bebop. Running bpref on bebop.bp compiling bebop.bp and comparing the md5 to gen4 would be DDC with zero new dependencies (python is a dev tool, not part of the 159-line trust root). T89 ("trust chain + DDC") is parked and marked as blocked on a witness compiler that cannot compile the current surface (ROADMAP-AUDIT F-D) -- bpref may or may not cover the current surface; if it does, the cost is one slot-run per promotion. Worth a probe, not a row.
- **NoREC** (2007.08292): detect optimisation bugs by evaluating a query through an optimising and a non-optimising engine -- bpref-vs-bebop.bin differential fuzzing is this, already in place (TG-DONE 4/8).
- **Pivoted Query Synthesis** (2001.04174: generate queries guaranteed to return a chosen pivot row; 123 bugs in SQLite/MySQL/PostgreSQL, abstract) is the test-generation idea for B7's DSL when it exists: a generated `q { ... }` with a known must-return row needs no oracle engine.
- WhiteFox (2310.15991) and the LLM compiler-fuzzing family need an LLM in the loop and target optimisation passes; not applicable to a pass-free compiler under a no-AI-in-runtime manifesto (they are R&D tools, so not forbidden, but there are no passes to target).

---

## 15. What could not be determined from abstracts alone

- The three mechanisms of section 2.2 are the literature's candidates; which one the box exhibits is unknowable without R1-R3. Nothing in the corpus reports f2fs `page_mkwrite` cost on Android specifically.
- Whether propagation blocking's 20-50 % / "order of magnitude" numbers (x86 servers with large L3s) transfer to a 3-core A78 with a small shared cache: the direction is certain, the factor is not; hence the >= 2x kill line in section 3.
- The per-fn emission-time distribution for A10 (section 5) is a property of bebop.bp, not of any paper.
- The LFI verifier's size (2508.15898 is a short paper; the abstract does not state lines of code), so the "~100 lines" in section 6.2 is an estimate from the restricted form, not a citation.
- The futex round-trip on this box (section 8) -- no paper; measure.
- Cache sizes of the A78 cluster on this SoC (private L2, shared L3), which set the bin count for section 3's experiment; read `/sys/devices/system/cpu/cpu4/cache/` before choosing 2^8 vs 2^12.

---

## 16. Papers worth reading in full, and the row each serves

| arXiv | title (venue as recorded) | row | why it is worth the read |
|---|---|---|---|
| 2409.10946 | Skip TLB flushes for reused pages within mmap's | B2(iii), A5 | mmap/page-cache eviction cycles as a misattributed bottleneck; the experiment design for R1-R3 |
| 2112.14013 | Reducing Minor Page Fault Overheads through Enhanced Page Walker (TACO) | B2(iii) | the cost model of a minor fault; the 29 % figure |
| 2310.16300 | Snapshot: Fast, Userspace Crash Consistency for CXL and PM Using msync (ICCD'23 ext.) | B2(iii), B1, B4 | the build-in-DRAM, sync-deltas-at-msync design that the store's publish rule would copy |
| 2306.05701 | CAWL: A Cache-aware Write Performance Model of Linux Systems | B2(iii) | Linux page cache + background writeback + throttling, modelled |
| 2002.11302 | Bandwidth-Optimized Parallel Algorithms for SpGEMM using Propagation Blocking | B2(i), B7 | the blocked form for the Gustavson template; roofline argument |
| 2501.07056 | MAGNUS: Generating Data Locality to Accelerate SpGEMM on CPUs (ICS'25) | B2(i), B7 | chunked intermediate product; input/system-aware chunk count |
| 1008.2849 | Faster Radix Sort via Virtual Memory and Write-Combining | B2(iii), B4 | the CSR build as a bandwidth-bound counting sort; 88 % of peak |
| 1709.07122 | Accelerating PageRank using Partition-Centric Processing (USENIX ATC'18) | B3, B6 | partition-centric layout, branch avoidance for scatter/gather |
| 2312.14874 | Parallel Prefix Sum with SIMD | A9, B4 | cache-partitioned prefix sums |
| 2505.22610 | TPDE: A Fast Adaptable Compiler Back-End Framework | A2b (ceiling) | one analysis + one combined pass; what O0-class one-pass codegen achieves |
| 2305.13241 | Whose baseline compiler is it anyway? | A2b (ceiling) | six production single-pass compilers placed in a tradeoff space |
| 2011.13127 | Copy-and-Patch Compilation | A2b, B3 | stencil-based codegen at AST-construction speed; the query-compiler use case |
| 1411.0352 | Simple and Effective Type Check Removal through Lazy Basic Block Versioning | A2b (shape) | the only one-pass specialisation technique in the corpus |
| 2502.20547 | The False Lead of Optimizing Inline Caches | A2b | negative result: fewer memory accesses did not mean less time |
| 2202.05328 | Forward Build Systems, Formally (CPP'22) | A10 | the correctness definition A10's md5 gate encodes; trace-based dependencies |
| 2108.12469 | LaForge: Always-Correct and Fast Incremental Builds | A10 | tracing instead of declaring dependencies |
| 2002.06183 | Constructing Hybrid Incremental Compilers ... with an Internal Build System | A10 | retrofitting incrementality onto a whole-program compiler |
| 1901.09056 | Not So Fast: WebAssembly vs. Native Code (USENIX ATC'19) | C1 | the 45-55 % number for the sandbox model |
| 2508.15898 | Automated Formal Verification of a Software Fault Isolation System (FMCAD'25) | C1 | LFI: machine-code subset + small verifier; the mask form |
| 1907.04241 | CHOP: Bypassing Runtime Bounds Checking | C1 | 80 % of dynamic checks redundant; motivates the hoisted form |
| 2403.02416 | Arrays in Practice (JVM access patterns) | C1 | 69.8 % uncomplicated traversals |
| 2606.18187 | Group Commit Self-Clocks | B1 | no timer; greedy-pipelined is within 0.1 % of oracle |
| 2503.01390 | Scalable and Accurate Application-Level Crash-Consistency Testing via Representative Testing (OOPSLA'25) | B1 | representative crash states for the torn harness |
| 2603.01384 | Unix Tools and the FITO Category Mistake | B1 | why the syscall return is not the commit boundary |
| 2301.05861 | Async-fork: Mitigating Query Latency Spikes ... Fork-based Snapshot | B3 | fork cost = page-table copy; motivates the resident runner |
| 1810.09555 | ShareJIT: JIT Code Cache Sharing across Processes (OOPSLA'18, Android) | B3 | specialisation vs shareability; the pool-identity defect's genus |
| 2403.17017 | Seer: Predictive Runtime Kernel Selection for Irregular Problems | C5 | per-input kernel selection with a decision tree |
| 2602.04181 | Piece of CAKE: Adaptive Execution Engines via Microsecond-Scale Learning | C5 | per-morsel kernel choice; counterfactual feedback |
| 2502.10959 | Revisiting the Design of In-Memory Dynamic Graph Storage | B4 | the 3.3-10.8x memory overheads; the missing size gate |
| 2305.05055 | CPMA: An Efficient Batch-Parallel Compressed Set Without Pointers | B4 | the pointer-free alternative to CoW row blocks |
| 2403.02665 | DGAP: Efficient Dynamic Graph Analysis on Persistent Memory | B4 | mutable CSR + per-section edge log, the published twin of sgraph2's design |
| 1904.08380 | Low-Latency Graph Streaming Using Compressed Purely-Functional Trees (Aspen, PLDI'19) | B4 | the functional-tree baseline B4 is implicitly competing with |
| 1701.05478 | Decoupled Access-Execute on ARM big.LITTLE | B6 | the only documented use of the idle A55s |
| 2606.22423 | When Is a Columnar Scan Bandwidth-Bound? A Decode-Throughput Law | C2, A2, A8, B7 | validated on NEON; selectivity parabola; zone maps clustering-gated; artifact runs in one command |
| 2103.02284 | Columnar Storage and List-based Processing for Graph DBMS (VLDB'21) | C2 | edge-property columns in a graph store |
| 2606.07795 | The Role of Semirings in Incremental View Maintenance | C4, B7 | the maintainability dichotomy; static pre-screen |
| 2303.08583 | F-IVM: Analytics over Relational Databases under Updates | C4 | the engine shape if a hierarchical view is ever maintained |
| 1906.03113 | Optimal algebraic Breadth-First Search for sparse graphs | B3 | O(n) algebraic BFS; up to 24x sequential over a GraphBLAS library (abstract) -- a gen_gb BFS template candidate |
| 1804.03327 | Implementing Push-Pull Efficiently in GraphBLAS (ICPP'18) | B3 | masking as the generalisable third of DOBFS |
| 2503.01662 | Scanning HTML at Tens of Gigabytes per Second on ARM Processors | A9 | vectorized classification for `scan` |
| 2202.09231 | Debootstrapping without Archeology: Stacked Implementations in Camlboot | self-hosting | DDC via a reference interpreter; the bpref route |
| 2007.08292 | Detecting Optimization Bugs in Database Engines via Non-Optimizing Reference Engine Construction | honesty floor | the bpref-differential idea, named |
| 2001.04174 | Testing Database Engines via Pivoted Query Synthesis | B7 tests | oracle-free query generation |

Not in the corpus but load-bearing for section 2, so named as absences: Crotty/Leis/Pavlo "Are You Sure You Want to Use MMAP in Your DBMS?" (CIDR 2022); Beamer/Asanovic/Patterson direction-optimizing BFS (SC'12); Neumann/Leis Umbra and HyPer adaptive compilation (VLDB/ICDE); the Salsa framework (no paper). The corpus cannot support or refute claims about them.
