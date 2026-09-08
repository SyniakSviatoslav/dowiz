Status: 2026-09-08, owner main session, grounded at 0b823d0. From docs/RESEARCH-CORPUS-IDEAS-2026-09-08.md §3 (proposed there as B2b). PROPOSAL pending operator decision; the payoff is PREDICTED, not measured.

# D2 memory-level parallelism in the generated probe kernels

## 0. The blind spot this row names

Every kernel row on the roadmap is about **words per iteration** (A2b: 12 -> 9,
K8H 1.8x -> 1.1x), **registers** (A1/A5/A6), **cells** (A8) or **cores** (B6).
**None is about how many DRAM misses are in flight**, and the measured kernels
say that is the binding term:

- frontier BFS runs at **23 ns/edge-slot in-process, 45 ns from the store**. The
  bandwidth floor for an 8-byte slot at this box's measured ~12 GB/s is
  **0.67 ns**. The kernel is 35-70x above the floor, so it is NOT
  bandwidth-bound -- it is bound by the latency of the dependent gather
  `x[ci[k]]`. `docs/LANG-DB-DESIGN.md:539` already names "1 line miss, ~10-20"
  per slot.
- the join twin's probe is worse: `k = Rk[r]`, then `rp[k]` is a random miss
  into an 8 MB array, `ci[rp[k]]` a second dependent miss, `Sb[ci[j]]` a third.
  An A78 can keep roughly a dozen misses in flight, but only if the addresses
  are in its reorder window; a dependent chain of three exposes them serially.

## 1. What the literature says (from abstracts)

Cimple (1807.01624, PACT'18) on "in-memory databases, key-value stores, and
graph analytics": "compilers and hardware struggle to expose ILP and MLP", fixed
by tasks that "yield execution at annotated long-latency operations" and are
"interleaved on a single thread". CoroBase (2010.15981, VLDB'21): coroutines
"ease the implementation of software prefetching to hide data stalls". Cuckoo
Trie (2201.09331, SOSP'21): an index "designed to have memory-level parallelism"
outperforms state-of-the-art indexes "by up to 20 %-360 %". Calico (2604.00423):
"group prefetch" for "irregular high-fan-out graph traversal". Helper Without
Threads (2009.00202): "inline software prefetching ... for delinquent irregular
loads". Decoupled Access-Execute on ARM big.LITTLE (1701.05478) confirms the
mechanism works on this CPU family.

**All but the last are x86 servers with large L3s. The direction is well
supported; the factor on a 3-core A78 with a small shared cache is not.**

## 2. Mechanism, in this codebase's terms

The kernels are GENERATED TEXT (`selfhost/std/gen_gb.bp`, 12 templates), so this
is a template change, not a compiler pass:

1. **one builtin**: `prefetch(addr)` emitting `prfm pldl1keep, [xN]` -- one
   word, a construct plus a bpref stub (bpref: a no-op), no register-model
   interaction (it reads a register and writes nothing);
2. **the probe template unrolled by G** (4 or 8): a first loop prefetches
   `rp[Rk[r+G]]` for the row G ahead; one iteration later a second prefetches
   `ci[rp[k]]` for the row G/2 ahead; the compute loop then runs on rows whose
   three lines are already in L1/L2. This is group prefetching / AMAC, and it
   needs no coroutines -- a fixed-depth software pipeline over an index array is
   straight-line code, which is exactly what a one-pass compiler emits well;
3. the frontier BFS template prefetches `x[ci[k+G]]` while processing slot k.

## 3. Gate

On the 1M x 1M join twin, both distributions: **probe-phase ns per R row <=
1.2x of the Rust `HashMap` twin's probe.** That is the number that decides B2's
own "(i) >= 0.7x best Rust" -- today the whole kernel is 0.67x of
Rust-at-bebop's-algorithm and the Rust best is a hash join.
On sgraph2: **frontier ns/edge-slot <= 0.6x of today's 45** (store arm), fold
unchanged.

## 4. Cost against the invariants

Zero dependencies: none. One-pass: none -- the compiler gains one builtin and
the schedule lives in the generator. i64: none. Word budget: the unrolled
template is G times larger, but the kernels are per-(op, semiring) and PreJIT'd,
so the COMPILER's own census does not move. Box: no extra process, no core.

## 5. Cheapest experiment that kills it

Hand-unroll the probe loop of `bench/vs_rust/std_tests/join_twin.bp` by 4 with a
scratch `prfm` builtin -- or, with **no compiler change at all**, emulate the
prefetch with a dummy `let _ = rp[Rk[r+4]]` load folded into a dead accumulator
-- and measure inside `tools/slot.sh`.

**If the probe phase does not drop by >= 25 %, this row dies**: the loop is
already at the bandwidth ceiling, or the A78's own window already overlaps the
misses. Second kill, if `perf_event_open` proves usable (`perf_event_paranoid`
reads -1 on this box today, contradicting `swpmu.bp:1-4`): the L2-miss count per
probe row before and after settles it in one run.
