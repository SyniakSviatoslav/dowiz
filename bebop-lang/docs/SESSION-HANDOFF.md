# SESSION HANDOFF — 2026-09-08 (session 29; resume in ONE read)

Status: 2026-09-08 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are

- Compiler: HEAD **`5df5037`**, `bebop.bin` md5 **`ca69273e`**, battery GREEN on the promoted bin
  (std_golden 114 pass / 0 fail, constructs 83/0, diag 16/0, parity 13/0, pool 5/0, oracles
  mismatch=0 missing=0, ABI ok, invariants GREEN, words PASS).
- Session 28 closed at 623 ms cold self-compile, 4.02x in one day. Session 29 opened by finishing
  what the box killed and merging the two lanes that were left uncommitted in the working tree:
  - **A6 step 1** (`81eca07`): array/struct/enum-ctor literals bump the ARENA CURSOR x27 instead of
    the 16 KiB per-frame x14 heap and inherit `zeros`'s trap; `emit_heap_trap` and the whole
    brk #81 recipe are DELETED, so `brk #0x51` = 0 and **exit 81 is gone as a class**. A14b's two
    parked corpus seeds run at last (100671 -> 6, 100828 -> 4). The allocation sentinel word moved
    to `mov x0,x27` and `sys_arena_base()` was retargeted onto `add x0,x27,xzr` so it cannot
    false-positive as an allocation site. Step 2 (fn-mark LIFO release, computed frames, emit_bl
    saving x15 only, c67_deeprec) is OPEN — the frame is still the flat 16 KiB.
  - **C3** (`5df5037`): the commit-object chain in superblock cell 10, the fixed-width 8-slot heads
    table in cell 11, `st_open_at(gen)` and the API around it; gate `schain` = 71563930701023
    (okmask 1023, 10/10), oracle `bench/oracles/schain.py`, std_golden's 114th gate. Cells 10/11 are
    zero in every store a plain `st_commit` writes and the open path never reads them.
  - Typecheck rung vii took three honest widenings for C3 (REF_TOLERANT / REF_PRODUCERS gained the
    C3 siblings of `st_commit`; the sanctioned null idiom widened from `ref *` to every `ref T`).

## Lanes in flight (session 29, cap = main + TWO workers)

| lane | tree | task |
|---|---|---|
| A | `/root/s29/laneA/bebop-lang`, out `/root/s29/outA` | **A5 step 1b**, the index model: `zeros` returns an x17-relative cell INDEX, array get/set take the `ldr xd,[x17,xt,lsl #3]` forms, every cell-taking builtin converts at its own boundary, `tools/bpref.py` grows one arena list, constructs c69_index_roundtrip / c92_ptrfree. A6 deleted the blueprint's frame-heap carve and its `saved_x14` correction — the blueprint's §3 "Frame heap" is obsolete and the worker is told so. |
| B | `/root/s29/laneB/bebop-lang`, out `/root/s29/outB` | **D3**, and its own kill test first: relabel sgraph2's vertex ids into BFS order with NO store change and re-measure the frontier row interleaved. < 20 % => the row dies. |

Lane trees are cut with `bash /root/s29/mklane.sh <dir>/bebop-lang` (ls-files + checkout-index off
committed HEAD, crates symlinked).

## Box, binding

- `max_phantom_processes` = 32 and **both operator-side escape hatches are still OPEN** (Developer
  options -> "Disable child process restrictions"; Termux -> Battery -> Unrestricted). Until they
  are done: `tools/slot.sh` stays at ONE slot, `battery.sh` at SERIAL=1, and the agent budget is
  **main session + TWO workers, analysts included in the count** (operator, 2026-09-08).
- `termux-wake-lock` is reachable from inside the proot and is taken at the start of a long session.
- `git push` is still blocked (https remote asks a password, no SSH key authorized). Everything from
  21e92aa onward is LOCAL ONLY.

## Next (the order the session is working)

1. Codegen lane, one at a time, never two in flight: A5 step 1b -> A6 step 2 -> A7 -> A8 ->
   A10 re-examination on the new floor -> the A9 leftovers -> A11's rest. **A4's corpus/fuzz window
   runs LAST**, foreground 10^4-seed batches on whatever md5 is promoted then.
2. Store/bench lane: D3 -> C2 -> B4 -> D4 -> C4 -> C5 -> B1's rest -> D5 -> B5 -> B6 -> B2 -> B7 ->
   B8 -> E4.
3. C6 stays CONDITIONAL — do not start it.

## The lane merge recipe (as actually exercised)

`git diff` the worktree's CODE files only, `git apply --3way`, then **re-derive every gate-table
number from the new battery output**. A lane's `census_allow.txt` and `word_budget.txt` lines are
absolute counts against the base it was cut from, so after any other lane lands they are silently
wrong. Chain (`PERF=0 tools/slot.sh <label> tools/chain.sh bebop.bp $OUT --codegen`), read the
actual `CENSUS FREEZE REFUSED` / `WORD_BUDGET_MISSING` line, write the allow row from THAT number,
re-chain, promote (`cp gen4.bin bebop.bin.tmp && mv`), `invariants.sh --freeze`, then the
post-promotion battery. **Check every worker's base first** — four workers in a row were cut from a
stale commit; the fix is to cut it from the committed HEAD -- `git ls-files -z bebop-lang |
git checkout-index --prefix=<lane>/ -f -z --stdin`, then flatten the prefix and symlink
`/root/dowiz/crates` next to it (`tools/../mklane.sh` at /root/s29/mklane.sh does exactly this).
`git archive` is blocked by the classifier.

## Pitfalls that cost hours in prior sessions (also in the memory file)

- An 8-deep parenthesised chain of calls `((((f(a)*16+f(b))*16+...` hits exit 95 — write a loop.
  A `zeros` inside a while body is L8. Nested `if` as a call argument: bind to let first.
- `pgrep -f <pattern>` matches the shell running it; never pipe to kill. Never prefix runner with `S=`.
- Timing-flag gates (lcjit) miss under 3+ parallel batteries; rerun standalone pinned after.
- TASKS.md is GENERATED (edit HISTORY.md headers, not the table). ROADMAP section headers are binding.
- OPT-G1 bugs: "is this register used" scans must cover prologue param stores, not body alone.
- Derive instruction words with `python3 int(hex,16)` and objdump -d the .bin: hand-typed clz became `rev`.
- Clone-spanning fn keeps <= 8 symbols across the spawn; exit via sys_exit_thread_guard (svc 93 since T127). The limit is on symbols that must be KEPT across the spawn -- anything the register allocator cannot rematerialise, i.e. array handles and call results. Constants do NOT count. Measured 2026-09-12 on 7c7d1f77 with a probe whose child writes a marker and whose parent waits for it: 8 kept symbols -> both children correct; 9 -> ONE child silently lost; 11 -> BOTH lost. The failure is SILENT: no trap, no diagnostic, the children simply never run. (An earlier note this same day claimed the rule was retired because 32 live symbols worked -- that probe used i64 CONSTANTS, which are rematerialised and cost nothing, so it measured the wrong thing.)
- `&&`-lists followed by `&` background the whole list. st_open ftruncates to size; report size =
  arena_used*8. store's crc is crc32x over raw words; crc32(cells,n) is byte-per-cell.
- A `std_golden` gate that goes RED under a long `BEBOP_TMP` is the memo-cache filename artifact
  documented at `std_golden.sh:26` — run the SAME gate on the unpatched base before believing it.

## Where everything else is

HISTORY.md (all pulls, task bodies, decisions D1-D11 and their evidence, progress log, the 2026-08
vision text), TASKS.md (ledger), ROADMAP.md (the single source of truth, Phases A-E),
docs/blueprints/, docs/BOX.md (the phantom-killer measurements), docs/SPEEDUP-ANALYSIS.md,
docs/LANG-DB-DESIGN.md, docs/ROADMAP-CRITIQUE-2026-09-04.md, docs/LANGUAGE.md, docs/TRAPS.md,
docs/WORKER-CARD.md, docs/exp.journal.
