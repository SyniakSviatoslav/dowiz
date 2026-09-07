# A3 lane card — LIN recurrence folding (worker-facing)

Status: 2026-09-08, written by the main session for the next free codegen lane. The DESIGN is
`docs/blueprints/A3-lin-recurrence-folding.md` (2026-09-06, still valid — its preconditions A1
and A2 step 0 are both landed). This card is only the current context, the lane rules and the
acceptance; read the design for the how.

## Baseline

Commit 21e92aa, `bebop.bin` fixpoint `7d8262a1` (ROADMAP A14). Two things changed under A3's
feet since the design was written and the worker must account for them:

1. **A14 (2026-09-07)** made the pre-`if` park path-independent: values live across an `if`
   whose arms can bind now go to temp SLOTS, and the temp-slot cursor `fntab[4574]` resets at
   every item boundary where the window is empty. A folded loop body that contains an `if` will
   therefore see slot parks where the design's author saw cs registers. The design's own scope
   line excludes `if` from a foldable body, so this should not bite — but if you widen the
   detector, it will.
2. **The slot bound is now `s0 + tsp0 > 256`** (was 64) and x14, the frame-heap base, sits at
   sp+0x900 (was sp+0x300). A folded loop that materialises k composed coefficients raises live
   values; if you hit exit 89 with `let binding while a call temp is live`, it is register
   pressure, not a park bug — split the fn or let-bind, per WORKER-CARD.

## Lane rules

`docs/blueprints/PARALLEL-LANES-2026-09-08.md` §1, §1b, §1c and §2 are binding: your own worktree,
`PERF=0 SERIAL=1` on the chain, `nice -n 10 taskset -c 0-3` on probes, no fuzzd, no installs, no
background jobs, no git commit/add/push/merge. The main session merges lanes one at a time.

Changed 2026-09-08 (§1b): every HEAVY job goes through the slot semaphore instead of the old
`reap.sh --check 30` + exit-97 retry, which is gone:

    OUT=/root/.cache/bebop/s26/laneA3; mkdir -p $OUT
    SERIAL=1 PERF=0 BEBOP_TMP=$OUT bash tools/slot.sh laneA3-chain bash tools/chain.sh bebop.bp $OUT --codegen

`tools/slot.sh` blocks for free on a flock until one of 3 heavy slots frees, confines the tree to
that slot's cores and raises PROC_CAP to 70. Never pass `SLOT_ONLY` (slot 1 is the main session's).
Light work (editing, greps, `python3 tools/*.py`, single probe compiles) takes no slot. §1c: route
routine prose -- log summaries, journal drafts, objdump explanations -- to a free model with
`tools/llm_route.sh`, never a number that enters a gate.

## Acceptance (this is the whole gate; nothing partial counts)

- `chain: fixpoint gen3 == gen4 <md5>` and `battery` green apart from the one known pre-existing
  RED below.
- **WORD_DELTA 0 on `c07_while`, `c33_loopalloc`, `c34_loopescape`, `c36_break`** — the
  non-folded path must be byte-identical. This is the single most important line in the run: it
  proves the detector did not change loops it does not fold.
- New constructs c73-c75 from the design, each with `EXPECT` from `python3 tools/bpref.py`, and
  bpref parity on every `bench/vs_rust/std_tests/*.bp` (the LCG/hash loops are the interesting
  ones — they are affine recurrences and will now fold).
- The two performance gates, measured **after the merge, not in-lane** (six workers are on the
  box; PARALLEL-LANES §1 makes in-lane timing rows inadmissible): `k1h_ms <= 0.5x` Rust and
  `k4_ms <= 0.6x` Rust on `bench/vs_rust/honest.sh`. In-lane you may report a K-row only if you
  give its `real`/`user`/`sys` triple and the concurrent-compiler count (`ps -e | grep -c seed`)
  and label it provisional — `/proc/loadavg` is frozen inside this proot and proves nothing.
- census: `bcond` may only rise with a `bench/vs_rust/census_allow.txt` line naming A3;
  `bin_words` growth needs a `bench/parity_constructs/word_budget.txt` line (a shape detector
  plus a folded emitter is a real fn-count increase — expect a few hundred words and justify
  them).
- `bebop.bp` must stay under 511 fns (`grep -c '^fn ' bebop.bp` — it is 268 today).

Known pre-existing RED, not yours, do not chase: the `std_golden` gate `store` traps 82
identically on the old committed `bebop.bin` (environmental; a separate worker owns it under
`docs/blueprints/STORE-GATE-TRAP82.md`).

## Traps specific to A3

- Exactness: the composed affine map must be exact in **wraparound i64**. Compose with Python
  first (`(a**k) % 2**64` and the geometric sum for b), print the coefficients, and put them in
  the construct's comment — a coefficient derived by hand is how this class of optimisation
  silently produces wrong answers for large inputs.
- The tail loop is not optional: trip counts that are not a multiple of k must run the original
  loop for the remainder, and a construct must cover a non-multiple trip count (e.g. 10 trips
  with k = 4).
- The design's §3.14 statement-boundary invariant: the folded loop must leave the free mask 255
  and cs 0. If `invariants: RED` names a window cell, that is this.
- The detector is a text scan with `strn` from the caller and must not advance the shared `pos`
  (WORKER-CARD).
