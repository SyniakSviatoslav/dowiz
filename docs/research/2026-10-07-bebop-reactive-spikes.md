# Reactive spikes on an acyclic graph instead of loops: how far it goes, and what it buys bebop

Status: 2026-10-07, lane R-SPIKE (research only; nothing under /root/dowiz was edited). Box: Android/proot aarch64,
core 4 = Cortex-A78, governor walt, frequency moved 1.34-2.11 GHz during the runs (read every ratio, not the absolute
ms; each RES line's MHz is in the raw log). `bebop.bin` md5 `8a0b4325`. Toolchains: gcc 15.2 `-O2 -march=armv8.2-a`,
python3 oracle (`-I`), bebop self-hosted compile (`seed/build/seed ./bebop.bin compile`). Every number is **MEASURED**
(probe + output line quoted), **RECORDED** (a number already in the tree, file named), **CITED** (external source) or
**ESTIMATE** (arithmetic on named inputs, said so). Probe sources and raw logs: `S/rspike/probes/*.c`, `*.bp`,
`S/rspike/results/raw.txt`, `raw2.txt`, `oracle.txt`, `dis/*.dis` (S = scratchpad of session 4384f304). Every heavy run went
through `tools/slot.sh`, `taskset -c 4`, median of 5. **Perf counters are NOT MEASURED**: `perf_event_open` returns EACCES
under this proot (`PERF_UNAVAILABLE errno=13 (Permission denied)`, probes/perfprobe.c), so mispredict rates are inferred
from timing differentials against a predictable-branch control.

Operator's question (Ukrainian, paraphrased): *instead of loops in the kernels, use reactive spikes on an acyclic graph.
How far is that possible, and if it is, what speed-up do we get by replacing loops entirely?*
Scope addition (coordinator, same day): four more loop alternatives -- full predication / bitmask streams, threaded code /
computed gotos, semiring (GraphBLAS) reduction, loop-free chunk pipelines -- judged on K1..K8 and on dowiz itself (§7).

---

## 0. The answer in twelve lines

1. **"Spikes on a DAG instead of loops" is five different ideas wearing one name** (§1). Only two of them can make a
   cold run faster, and neither is "spikes": **algebraic elimination** of the loop (a closed form; reading d) and
   **reuse through sharing** (memoisation over the DAG; reading b's degenerate case). Event-driven propagation itself
   (readings b, c) never does less work on a cold run and always pays a dispatch tax.
2. **Work is conserved** (§2). A loop-carried recurrence `s_{i+1} = f(s_i, i)` is a *chain*: node i+1 depends on node i, so a
   change anywhere re-fires everything downstream. There is no sparsity to exploit; a spike engine on a chain is the
   loop plus a queue. **MEASURED: 3.2x slower** than the loop on K4 (9.69 vs 2.99 ms per 2M steps) and on K8H (74.2 vs
   23.5 µs per 20k steps) and **3.6x slower on K1H** (4.20 vs 1.16 ms per 1M steps; 4.0x with a per-node function pointer).
3. **The dispatch tax on this A78 is 3.7-4.8 ns per in-cache spike** (queue push/pop + change test + stores; MEASURED
   4.84 ns/node on K4, 3.71 on K8H) against **~1-1.6 ns per iteration** for the counted loop (K1H 0.98, K4 1.49, K3H 1.63).
   The earlier dowiz substrate measured the same thing: 41x on dense work, crossover 0.39 % change
   (`bench/substrate_spike/RESULT.md`, RECORDED). Out of cache a spike costs ~90 ns (one miss per node: §2.3).
4. **Where event-driven propagation wins: incremental re-runs of *tree-shaped* reductions with a small change set.**
   MEASURED on a 1M-element sum (§5.4): 1 changed leaf **400x** faster than the dense loop, 16 leaves 57x, 256 leaves 11.6x,
   1024 (0.1 %) 5.6x, **1 % changed: 0.38x -- slower than recomputing**, 10 %: 0.18x. Crossover ≈ 0.3 % of the input, the
   same 0.39 % the 2026-09 substrate found. **At the operator's "1 % changed" a spike engine loses to the dense loop.**
5. **Algebra beats both.** For a linear fold the delta is `Σ Δa[j]·w_j`: O(k). MEASURED 1 % change: 6.5 µs vs 0.80 ms
   dense (**124x**) vs 2.39 ms spike (**370x**). K1H's fold is linear in its inputs (`s = Σ a[j]·3^(N-1-j)`), so the same O(k)
   delta applies to it: MEASURED 1 % change 62.6 µs vs 1.02 ms full recompute (**16x**) vs 3.12 ms for the spike engine, which
   re-fired **999,998 of 1,000,000 nodes** (§5.1).
6. **Closed forms delete the loop entirely, for the kernels that have one.** K1H, K3H, K4 are affine recurrences in state
   (s, i, 1); their N-step map is a 3x3 (K3H: 4x4) matrix power over Z/2^64 -- O(log N). MEASURED in C: K4 2.99 ms →
   0.88 µs (**3,400x**), K3H 147 µs → 1.5 µs (**98x**). MEASURED in bebop itself: `k1h_closed.bp` runs the 1M-step K1H in
   **~1.45 µs vs 1.0 ms** for `bench630/k1ht.bp`'s loop (**~690x**), and agrees with the loop bit-for-bit over 20 carried
   reps (`BPCHECK k1h_closed_check -12017413204599424`, which is the agreed value, not the -1 disagreement code). LLVM's
   SCEV does this only for *polynomial* recurrences (K1 plain: yes -- that is why the `h` twins exist; K1H/K3H/K4: no,
   they are geometric). This is the one piece of the idea the bebop compiler does not have yet.
7. **K2H (fib) is the DAG-sharing case**: the recursion tree has 242,785 nodes but only 26 distinct ones. Sharing
   (memo, bottom-up) 0.2215 ms → 15 ns (**14,700x**); fast doubling 11 ns (**20,000x**); a spike engine over the *shared*
   DAG 156 ns (1,400x -- but 10x slower than the plain loop over the same 26 nodes). This is reuse, not spikes.
8. **K8H cannot be eliminated**: `acc` sums a data-dependent choice over every LCG state; the chain must be walked. What
   matters there is the branch, and **bebop already emits `csel`** (MEASURED objdump, §7.1: one `csel x19, x0, x1, eq` in a
   9-word loop at 0x4c-0x6c, no data-dependent `b.cond`). gcc's forced-branch version is 3.35x slower than csel (78.7 vs
   23.5 µs/rep); bebop's hot K8H row (0.024 ms/rep, RECORDED) *is* the csel number.
9. **The four extra alternatives (§7)**: predication -- done where it matters (csel); explicit bitmasks are **1.28x slower**
   than csel on K8H; threaded code -- an indirect `br` with a static target costs **0.00x** on the counted loop (0.978 vs
   0.983 ms: the back-edge is already free) and threaded dispatch of a VM is only 1.34x better than `switch`, 9x worse
   than straight code; semirings -- the right *name* for dowiz's folds, no speed-up at 165 rows; chunk pipelines -- a
   cache optimisation for data larger than L2, which no kernel and no dowiz request touches.
10. **Direct answer to "bitmasks or a graph of computed jumps?": bitmasks (csel) -- and bebop has them.** The jump graph
    is a model of control, and control is not where the time is: every K1..K8 back-edge is predictable, the one
    unpredictable branch is already a csel, and the measured cost of removing the loop header is zero.
11. **For dowiz** (§7b): the request path is microseconds of integer work over ≤ 165 items inside a 10 ms CPU cap; the
    time is in JSON parsing (6.33 ms per catalogue parse, RECORDED) and byte-serial decode/FNV loops, not in any loop a
    spike or a mask would change. The one loop family whose *shape* rewards this thinking is the projection fold, and it
    is already incremental by the write (`fold_step` 254x, RECORDED). Concrete rows with file:function in §7.6.
12. **Recommendation (§8)**: **GO** on SCEV-style closed-form elimination of affine loops in `emit_while_stmt`
    (measured 98-3,400x on three of five hot kernels, O(log N) code, zero runtime cost, bebop-side proof exists);
    **NO-GO** on an event-driven/spiking execution model for kernels (3.2x slower cold, loses above 0.3 % change);
    **NO-GO** on a DBSP-like general runtime for dowiz (already incremental by the write at the right granularity; dense
    sets are ≤ 165 rows); **GO-small** on O(k) algebraic deltas + early cutoff for counted aggregates (Z-set weights,
    already queued as AX1/AX3); **NO-GO** on masks/threaded code/semiring engines/chunk pipelines for the kernels.

---

## 1. What "reactive spikes on a DAG instead of loops" can mean

| reading | precise statement | possible on an AArch64 CPU? | what it changes |
|---|---|---|---|
| **(a) full unrolling** into a static acyclic dataflow graph | replace `while` by N copies of the body, each a node with an edge to the next | only for a **known, bounded** trip count; a data-dependent trip count has no finite acyclic graph (§2.1). K1H unrolled = 1M nodes ≈ 4 MB of code against a 64 KB I-cache: it runs *slower* than the loop | removes the counter and the back-edge -- worth 0.00x, MEASURED §7.2 |
| **(b) event-driven / spiking propagation**: a node fires only when an input changed (self-adjusting computation, Adapton, DBSP, differential dataflow, Salsa, signals) | a dependency graph with a change queue; cold run = fire everything; re-run = fire the dirty cone | yes; a library/runtime pattern, not an ISA feature | cold: the same work + dispatch tax; re-run: work ∝ size of the dirty cone |
| **(c) spiking-neural-network execution** | neurons with thresholds, events on spike crossings, time-stepped or event-driven | yes as simulation; the arithmetic is float / leaky-integrate, approximate, trained | a *different* computation; not a semantics-preserving replacement for an integer kernel. The SNN literature's own finding: event-driven beats time-stepped only at low firing rates (§3 G) |
| **(d) algebraic elimination**: recurrence → formula; linear recurrence → matrix power | `s_N = M^N s_0` for affine maps; sums → closed forms; SCEV exit-value rewriting | yes; pure integer arithmetic over Z/2^64 | O(N) → O(log N) or O(1); only for linear/affine/polynomial recurrences with no data-dependent control |
| **(e) DAG of SIMD / parallel tasks** | split the trip space into independent tasks, run on cores or lanes | yes for *reductions* (associative) and maps; **no for a chain** (each step needs the previous one) | K1H..K4, K8H are chains: no parallelism at all; a sum is a reduction: NEON 2 lanes of i64, 3 big cores (DG6 2.5-2.7x on ≥ 40 ms tasks, RECORDED) |

The operator's phrase most naturally means (b) with the vocabulary of (c). Readings (b) and (c) are *execution models*
for the same work; (d) and sharing are the only readings that *remove* work. §4 says which applies to each kernel.

## 2. Theoretical limits

### 2.1 A data-dependent trip count has no finite acyclic graph
`while cond(state) { state = f(state) }` with `cond` unknown at build time needs a cycle (or an unbounded graph). Every
"loop-free" representation either (i) bounds N statically (K1..K4, K8 do: N is a literal), (ii) keeps one cycle in a
scheduler (an event queue drained by a `while` -- the loop moved, it did not vanish), or (iii) eliminates the recurrence
algebraically. K5's `mpow` (`while cont == 1`), K7's nearest-key search and the compiler's own parser loops are
data-dependent and fall under (ii) or (iii) only.

### 2.2 Work is conserved on a cold run
A single evaluation must perform the operations the computation requires unless it (a) reuses results already computed
(sharing / memoisation; valid when subcomputations repeat -- fib), or (b) uses algebra (a shorter derivation of the same
value -- the closed form). A spike engine does neither on a cold run: every node fires once, so it performs *at least*
the loop's work plus a dispatch cost per node. CITED: self-adjusting computation's from-scratch overhead is 3.4-18.4x
(Hammer et al. 2011, Table 2) and 1.7-7.3x in the parallel version (Anderson et al. 2021 §6.1); classic Adapton is
"sometimes orders of magnitude slower than from-scratch computation" (Hammer et al. 2015 §6); and even maintaining a
prefix sum under updates costs Ω(log n) per operation in the cell-probe model (Pătraşcu & Demaine 2006) -- incrementality
is never free, not even asymptotically.

### 2.3 What a spike costs here, and what a loop iteration costs
MEASURED (probes/k4.c, k8h.c, dispatch.c, sum.c; medians of 5 pinned runs, core 4, 1.9-2.1 GHz):

| per-element cost | ns | source line (results/raw.txt, median run) |
|---|---|---|
| counted loop iteration, K1H body (`madd`, `sub`, `cmp`, `b.gt`) | **0.98** | `RES k1h_counted_loop 19.66 20` → 0.983 ms / 1M |
| counted loop iteration, K4 body | **1.49** | `RES k4_loop 29.47 10` → 2.99 ms / 2M |
| counted loop iteration, K3H body | **1.63** | `RES k3h_loop 28.50 200` → 0.147 ms / 90k |
| spike over the unrolled chain, K4 body (queue, change test, store) | **4.84** | `RES k4_spike_cold 95.12 10` → 9.69 ms / 2M |
| spike over the unrolled chain, K8H body (two values per node) | **3.71** | `RES k8h_spike_cold 222.66 3000` → 74.2 µs / 20k |
| spike over the unrolled chain, K1H body | **4.20** | `RES k1h_spike_fnptr`-adjacent: `RES k1h_spike_cold 84.0 20` → 4.20 ms / 1M (results/raw3.txt) |
| spike with a per-node function pointer (a generic reactive node), K1H | **4.65** | `RES k1h_spike_fnptr 93.0 20` → 4.65 ms / 1M |
| `switch` dispatch per VM op (8 opcodes, 4096-op program) | **3.72** | `RES vm_switch 30.45 2000` / 4096 |
| computed-goto threaded dispatch per VM op | **2.77** | `RES vm_threaded_goto 22.34 2000` / 4096 |
| the same 4096 ops as straight-line native code | **0.30** | `RES vm_native_straightline 2.46 2000` / 4096 |
| tree-spike per fired node, 16 MB tree out of L2 (1 changed leaf, 20 levels) | **≈ 92** | `RES sum_tree_spike_k1 0.0369 20` → 1.85 µs / 20 |
| one branch mispredict on the A78, inferred (perf counters NOT MEASURED) | **≈ 5.9 (≈ 12 cycles)** | (78.7 − 19.5 µs) / 10,000 mispredicts per rep: `k8h_branch` vs `k8h_predictable_branch_CONTROL` |

So the floor for a spike is 3.7-4.8 ns in-cache (3-5x a loop iteration) and ~90 ns when the node table does not fit in
L2 -- and a reactive runtime's node table is exactly the pointer-chasing structure that misses (the NEST authors measure
spike delivery at 70-80 % of simulation time for the same reason: Pronold et al. 2022, CITED).

### 2.4 Where a speed-up can come from, stated plainly
1. **Algebraic elimination** (closed form, matrix power, SCEV exit values): O(N) → O(log N) / O(1); affine or polynomial
   recurrences with no data-dependent control. Cold *and* warm. **This is the only reading that speeds up a cold run of
   K1H/K3H/K4.**
2. **Sharing** (memoisation of repeated subcomputations): fib-shaped recursion; applies when the call DAG has far fewer
   distinct nodes than the call tree. Cold and warm.
3. **Incrementality** (re-run after a small change): work ∝ dirty cone; pays only when the cone is small -- tree-shaped
   dependencies with a change fraction under the crossover (~0.3 % here), never on a chain.
4. **Sparsity** (most inputs are zero / most events do nothing): event-driven simulation's actual niche (an SNN fires
   ~1 % of its neurons per step; Brette's cost model, §3 G).
5. **Parallelism** (independent tasks): reductions and maps only, ≥ 40 ms per task on this box (DG6, RECORDED).

None of these is "spikes"; the spike is the delivery mechanism of (3) and (4), and it costs §2.3's tax.

## 3. Literature, each with a number

Legend: CITED = fetched primary text or official docs; CITED(abstract) = abstract or project page only; UNVERIFIED =
could not be fetched, stated for completeness. Full URLs in §9.

| line | citation | the number | what it says about spikes-instead-of-loops |
|---|---|---|---|
| **A. Self-adjusting computation** | Acar, Blelloch, Blume, Tangwongsan, PLDI 2006 / TOPLAS 32(1) 2009 | "orders of magnitude faster" on small modifications (abstract); the PLDI'06 from-scratch factor UNVERIFIED | the founding statement of reading (b) |
| | Hammer, Neis, Chen, Acar, "Self-Adjusting Stack Machines", OOPSLA 2011 (arXiv 1108.3265) Table 2 | **from-scratch overhead 3.4x-18.4x** (sum 9.6x, map 18.4x, quicksort 8.2x); change propagation 6.9·10² to 4.9·10⁴ faster per update | the exact price of turning a loop into a tracked graph; payoff only for k ≪ n |
| | Anderson, Blelloch, Baweja, Acar, "Efficient Parallel Self-Adjusting Computation", arXiv 2105.06712 §6.1 | initial-run overhead **1.7x (hashing) to 7.3x (tree contraction)**; work savings 9x (10⁵ changes) to 1,180x (1 change) | the parallel variant pays the same tax |
| **B. Adapton** | Hammer, Khoo, Hicks, Foster, PLDI 2014 | "reliable speedups ... dramatically outperforms state-of-the-art IC" (abstract); own tables UNVERIFIED | demand-driven (pull) sibling of push spikes |
| | Hammer et al., "Incremental Computation with Names", OOPSLA 2015 (arXiv 1503.07792) §6, Table 1 | up to **10,900x** over from-scratch; but classic Adapton "**sometimes orders of magnitude slower than from-scratch**", eager map "one to two orders of magnitude" slower | the granularity warning in numbers: per-element thunks lose |
| **C. DBSP** | Budiu, Chajed, McSherry, Ryzhyk, Tannen, PVLDB 16(7) 2023, best paper (arXiv 2203.16684) | Q^Δ = D ∘ Q ∘ I; linear/bilinear operators incrementalise at cost **∝ |Δ|** (Thm 3.3/3.4); eval tables UNVERIFIED | the algebra behind reading (b): work ∝ change size, *for linear operators* |
| | Feldera blog, Nexmark vs Flink, 2024-09-10 | up to **6.2x** faster than Flink, geomean 2.2x, memory geomean 0.24x (100M events, 64-core box) | incremental engine vs incremental engine; not batch vs incremental |
| **D. Differential / timely dataflow** | Murray et al., Naiad, SOSP 2013; MSR project page | coordination "< 1 ms on 64 machines"; Twitter-volume SCC "sub-second latency" | the system that made reading (b) real at scale |
| | TimelyDataflow/differential-dataflow README | 10M nodes/50M edges degree count: load **15.47 s**, one-edge update **228 µs** (≈ 6.8·10⁴x), 100k batched updates **505 ms (≈ 5 µs each)** | per-spike cost ~230 µs unbatched, ~5 µs batched; needs a 15 s indexed warm-up |
| | McSherry, Isard, Murray, "Scalability! But at what COST?", HotOS 2015 | Naiad needs **16 cores** to beat one competent thread; GraphLab 512; GraphX never | a tight loop on one core is the baseline every graph engine must beat |
| **E. Salsa / rustc incremental** | Woerister, Rust blog 2016-09-08 | best case rebuild **16-22 %** of full; worst case **3 %** faster; first incremental compile *slower* than non-incremental | dependency tracking costs on the cold run; wins only with coarse, stable nodes |
| | rust-analyzer analysis-bench on chalk (dev docs sample, UNVERIFIED snippet) | from scratch 8.49 s, no change 445 µs, **trivial change 95.6 ms** (validation walk, zero recompute) | the cost of walking a query DAG to find nothing dirty |
| **F. Reactive runtimes** | Elliott & Hudak, Fran, ICFP 1997; Meyerovich et al., Flapjax, OOPSLA 2009 | no overhead numbers in fetched material (UNVERIFIED) | — |
| | krausest js-framework-benchmark; milomg js-reactivity-benchmark; "Super Charging Fine-Grained Reactive Performance" | benchmark unit is **updated reactive elements per ms** (i.e. ~µs per node); authors: "plenty fast for typical applications" | signal graphs run at ~1 µs per node, 10³x an ALU op: nodes must be coarse |
| **G. SNN on CPU** | Brette et al., J. Comput. Neurosci. 23 (2007), arXiv q-bio/0611089 §1.3-1.4 | clock-driven cost `c_U·N/dt + c_P·F·N·p`; event-driven `(c_U+c_S+c_Q)·F·N·p`; event-driven wins only when **F·p·dt ≪ 1**; cortex example F·p·dt = 1 | the formal sparsity condition; dense activity → the time-stepped loop wins |
| | Hanuschkin et al., Front. Neuroinform. 4:113 (2010) §3.2 | time-driven **2.5x faster** than embedded event-driven on 12,600 neurons | measured: the loop beat the spikes |
| | Stimberg, Brette, Goodman, Brian 2, eLife 2019 | "dramatically faster" with homogeneous neurons via vectorised code; loses the advantage with heterogeneous parameters; GPU 45x on one example | Brian 2's speed comes from *vectorising the loop*, not from events |
| | Jordan et al., Front. Neuroinform. 12:2 (2018); Pronold et al., Front. Neuroinform. 15 (2022) | 1 s biological = 16 s wall (10⁶ neurons); **spike delivery = 70-80 % of simulation time**; refactoring it −40 % | the spike mechanism itself is the bottleneck of the best CPU kernel |
| | Knight & Nowotny, Front. Neurosci. 12:941 (2018) | 77k neurons / 0.3·10⁹ synapses, 10 s: NEST 12 nodes ≈ 70 s, V100 ≈ 22 s, SpiNNaker ≈ 200 s; energy/synaptic event NEST 4.4 µJ, K40c 1.08 µJ, SpiNNaker 5.9 µJ | event hardware did not win on speed or energy here |
| | Yavuz, Turner, Nowotny, GeNN, Sci. Rep. 6:18854 (2016) | GPU 4.7-10.8x one CPU core on Izhikevich; speed-up falls as spike traffic rises | more spikes = less speed-up |
| | Zenke & Gerstner, Front. Neuroinform. 8:76 (2014) | "> 1/10 real time is difficult"; event-based trace updates pay off "only at low firing rates" | same condition from a third group |
| | Davies et al., Loihi, IEEE Micro 2018 (via Cılasun et al. arXiv 2006.03007 Table 5); Davies et al., Proc. IEEE 2021 | **3.5 ns / 23.6 pJ per spike op**; "conventional feedforward deep neural networks show modest if any benefit on Loihi" | even purpose-built spike silicon pays ~7 cycles per spike and gains nothing on dense work |
| **H. Polyhedral / dataflow machines** | Bondhugula et al., Pluto, PLDI 2008 (most influential 2018) | "very high speedups" via affine re-timing; table UNVERIFIED | the winning move: keep the loop, retime it algebraically |
| | Culler, Schauser, von Eicken, UCB/CSD-92-716 (1992) | dataflow's "local greedy scheduling policy ... is inadequate" under a realistic storage hierarchy | why fine-grain token dataflow lost: memory, not ALUs |
| | Swanson et al., WaveScalar, MICRO 2003 / TOCS 2007; Gebhart et al., TRIPS, ASPLOS 2009 | simulated 2-7x; silicon: TRIPS "Core 2 outperforms compiled TRIPS code in most cases", matches a Pentium 4 | the per-instruction-spike machine did not beat the superscalar loop |
| **I. Closed forms in compilers** | LLVM Passes (indvars, scalar-evolution, loop-deletion); IndVarSimplify.cpp `ReplaceExitValue = OnlyCheapRepl` | exit values of add-recs rewritten outside the loop; loops with computable trip count and no live result **deleted**; count statistic UNVERIFIED (`-mllvm -stats`) | LLVM already does reading (d) for polynomial recurrences |
| | GCC `-ftree-scev-cprop` (on at -O1) | "final value replacement ... provided it is sufficiently cheap" | same, in GCC |
| | Bachmann, Wang, Zima, ISSAC 1994; Van Engelen, CC 2001; Gries & Levin, IPL 1980 | chains of recurrences; Fibonacci in O(log n) by matrix squaring | the theory behind §5's matrix power |
| **J. Task-graph overhead** | Dask docs | threaded scheduler **~50 µs/task**; distributed **~1 ms/task**; "functions faster than 100 ms ... might not see any speedup" | a scalar spike is 10⁴-10⁵x too small a node |
| | oneTBB Developer Guide | grainsize ≥ **100,000 cycles** per chunk | same floor from the C++ runtime |
| | Frigo, Leiserson, Randall, Cilk-5, PLDI 1998 | spawn = **2-6 C function calls** (the best case ever measured) | even the cheapest task is ≥ 10 ns |
| **K. Work bounds** | Blelloch, CACM 39(3) 1996; Pătraşcu & Demaine, SIAM J. Comput. 35(4) 2006 | work/depth model; **Ω(log n) per update** for dynamic partial sums in the cell-probe model | no incremental scheme beats one evaluation's work without reuse or algebra; per-update cost is never O(1) once results are maintained |

## 4. Per-kernel verdict

"Cold" = one run from scratch; "incr 1 %" = re-run after 1 % of the *input* changed. Most hot kernels have no array
input -- their only input is the literal N -- so "1 % changed" is read as: K1H/K3H/K4/K8H with 1 % of their N inputs
(the fold form, `a[j]`) changed; K2H with the argument changed; K5-K7 with 1 % of their array cells changed.
Speed-ups are relative to the kernel's own loop as measured here (C, gcc -O2) or RECORDED (bebop hot row).

| kernel | shape | reading that applies | best replacement | cold single run | incr, 1 % of input changed | basis |
|---|---|---|---|---|---|---|
| **K1** sum 1..N | polynomial add-rec | (d) SCEV | `N(N+1)/2` | **~10⁶x** (O(1)) | O(1) | LLVM already does it (RECORDED: that is why K1H exists) |
| **K1H** `s = 3s + i` | affine chain | (d) matrix power; fold form also linear → O(k) delta | 3x3 `M^N` over Z/2^64 | **~690x in bebop** (1.0 ms → ~1.45 µs); **1,165x in C** (1.165 ms → 1.00 µs), MEASURED §5.1 | fold form: O(k) delta **16x** (62.6 µs vs 1.02 ms); spike **0.33x** -- 999,998 of 1M nodes re-fire, MEASURED | §5.1 |
| **K2H** fib(25) | shared DAG, 242,785 calls / 26 distinct | sharing (memo) or (d) fast doubling | bottom-up DP or doubling | **14,700x / 20,000x** (0.2215 ms → 15 / 11 ns, MEASURED) | memo of fib(25): O(1) hit; argument change ±1: one more node | §5.3; spike over the shared DAG 156 ns = 10x slower than DP |
| **K3H** nested 300×300 | affine chain, two nested counters | (d) 4x4 affine composition (inner^300 then outer^300) | matrix power | **98x** (147 µs → 1.5 µs, MEASURED) | n/a (no array input); fold form: O(k) | §5.2 |
| **K4** `v = (v+7i)·3 − 11` | affine chain | (d) matrix power | 3x3 `M^N` | **3,400x** (2.99 ms → 0.88 µs, MEASURED) | fold form: O(k) | §5.2; spike cold **0.31x** (3.2x slower) |
| **K5** NTT 512, `mpow` | data-dependent `while`, array butterflies | (e) for the butterflies (a *fixed* DAG of 2,304 butterflies — the FFT literally is a DAG); (d) for `mpow` → precomputed twiddle table | tabulate `w`, keep the loop | ESTIMATE 2-5x from *removing the mpow recomputation* (two `mpow` per butterfly today), not from spikes | 1 % of inputs changed → every output changes (the DFT is dense): spike 0 gain; recompute | shape read from kernels/k5.bp |
| **K6** HV bind + popcount, 16 words | tiny reduction | (e) NEON `cnt`/`addv` (the `hvham2` builtin exists, RECORDED) | builtin | ESTIMATE ≤ 2x; the kernel is ~16×12 ops | 1 % of 16 words = 1 word: delta popcount O(1) — trivial, but the whole kernel is ~100 ns | §7.3 |
| **K7** VSA nearest key, 8×16 words | reduction + argmin | (e) NEON Hamming (`hvham2`); argmin is a data-dependent `csel` chain | builtin + csel | ESTIMATE ≤ 2x | 1 % of memory words changed → recompute the affected key's distance only: O(k) with a per-key distance cache (16x less work, ESTIMATE) | §7.3 |
| **K8H** LCG + 50 % branch | chain with data-dependent select; no closed form for `acc` | none of (a)-(e) removes work; predication already applied | keep loop + `csel` (done) | **1.0x** (bebop 0.024 ms/rep = C csel 0.0235, RECORDED/MEASURED); spike cold **0.32x**; branchy **0.30x** | seed change → everything changes: 0 gain | §5.5, §7.1 |

Bottom line of the table: **three of five hot kernels (K1H, K3H, K4) vanish under algebra (98x-3,400x); one (K2H) vanishes
under sharing; one (K8H) cannot vanish and is already at the predication optimum.** No kernel gains from event-driven
propagation: the chains re-fire entirely, the small reductions are already in registers.

## 5. Measurements

Harness: `probes/run.sh` and `run2.sh` under `tools/slot.sh rspike ...`; `taskset -c 4`; 5 runs per binary; medians by
`probes/median.py`; every result value identical across the 5 runs (the `result` column) and checked as below.
Oracles: (i) the Python loop semantics over Z/2^64 (`probes/oracle.py -I`, results/oracle.txt): `ORACLE k1h 18434726660504952192`
(= C loop = C closed-form CHECK = bebop loop = bebop closed form, which printed the same value as signed i64 −12017413204599424),
`ORACLE k4 16575662372173031809`, `ORACLE k3h 2875824312501384256`, `ORACLE k1h_arr_full 6579524405676188762` (= the O(k) delta's
result), `ORACLE sum_base 9233781064130140587` (= every sum version); K8H's 3000-rep value is checked by the agreement of four
independently written versions (branch, csel, mask, spike), the Python one-rep oracle `k8h_one 2455774231069866078` was not
compared against a C one-rep run;
(ii) cross-agreement of independently written versions (loop vs spike vs closed vs mask) within each probe;
(iii) fib = 75025; (iv) the bebop closed form vs the bebop loop inside one binary (`k1h_closed_check.bp`).

### 5.1 K1H: loop vs spike vs spike-with-function-pointer vs closed form; fold form incremental (probes/k1h.c; raw3.txt, raw5.txt)

| version | ms per run (1M steps) | vs loop | result / check |
|---|---|---|---|
| loop `s = 3s + i` (register form) | **1.165** | 1.00x | 18434726660504952192 = Python oracle |
| spike, unrolled chain, cold (queue + change test) | 4.200 | **0.28x** (3.6x slower; 4.2 ns/node) | same |
| spike with a per-node function pointer (generic reactive node) | 4.650 | **0.25x** (4.0x slower; 4.65 ns/node) | same |
| closed form, 3x3 matrix power (20 squarings) | **0.00100** | **1,165x** | `CHECK k1h_closed_20reps 18434726660504952192` = oracle |
| fold form `s = 3s + a[j]`, full recompute after 1 % of `a` changed | 1.018 | 1.00x (ref) | delta result 6579524405676188762 = `ORACLE k1h_arr_full` |
| fold form, O(k) algebraic delta `Σ Δa[j]·3^(N-1-j)` (k = 10,000) | **0.0626** | **16x** faster than recompute | same |
| fold form, spike engine re-run (topological sweep, dirty bits, early cutoff) | 3.118 | **0.33x** -- 3.1x slower than recomputing | `FIRED avg_nodes_fired_per_rerun=999998 of 1000000` |

The chain result in one line: with 1 % of the inputs changed at random positions, the earliest change sits at index ≈ 100
and every node after it changes value, so early cutoff never fires and the "incremental" engine does 99.9998 % of the
cold work at 3x the per-node price. (A FIFO queue on a chain is worse still -- quadratic, each node re-fires once per
upstream spike; the first version of this probe died of that and was replaced by the topological sweep.) The O(k) delta
is only 16x, not 100x, because each of the 10,000 terms is a random read into an 8 MB `3^t` table (MEASURED 124 ns per
dependent random step on 8 MB, `STREAM randwalk`, raw2.txt) -- the algebra wins, memory sets its ceiling.

### 5.2 K4 and K3H: loop vs spike vs closed form (probes/k4.c, k3h.c)

| version | ms per run | vs loop | result |
|---|---|---|---|
| K4 loop, 2M steps | **2.988** | 1.00x | 16575662372173031809 (all 5 runs) |
| K4 spike, unrolled chain, cold | 9.690 | **0.31x** (3.2x slower) | same |
| K4 closed form, 3x3 matrix power (21 squarings) | **0.000875** | **3,414x** | `CHECK k4_closed_10reps 16575662372173031809` = loop = oracle |
| K3H loop, 300×300 | **0.1467** | 1.00x | 2875824312501384256 |
| K3H closed form, 4x4 inner^300 then outer^300 | **0.00150** | **98x** | `CHECK k3h_closed_200reps 2875824312501384256` = loop = oracle |

(The first batch timed the closed forms at 1000x/100x the loop's rep count, so their *carried* results differed from the
loop's by construction -- that was a harness error, not a disagreement; batch 2 adds a `CHECK` at equal rep counts.)

### 5.3 K2H fib(25): recursion vs shared DAG vs doubling vs spike (probes/fib.c)

| version | ns per fib(25) | vs recursive | result |
|---|---|---|---|
| recursive, one real call per node (gcc; bebop 576 µs, Rust 299 µs RECORDED) | 221,518 | 1.00x | 75025 |
| shared DAG, bottom-up (26 nodes) | **15** | 14,700x | 75025 |
| closed form, fast doubling (O(log n)) | **11** | 20,000x | 75025 |
| spike engine over the shared DAG (count-in-degree, queue) | 156 | 1,400x; **10x slower than the plain loop over the same 26 nodes** | 75025 |

### 5.4 Reduction with a small change set: dense loop vs tree-DAG spikes vs algebraic delta (probes/sum.c)
N = 2²⁰ = 1,048,576 i64; k changed leaves per re-run; the tree engine has early cutoff (a node that does not change stops
the spike) and dirty bits; every version returns the same checksum 9233781064130140587 after each even number of reps.

| k changed (fraction) | dense loop recompute, ms | tree-spike re-run, ms | algebraic delta `Σ Δ`, ms | spike vs dense | delta vs dense |
|---|---|---|---|---|---|
| 1 (10⁻⁶) | 0.724 (live `sum_dense_loop` 0.667) | **0.00185** | 0.000002 | **390x** | 4·10⁵x |
| 16 | 0.732 | 0.0130 | 0.000009 | 56x | 8·10⁴x |
| 256 (0.02 %) | 0.753 | 0.0645 | 0.00015 | 11.7x | 5,000x |
| 1,024 (0.1 %) | 0.743 | 0.134 | 0.0006 | 5.5x | 1,200x |
| **10,486 (1 %)** | 0.918 | **2.385** | **0.0065** | **0.38x (spike LOSES)** | **142x** |
| 104,858 (10 %) | 2.862 | 15.70 | 0.067 | 0.18x | 43x |

Crossover for spikes ≈ 0.3 % of the input (between the 1,024 and 10,486 rows) -- the 2026-09 substrate found 0.39 % on
a 65,536-cell graph (`bench/substrate_spike/RESULT-incr.md`, RECORDED). Two independent engines, two sizes, one answer.

### 5.5 K8H: branch vs csel vs mask vs spike, and the predictable-branch control (probes/k8h.c)
3000 reps × 20,000 steps = 60M steps; acc carried; all four semantics-preserving versions agree (10191605407255100256).

| version | µs per rep (20k steps) | ns/step | vs csel | objdump of the gcc function |
|---|---|---|---|---|
| forced conditional branch (asm barriers in both arms) | 78.72 | 3.94 | 0.30x | `bcond_count=4, csel_count=0` |
| compiler if-conversion (gcc chose csel) | **23.53** | **1.18** | 1.00x | `csel_count=1, bcond_count=2` (the loop test) |
| explicit bitmask select `(x & m) \| (−i & ~m)` | 30.19 | 1.51 | **0.78x** | `csel_count=0`; 3 extra ALU ops on the dependency chain |
| spike engine over the unrolled chain | 74.22 | 3.71 | 0.32x | — |
| CONTROL: branch on a predictable bit `(i>>4)&1` | 19.55 | 0.98 | (different result by design) | — |
| bebop hot row, RECORDED (honest.sh 2026-10-07) | 24 | 1.2 | ≈ 1.0x | bebop emits `csel x19, x0, x1, eq` (§7.1) |

Inferred mispredict cost: (78.72 − 19.55) µs / 10,000 mispredicts ≈ **5.9 ns ≈ 12 cycles** at 2.05 GHz, consistent with
the A78 pipeline. Perf counters NOT MEASURED (EACCES).

### 5.6 Threaded code vs counted loop vs switch (probes/dispatch.c)

| version | ms | per element | vs baseline |
|---|---|---|---|
| K1H as a counted loop (1M) | 0.983 | 0.98 ns | 1.00x |
| K1H as 16 blocks linked by `goto *next[k]` (indirect `br`, static targets, no loop header) | **0.978** | 0.98 ns | **1.00x** -- removing the loop header buys nothing |
| VM: `switch` dispatch, 8 opcodes, 4096 ops × 2000 | 30.45 | 3.72 ns/op | 1.00x |
| VM: computed-goto threaded dispatch | 22.34 | 2.77 ns/op | 1.36x |
| VM program as straight-line native code (what a compiler emits) | **2.46** | **0.30 ns/op** | **12.4x** |

### 5.7 bebop-side proof (probes/k1h_closed.bp, k1h_closed_check.bp; results/raw.txt `BP` lines)
`k1h_closed.bp` = the 3x3 matrix power written in bebop (`if` as an expression, `>>` logical on a positive exponent, arrays
via `zeros(9)`); compiled with `bebop.bin 8a0b4325`.

| | total ms (5 runs) | per evaluation | core MHz |
|---|---|---|---|
| `bench630/k1ht.bp` loop, 100 reps × 1M | 99, 100, 100, 101, 100 | **1.00 ms** | 1.34-2.05 GHz |
| `k1h_closed.bp`, 20,000 evaluations | 28, 23, 30, 30, 34 | **1.15-1.70 µs** (median 1.5) | 1.90-2.05 GHz |
| agreement: `k1h_closed_check.bp` returns loop value if `s == c` else −1 | `BPCHECK k1h_closed_check -12017413204599424` | agree (20 carried reps) | |

**~690x in bebop with today's compiler and no new codegen** -- the closed form is ordinary bebop code (≈ 21 squarings ×
(27 mul + 18 add) ≈ 1,000 ops). A compiler pass that emits it needs no new instruction kinds.

## 6. Why the idea fails where it fails, and where it works -- in one paragraph each

**Fails on chains.** Every hot kernel except K2H is `s ← f(s, i)`: node i+1 has exactly one predecessor, node i. The
dependency "graph" is a path; its dirty cone from any node is the whole suffix. Event-driven evaluation of a path is
the loop with a queue bolted on: same N firings, 3-5x the per-step cost (§2.3, §5.2, §5.5). Re-runs do not help: the first
changed input invalidates everything after it (MEASURED §5.1: K1H fold with 1 % changed re-fires 999,998 of 1M nodes, 3.1x slower than recomputing).

**Fails on dense reductions above ~0.3 % change.** The tree engine's per-node cost (~90 ns out of cache, ~5 ns in cache)
times k·log₂N firings crosses the dense loop's 0.75 ns/element at k/N ≈ 0.3 % (§5.4). The operator's "1 % changed" sits
on the losing side by 2.6x.

**Works as algebra.** Affine recurrences have a closed form; linear folds have an O(k) delta; repeated subtrees have a
memo. All three are compile-time or data-structure facts, not runtime events, and they gave 98x-20,000x here.

**Works as *coarse* incremental nodes.** Function-sized (the `.dag` memo: self-compile 0.84 s → 0.04 s, RECORDED) or
projection-sized nodes (`fold_step` 254x, RECORDED) where one node's work is ≥ microseconds and the dirty set is tiny.
That is exactly what dowiz already has; the literature's measured sweet spot (Dask ≥ 100 ms tasks, TBB ≥ 10⁵ cycles,
Salsa per-query) agrees.

## 7. Four more loop alternatives (coordinator's scope addition, operator's list)

First, what the two compilers already emit for the kernels (MEASURED, `objdump -D -b binary -m aarch64` on fresh
`bebop.bin 8a0b4325` compiles; results/dis/*.dis; gcc counts from results/raw.txt `OBJDUMP` lines):

| kernel | bebop inner loop | data-dependent branch? | `csel` | `cbz` | `br` |
|---|---|---|---|---|---|
| K1H | 5 words: `mov #3; madd; sub; cmp; b.gt` | no -- the only branch is the counter test | 0 | 2 (runtime stub, not the kernel) | 0 |
| K3H | inner 7 words `add,lsl; lsl; add; mov #3; madd; sub; cmp; b.gt`; outer `sub; cmp; b.gt` | no | 0 | 2 (stub) | 0 |
| K4 | 7 words: `mov #7; madd; add,lsl#1; sub #11; sub; cmp; b.gt` | no | 0 | 2 (stub) | 0 |
| K8H | 9 words: `madd; ubfx #60,#1; cmp #1; add; sub; csel eq; sub; cmp; b.gt` | **the 50 % bit is a `csel`, not a branch** | **1** | 2 (stub) | 0 |
| K2H fib | `cmp #2; b.ge` (the base case), two `bl`, `add` | yes: `n < 2`, ≈ 50 % of calls are leaves, but patterned (history predictors do well; NOT MEASURED) | 0 | 2 (stub) | 0 |
| gcc -O2 K8H `k8_csel` | — | — | 1 (`csel_count=1`) | — | — |
| gcc -O2 K8H `k8_branch` (asm barriers) | — | `b.cond` | 0 (`bcond_count=4`) | — | — |
| gcc -O2 dispatch probe | — | — | — | — | 35 `br` (the computed gotos) |

Every back-edge in K1..K8 is a counted `cmp; b.gt` / `b.ge` on a down-counter: taken N−1 times, not-taken once. A
modern TAGE-class predictor mispredicts such a branch at most once per loop exit (CITED as general knowledge of
loop predictors; the per-kernel rate is NOT MEASURED here -- perf is EACCES). The timing control supports it:
the K8H predictable-bit variant with a *real* branch (0.98 ns/step) is as fast as the branch-free csel version
(1.18 ns/step) minus the csel's extra ALU op -- i.e. a predicted branch costs nothing measurable.

### 7.1 Full predication / bitmask streams (NEON/SVE masks, `csel`)

**(a) Micro-kernels.** Already applied where it matters: bebop's `emit_cond` pre-scans both arms and emits `csel` when
both are pure (A2 commit 1, `bebop.bp:1645-1720`, RECORDED), and the K8H loop shows it. MEASURED on K8H in C: csel
**1.18 ns/step**; explicit bitmask select **1.51 ns/step (0.78x -- slower)**, because `m = -(bit); (x&m)|(−i&~m)` is three
more ALU ops on the `acc` dependency chain where `csel` is one; forced branch 3.94 ns/step (0.30x). K1H/K3H/K4 have no
data-dependent branch to predicate. NEON/SVE masks: this CPU has no SVE (RECORDED, R-FAST header); NEON has no i64
multiply, no i64 divide and no mulhi (RECORDED, bebop-dag §12.3), so the K1H/K3H/K4/K8H bodies (`madd` on i64) cannot be
expressed as 128-bit lanes at all, and chains have no lanes to fill anyway. **Verdict: nothing left for K1..K8.**

**(b) dowiz.** Candidates with a data-dependent branch inside a hot scan:
- `crates/dowiz-hub/src/block/taste.rs:top_k` (:188-222): per-row filter `FLAGS & ON_SALE == 0 || ALLERGENS & avoid != 0
  → continue`, then `cos_row` with six `if n > 0` and two `while bits != 0` popcount walks. 165 rows, RECORDED **5.6 µs**
  total. A predicated form (compute every row, mask the score) would remove ~165 unpredictable-ish branches at ≤ 6 ns
  each: ESTIMATE ≤ 1 µs saved of a 5.6 µs kernel inside a 1-10 ms request. Not worth a row.
- `crates/dowiz-hub/src/rank.rs:fade` (:51-66): two branches (`days<=0`, `s>52`) per dish, predictable. Nothing.
- The bit-plane walks (`while bits != 0 { trailing_zeros; bits &= bits-1 }`) are already the branch-free idiom; `cnt`
  on NEON (the `hvham2` builtin exists) would make them one instruction -- ESTIMATE 100-200 ns on 165 rows.
**Verdict: ESTIMATE < 1 µs per request anywhere; the request's time is JSON (6.33 ms per catalogue parse, RECORDED).**

### 7.2 Threaded code / computed gotos (`br xN` chains, no loop header)

**(a) Micro-kernels.** MEASURED (§5.6): K1H as 16 blocks linked by `goto *next[k]` (an indirect `br` with a static
target per block) runs **0.978 ms vs 0.983 ms** for the counted loop -- **1.00x**. The loop header (`sub; cmp; b.gt` =
3 of K1H's 5 words) costs nothing because the A78 predicts the back-edge and the three ops are off the critical
`madd` dependency chain (recurrence latency 2.2 cycles/iter dominates, RECORDED R-FAST §4). An indirect branch with a
stable target hits the BTB and is as free as a direct one; with a *varying* target (a VM dispatch) it costs 2.77 ns
(threaded) vs 3.72 (switch) vs 0.30 for straight-line code. bebop emits **zero** `br`/`blr` today (MEASURED census above),
and its kernels are straight-line native code -- the 12.4x column that threaded code is trying to approach from below.
**Verdict: a computed-jump graph is a model of control flow; control flow is not where K1..K8 spend time.**

**(b) dowiz.** The one place dispatch is real: `workers/api/src/hubdo.rs:route` (:1137-1420) -- `match (method, segment)`
with 34 string arms (length check + memcmp chains), after `lib.rs:router` rebuilds a 252-route matchit radix trie **on
every request** (`lib.rs:302-304`, RECORDED comment "every request's first line"). A threaded/jump-table dispatch would
turn ~34 memcmps (ESTIMATE 0.3-1 µs) into one hash; the trie rebuild is ESTIMATE 50-200 µs of allocation per request --
that is the row (make the router `static`/lazy), and it is a data-structure fix, not threaded code. `sched.bp` already
calls node functions indirectly (`call_fn(fv, ctx, i)`). **Verdict: NO for threaded code; a small YES for hoisting the
route table (ESTIMATE 50-200 µs per Worker request, unmeasured, out of this lane's scope).**

### 7.3 Algebraic reduction via semirings (GraphBLAS: a loop as a sparse-matrix op over (min,+) or (+,×))

**(a) Micro-kernels.** A semiring matrix-vector product *is* a loop (two nested ones); GraphBLAS wins by exploiting
sparsity and by a mature blocked implementation, not by removing iterations. K1H/K3H/K4 are dense chains of length N
with one nonzero per row (bidiagonal): the sparse product has exactly N multiply-adds plus index overhead. K6/K7
(Hamming over 16-word vectors) are a (+,⊕-popcount) semiring reduction of 16 terms -- in registers already. K2H is
not a matrix op. **The semiring view of a *linear* recurrence is the matrix power of §5** -- (+,×) over Z/2^64 with
repeated squaring -- so semirings *do* contain the one winning move, but as algebra, not as a sparse-matrix library.
**Verdict: no engine; the algebra is already used in §5.**

**(b) dowiz.** This is the right *name* for what dowiz computes: availability = (min,+) over the recipe matrix ×
stock (RECORDED bebop-dag §12.3: CSR availability **3,355 ns** vs dense 15,476 ns at 165 dishes; the recipe matrix is
1.8 % dense; the sparse mask beats the best SIMD dense form 8x); Datalog semi-naive = (∨,∧) closure (82 ns/event
RECORDED); PPR in `crates/dowiz-hub/src/graph.rs:ppr` (:98-150) = (+,×) matvec iterated. All already written as the
sparse scalar loops GraphBLAS would generate; a GraphBLAS dependency (SuiteSparse is 100k+ lines of C) would add
wasm size for ESTIMATE 0 % at these sizes. `graph.rs:ppr`'s dangling-node inner loop is O(n²) per iteration
(RECORDED survey) -- an algebraic fix (one dangling-mass scalar added to every node, standard PageRank) is worth a
row: ESTIMATE from O(iters·n²) to O(iters·(n+m)); unmeasured. **Verdict: adopt the vocabulary, not a library; one PPR fix.**

### 7.4 Loop-free pipelining (stencil / chunk pipelines: L1-sized chunks through micro-stages)

**(a) Micro-kernels.** Chunk pipelining trades a loop over N for a loop over chunks times a loop over stages; it is a
cache-blocking transform for data larger than L2 with ≥ 2 passes. K1H..K4, K8H touch **no memory** (state in
registers) -- MEASURED 5-9 word loops, no `ldr`/`str` -- so there is nothing to block. K5 (512-cell NTT) is 4 KB: in
L1 already. **Verdict: not applicable; 0.**

**(b) dowiz.** The one multi-pass over a large buffer is the catalogue: `Kv::decode` (`crates/bebop-store/src/kv.rs:175-258`)
makes **two byte-at-a-time copies** via `blob_byte` (:76-82, a version branch + bounds-checked cell load + shift *per
byte*) over ~530 KB, then `String::from_utf8_lossy` per key, then `snapshot_root_u64` FNV byte-serially over the same
530 KB (:299-310), and `Catalog::load` runs at **31 call sites** in the Worker (RECORDED survey). This is where a
"chunk pipeline" idea actually lands -- but the fix is simpler than pipelining: copy 8 bytes per cell
(`u64::to_le_bytes` -- the cell *is* 8 packed bytes), hash 8 bytes per step, and don't decode 165 values to look one up
(the zero-copy overlay `kv/zc/overlay.rs` exists). ESTIMATE: 530k byte iterations × ~2-3 ns ≈ **1-1.6 ms per `Catalog::load`
→ ~0.2 ms**; against a 10 ms CPU cap that is the single biggest loop-shaped item in this survey. MEASURED here only by
analogy (STREAM: 0.63 ns/element sequential at 12.6 GB/s, 124 ns per dependent random step on 8 MB, results/raw2.txt),
so it stays an ESTIMATE until `measure_menu_before_and_after` (menu/measure.rs:85-133, `#[ignore]`) is run.
**Verdict: YES to the *idea* (stream the bytes once, 8 at a time), NO to a pipeline framework.**

### 7.5 Direct answer: "branches evaporated through bitmasks, or a graph built from computed jump addresses?"

**Bitmasks -- specifically `csel`, which bebop already emits.** Reasons, all measured above: (1) the only
unpredictable branch in K1..K8 is K8H's coin flip and it is already a `csel`; a true branch there costs 3.35x, an
explicit mask costs 1.28x *more* than csel; (2) every other branch is a counted back-edge that the predictor gets
right, and removing it entirely (threaded blocks) changed the time by 0.5 %; (3) a computed-jump graph makes control
*explicit* -- a VM -- and the VM dispatch floor is 2.77 ns/op against 0.30 for the straight-line code bebop already
emits; (4) masks compose with the one real win (closed forms produce straight-line arithmetic), jump graphs do not.
The caveat: masks lose to `csel` on a dependency chain and lose to a predicted branch when one arm is expensive
(predication executes both arms); the compiler's `arm_is_pure` test is the right gate and exists.

### 7.6 dowiz candidates, named (file:function → shape → expected gain → status)

| candidate | loop shape | which idea | expected gain | status |
|---|---|---|---|---|
| `crates/bebop-store/src/kv.rs:Kv::decode` + `blob_byte` (:76-82, :222-246); `snapshot_root_u64` (:299-310); `Catalog::load` ×31 sites | ~530k serial byte iterations, two copies + FNV, per load | §7.4 stream 8 B/cell; hash per cell | ESTIMATE 1-1.6 ms → ~0.2 ms per load; the biggest loop item under the 10 ms cap | ESTIMATE; run `menu/measure.rs:measure_menu_before_and_after` first |
| `workers/api/src/fold/menu.rs:Memo::build` (:112-176): `serde_json::from_str` per product ×165 | parse, not a loop | none of the four; the block codec (RECORDED 6.33 ms → 2.4 µs) | RECORDED 760x on the parse | already a roadmap row (codec) |
| `workers/api/src/lib.rs:router` (:302-304) rebuilt per request; `hubdo.rs:route` 34-arm string match | dispatch | §7.2 -- but as a static table, not threaded code | ESTIMATE 50-200 µs per request | ESTIMATE |
| `workers/api/src/hubdo.rs:HubImages::image` (:275-277) `hit.clone()` of ~0.5 MB per call | memcpy | none; return a borrow | ESTIMATE 50-100 µs per call (12.6 GB/s MEASURED stream rate) | ESTIMATE |
| `crates/dowiz-hub/src/graph.rs:ppr` (:98-150) dangling-node O(n²) inner loop | iterated (+,×) matvec | §7.3 algebra (dangling mass as one scalar) | ESTIMATE O(n²)→O(n) per iteration; "~20 ms fold" RECORDED | ESTIMATE |
| `crates/dowiz-hub/src/block/taste.rs:top_k` (:188-222) | 165-row filtered scan | §7.1 predication / NEON `cnt` | ESTIMATE ≤ 1 µs of 5.6 µs | not worth a row |
| `crates/dowiz-hub/src/snn/infer.rs:stalks/scores` (:78-146) | 165 × 8 × 27 i128 MACs, `Vec<Vec>` per dish per layer | reading (e) would apply, but the time is allocation | ESTIMATE: allocation-free rewrite 2-5x; budget test says < 1 ms, value unrecorded | ESTIMATE; shadow mode, low priority |
| `bebop-lang/selfhost/std/kv.bp:kv3_fold/kv3_pick/kv3_find` (:83-109, :202-213) | O((n+D)·D) chain walks, D ≤ 32 | sort the chain once (overlay merge, as `zc/overlay.rs` does) | ESTIMATE 32x fewer compares at D=32 | ESTIMATE |
| projection fold `fold_step` / `.dag` memo | coarse incremental nodes | reading (b) *done right* | RECORDED 254x / 21x | exists; nothing to add except AX3 early cutoff |
| `crates/dowiz-hub/src/snn/shadow.rs`, `services/customers/taste/snn.rs:observe` (:63-79) parses 165 products **twice** per guest request | duplicated work | none; share the parsed menu | ESTIMATE halves the shadow path | ESTIMATE |

**Where spikes would have helped dowiz and don't:** the catalogue has ≤ 165 dishes; a 1 %-of-input change is 1-2 dishes;
the full dense recompute of every kernel is **8.3 µs** (RECORDED §12.3). A spike engine's dirty cone would save
microseconds while its node table costs the same microseconds; the request's milliseconds are in parse and copy.

## 8. Recommendation for bebop: GO / NO-GO per piece

| piece | what to build | expected gain | cost | verdict |
|---|---|---|---|---|
| **R-CF: closed-form elimination of affine loops** in `emit_while_stmt` (SCEV-style, extended to geometric recurrences) | detect a `while` whose body is `let s = a·s + b·i + c; let i = i ± d` (all `let`s affine in the loop's own symbols, constant coefficients, counted exit, no calls/stores/`if`); emit the 3x3 (or k×k) matrix power over Z/2^64 as straight-line bebop-emitted code -- or, cheaper, emit a *call* to a prelude `affine_pow3` with the coefficients, which is exactly `probes/k1h_closed.bp` | K1H 1.0 ms → ~1.5 µs (**~690x, MEASURED in bebop**); K4 **3,400x**, K3H **98x** (MEASURED in C); K1 plain O(1); zero cost on loops that do not match | 2-4 days: an affine-recognition scan in the style of `arm_is_pure`/`hoist_scan`, a prelude routine, parity constructs for 3 shapes, the 360-program sweep. Risk: the census question -- how many loops in `bebop.bp`/std/prelude are affine chains? ESTIMATE few (the compiler's loops are parsers and table scans); the win is on *user* kernels and on query specialisations (`gen_gb`'s counted scans over constants). Apply the A3 rule: if the census finds 0 real loops outside the benchmark, ship it as a prelude routine + a `bench/vs_rust` row, not a pass | **GO** (small, measured, honest: it changes the K1H/K3H/K4 rows from 1.0-1.4x to ≪ 0.01x, which must then be reported as "loop eliminated", not as codegen parity) |
| **R-TRE / sharing** (fib) | already planned (R-FAST R2) | K2H 1.9x → ~1.1x on the TRE shape | — | already GO elsewhere; the memo/DP form is the user's job |
| **Event-driven / spiking execution model for kernels** | a runtime that fires nodes on change | cold **0.31-0.32x** (3.2x slower, MEASURED); re-runs win only under ~0.3 % change and never on a chain (MEASURED 999,998 of 1M nodes re-fire at 1 %) | high (a runtime + a scheduler) | **NO-GO** |
| **Full unrolling into a static DAG** | N copies of the body | 1.00x at best (MEASURED threaded blocks), I-cache misses above ~16k nodes | — | **NO-GO** |
| **SNN-style execution** | approximate, float, trained | not a semantics-preserving replacement; S7 E3/E7 found no signal (RECORDED) | — | **NO-GO** (category error for integer kernels) |
| **DBSP-like general incremental runtime for dowiz** | operator graph with Z-set deltas | the folds are already incremental by the write (`fold_step` 254x RECORDED); dense sets ≤ 165 rows; the parse dominates | high | **NO-GO** (BN §4.2's judgement stands) |
| **O(k) algebraic deltas + early cutoff for counted aggregates** | Z-set weights for counts/sums (deletion = weight −1), cutoff when the output bytes do not change | MEASURED here: 1 % change 124x vs dense, 370x vs a spike engine; asymptotic, trigger rare at 165 rows | small | **GO-small** (= AX1/AX3, already queued; no new row) |
| **Predication beyond `csel`** (explicit masks, NEON/SVE) | mask streams | masks 0.78x of csel on K8H (MEASURED); no SVE; no i64 lanes | — | **NO-GO**; keep `csel` + `arm_is_pure` |
| **Threaded code / computed-jump graph** | `br`-linked blocks | 1.00x (MEASURED) | — | **NO-GO** |
| **Semiring engine (GraphBLAS)** | library dependency | 0 % at 165 rows (RECORDED CSR numbers already optimal) | wasm size | **NO-GO**; one algebraic PPR fix as an ESTIMATE row |
| **Chunk pipelines** | staged L1 chunks | kernels touch no memory; dowiz's one big buffer wants 8-byte copies, not a pipeline | — | **NO-GO** as a framework; **GO** as the `Kv::decode` byte-loop fix (ESTIMATE 1-1.6 ms → 0.2 ms per load, to be MEASURED with `menu/measure.rs`) |

**Honesty note for the benchmark table.** If R-CF lands, `honest.sh`'s K1H/K3H/K4 rows become meaningless as *codegen*
comparisons (bebop would report µs against Rust's ms for a loop LLVM deliberately keeps). The right move is what the
`h` twins did in reverse: keep the loop rows with elimination disabled for the codegen claim, and add a separate
"loop eliminated" row with the Rust twin allowed its own closed form (or `black_box` inside the loop on both sides).
A number that compares an eliminated loop with a kept one is a gate measuring its own epitaph.

## 9. Sources

Measured here: `S/rspike/probes/{common.h,k1h.c,k4.c,k3h.c,k8h.c,fib.c,sum.c,dispatch.c,stream.c,perfprobe.c,k1h_closed.bp,
k1h_closed_check.bp,oracle.py,median.py,run*.sh}`; raw: `S/rspike/results/{raw.txt,raw2.txt,raw3.txt,raw4.txt,oracle.txt,dis/}`.
Recorded in the tree: `bench/vs_rust/REPORT-honest.md` (hot rows 2026-10-07), `REPORT-cold.md`, `bench/substrate_spike/RESULT.md`,
`RESULT-incr.md`, `docs/research/2026-10-07-bebop-fastest-runtime.md` §4/§9, `2026-09-28-bebop-dag.md` §12.3/§13.2,
`2026-10-02-arxiv-dag-optimisations.md` §2.1-2.2, `2026-10-06-s7-sheaf-engine-arxiv-and-experiments.md`, code survey of
`crates/dowiz-hub`, `crates/bebop-store`, `workers/api` (file:line in §7).

Cited (fetched unless marked UNVERIFIED in §3): Acar et al. PLDI 2006 / TOPLAS 2009 (doi 10.1145/1596527.1596530);
Hammer, Neis, Chen, Acar, OOPSLA 2011, arXiv 1108.3265; Anderson, Blelloch, Baweja, Acar, arXiv 2105.06712; Hammer et al.
Adapton PLDI 2014 (doi 10.1145/2594291.2594324); Hammer et al. OOPSLA 2015, arXiv 1503.07792; Fisher et al. miniAdapton
arXiv 1609.05337; Budiu et al. DBSP PVLDB 16(7) 2023, arXiv 2203.16684; Feldera blog 2024-09-10 (feldera.com/blog/nexmark-vs-flink);
Murray et al. Naiad SOSP 2013 + microsoft.com/en-us/research/project/naiad; McSherry et al. CIDR 2013;
github.com/TimelyDataflow/differential-dataflow README; McSherry, Isard, Murray HotOS 2015 (COST); Woerister, blog.rust-lang.org
2016-09-08; rust-analyzer analysis-bench (UNVERIFIED snippet); Elliott & Hudak ICFP 1997; Meyerovich et al. OOPSLA 2009;
krausest/js-framework-benchmark; milomg/js-reactivity-benchmark; Brette et al. J. Comput. Neurosci. 23 (2007), arXiv q-bio/0611089;
Hanuschkin et al. Front. Neuroinform. 4:113 (2010); Stimberg, Brette, Goodman eLife 8:e47314 (2019); Jordan et al. Front.
Neuroinform. 12:2 (2018); Pronold et al. Front. Neuroinform. 15:785068 (2022), arXiv 2109.11358; Knight & Nowotny Front. Neurosci.
12:941 (2018); Yavuz, Turner, Nowotny Sci. Rep. 6:18854 (2016); Zenke & Gerstner Front. Neuroinform. 8:76 (2014); Davies et al.
IEEE Micro 38(1) 2018 via Cılasun et al. arXiv 2006.03007; Davies et al. Proc. IEEE 109(5) 2021; Bondhugula et al. PLDI 2008;
Verdoolaege, Polyhedral Process Networks (2010); Culler, Schauser, von Eicken UCB/CSD-92-716; Swanson et al. MICRO 2003 / TOCS
2007; Gebhart et al. ASPLOS 2009; llvm.org/docs/Passes.html + IndVarSimplify.cpp; gcc.gnu.org Optimize-Options
(`-ftree-scev-cprop`); Bachmann, Wang, Zima ISSAC 1994; Van Engelen CC 2001; Gries & Levin IPL 1980; docs.dask.org
scheduling/best-practices, distributed.dask.org efficiency; oneTBB Developer Guide "Controlling Chunking"; Huang et al.
Taskflow TPDS 2022, arXiv 2004.10908; Frigo, Leiserson, Randall PLDI 1998; Blelloch CACM 39(3) 1996; Pătraşcu & Demaine SIAM J.
Comput. 35(4) 2006, arXiv cs/0502041.

## 10. Proposed docs/exp.journal line

`1791399342 H:reactive spikes on a DAG can replace kernel loops and speed bebop up | DID:formalised 5 readings; lit (Acar/Adapton/DBSP/DD/Salsa/SNN/dataflow machines/SCEV, 40+ sources); C+bebop probes on K1H/K2H/K3H/K4/K8H + 1M-sum + VM dispatch, slot.sh, core 4, median of 5, Python oracle; objdump of bebop kernels; dowiz code survey | GOT:spike engine cold 0.25-0.33x (3-4x slower, 3.7-4.8 ns/node vs 1-1.6 ns/iter); chain re-run at 1% change re-fires 999998/1M nodes, 0.33x; tree-sum spikes 390x at 1 leaf, 0.38x at 1% (crossover ~0.3%, = substrate 0.39%); closed forms K1H 1165x C / ~690x in bebop (bit-exact), K4 3414x, K3H 98x; fib sharing 14700x; K8H no closed form, bebop already csel (=C csel 1.18 ns/step; mask 0.78x, branch 0.30x); threaded blocks 1.00x; perf EACCES | VERDICT:spikes NO-GO for kernels and for a dowiz runtime; GO on affine closed-form loop elimination (R-CF) + O(k) deltas (AX1/AX3); masks=csel already; Kv::decode byte loop is the dowiz loop to fix (ESTIMATE 1-1.6 ms/load) | COST:~2.5 h lane, 3 slot batches + 3 reruns (two harness bugs: carried-rep mismatch, FIFO-on-chain quadratic), 1 rc=137 on the Python oracle`

