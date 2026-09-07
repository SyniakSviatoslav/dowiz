# Parallel lanes — 2026-09-08 (operator: 6 workers, 4 of them on codegen)

Status: 2026-09-08 PROTOCOL + lane cards, written by the main session. Binding for every worker
started from it. Supersedes, for these lanes only, WORKER-CARD's "ONE compile/run/chain/battery
at a time" — the isolation is now per worktree, not per box, and the box limits below replace it.

## 0. Why worktrees

The repo rule "never two codegen tasks in flight" exists because a codegen task owns three shared
things: `bebop.bp`, the promoted `bebop.bin`, and the 70 frozen construct binaries. The operator
wants four codegen lanes at once, so each codegen lane runs in **its own git worktree** — a
private copy of all three — and the main session merges the lanes back **one at a time**, running
one chain per merge. Nothing else changes: inside a lane the WORKER-CARD gates are unchanged.

## 1. Box limits while lanes run (hard)

- ONE chain per lane, in the foreground, never two inside the same lane, never backgrounded.
- `PERF=0` on every chain (the perf stage is ~60 s of extra load and its numbers are meaningless
  with four lanes running; the main session re-measures after the merge).
- Prefix every probe compile/run with `nice -n 10 taskset -c 0-3`; leave the chain's own pinning
  alone.
- `bash tools/reap.sh --check 30` before a chain; on exit 97: `tools/reap.sh`, then
  `tools/reap.sh kill`, then ONE retry.
- **Timing rows are invalid while lanes run.** `/proc/loadavg` is useless here: inside this proot
  it read `0.12 0.07 0.02` continuously while six workers compiled, i.e. it is frozen, not
  measured. If your gate is a time (K-rows, lcjit, honest.sh), report instead (a) the
  `real`/`user`/`sys` triple of every run — real inflating while user stays flat IS the
  contention signal — and (b) the number of concurrent compiler processes at that moment
  (`ps -e | grep -c seed`; a count only, never pipe a process match to kill). A timing claim
  without those is not evidence, and a delta measured across differently-contended arms is not a
  delta.
- Never `tools/fuzzd.sh` (the daemon is deleted permanently — operator 2026-09-07). Never install
  anything on the box. Never a background job, a supervisor, a wake lock or a cron entry.
- Never `git commit`, `git add`, `git push`, `git merge`, `git rebase`. The main session merges.

## 1b. Box capacity, revised (operator 2026-09-08: "max agents, no overload")

Measured this session, not assumed: 7 cores in the affinity mask (4,5,6 = A78 big, 0-3 little),
7.5 GB RAM with ~2.5 GB available, one self-compile of `bebop.bp` = **2.0 s wall / 22 MB maxrss**,
a `SERIAL=1` chain = ~13 extra procs. So neither memory nor the old `PROC_CAP=30` is the real
limit -- CPU contention and the battery's fork storm are. The cap therefore stops being a refusal
and becomes a **semaphore**:

- Every HEAVY job (chain.sh, battery.sh, a corpus sweep, a fuzz batch) runs as
  `bash tools/slot.sh <label> <command ...>`. It blocks -- for free, on a flock, no poll loop, no
  CPU -- until one of the **3 heavy slots** is free, then confines the whole process tree to that
  slot's cores (slot 1 = the three A78 big cores 4,5,6, reserved for the main session's authoritative merge chain via `SLOT_ONLY=1`; slot 2 -> 0,1; slot 3 -> 2,3) and exports `PIN` so chain.sh cannot re-pin out of the slot and sets `PROC_CAP=70` so chain.sh's
  own gate does not refuse a legitimate third lane. Light agents run unpinned at `nice -n 10`.
- Consequence: exit 97 and the "reap, reap kill, ONE retry" dance are GONE for heavy jobs. A lane
  never dies because another lane was compiling. `tools/slot.sh --status` prints who holds what.
- The one hard refusal (exit 96) is `MemAvailable < 600 MB`. That is the only failure mode that
  takes the box down instead of merely slowing it.
- LIGHT work -- editing, reading, analysis, `python3 tools/*.py`, oracles, single probe compiles
  under `nice -n 10 taskset -c 0-3`, `tools/llm_route.sh` -- takes NO slot and has no agent limit.
  Run as many lanes as there is work for; only the heavy phase queues.
- Unchanged and still hard: no `&` background job, no daemon/supervisor/cron/wake-lock, never
  `tools/fuzzd.sh`, never `pkill -f`, never a git write from a lane.

## 1c. Token economy in a lane (operator 2026-09-08)

Routine text work does NOT belong to a paid model. Route it to a free one with
`tools/llm_route.sh` (groq -> openrouter -> kilo -> ovh -> llm7 -> mistral -> gemini -> nim, keys
already installed):

    printf '%s' "$prompt" | tools/llm_route.sh            # auto-fallback
    LLM_MAX=2000 tools/llm_route.sh groq < prompt.txt     # a named provider

Use it for: summarising a long log or diff, drafting a doc/journal paragraph, proposing shrink
candidates for a repro, explaining an objdump block, naming things. Do NOT use it for: anything
that decides a gate, a number that goes in a journal line, or a patch to `bebop.bp` -- a free
model's output is a draft you verify with a script, never evidence. Never put a key, a token or
the contents of `~/.config` into a prompt.

## 2. What a lane hands back

One VERDICT block as the final message: the fixpoint md5, the battery gate lines verbatim, the
files changed, the gate-table lines added (`census_allow.txt`, `word_budget.txt`,
`check_abi.py` ZONES, `construct_parity.sh` EXPECTs) with the reason for each, and the exact
journal line appended to `docs/exp.journal`. Plus, for the merge: the list of files your lane
touched, so the main session can see conflicts before it merges.

Known pre-existing RED, owned by another worker, never yours to chase: the `std_golden` gate
`store` traps 82 identically on the old committed `bebop.bin` (environmental, box restored
2026-09-07).

## 3. The lanes

Baseline for all four: commit 21e92aa, `bebop.bin` fixpoint `7d8262a1`.

### Lane A — A13 + A15 (running in the main tree)
Spec: `docs/blueprints/A13-A15-diagnostics.md`. Owns `bebop.bp` in the main tree.

### Lane B — A12 rung 1, the measure-first probe ONLY
Spec: `docs/blueprints/A12-flat-ir-and-graph-ra.md` §3 "Measure first".
The blueprint's own gate is conditional and this lane is that condition — **it does not build the
IR.** Hand-rewrite `skip_ws` and `read_ident` in a scratch copy of `bebop.bp` (hoist `pos[0]`
into a local and write it back at exit; inline `slen`/`is_alpha`), compile with the promoted
compiler, and measure K5 (self-compile of `bebop.bp`, cold, median of 3, `taskset -c 4`).
Verdict: delta ≥ 5 % → A12 rung 1 is worth its ~30 h and the roadmap keeps it first; delta < 5 %
→ journal `VERDICT:refuted` and A12 rung 1 is dropped from the critical path (rung 2 alone is not
worth it either, per the blueprint). Report the `real`/`user`/`sys` triple and the concurrent-compiler
count with every number (§1: `/proc/loadavg` is frozen in this proot and proves nothing).
Nothing is promoted, nothing is committed: the scratch copy stays in the lane's scratch dir.

### Lane C — B1 step 1: `sys_fsync` builtin + directory fsync after compaction
Spec: `docs/blueprints/B1-durability-torn-write.md` (the OPEN part of the ROADMAP B1 row: "step 1
sys_fsync builtin + st_compact directory fsync after A4").
A new builtin is codegen: add it beside the existing `sys_*` builtins (`grep -n "sys_rename\|
sys_export" bebop.bp` shows the shape — dispatch by name hash, no arity), give it the syscall
number for `fsync` on aarch64 (**derive it, do not guess: `grep -rn fsync /usr/include/asm-generic/unistd.h`**),
add a construct that fsyncs a written file and returns a known value, then use it in
`selfhost/prelude/store.bp`'s `st_compact` to fsync the *directory* after the rename so the
publish survives a power cut. Gate: chain fixpoint + the new construct + `std_golden` unchanged
apart from the store rows, and the existing `scrash_torn` gate still 0 invalid reopens
(`TRIALS=50`, its golden is 0).

### Lane D — A9: the `scan` builtin (first of the four, the only one that needs no A8)
Spec: `docs/blueprints/A9-neon-builtins.md`. The ROADMAP row says "A1 (scan any time), A8
(cmp_mask on u32)" — so this lane lands **`scan` only** and leaves `cmp_mask`/`sum64`/`umulh` to
a later lane after A8. Emit the NEON words with the card's recipe: `as` the instruction,
`objdump -d` it into `$BEBOP_TMP/words.objdump` BEFORE editing `bebop.bp`, then insert the
decimal via a script (a hand-typed word became a `rev` once — that is why the recipe exists).
The builtin needs a `tools/bpref.py` stub so constructs have an oracle. Gate: chain fixpoint,
the new construct, and a K5 row measured after the merge (not in-lane: see §1).

## 4. Merge protocol (main session, not the workers)

Lanes merge one at a time, in this order: A13+A15 → B1 step 1 → A9 scan → (A12 probe merges
nothing; it produces a verdict). After each merge the main session re-runs ONE
`SERIAL=1 PROC_CAP=30 tools/chain.sh bebop.bp $OUT --codegen`, re-freezes the constructs, and
commits. A lane whose fixpoint no longer holds after the merge is rebased by its own worker, not
patched by the merger.
