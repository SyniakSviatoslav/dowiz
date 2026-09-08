Status: 2026-09-08, owner main session, grounded at 1ed3719. Derived from docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md §7 (proposal 6, mid-loop OSR, verdict NARROW). Attaches to B3/B6. PROPOSAL pending operator decision.

# C5 chunk-boundary tier and flavour switching

## 0. Why not OSR as proposed

Byte-level "stop mid-loop, recompile, resume at the same byte" cannot be built on this codebase
today, for three independent reasons, any one of which is fatal:

1. **There is no in-process resume point.** `sys_run` must execute in a forked child because
   entering another image's entry stub reconfigures the caller's arena registers
   (`selfhost/std/gb_run.bp:241-251`, `:566-568`). There is no "stop here and continue there"
   path at all.
2. **No frame map.** The compiler emits no frame-layout side table, and mapping loop state
   between two differently-compiled kernels is the thing V8's OSR is actually made of.
3. **The trigger was thought to be unavailable**, and this one is now in doubt in the project's
   favour: `selfhost/std/swpmu.bp:1-4` records `perf_event_open` returning EACCES under Android
   seccomp, but `/proc/sys/kernel/perf_event_paranoid` reads **-1** on this box today. That
   comment is STALE and must not be cited as the reason hardware cache-miss counters are
   unavailable. **Re-probe before building on it or ruling it out** -- whether the proot seccomp
   filter passes syscall 241 is a separate question and was not tested.

## 1. What to build instead

What HyPer and Umbra do, and what this tree already does at a coarser granularity: **switch
strategy at chunk boundaries.**

Two facts make this nearly free here:
- B3's tier-0 / specialised switch already does exactly this at QUERY granularity (tier-0 answers
  in ~1 ms while the specialised kernel compiles in the background);
- the Beamer push/pull switch at alpha 14 already changes strategy **mid-BFS**, per frontier
  phase (ROADMAP.md:188, sgraph2 frontier row).

So C5 is: generalise "per frontier phase" into "per chunk of N rows", and let the choice be
re-made at each boundary from a cheap statistic gathered in the previous chunk (selectivity,
distinct-key count, or -- if the perf probe above succeeds -- an actual miss rate).

## 2. Design

- A kernel processes rows in chunks of N (N chosen so a chunk is >= the ~100 ms floor is NOT
  required here -- chunks are far smaller; N is chosen so the per-chunk decision cost is under
  1 % of chunk work).
- At each boundary the driver, not the kernel, picks the next flavour. The kernel stays a pure
  function over a row range, which is what keeps it dispatchable under C1's checked dialect.
- Flavours come from the pool B3 already has, keyed by digest -- a flavour switch is a pool
  lookup, not a compile.

## 3. Gate

On a deliberately skewed input (Zipf keys, the distribution B2 already generates): the
chunk-switching driver is **within 1.1x of the better fixed flavour**, and **better than 1.5x of
the worse one**. If it cannot beat the worse fixed flavour by 1.5x, the switch is not paying for
its bookkeeping and the row is refuted.

Second gate, structural: the fold is identical to the single-flavour run. A driver that changes
the answer is a bug, not an optimisation.

## 4. Cheapest refutation

The sgraph2 frontier already switches. Instrument it to log which side it picks per phase on
uniform and on Zipf inputs. If the switch almost never fires on realistic inputs, chunk-level
switching generalises a mechanism that does not trigger, and C5 should be closed with that
evidence.
