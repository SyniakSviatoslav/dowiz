Status: 2026-09-12, research pass (read-only, NO code written, NO run executed). Every duration is
QUOTED from `docs/PERF.md`, `bench/perf.csv`, `ROADMAP.md` or a script comment with its line, or derived
and marked so. Grounded at HEAD `d75172c`. Roadmap row A20 (new, harness only). Delivers language item 7
(a faster feedback loop). Agent framing: a worker's context window is the scarce resource
(`docs/WORKER-CARD.md` "Token economy"); a per-edit answer in 30 s instead of 104-139 s is the goal.

# A20 -- a 30-second fast lane per edit; the chain stays the merge criterion

## 0. Three corrections this blueprint rests on

**(0a) The chain is 516 s as of 2026-09-12, and the self-compiles are not the cost.** CORRECTED by the
main session after this blueprint was drafted: the research pass read "516" as the MILLISECONDS figure of
journal 1789233153 (a different, already-closed defect). It is seconds, and it was measured on this box on
2026-09-12 by two independent printers in the same run -- `chain: fixpoint gen3 == gen4 1d1e634f (516 s
total)` from `chain.sh`, and `slot: released 1 after 517 s` from `slot.sh`. Both figures below are also
real: `chain_wall` was 104-139 s when `docs/PERF.md:9` recorded it, and the battery has since grown the
`b6core` gate (10^7-element folds, 8 runs) among others. **So the loop has regressed by ~4x since the last
pinned measurement, and this row's case is stronger than it was drafted to be, not weaker.** The original
text follows, and its decomposition still holds: Measured: `chain_wall` 104 / 139
/ 107 s and `chain_cpu` 54 / 82 / 57 s with the battery, CG=1 (`docs/PERF.md:9-10`, `bench/perf.csv:1418,
1473`); commit `e5b64f7` reports 186 s. The three generations cost 3 x 1.5-1.7 s (`ROADMAP.md:290`) =
~5 s. Everything else is the battery (`tools/battery.sh:33-46`: `std_par.sh` over 117 gates, 104
constructs compiled + run + `cmp`, `parity_driver`, `pool_parity`, `run_all` over 128 oracles,
`invariants.sh`'s rungs, census, `check_abi`, `diag_check`, `check_words`, `f8_dt`) plus `perf.py`
(`chain.sh:50`: "PERF=0 skips (~60 s)"). Re-measure `chain_wall` into `perf.csv` as step 0 of this row:
the 104-139 s rows are stale and the 516 s run is unpinned, so neither is a number this row may quote.

**(0b) The gate memo cannot help a codegen edit.** `std_golden.sh:27-54` replays a PASS keyed by the
`.bin`'s md5; a codegen change changes every `.bin`, so every key misses. The memo is for non-codegen
lanes and it already works there.

**(0c) The cheapest instrument already exists and nobody runs it per edit.** `bebop.bin check <src>`
(T90 2b, `cli_check:7244-7265`) runs the whole front end with diagnostics and writes no output; it costs
one compile of the file (346 ms cold for a std gate, 113 warm, 106 floor: `ROADMAP.md:291`). For
`bebop.bp` itself that is one self-compile, ~1.6 s. An agent that edits `bebop.bp` and runs `check` gets
every exit-89/95/97/101 answer in under two seconds before touching the slot.

---

## 1. Scope

### A20 IS

1. **`tools/fast.sh <files...>`**: a per-edit lane, no slot, that runs in this order and STOPS at the
   first RED: (i) `bebop.bin check` on each edited `.bp` (~0.1-1.6 s each); (ii) if `bebop.bp` was
   edited, ONE self-compile to `$OUT/gen2.bin` (~1.6 s) and the size line from `tools/perf.py size`
   (`bin_words`, `push_words`); (iii) `construct_parity.sh` against gen2 with `FREEZE=0` -- compile + run
   + `cmp` of 104 constructs (~104 x ~0.1 s, derived from the 106 ms floor = ~11 s); (iv)
   `tools/typecheck.py` over the edited files; (v) `tools/check_words.py`. One summary line:
   `fast: check <n>/<n> words <bin_words> constructs pass=<n> fail=<n> typecheck <k> findings <s> s`.
2. **A ceiling on its own wall**: <= 30 s median of 3 on `taskset -c 4` for a one-line edit to
   `bebop.bp`, recorded in `bench/perf.csv` as `fast_wall` by `tools/perf.py record` (the mechanism
   `chain.sh:53-54` already uses). RED above 30 s.
3. **`PERF=0` for a worker's chain**, `PERF=1` only on the promotion chain the main session runs:
   ~60 s off every worker chain (`chain.sh:50`), no information lost because promotion re-measures.
4. **The rule written into WORKER-CARD**: `fast.sh` before every slot request; the chain unchanged as the
   merge criterion; `fast.sh` GREEN is never evidence in a VERDICT.
5. **An optional `--json`** summary for `battery.sh`'s eleven `line()` regexes (`battery.sh:47-63`), ~40
   lines of python, so an agent parses one object instead of eleven lines. Optional because the lines are
   already regex-shaped.

### A20 IS NOT

1. **Not a change to the chain, the battery, the freeze, the word budget or the census.** The honesty
   floor (thesis clause 4) is untouched; `fast.sh` cannot promote and cannot re-freeze.
2. **Not a per-fn memo** (A10, refuted at 21.6 %/78.4 %, re-examined at 623 ms; its own row).
3. **Not parallelism.** One slot, one heavy job (`slot.sh:35-37`); `fast.sh` is light (one compile at a
   time, `nice -n 10 taskset -c 0-3` per WORKER-CARD) and does not take the slot.
4. **Not a compiler change.** Zero words, zero fns.
5. **Not a replacement for `bebop.bin check`'s missing half**: `check` does not run the program. A
   value check is the construct lane, step (iii).

---

## 2. The gate

### 2.1 Files

| file | what |
|---|---|
| `tools/fast.sh` | the lane |
| `bench/perf.csv` + `docs/PERF.md` | `fast_wall` row via `tools/perf.py record` |
| `docs/WORKER-CARD.md` | the rule (one paragraph) |
| `tools/chain.sh` | no change; `PERF=0` is an env var already honoured (`chain.sh:49`) |

### 2.2 The number

```
fast: check 1/1 words 43491 constructs pass=104 fail=0 typecheck 0 findings 27 s
fast_wall <= 30 s   (perf.csv, median of 3, taskset -c 4, one-line edit to bebop.bp)
```

### 2.3 How each assertion goes RED

| # | assertion | goes RED when |
|---|---|---|
| A | `fast_wall <= 30` | the construct lane runs frozen `cmp`+run for all 104 at more than ~0.2 s each (the box is loaded -- `slot.sh --status` first, as WORKER-CARD says), or `check` on `bebop.bp` takes > 3 s (a scanning regression: the A10 story) |
| B | a deliberate one-word miscompile in a scratch copy (e.g. flip the `vs_try_madd` fold) is caught by step (iii) | the constructs are not sensitive (they are: A2b froze `c74_madd` for exactly that) |
| C | a deliberate syntax error in `bebop.bp` is reported by step (i) in < 3 s with A17's line shape | `check` is not run first, or its output is swallowed (`2>/dev/null` is the AGENTS.md "never discard stderr" violation; `arch_check` loud-failure would catch it in a gate script -- `fast.sh` must be listed as a tool script so the check covers it) |
| D | `fast.sh` refuses to run when `bebop.bin` is missing/empty (L12) | the guard from `construct_parity.sh:14-15` was not copied |
| E | `fast: ... GREEN` never appears in a VERDICT block as evidence | a worker cites it; the main session rejects the verdict (a process rule, checked by reading) |

---

## 3. Which existing work A20 sits on

| existing | verdict | reason |
|---|---|---|
| `bebop.bin check` (`cli_check:7244`) | **KEPT, promoted to the first step** | it is the sub-second front-end answer |
| `bench/vs_rust/construct_parity.sh` | **REUSED** with `FREEZE=0`, `BEBOP_BIN=$OUT/gen2.bin`, `BEBOP_TMP=$OUT/fast` | already reports `construct parity: pass= fail=` and `WORD_MISMATCH` per construct |
| `tools/perf.py size` | REUSED for `bin_words`/`push_words` (`invariants.sh:77`) | |
| `tools/typecheck.py`, `tools/check_words.py` | REUSED | non-codegen checks WORKER-CARD already prescribes |
| `tools/chain.sh` `PERF` env (`:49`) | REUSED | `PERF=0` for worker chains |
| gate memo (`std_golden.sh:27-54`) | IGNORED | useless for codegen edits (0b) |
| `tools/std_par.sh` (117 gates sharded) | NOT in the fast lane | 117 gates x compile+run is the battery; the fast lane is the constructs |
| `tools/slot.sh` | NOT taken | the lane is light; taking the slot would serialise it behind the chain it is meant to precede |

---

## 4. Constraints as design input

1. **The box**: procs must stay < 26/32 (WORKER-CARD); `fast.sh` runs `construct_parity.sh` serially
   (it already is: one `for` loop), one compile at a time, `nice -n 10 taskset -c 0-3`.
2. **Never `2>/dev/null` a compile** (AGENTS.md law "FAILURES ARE LOUD" item 2); `construct_parity.sh:23`
   does exactly that today for the compile step -- `fast.sh` must run the construct compile itself with
   stderr captured, or `construct_parity.sh` gains a `VERBOSE=1` that keeps stderr (one line).
3. **Artifact identity (L12)**: gen2 is built into `$OUT` and named by md5 in the summary line, so a
   stale `gen2.bin` cannot masquerade.
4. **No slot, no background, no `&`** (WORKER-CARD box rules).

---

## 5. Steps, in order, each with a number that can kill it

### Step 1 -- write the lane (0.5 day)

**Expected first output** on an unchanged tree: `fast: check 1/1 words 43491 constructs pass=104 fail=0
typecheck 0 findings <s> s` with `s` in 15-30. **Kills the step:** `s > 60` on an idle box (then the
construct lane is not ~0.1 s each and the fast lane needs a subset; measure per-construct ms from
`gates.txt`-style timing before choosing one).

### Step 2 -- the mutation pair (0.5 day)

Scratch copies with (a) a syntax error and (b) the `vs_try_madd` flip; **expected**: (a) RED at step (i)
in < 3 s, (b) RED at step (iii) naming `c74_madd`. **Kills the step:** (b) GREEN -- the lane is blind
and must not be advertised.

### Step 3 -- record and write the rule (0.5 day)

`fast_wall` in `perf.csv`; the WORKER-CARD paragraph; `PERF=0` in the worker's chain command line in
WORKER-CARD "Gates".

---

## 6. The honest ceiling

- **A fast lane cannot be faster than one self-compile + 104 construct compiles**: ~1.6 s + ~11 s
  derived, ~15-25 s in practice on a loaded phone. Below that needs A10's memo, which is a compiler
  change and its own row.
- **It shortens the loop, not the chain.** The chain stays 104-139 s minus `perf.py`'s ~60 s = ~45-80 s
  for a worker; the battery is what it is because 117 gates and 104 constructs are what the honesty
  floor requires. The only way to make the merge criterion cheaper is fewer gates, which is not on offer.
- **What it does not catch:** anything only a std gate sees (the store, threads, generated kernels). A
  worker who edits `store.bp` runs `std_golden.sh` per WORKER-CARD "non-codegen change"; `fast.sh`
  is for `bebop.bp` edits and constructs.

---

## 7. VERDICT format for an A20 worker

```
VERDICT: GREEN|RED
fast: check <n>/<n> words <w> constructs pass=<n> fail=<n> typecheck <k> findings <s> s   (gen2 <md5>)
fast_wall: <s1> <s2> <s3> s, median <m> (<= 30)
mutations: syntax -> RED at step (i) in <s> s; madd flip -> RED at step (iii) naming <construct>
stderr: captured on every compile <yes|no>
worker-card: rule added <yes|no>; PERF=0 in the worker chain line <yes|no>
journal: <one line, WORKER-CARD format, with COST:>
open: <deviations>
```
