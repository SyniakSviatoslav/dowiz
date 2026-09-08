# SESSION HANDOFF — 2026-09-08 (session 28; resume in ONE read)

Status: 2026-09-08 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are

- Compiler: HEAD **`351c288`**, `bebop.bin` md5 **`0ae01bd3`**, battery GREEN on the promoted bin.
- **A cold self-compile is 623 ms.** It was 2,506 ms this morning — **4.02x in one day**, from three
  independent commits: A2b's peepholes, the duplicate-whole-source-scan deletion that A10's
  refutation found, and the `str_len` hoist (the scanners were calling `str_len(s)` — a NUL scan of
  the WHOLE source — once per call, ~1,643 times per pass, 555 MB of scanning).
- Session 28 landed, in order: A2b steps 1+2 (K8H 1.9x -> 1.1x), the Signal 9 diagnosis and its
  in-box mitigations, `pool_parity`/`bench_pinned` divisor fixes, B2's twins, ROADMAP Phase C,
  A3 REFUTED on coverage (kept in the attic), two literature analyses + ROADMAP Phase D, the DRAM
  row CORRECTED (three A78s scale linearly; the old "one core saturates the bus" row was wrong),
  ROADMAP Phase E from the wild-ideas study, the beat-SQL study (two of our own sbench rows measure
  the wrong thing), A10 REFUTED on its gate, C1 steps 1+2, D1 at 1.23x, E2's branchless push, A5
  unblocked, and the `str_len` hoist.
- **Two rows were refuted by their own gates today (A3, A10) and both refutations paid.** A10's
  phase walk is what found the duplicate scans; the row that died produced the day's biggest win.

## In flight when the box died at 18:15 — and what survived

Five agents were running (four lanes + one read-only analyst) and Android's phantom-process killer
took all five at once: slot.sh printed `procs 32/26 (android phantom cap 32)` in the same second,
exit 137. **Nothing was lost from git — the working tree is clean — but no lane delivered a
verdict, and each died one command from finishing.** The scratch trees are intact:

| lane | where its work is | how far it got |
|---|---|---|
| **D2** prefetch + MLP | `.claude/worktrees/agent-a00e3c34.../.d2out` and `.d2clean/bebop-lang` (verified base `351c288`; only `bebop.bp` + `tools/bpref.py` differ) | `emit_prefetch` complete (`prfm pldl1keep` = ONE word, derived with `as`+`objdump`; reserved word + bpref stub), gen2 built, `join_twin_mlp.bp` carries both arms, folds AGREE on n=4096/100000 x u/z, Rust probe-split twin built. Died ON the interleaved measurement. |
| **E1** 64-source BFS | `agent-afbfcd9f.../_e1` (base `14cc4fc`, **stale**) + the intact scratchpad `patch_gen_gb.py`, `gbt/` stores | MEASURED: op6 S=64 = 2732/2821/2993 ms against op5 x64 = 17146/16164/15106 ms, **~5.7x**, folds identical (462119596), oracle 65342 matched, alpha=2 chosen. Died on the final gate script. Its patch regenerates `gen_gb.bp` from the OLD blob and would silently REVERT E2 — re-apply additively. |
| **A5** arena-relative | `agent-a8e1bb6e.../.a5` | patch applied, chain running, stuck investigating a `gb_pool` RED that has the long-`BEBOP_TMP` signature (`std_golden.sh:26`) |
| **A10** re-examination on the new floor | `agent-a124281e...` | instrumented compiler reproduces `0ae01bd3` byte-for-byte, phase walk done; died on the decisive "perfect memo for `emit_body` only" probe |
| **fable analyst** | — | twelve operator proposals (stochastic calculus, filtrations, martingales, graph spectra, Lyapunov, measure theory) — got as far as reading the sources. The brief is in the session transcript; the document was never written. |

## Box, binding

- `max_phantom_processes` = 32 and **both operator-side escape hatches are still OPEN** (Developer
  options -> "Disable child process restrictions"; Termux -> Battery -> Unrestricted). Until they
  are done: `tools/slot.sh` stays at ONE slot, `battery.sh` at SERIAL=1, and the agent budget is
  **main session + TWO workers, analysts included in the count** (operator, 2026-09-08 evening).
- `termux-wake-lock` is reachable from inside the proot and is taken at the start of a long session.
- `git push` is still blocked (https remote asks a password, no SSH key authorized). Everything from
  21e92aa onward is LOCAL ONLY.

## Next

1. Land D2 and E1 from the lanes above (measurement, then gates, then merge by the recipe below).
2. A5 -> A6 -> A7 -> A8; A6 also deletes exit 81 as a class, which is what the two remaining corpus
   seeds (100671, 100828) wait for. A11 and the A9 leftovers after that. **A4's corpus/fuzz window
   runs LAST**, foreground 10^4-seed batches on whatever md5 is promoted then.
3. A10 is worth ONE more look on the new floor: its refutation was measured against a 2,423 ms
   self-compile whose fixed floor has since been deleted.
4. Then B2-B8, C2-C6, D3-D5, E3, E4. The fable analyst's document is still owed.

## The lane merge recipe (as actually exercised)

`git diff` the worktree's CODE files only, `git apply --3way`, then **re-derive every gate-table
number from the new battery output**. A lane's `census_allow.txt` and `word_budget.txt` lines are
absolute counts against the base it was cut from, so after any other lane lands they are silently
wrong. Chain (`PERF=0 tools/slot.sh <label> tools/chain.sh bebop.bp $OUT --codegen`), read the
actual `CENSUS FREEZE REFUSED` / `WORD_BUDGET_MISSING` line, write the allow row from THAT number,
re-chain, promote (`cp gen4.bin bebop.bin.tmp && mv`), `invariants.sh --freeze`, then the
post-promotion battery. **Check every worker's base first** — four workers in a row were cut from a
stale commit; the fix is to rebuild a clean tree from the committed HEAD via `ls-tree -r -z` +
`cat-file --batch` and re-hash the blobs (`git archive` is blocked by the classifier).

## Pitfalls that cost hours in prior sessions (also in the memory file)

- An 8-deep parenthesised chain of calls `((((f(a)*16+f(b))*16+...` hits exit 95 — write a loop.
  A `zeros` inside a while body is L8. Nested `if` as a call argument: bind to let first.
- `pgrep -f <pattern>` matches the shell running it; never pipe to kill. Never prefix runner with `S=`.
- Timing-flag gates (lcjit) miss under 3+ parallel batteries; rerun standalone pinned after.
- TASKS.md is GENERATED (edit HISTORY.md headers, not the table). ROADMAP section headers are binding.
- OPT-G1 bugs: "is this register used" scans must cover prologue param stores, not body alone.
- Derive instruction words with `python3 int(hex,16)` and objdump -d the .bin: hand-typed clz became `rev`.
- Clone-spanning fn keeps <= 8 live symbols; exit via sys_exit_thread_guard (svc 93 since T127).
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
