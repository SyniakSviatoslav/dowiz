# selfhost/std/attic — modules retired from the gate tree (T38, 2026-09-04)

Status: CURRENT. Nothing here is compiled by any gate, script or by
`bebop.bp`. A module leaves the attic only by getting a `fn main` fold +
`bench/oracles/<g>.py` + a `gate` line (law L17). Reason classes:

- **f64**: the API is over `[f64]`; `bebop.bin` is i64-only, no exact fold.
- **str**: the API takes/returns `str`; `.bin` has no str literals (R3.x d).
- **if-while**: a `while` inside an if-branch let-chain returns 0 under
  `bebop.bin` (T42 repro: `if n < 0 then 0 else ( let i = 0; while ... ; s )`
  yields 0, the same loop at fn top level is right) — the module needs a
  rewrite before it can be folded.
- **retired-interp dialect**: `let _ = a[i] = v in 0` (crashes `.bin`).
- **no consumer**: no main, no script or gate references the file; i64-only
  and gate-able later if anyone needs it — nothing consumes it today.

| file | reason |
|---|---|
| automaton.bp | no consumer — DFA runner over a flat transition table |
| bignum.bp | no consumer — little-endian limb add / mul_small / cmp |
| bitfield.bp | no consumer — field extract/insert via pow2 division (bit 63 unreadable) |
| bits.bp | no consumer — popcount/clz/rotate via pow2 division; gates use the SWAR `prelude/bits.bp` |
| blas.bp | f64 — BLAS-1 saxpy/dot/nrm2 (+ its own sqrt) |
| combinatorics.bp | if-while — binomial/permutations return 0 under bebop.bin (factorial alone is right) |
| dist.bp | no consumer — uniform/bernoulli/binomial over a 2-arg lcg_next (differs from `prelude/rng.bp`) |
| effect.bp | no consumer — pure/io/state effect registry as small ints |
| encoding.bp | no consumer — URL percent-encoding over byte cells |
| event.bp | no consumer — event-sourced FSM apply/fold |
| file_io.bp | no consumer — M2 syscall wrapper stubs; store.bp/bebop.bp call sys_* directly |
| fmt.bp | str — digit count/extraction returning str |
| gcra.bp | no consumer — GCRA limiter step (ratelimit.bp duplicates it) |
| graph.bp | no consumer — adjacency-matrix has_edge/degree |
| hash.bp | str — djb2/sum/poly over str (gates hash cells with `prelude/hash.bp` fnv_cells) |
| heap.bp | retired-interp dialect + if-while — heap_pop crashes, push/pop fold is 0 |
| list.bp | no consumer — parallel-array singly-linked list |
| log.bp | f64 — log2/log10 via ln |
| markov.bp | no consumer — first-order Markov transition counts |
| math.bp | no consumer — sq/abs one-liners (every gate inlines them) |
| matrix.bp | f64 — matvec/matmul/transpose + 2x2/3x3 det |
| nat.bp | no consumer — Peano succ/pred recursion |
| numeric.bp | no consumer — is_even/gcd/isqrt (Newton isqrt differs from the gates' digit-by-digit `prelude/fp.bp` isqrt) |
| ops.bp | no consumer — arithmetic stand-ins for and/or/xor/not (the language has & \| ^ now) |
| pac.bp | no consumer — pointer-auth mixing toy |
| permutation.bp | if-while — next_permutation/rank/unrank return 0/identity under bebop.bin |
| pid.bp | f64 — PID controller step |
| polynomial.bp | no consumer — Horner evaluation |
| primes.bp | if-while — is_prime returns 0 for every n, next_prime loops forever |
| queue.bp | no consumer — circular queue size/push/pop |
| quicksort.bp | retired-interp dialect — quicksort(a, 4) crashes, partition returns 0 |
| radix.bp | str — i64 <-> str base conversion |
| ratelimit.bp | no consumer — token bucket + GCRA (integer) |
| ring.bp | no consumer — fixed-capacity ring buffer init/push/pop/peek |
| session.bp | no consumer — session-type duality encoded in i64 |
| stack.bp | no consumer — LIFO over [i64] + top index |
| statistics.bp | f64 — mean/variance/stddev/min/max over [f64] |
| stats.bp | no consumer — integer sum/mean/variance |
| string.bp | str — helpers over str_len/char/chr |
| strutil.bp | str — char classes + i64<->str |
| tensor.bp | f64 — dot/matvec/sum/mse over [f64] |
| token_bucket.bp | no consumer — token bucket limiter (ratelimit.bp duplicates it) |
| vec.bp | no consumer — `vec_new` stub, one line |
| version.bp | no consumer — semver compare packed in i64 |
