Status: 2026-09-12, research pass (Opus), NO code written, NO run executed — the box was at
`slot 1 BUSY auto:std_golden.sh` / `procs 29/26` for the whole session, above WORKER-CARD's
26/32 wait line, so every number below is either QUOTED from a committed file with its
line, or DERIVED by arithmetic from such a number and marked as derived. Nothing here is a
fresh measurement. Every step in §5 carries the command a worker runs to make it one.

FILENAME NOTE: ROADMAP B6 cites `docs/blueprints/B6-multi-core-kernels.md`, which exists
(92 lines, dated 2026-09-06, grounded at HEAD b1c0175). This document SUPERSEDES it and
should land AT THAT PATH, not at `B6-multicore.md` — `tools/arch_check.py`'s
`max_missing_citations` ratchet (tools/arch_ratchet.txt) counts documents citing files that
do not exist, and a new name plus a stale citation raises it.

# B6 — multi-core kernels: what it is, the one number that proves it, and the ceiling

## 0. The three corrections this blueprint rests on

Read these before §1. Two of them contradict ROADMAP B6's own text.

**(0a) The row's traversal arithmetic is wrong by a factor of ~80, and it inverts the
conclusion.** B6 says "a random gather is latency-bound at 133 ns against a 0.67 ns traffic
floor", and concludes that on traversal-shaped work cores and per-core instruction reduction
DO multiply because "the memory system is never approached". The source is docs/exp.journal
line 852, which says:

> "the random-gather pair settles a separate question: **133 ms with i64 indices** against
> 128 ms packed, 1.06x, because a gather pays for a whole 64-byte cache line however narrow
> the index is"

That is 133 **milliseconds for 10^7 gathers** = **13.3 ns per gather**, not 133 ns. Ten times
smaller. And the "0.67 ns traffic floor" is 8 bytes ÷ 12 GB/s — it prices the 8 useful bytes
when the same journal line says in the same sentence that a gather moves a whole **64-byte
line**. The honest floor is 64 B ÷ 11.05 GB/s = **5.8 ns**. The two slips compound: the row
believes the latency-to-traffic ratio is 133 / 0.67 ≈ 200:1 (unlimited headroom); it is
13.3 / 5.8 = **2.3:1**. Per core it is worse than 1:1 — 64 B at that core's own measured
2.86 GB/s stream rate is 22.4 ns, LONGER than the 13.3 ns the gather actually took, which is
only possible because the sequential loop is instruction-bound (journal 852: "3.3 ns each,
about 7 cycles for a loop doing one load and one add ... INSTRUCTION-bound at ~24 % of the
memory system") and the gather loop is not.

**Consequence: traversal is the shape with LESS headroom on this box, not more.** The row's
consequence (b) is refuted, and it is refuted by the row's own other datum, which it states
two sentences later without noticing the contradiction: `parscat.bp` measured a random
scatter at **1.64x on three cores, 55 % of ideal**. The row asserts traversal multiplies and
then reports the only traversal-shaped 3-core measurement in the project as the WORST scaling
number in the project. §6 does the arithmetic.

**(0b) `parscat.bp` does not exist.** `find . -name "parscat*"` returns nothing, and the
string appears nowhere in docs/exp.journal. The 100/100/68/61 ms at W=0/1/2/3 is a scratchpad
number with no committed script behind it, which is exactly what the thesis' clause 4
("every gate has an oracle ... every speed number is pinned, in-process, against an honest
twin") forbids. It is the most important number in the row and it cannot be re-run. §2's gate
supersedes it; the row should mark it `scratch, unreproducible`.

**(0c) The "no gate notices a corrupted concurrency emitter" record is STALE, not wrong.**
`tools/mutation_coverage.txt` rows for `emit_sys_cond_set`, `emit_sys_futex_wait_guard`,
`emit_sys_futex_wake`, `emit_sys_atomic_add`, `emit_sys_exit_thread_guard`,
`emit_sys_setaffinity` and `emit_sys_msync` all read `SURVIVED pass=95==95`. But today's
`tools/mutate_compiler.py` writes `SURVIVED pass=%d, threads ok` (:124) after running the smw
thread gate (:47-65, `smw.bin 2 200`, whose fold is 2*(200+2)=404). The seven rows carry the
verdict string of a tool version **that had no thread gate**, i.e. they were recorded before
smw was wired in and were never re-derived. The layer may be covered now; nobody has looked.
Step 0 in §5 looks, in about seven minutes.

A second, separate limitation of that tool matters here and is not recorded anywhere:
`sites()` (:28-38) mutates only the **first** `em(insns, n, <literal>)` in each emitter. For
`emit_sys_cond_set` that is `2332102177` (`add x1,x17,x1,lsl #3`, bebop.bp:1337), so the word
that actually publishes — `4177526819` = `0xF9000023` = `str x3,[x1]` at bebop.bp:1341 — has
**never been mutated by anything**. "The emitter is covered" and "the store word is covered"
are different claims and only the first is even being asked.

---

## 1. Scope: five bullets each way

### B6 IS

1. **A fork–join worker shape plus one committed speedup gate** for kernels reading arrays
   that already live in one process's arena or store mapping, on exactly **3 pinned A78
   cores** (docs/BOX.md: "7 in the affinity mask: 4,5,6 = A78 big, 0-3 little"; nn4.sh:3-4:
   "cpu 7 refuses taskset"). Usable width is 3 and is not a tunable.
2. **Two work shapes measured separately and never blended**: SCAN (row-range sequential
   reads, B3's mxv/reduce/select and K6) and GATHER (random indexed reads, BFS frontier
   expand). They have different ceilings (§6) and therefore different thresholds. A single
   headline "B6 speedup" number is a lie by construction.
3. **Bit-exact fold identity at every W ∈ {0,1,2,3}** as a hard precondition of any timing
   claim — the same discipline nn4.bp:133 already applies (`ok = if fold_par == fold_seq`).
4. **Making the concurrency primitive layer test-covered and memory-ordering-sound**, because
   every number in (1)-(3) is produced by it. Concretely: re-derive the mutation rows (§5
   step 0) and convert the done-flag handshake from a plain `str` to a real release/acquire
   pair (§5 step 2), which costs **zero new instruction words**.
5. **Discharging the 8-symbol clone constraint by construction**, not by counting — one
   `env: [i64]` carries everything, `tools/arch_check.py` CHECK 20 stays at ratchet 0
   violations, and the worker shape in §4 is the only shape any B6 program uses.

### B6 IS NOT

1. **Not async, futures, green threads, or a scheduler.** The thesis' clause 3 is "cores only
   for parallel scans (T106) ... no runtime cells for ordinary code". B6 is `sys_clone` +
   futex + join, with no runtime, exactly as pool.bp:3-18 describes. The operator's word
   "async" in the 2026-09-08 note is satisfied by *concurrent execution*, not by a language
   feature; a `sys_clone` costs nothing to start and there is nothing for a scheduler to do
   with a fixed width of 3.
2. **Not work stealing, dynamic queues, or A55 offload.** Over-decomposition and an atomic
   work queue are what the superseded blueprint proposed (B6-multi-core-kernels.md §3); with
   W=3 and static ranges over 10^7 elements the imbalance is one element and the queue is
   pure overhead. A55 offload is separately **refuted on latency** in §6: it buys 1.29x of
   aggregate bandwidth and costs the A78s 1.54x of their own latency (28 ms → 43 ms,
   journal 852). It belongs to work with no deadline (B3 compile-on-miss, crc, compaction),
   which is not a gated kernel.
3. **Not multi-core COMMIT throughput.** That is B5's territory and it is already measured as
   ANTI-scaling: P=1 53.3 us, P=2 28.6 us, **P=3 42.6 us**. No B6 gate may be a commit rate,
   and no B6 worker may commit. smw is the store's gate, not a core's.
4. **Not the decision to parallelise.** Choosing W per query, and the stop-at-the-bandwidth-
   ceiling rule the old blueprint put in §3, belong to **B7** (whose dependency line already
   reads `B3, B6`). B6 supplies the mechanism and the measured efficiencies; B7 spends them.
5. **Not per-core instruction reduction.** Unrolling, `cmp_mask`, `sum64`, NEON — that is
   **A9**, and on scan-shaped work it draws on the *same* ~3.9x budget (§6). Landing any part
   of A9 inside B6 makes the gate stop measuring cores. B6 must land on today's codegen and
   the gate's ratio must be re-derived, not re-used, after A9.

Also out: parallel durability (the mount is `fsync_mode=nobarrier`, so durable writes are not
durable here and an msync row is unmeasurable), and process-level parallelism (§3, gb_run).

---

## 2. The gate

One program, one script, one number per shape. It cannot be gamed because **every arm is the
same function on the same data inside the same process invocation**, so the only free
variable is the core count.

### 2.1 Files

| file | what |
|---|---|
| `bench/vs_rust/std_tests/b6core.bp` | the program: 2 shapes × W ∈ {0,1,2,3}, one run |
| `bench/oracles/b6core.py` | the fold oracle; its LAST line is the golden (std_golden.sh convention, WORKER-CARD "Gates") |
| `bench/vs_rust/b6core.sh` | the timing gate; asserts the five conditions in §2.4 |
| `bench/vs_rust/std_golden.sh` | one `gate b6core <golden-fold>` block, W=1 only, no timing |

The **fold** goes in std_golden.sh (cheap, memoised, deterministic, no timing). The **ratio**
goes in `b6core.sh`, run pinned and separately, like nn4.sh — a timing assertion must never
sit inside a memoised battery lane (std_golden.sh:34-54 replays stored stdout on a memo hit,
so a memoised timing gate measures nothing).

### 2.2 The work

`n = 10,000,000` i64 = 80 MB, chosen so the ceiling arithmetic of §6 transfers directly: that
is exactly the array size of the journal-852 probe that produced 2.86 GB/s sequential and
13.3 ns/gather.

- **shape 0, SCAN**: `b6_scan(env, lo, hi)` sums `a[i]` over `[lo,hi)`.
- **shape 1, GATHER**: `b6_gather(env, lo, hi)` sums `a[idx[i]]` over `[lo,hi)`, where `idx`
  is filled once by the parent from the same LCG as nn4.bp:9
  (`x * 6364136223846793005 + 1442695040888963407`), masked to `n`.

Both folds are **plain i64 sums**, so they are split-independent and wrap-around-exact: the
fold is bit-identical at every W by construction, not by luck, and a lost or duplicated
worker result changes it. This is what makes the fold check a race detector rather than a
formality.

A **gather, not a scatter**. Scatter's RFO doubles line traffic per access (§6) and it is not
the shape any B6 kernel has: BFS frontier expand, mxv pull and CSR neighbour walks all read
randomly and write sequentially into a private accumulator. Gating on a scatter would gate on
the one shape B6 does not do, which is how `parscat`'s 1.64x came to stand for traversal.

### 2.3 What the program returns

```
b6core <shape> <W>   ->   ok*10^15 + pin*10^14 + noalloc*10^13 + ms0*10^7 + msW
```
where, in ONE invocation:
- `ms0` = `clock_ms()` around `b6_work(env, 0, n)` called directly, no clone — the baseline;
- `msW` = `clock_ms()` around `b6_par(env, cells, W)` — the same `b6_work`, W pinned threads;
- `ok`  = 1 iff `foldW == fold0` exactly;
- `pin` = 1 iff every worker's `sys_setaffinity` return value, recorded into
  `cells[3W+1+w]`, is 0. **nn4.bp:61 throws this value away**, and `emit_sys_setaffinity`
  is one of the seven unproven emitters (§0c) — so today nothing in the tree proves a single
  worker is ever pinned, and the whole T106 row rests on it;
- `noalloc` = 1 iff `sys_arena_base()` sampled at the first and last statement of every
  worker is **equal** for every worker. `sys_arena_base()` returns the arena CURSOR as a cell
  index (bebop.bp:1247-1251), a child's arena window is only `[sp+4 MiB, sp+12 MiB)` =
  8 MiB (bebop.bp:1312-1315), and all workers' windows overlap at a 64 KiB stack pitch. This
  one cell is the cheapest possible detector of the entire sconc failure class (§7.1); any
  future B6 worker that calls something which calls `zeros` turns it to 0.

`ms < 10^7` (2.7 hours) and `W ≤ 3`, so the packing is unambiguous.

### 2.4 The five assertions in `b6core.sh` — and how each goes RED

Runs `taskset -c 4-6`, `R=5`, median of each field, both shapes, W ∈ {1,2,3}.

| # | assertion | goes RED when |
|---|---|---|
| A | `ok == 1` at every (shape, W) | a race, a lost child (the 8-symbol trap is SILENT), a reordered publish |
| B | `pin == 1` and `noalloc == 1` at every (shape, W) | pinning silently failed, or a worker allocated — either makes the timing meaningless rather than wrong |
| C | `\|msW(W=1) − ms0\| / ms0 ≤ 0.05` | the harness is not free, i.e. the baseline was inflated by spawn/join/pin cost. **This is the anti-gaming clause**: a slow baseline is the only way to fake a ratio, and W=1 runs the identical clone/futex/pin path over the identical full range |
| D | **scan: `ms0 / msW(W=3) ≥ 2.50`**; **gather: `ms0 / msW(W=3) ≥ 1.80`** | see §2.5 |
| E | `msW(W=3) < msW(W=2) < msW(W=1)` | adding a core made it slower — the shape B5's commit path already shows (P=2 28.6 us, P=3 42.6 us) and the one failure mode a two-point ratio hides |

RED on day one: today's equivalent programs give scan **2.21x** (nn4, journal 631) and the only
random-shape number is **1.64x**. Both are below D. **The gate is RED the hour it lands and
stays RED until B6 does work** — which is the only property that distinguishes a gate from a
record.

Why this is robust on a shared, ptrace-inflated box: D and E are **ratios of two timings from
the same process invocation over the same bytes**. A slower box, a busier box, thermal
throttle, proot's 10-100x syscall tax — all of it multiplies both arms. C then proves the only
per-arm asymmetry (spawn/join/pin) is under 5 %. No absolute millisecond figure appears in any
assertion.

### 2.5 Why those two thresholds and not others

**Scan, 2.50x.** The measured bracket is: in-process today **2.21x** (nn4: seq 219 ms / par
99 ms, journal 631, `folds equal`); three separate PROCESSES on the same sequential sum
**3.00x with zero degradation** (journal 852: "28 / 28 / 28 ms ... byte-for-byte the solo
time"). The 0.79x gap between the same hardware doing the same thing in threads vs in
processes is B6's actual research question and nobody has isolated it (§5 step 4 names four
candidates and how to kill each). 2.50 is the midpoint: it demands that **at least 37 % of the
unexplained gap be closed** and it is bounded above by a number measured on this silicon, so
it is not aspirational. It is not 3.00 because 3.00 was measured with three page tables and
B6 has one; it is not 2.21 because a threshold equal to the status quo gates nothing.

**Gather, 1.80x.** Derivation, all from journal 852:
- one core: 13.3 ns/gather, 64 B line each → **4.81 GB/s** of line traffic from one core;
- aggregate ceiling: 11.05 GB/s (§6 derives it from the seven-core run);
- arithmetic cap on 3 cores: 11.05 / 4.81 = **2.30x**, and random line streams do not reach
  sequential-stream bandwidth, so the true cap is below it;
- the one measured random-shape 3-core point is **1.64x**, for a SCATTER, whose line traffic
  per access is ~2x a gather's (read-for-ownership plus writeback vs one read). 1.64 against
  a scatter cap of 11.05 / 5.12 = 2.16x is **76 % efficiency**;
- applying that same measured efficiency to the gather's 2.30x cap gives 1.75x. **1.80x** is
  just above it and comfortably above 1.64x, so it cannot be passed by a scatter mislabelled
  as a gather, nor by the status quo.

If step 5 lands and gather sits at 1.7x with the cause identified, **lower the threshold and
say so in the row** rather than keep an unreachable gate. A gate nobody can pass is abandoned
within a week and then defends nothing.

### 2.6 Registration

- `bench/vs_rust/std_golden.sh`, next to the smw block (:701-707):
  ```
  rm -f b6core.store
  r=$(./seed/build/seed ${BEBOP_BIN:-bebop.bin} compile bench/vs_rust/std_tests/b6core.bp ${BEBOP_TMP:-/tmp/opencode}/b6core_test.bin >/dev/null 2>&1 && run 120 ${BEBOP_TMP:-/tmp/opencode}/b6core_test.bin 0 1 | tail -1)
  gate b6core <fold-from-bench/oracles/b6core.py> "$r"
  ```
- `bench/oracles/b6core.py` — reproduces the LCG fill and both folds in python; last line is
  the golden. Required by `check_gate_has_oracle` in tools/arch_check.py.
- `bench/vs_rust/b6core.sh` — the timing gate, re-execs through `tools/slot.sh` exactly as
  std_golden.sh:8-12 does, appends its row to `bench/vs_rust/RESULT-b6.md`.
- NOT added to `tools/battery.sh`. The battery must stay a correctness instrument.

---

## 3. Which existing work B6 sits on

### `bench/tq_sqlite/nn4.bp` + `nn4.sh` — **the foundation.** Build b6core from this file.

It is the only program in the tree that does all four B6 things at once:
- shards a read-only kernel over W clone'd threads (`par`, :55-86);
- **pins** each worker: `if r == 0 then sys_setaffinity(cells, W * 2 + 1 + i)` (:61), masks
  `16/32/64` = cores 4/5/6 written at :109-111;
- checks parallel fold == sequential fold **in process, in the same run** (:133);
- satisfies the 8-symbol rule deliberately and says so: ":54 the parallel region: 4 params +
  4 locals = 8 symbols, nothing spilled (pool.bp rule)".

Its four defects as a B6 gate, each of which §2 fixes:
1. **`nn4.sh` has no speed assertion.** Line 25 is `[ "$ok" = 1 ] || { echo "T106 FAIL:
   parallel fold != sequential fold"; exit 1; }` — that is the ONLY failure condition. The
   2.21x is printed into RESULT.md and defended by nothing. T106 is a record, not a gate.
2. **The baseline is not the same path.** `:124` times `work(env, n, 0, q)` called directly;
   the par arm adds clone + pin + futex. A harness cost is invisible and inflates the ratio.
   §2.4 assertion C is exactly this hole.
3. **`sys_setaffinity`'s return value is discarded** (:61) and that emitter is unproven
   (§0c). Nothing anywhere asserts a worker is pinned.
4. **W is hardcoded 3** (:102), so there is no W-sweep and assertion E is not expressible.

### `selfhost/std/pool.bp` — **right idiom, wrong level. Keep it exactly as it is.**

`par_sum`/`par_merge`/`par_tids` are liveness and correctness probes for clone+futex+atomic
and they should never become a B6 gate, because **they do no memory work at all**: par_sum's
entire "work" is `let part = is_child * (i + 1) * per` (:38), a constant fold. `par_sum(4,
1000) == 10000` is computed with zero loads and cannot say anything about cores. It also
spawns W=4 when usable width is 3, and it never pins. Its real value to B6 is the comment at
:5-8 and `par_merge`'s `sys_atomic_add` merge (:81), which is the ONLY release-ordered publish
in the tree and the model for §5 step 2.

### `bench/vs_rust/std_tests/smw.bp` — **the shape exemplar, not a foundation.**

It is the tree's proof that the 8-symbol rule can be satisfied by construction (:4-12, :161-177
keep exactly four across the spawn) and §4 is built from it. But its work is COMMIT-bound and
its scaling is *negative* (P=2 28.6 us vs P=3 42.6 us per commit), so it measures the store,
not the cores. Keep it as B5's gate and as `tools/mutate_compiler.py`'s thread gate (:47-65).
Do not extend it.

### `bench/vs_rust/std_tests/sconc.bp` — **dead end as a foundation; fix it for a different reason.**

It is a **single global lock**: every writer spins on `sys_atomic_add(cells, 0, 1)` until it
reads 0 (:43-49), holds it across the whole transaction (:50-69) and releases at :70. By
construction four writers cannot scale — they serialise, and that is the point of G6. It is a
liveness and snapshot-consistency test, not a throughput test, and it has no bearing on B6's
question. Fix it because `gate sconc 40000` (std_golden.sh:717) is RED and the battery is the
merge criterion; see §5 step 1 and §7.1 for the root cause, which is a hazard class B6 workers
must never re-enter.

### `selfhost/std/gb_run.bp` fork dispatch — **correctly a dead end, and the reasons are the design input.**

`:359` and `:534` use `sys_clone(17, ...)` — SIGCHLD alone, no CLONE_VM, a **real fork**
(`:241-242` says so). Four reasons B6 must not use it, every one already recorded in that file:
1. a fork is a PROCESS and counts against Android's 32-phantom cap (docs/BOX.md); ROADMAP B6's
   own step 0 established the complement — `sys_clone(68864)` is CLONE_THREAD, `ps -e` does not
   list it, "threads are FREE against the cap and forks are NOT";
2. a forked child's arena window is capped to ~8-12 MiB (`:416-430`);
3. COW makes the child's writes invisible except through a MAP_SHARED store file (`:241-246`),
   so every result hand-back is a file round-trip (`gb_str_to_file`, `:385`);
4. a **1.04 ms fork floor** (`:246`) against a per-commit cost of 53 us.
It is the right mechanism for isolating a `sys_run` of a foreign image (the register-state
reason at `:43`) and the wrong one for data parallelism.

### `bench/wip/gb_par_{scan,mxv,mxm,reduce}.bp` — **delete or ignore.**

641 lines. `gb_par_scan.bp` is a verbatim copy of nn4.bp's `lcg`/`nn_scan`/`work` with a
different header. docs/VERIFIED-STATE-2026-09-12.md:44 records the rest: "`par_run` appears
only inside comments of `gb_par_{mxm,mxv,reduce,scan}.bp`; never defined or called — NOT
IMPLEMENTED". They are four copies of a file that already exists plus a function that does
not. Do not build on them; do not update them.

---

## 4. The 8-symbol constraint as a design input

CHECK 20 (`tools/arch_check.py:501-543`) counts, for every `fn` containing a `sys_clone(` call:
the number of typed parameters, plus every `let <name> = <rhs>` between the fn header and the
spawn line whose rhs is **not** pure integer arithmetic (`[-+*/%()\d\s]+`). Cap 8, ratchet
`max_clone_violations = 0` (default; the line is absent from tools/arch_ratchet.txt).

smw's insight, stated at smw.bp:10-12: *"everything else travels in `env` as an i64, arrays
included: `[i64]` values are cell indices and `st_addr`/`st_cells` are identities, so a handle
packs into a slot exactly."* That turns the constraint from an arithmetic worry into a **type
rule**.

### The rule, and the shape that satisfies it by construction

> **A B6 parallel region takes exactly three parameters — `env: [i64]`, `cells: [i64]`,
> `W: i64` — and binds exactly one non-constant local before the spawn: `cur`. Everything
> else is an i64 in `env`. No exceptions, no counting.**

That is 3 + 1 = **4**, half the cap, with four slots of headroom that nobody may spend.

```
// stk: CELLS, not bytes. emit_sys_clone converts (bebop.bp:1287, add x1,x17,x1,lsl #3),
// so 8192 here is 8192 cells = a 64 KiB stack pitch, as pool.bp:5 intends.
fn b6_stk(cur: i64, w: i64) -> i64 { cur + 16384 + w * 8192 }

fn b6_par(env: [i64], cells: [i64], W: i64) -> i64 {
  let cur = sys_arena_base();                       // kept #4. Sampled AFTER every
  let w = 0;                                        // allocation (smw.bp:157).
  while w < W {
    let r = sys_clone(68864, b6_stk(cur, w));
    let is_child = if r == 0 then 1 else 0;
    let _ = sys_cond_set(is_child, cells, 3 * W + 1 + w, sys_setaffinity(cells, 2 * W + 1 + w));
    let _ = if is_child == 1 then b6_work(env, cells, w, W) else 0;
    let _ = sys_atomic_add(cells, W + w, is_child);  // RELEASE publish (§5 step 2)
    let _ = sys_futex_wake(cells, W + w, 64);
    ...park, exit_thread_guard, w = w + 1...
  };
  ...join: sys_atomic_add(cells, W + k, 0) == 1  // ACQUIRE observe
}
```

`b6_work` receives `cells` so it can write its partial sum (`sys_atomic_add(cells, w, part)`),
its arena-delta witness and its pin result — it does not span a clone, so it has no budget.
`env` carries the array handles, `n`, and the shape selector, as i64 cell indices.

### Three corollaries a worker must be handed as rules, not as advice

1. **A B6 worker calls no `zeros` and no function that does.** The cost is not a style point:
   a child's arena window is 8 MiB (bebop.bp:1312-1315) and with a 64 KiB stack pitch **every
   worker's window overlaps every other's**, so a `zeros` in a child both exhausts (trap 80)
   and collides. `st_digest` is the specific landmine — one `zeros(256)` per call
   (store.bp:831-835) — and `st_commit` reaches it on every single commit (store.bp:357).
   §2.3's `noalloc` witness cell is the enforcement.
2. **A handle is an i64.** `st_addr(a)` / `st_cells(i)` are identities; put handles in `env`
   and never add a `[i64]` parameter to a clone-spanning fn.
3. **Constants are free but calls are not.** `let x = 16384 * 3` costs nothing; `let x =
   foo()` costs a slot even if `foo` returns a constant. CHECK 20's regex is the arbiter and it
   is deliberately conservative (`:508-511`).

---

## 5. Steps, in order, each with a number that can kill it

Every step runs its heavy commands through `bash tools/slot.sh <label> <cmd>` and checks
`bash tools/slot.sh --status` first (WORKER-CARD: wait if procs ≥ 26/32).

### Step 0 — re-derive the concurrency mutation rows. (blocks nothing; changes what the rest assumes)

**Why:** §0c. The seven `SURVIVED pass=95==95` rows predate the smw thread gate.

**Do:**
```
cd /root/dowiz/bebop-lang
python3 - <<'EOF'
import re
p="tools/mutation_coverage.txt"
drop={"emit_sys_cond_set","emit_sys_futex_wait_guard","emit_sys_futex_wake",
      "emit_sys_atomic_add","emit_sys_exit_thread_guard","emit_sys_setaffinity","emit_sys_msync"}
ls=[l for l in open(p) if l.startswith("#") or l.split()[0] not in drop]
open(p,"w").writelines(ls)
EOF
for e in emit_sys_cond_set emit_sys_futex_wait_guard emit_sys_futex_wake \
         emit_sys_atomic_add emit_sys_exit_thread_guard emit_sys_setaffinity emit_sys_msync; do
  bash tools/slot.sh b6-mut python3 tools/mutate_compiler.py --emitter $e --limit 1
done
```
**Expected first line:** `thread-gate baseline: 404` (smw at P=2 N=200 folds to 2*(200+2)).
**Expected per emitter:** either `<emitter> KILLED threads=<x>!=404 <t>s` or
`<emitter> SURVIVED pass=NN, threads ok <t>s`. The verdict string alone tells you whether the
row is current.
**Effort:** ~7 × 60 s plus slot waits.
**Kills the step:** `thread-gate baseline` prints `hang` or `compile-failed` — then the tool
is broken before any mutant and nothing it has ever written can be believed.
**Also record** (documentation only, no code): `sites()` mutates one literal per emitter, so
`emit_sys_cond_set`'s publish word `4177526819` (bebop.bp:1341) has never been mutated. If
step 2 lands, that word stops existing at the flag sites and the point is moot for flags.

### Step 1 — sconc trap 80. (blocks the MERGE, not the measurement)

**Diagnosis, static, to be confirmed by the worker in one probe:**
`st_commit` (store.bp:331) → `st_commit_m` (:335) → line 357:
```
let _ = base[pt] = ((st_digest("PartTab") & 4294967295) << 32) | st_parttab_cells(1);
```
`st_digest` (:831-835) opens with `let b = zeros(256)` — **2048 bytes of ARENA per commit**.
sconc's writers call `st_commit` inside the loop (sconc.bp:69) 10^4 times each, **inside a
cloned child**. A child's arena is `[sp+4 MiB, sp+12 MiB)` = 8 MiB (bebop.bp:1312-1315), so
8 MiB / 2048 B = **4096 commits** before `cmp x27,x28 ; b.ls +2 ; brk #80`
(bebop.bp:6524-6529; docs/TRAPS.md:10 `trap 80: arena exhausted (zeros crossed x28)`).
10^4 > 4096. **One iteration passes; 10^4 does not.** Worse: sconc's `stk` (:94) pitches
stacks 8192 cells = 64 KiB apart, so all eight children's 8 MiB windows overlap almost
entirely and eight private `x27` copies bump through one region.
smw does not trap because it **hoists the digest**: `env[6] = st_digest("PartTab")`
(smw.bp:34) is computed once in the parent and passed to `st_commit_p` (smw.bp:98), so no
child ever calls `st_digest`.

**Confirm before fixing (one probe, ~2 min):** add
`let _ = if k < 3 then cells[24 + k] = sys_arena_base() else 0;` at the top of sconc's writer
loop, run `seed sconc_test.bin`, print cells[24..26].
- advances by **exactly 256 cells** per iteration → confirmed, apply the fix;
- **does not advance** → refuted; the diagnosis is wrong and the next suspect is the 64 KiB
  stack pitch against the frame size, not the arena. **Report and stop; do not guess again.**
  (This blueprint records that an earlier observer reported the cursor as not moving. Either
  that observation or this diagnosis is wrong and the probe settles it; nothing downstream may
  be built on the untested branch.)

**Fix (mirrors smw exactly):**
1. `selfhost/prelude/store.bp`: add `fn st_commit_d(base: [i64], tx: [i64], root: i64, ptdig:
   i64, tmp: [i64]) -> i64` — the body of `st_commit_m` with `st_digest("PartTab")` at :357
   replaced by the `ptdig` parameter; redefine `st_commit_m` to call it with
   `st_digest("PartTab")` so every existing caller is byte-identical.
2. `bench/vs_rust/std_tests/sconc.bp` and `selfhost/std/sconc.bp` (**they are byte-identical
   today; keep them so**): widen `let env = zeros(8)` (:141) to `zeros(9)`, add
   `let _ = env[8] = st_digest("PartTab");` next to :150, change :69 to
   `let _ = st_commit_d(base, tx, r, env[8], tmp);` and bind `let ptd = env[8];` beside the
   other env reads at :39-40.
**Expected output:** `run 300 sconc_test.bin | tail -1` → `40000`; `gate sconc 40000` PASS;
std_golden's fail count drops by exactly one.
**Effort:** small — one new fn, four edited lines, no codegen, so no chain and no battery
(WORKER-CARD "non-codegen change": typecheck 0 findings + `run_all` mismatch=0 missing=0 +
one full std_golden.sh).
**Kills the step:** sconc still traps 80 after the fix → something else in the child allocates;
re-run the probe and report the per-iteration cursor delta rather than patching further.
**Blocks B6?** Not the measurement — b6core allocates nothing in children and can be measured
with sconc RED. It blocks the **merge**, because `gate sconc 40000` sits in std_golden.sh:717
and a GREEN battery is the merge criterion. And it is the canonical instance of the hazard
§4's corollary 1 exists to prevent.

### Step 2 — a real release/acquire handshake, zero new instruction words. (degrades confidence; do it because it is free)

**Fact:** `emit_sys_cond_set`'s publish word is `4177526819` = `0xF9000023` = `str x3,[x1]`
(bebop.bp:1341) — a **plain store**. The idiom "child writes result, then child sets flag"
(pool.bp:39-40, nn4.bp:63, smw.bp:164-166) is two plain stores to different addresses, and on
AArch64 they may become visible out of order. Not observed to fail here; architecturally
unsound, and B6 is about to triple the number of spawn sites.

**Fact that makes it free:** `emit_sys_atomic_add`'s word is `4175560705` = `0xF8E20001` =
**`LDADDAL`** (bebop.bp:1425, comment: "LSE LDADDAL ... full barrier") — acquire **and**
release, already in the compiler, already gated by `par_merge`.

**Do**, at every done-flag site:
- child publishes `let _ = sys_atomic_add(cells, <flag>, is_child);` — parent adds 0, child
  adds 1; the RMW's **release** orders every earlier store by that thread;
- parent observes `let d = if sys_atomic_add(cells, <flag>, 0) == 1 then 1 else 0;` — the
  RMW's **acquire** orders every later load. Replaces the `cells[<flag>] == 1` plain read.
- for a value cell, prefer `sys_atomic_add(cells, w, part)` (pool.bp:81's existing pattern)
  over `sys_cond_set`; then the value publish is itself release-ordered.

Sites: `selfhost/std/pool.bp:40,45,57` and `:82,87,98`; `bench/tq_sqlite/nn4.bp:63,68,79`;
`bench/vs_rust/std_tests/smw.bp:166,171,183` and `selfhost/std/sconc.bp:105,158` (+ the
byte-identical bench copy). `sys_cond_set` stays in the language and stays used where the
condition, not the ordering, is the point.
**Acceptance:** `pool_parity: 5 pass, 0 fail`; `gate smw 303000`; `gate sconc 40000`;
`objdump -D -b binary -m aarch64 <bin>` shows `f8e2` at the flag sites and no `f9000023`
there. No compiler change, so no chain.
**Effort:** small, ~12 edited lines.
**Kills the step:** `par_tids` changes value (it counts nonzero tids, pool.bp:135, and must
stay 4) or smw's fold moves off 303000 — either means the flag semantics changed, not just its
ordering.
**Blocks B6?** No. The failure it permits is a lost or early done flag, which shows up as a
**wrong fold**, and §2.4 assertion A checks that bit-exactly at every W. It degrades
confidence, not the measurement.

### Step 3 — build the gate. (needs steps 0-2 for confidence; needs none of them to run)

Write `bench/vs_rust/std_tests/b6core.bp` to §2.2-2.3, `bench/oracles/b6core.py`,
`bench/vs_rust/b6core.sh` to §2.4, and the std_golden.sh block to §2.6.
**Expected first output** (this is a prediction, and it failing is information):
```
| b6core scan,   1 vs 3 A78 (pinned, R=5) | W1 <a> ms / W3 <b> ms | ~2.2x | ok 1 pin 1 noalloc 1 |
| b6core gather, 1 vs 3 A78 (pinned, R=5) | W1 <c> ms / W3 <d> ms | ~1.6-2.0x | ok 1 pin 1 noalloc 1 |
b6core: RED (scan 2.2x < 2.50; gather <x> vs 1.80)
```
**Effort:** medium — one 150-line .bp in the nn4 shape, one oracle, one script.
**Kills the step:** `ok == 0` at any W (a race or a lost child — re-check §4's symbol count
first, it is silent), or `pin == 0` (then `sys_setaffinity` does not work and **every**
multi-core number in this project, T106 included, is unpinned and must be re-derived), or
assertion C fails at W=1 by more than 5 % (the harness is not free and the ratio is not a core
measurement).

### Step 4 — close the scan gap from 2.21x to ≥ 2.50x. This is the actual research.

Four candidate causes of the 3.00x (processes) vs 2.21x (threads) gap, each with the
experiment that kills it. Run them in this order; stop at the first that explains ≥ 0.2x.

**(a) DVFS / thermal.** The 3.00x datum ran for 28 ms; nn4's par arm runs 99 ms. Run b6core
scan at `n` giving a W=3 arm of ~30 ms and again at ~300 ms. **Confirmed if
ratio(30 ms) − ratio(300 ms) ≥ 0.2.** Then `n` must be pinned small in the gate and the row
must say the number is a short-burst number.

**(b) Unpinned baseline.** nn4's seq arm (:124) runs unpinned inside `taskset -c 4-6` while
the par arm pins. §2.3 already pins the W=1 arm identically. **Refuted if the ratio moves by
< 0.05** — which it should, and this is listed only so nobody spends a day on it twice.

**(c) Remainder and imbalance.** `n/W` truncates and the parent sweeps the remainder. At
n = 10^7, W = 3 the remainder is **1 element**. **Refuted a priori.** The row names this as one
of its "three traps"; at this n it does not bind, and it must not absorb time.

**(d) Page walks / first-touch in one address space.** Three processes have three page tables;
three threads share one. 80 MB is 20,480 4 KiB pages. Compare the ratio at n = 10^6 (8 MB) and
n = 10^7 (80 MB). **Confirmed if small gets ≥ 2.8x and large stays ≤ 2.3x** — then the gate's
scan arm is bandwidth/TLB-limited, the 2.50 threshold must be re-derived on the cache-resident
size, and the large size is documented as the ceiling case, not as a defect.

**Effort:** one full session. **Kills the step:** all four refuted and scan stays at 2.21x with
no cause. Then **lower D to 2.20x, record the shortfall as unexplained in the ROADMAP row, and
stop** — an honest low gate beats an aspirational dead one.

### Step 5 — the traversal arm on a real kernel. (needs steps 1 and 3)

Shard `gb_bfs`'s frontier expand by row range over 3 pinned cores;
`bench/vs_rust/std_tests/gb_bfs.bp` and `gb_mxv_generic` in `selfhost/prelude/gb.bp` are the
starting points, and `bench/oracles/gb_lagraph.py`'s `bfs_fold()` is the existing oracle.
**Expected output:** the existing `gb_bfs` golden **unchanged** (journal 807 records `-3` for
the 1k/4000-edge graph) at W ∈ {1,2,3}, plus a time ratio row.
**Threshold:** ≥ 1.80x, the §2.5 derivation, on the same gather grounds.
**Kills the step:** the fold moves at any W (the frontier merge is not order-independent — fix
the merge, do not weaken the check), or the ratio lands below 1.64x, at which point a real BFS
gather scales **worse than a synthetic scatter** and the whole traversal half of the row must
be re-scoped.

### Step 6 — correct the ROADMAP row and the measured table. (docs only)

Strike "133 ns" → 13.3 ns; strike "0.67 ns traffic floor" → 5.8 ns for a 64-byte line; strike
consequence (b) and replace it with §6's arithmetic; mark `parscat.bp` `scratch,
unreproducible`; replace the T106 citation with the b6core row. One journal line per §5 step,
in the WORKER-CARD format.

---

## 6. The honest ceiling

### 6.1 The aggregate ceiling, derived rather than quoted

journal 852, seven instances of an 80 MB sequential sum, one per core:

| cores | ms | GB/s |
|---|---|---|
| A78 ×3 | 28 / 29 / 43 | 2.86 + 2.76 + 1.86 = **7.48** |
| A55 ×4 | 82 / 83 / 84 / 117 | 0.98 + 0.96 + 0.95 + 0.68 = **3.58** |
| | | **11.05 GB/s aggregate** |

At that load one A78 went **28 → 43 ms**. So ~11 GB/s is where the memory system starts
pushing back, and "~12 GB/s" in the row is a rounding of this, not an independent datum. Note
what it is NOT: three A78s alone reached 8.57 GB/s (3 × 2.86) **with zero degradation**, so
8.57 is demonstrably below the knee.

### 6.2 SCAN — what 3 cores buy, and what is left over

- one A78, today's codegen: 3.3 ns/element, **2.86 GB/s**, ~7 cycles for one load and one add
  — **instruction-bound at ~24 % of the memory system** (journal 852, explicitly);
- three A78, three processes: **3.00x, zero degradation** (28/28/28 ms);
- total budget from the one-core baseline: 11.05 / 2.86 = **3.87x**.

The row says cores and per-core instruction reduction "share ONE budget of about 4x". That is
right, and it is worth sharpening, because the sharing is not symmetric:

| order | first buys | leaves |
|---|---|---|
| cores first | 3.00x (measured) | 3.87 / 3.00 = **1.29x** for A9/unrolling |
| A9 first | 1.57x (measured: the 32-bit packing arm, 21 vs 33 ms) | 3.87 / 1.57 = **2.46x** for cores |

**Cores get there first and cheaper.** 3 cores are already 78 % of the whole SoC's memory
system on a DRAM-resident scan, and A9 on top of them is worth 1.29x, not 1.57x. The product is
capped at 3.87x either way — 3 × 4 = 12x is not available at any ordering, and the row is right
about that.

**The caveat that matters more than the cap:** this binds **only when the per-core slice misses
cache**. An A78 has 512 KB of L2. A kernel whose per-core working set stays under ~512 KB
never touches the shared ceiling and keeps the clean 3.00x plus the full A9 factor. That is
exactly the shape of B3's chunked kernels and B2's phase table, and it is where B6's real value
is. **B6 should prefer chunked kernels over whole-array kernels for that reason alone**, and
step 4(d) will measure the crossover directly.

### 6.3 TRAVERSAL — the row's claim, refuted arithmetically

- one A78, random gather over 80 MB: 133 ms / 10^7 = **13.3 ns/gather** (journal 852);
- narrowing the index does essentially nothing: 133 vs 128 ms, **1.06x**, "because a gather
  pays for a whole 64-byte cache line however narrow the index is" — which also settles ROADMAP
  **D4** for traversal;
- **line traffic from one core**: 64 B / 13.3 ns = **4.81 GB/s**. That is *higher* than the
  same core's sequential-stream rate of 2.86 GB/s, because the stream loop is instruction-bound
  and the gather loop is not. **The gather is already closer to the memory system than the scan
  is** — the exact opposite of the row's premise;
- three cores demand 3 × 4.81 = **14.4 GB/s** against ~11.05 available → arithmetic cap
  **11.05 / 4.81 = 2.30x**, and random 64 B lines do not achieve sequential-stream bandwidth
  (row-buffer locality), so the real number is below it;
- the only measured random-shape 3-core point in the project: **1.64x** (scatter; scatter's
  RFO makes it ~128 B per access → its own cap is 11.05 / 5.12 = 2.16x, so 1.64 is 76 % of its
  cap, and 76 % of the gather's 2.30x cap is **1.75x**).

**So: traversal on 3 cores buys 1.6-2.3x, not 3x, and it is the SHORTER of the two ceilings.**
The row's consequence (b) — "they DO multiply, because ... three cores hold three times as many
misses in flight and the memory system is never approached" — is false. Three cores hold three
times as many misses in flight *and the memory system is approached at 14.4 GB/s against an
11 GB/s ceiling*. The row reached the opposite conclusion from the two arithmetic slips in §0a;
correcting them turns a 200:1 headroom into 2.3:1.

**Part of the row's ambition is not reachable on this hardware, and it should be struck rather
than planned around:** there is no combination of 3 cores and per-element work reduction that
reaches 12x on a DRAM-resident scan, and there is no traversal kernel that reaches 3x on 3
cores. The reachable envelope is **≤ 3.87x total on scans (of which cores are ≤ 3.00x) and
≤ 2.30x on gathers**.

### 6.4 A55 offload — refuted on latency

The row is right to correct its own overstatement and should go one step further. Loading the
four A55s adds 3.58 GB/s of aggregate (11.05 vs 7.48, **1.48x**) and costs one A78
**28 → 43 ms (1.54x)**. For any latency-gated kernel that is a straight loss: the query gets
1.54x slower so that unrelated work runs. A55 offload is correct **only for work with no
deadline** — B3's compile-on-miss, crc verification, compaction — and **no part of a gated B6
kernel may run there**. It should be named in B6's "IS NOT" and handed to whichever row owns
background work.

---

## 7. What must be fixed before B6 can be measured — blocks vs degrades

| # | thing | verdict | justification |
|---|---|---|---|
| 1 | sconc `trap 80` | **blocks the MERGE, not the measurement** | b6core allocates nothing in children (§2.3's `noalloc` witness proves it per run), so the B6 number is obtainable with sconc RED. But `gate sconc 40000` is std_golden.sh:717 and a GREEN battery is the merge criterion (WORKER-CARD "Gates"), so no B6 change lands while it is RED. Root cause (§5 step 1) is `st_digest`'s `zeros(256)` per commit inside an 8 MiB child arena — the exact hazard §4's corollary 1 exists to prevent, so fixing it is also how the rule gets taught. |
| 2 | `emit_sys_setaffinity` unproven | **BLOCKS** | A corrupted or failing setaffinity does not pin, the **folds stay correct**, and the speedup number silently becomes a measurement of nothing. It is the one concurrency emitter whose failure mode is invisible to a fold check. Today nn4.bp:61 discards the return value, so **nothing in the tree proves a worker is ever pinned** and T106's 2.21x is unpinned as far as any gate knows. §2.3's `pin` cell and §2.4's assertion B close it. |
| 3 | the other six unproven concurrency emitters (`cond_set`, `futex_wake`, `futex_wait_guard`, `atomic_add`, `exit_thread_guard`, `msync`) | **degrades confidence; the record is also stale** | §0c: the rows predate the smw thread gate and must be re-derived (step 0) before anyone plans around them. Even if they still survive, a corrupted word in any of them manifests as a **wrong fold or a hang**, both of which §2.4's assertions A and the script's `timeout` catch — so the B6 gate is itself a partial mutation test for five of the six. `msync` is outside B6 entirely (the mount is `nobarrier`; durability is unmeasurable here). |
| 4 | plain-store `sys_cond_set` | **degrades confidence** | Architecturally unsound on AArch64 (two plain stores, bebop.bp:1341 = `0xF9000023`), not observed to fail. It does not block because the reordering it permits produces a **wrong number**, which assertion A catches, not a wrong **time**. Fix it anyway in step 2: `LDADDAL` is already in the compiler (bebop.bp:1425), the change is ~12 lines and **zero new instruction words**, and B6 is about to triple the spawn-site count. |
| 5 | `nn4.sh` has no speed assertion | **blocks the CLAIM, not the measurement** | nn4.sh:25 fails only on fold mismatch. T106's 2.21x is defended by nothing and cannot regress visibly. Superseded by b6core.sh, after which nn4.sh should either adopt assertion D or be retired to a report row. |
| 6 | `parscat.bp` absent from the tree | **blocks the CLAIM** | The row's headline traversal number has no committed script (§0b). b6core's gather arm replaces it; until then no B6 plan may cite 1.64x as a gate, only as a scratch observation. |

**Not on this list, deliberately:** the P=3 commit anti-scaling (53.3 / 28.6 / 42.6 us). It is
real, it is B5's, and B6 must not gate on it or route around it — §1's "IS NOT" bullet 3 says
no B6 worker commits, which makes it irrelevant to every number here.

---

## 8. VERDICT format for a B6 worker

```
VERDICT: GREEN|RED
step: <0-6>
scan:   W0 <ms> W1 <ms> W2 <ms> W3 <ms>  ratio <x>  (threshold 2.50)
gather: W0 <ms> W1 <ms> W2 <ms> W3 <ms>  ratio <x>  (threshold 1.80)
witnesses: ok <0|1>  pin <0|1>  noalloc <0|1>   harness-free |W1-W0|/W0 <f> (<= 0.05)
folds: identical at W 0/1/2/3 <yes|no>  value <i64>
gates: std_golden <p>/<f>  pool_parity <p>/<f>  sconc <val>  smw <val>
clone-symbols: tools/arch_check.py -> "every spawn site keeps <= 8 symbols across it"
journal: <one line, WORKER-CARD format>
open: <deviations from this blueprint, each with the line it deviates from>
```
