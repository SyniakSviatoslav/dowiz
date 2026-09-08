Status: 2026-09-08, owner main session, grounded at 1ed3719. Derived from docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md §6 (proposal 5, reactive tensor materialisation, verdict NARROW). Attaches to B4. PROPOSAL pending operator decision.

# C4 declared materialised views maintained from B4's tail delta

## 0. Read this first: the paradigm was already measured here and rejected

The operator's proposal is that the DB runs like a neural-net forward pass -- every mutation
propagates as a wave through the tensor graph and the client reads a cell that is always current.
As **the** execution model, this tree already tested it and it lost:

- the sweep/dataflow engine is **41x slower than linear code on dense work**
  (`bench/substrate_spike/RESULT.md`; ROADMAP.md:193-197);
- the incremental curve **crosses over at k/N = 0.39 %** (`RESULT-incr.md`; ROADMAP.md:199-202) --
  i.e. incremental maintenance wins only when under 0.4 % of the data changed between reads;
- `LANG-DB-DESIGN.md §7 item 2 (:392)` records "whole-graph incremental recomputation as the
  default engine" as precisely the thing **Eve died of**.

A third objection is operational rather than architectural: "background propagation" implies a
resident process per store, on a box that SIGKILLs the 33rd process (`docs/BOX.md`).

So the row is not "make the DB reactive". It is: **keep the 0.39 % regime and pay for nothing
else.**

## 1. Scope

**In.** A type may declare a `view`: a named cell (or small object) plus a fold, maintained at
COMMIT time from the transaction's own tail delta -- the set of rows B4 already knows changed,
because CoW-append means the delta is what was appended. No background process, no propagation
wave, no scheduler.

**Out.** Any maintenance that must walk beyond the delta; any view whose fold is not associative
enough to update from a delta (those recompute on read, as today, and the compiler should say so
at declaration time rather than at runtime).

## 2. Design

At `st_commit`, for each declared view whose inputs intersect the appended range:
`view' = combine(view, fold(delta))`. This is the GraphBLAS shape the store already speaks --
`combine` is the semiring's additive operator, and B3's folds (`gb_tc`/`gb_cc`/`gb_pr`) are
already written as reductions.

The cost lands on the WRITER, which is the correct place: reliability-over-latency (D0) says a
reader should not pay for freshness it did not ask for, and a writer already pays an msync.

## 3. The gate is a crossover, measured, not a ratio

For each candidate view: measure (a) read-time recompute and (b) commit-time maintenance, and
report the **k/N crossover** the way `RESULT-incr.md` already does.

A view is adopted **only if the workload's measured k/N sits below its crossover.** W (the
dowiz-core order log, B8) is the workload that decides it: an order-event log has a small delta
per commit and many reads, which is exactly the regime where this wins -- but that must be
measured on W, not assumed from the shape.

Falsifiable form: `view_maintain_us` per commit and `view_read_us` recorded as perf rows; the row
is REFUTED for any view whose maintenance cost exceeds the read cost it removes, at W's measured
k/N.

## 4. Cheapest refutation

Take one fold that B3 already computes (`gb_cc`, connected components) and one already-measured
update workload (`sbench`'s 10^5 single-row updates). Maintain the fold at commit and compare
against recomputing it on read at the same read/write ratio. If the crossover lands above W's
k/N, C4 is refuted for W and stays a per-type opt-in for whoever measures a workload where it
holds.
