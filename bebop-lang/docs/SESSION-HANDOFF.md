# SESSION HANDOFF — 2026-09-06 (session 14; resume in ONE read)

Status: 2026-09-06 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are
- Compiler: fixpoint c3f58e8e (2026-09-06). Landed this session: T90 step 1 (exits 95-99
  print `line:col: <message>`; bench/diag_neg + diag_check.sh in the battery), DIVERGE-42122
  = a real miscompile fixed (a 9+-param callee whose body never touched x15 had its
  `add x15,sp,#256` NOPed by OPT-G1 and stored p8 into the CALLER's spill slot 0; the scan
  now starts at pbase+10; construct c53_param9), fuzz_batch.py (in-process shards: 3.5
  programs/s on 3 cores, 1.1/s on one), docs/DEV-LOOP.md (dev_loop.sh was folded into tools/chain.sh).
- Battery: std_golden 99/99, constructs 52 (+ neg c48/c51/c52), diag 8/8, run_all 99,
  parity 12/12+1, pool 5/5, invariants GREEN (census bcond 1536, cbz 113 ALLOWed).
- ONE COMMAND: `tools/chain.sh <src.bp> <out-dir> [--codegen]` (gen2, then gen3->gen4 in
  parallel with tools/battery.sh); a non-codegen change costs ~95 s end to end, a codegen
  change ~125 s (FREEZE=1 in the env re-freezes constructs). invariants.sh after promotion
  (`--freeze` when the census moves + a census_allow.txt line). Promote with cp-to-temp + mv.
- Inner loop numbers: docs/DEV-LOOP.md (self-compile 14-17 s, std gate compile 0.5 s,
  std_golden 8.8 s sharded, fuzz 1.1/s per core).
- Store: unchanged this session (G1-G8 numbers in ROADMAP Measured / RESULT-*.md).

## Session 14 (2026-09-06, HEAD c2173f8 pushed)
- T90 DONE. 2b: `bebop.bin check <src>` (cli_check; fixpoint deef28e0). 2c (operator chose brk):
  every runtime trap site is one `brk #code` word (80 zeros, 81 frame heap x4, 87 unresolved call);
  the entry stub (39 -> 131 words, scratch brk/stub.S assembled with as) registers SIGTRAP and its
  handler prints `trap NN: <text>` + exits with the code (82 for SEGV/BUS). Fixpoint d785e062,
  44 constructs re-frozen at +92 words (word_budget.txt), census.txt k7 603 -> 599, diag 17/17
  with four runtime probes. Rebuilding the stub: edit brk/stub.S in a scratchpad, `as` + `objcopy
  -O binary -j .text`, regenerate the `st[i] =` lines with python, update the 131/42/132 constants
  in cli_compile (check_abi finds the `b .` placeholder itself).
- AGENTS L21 + tools/reap.sh: after every task list/kill orphaned work shells and zombie
  parents (`REAP_PS=tools/reap.fixture tools/reap.sh` is the self-test). Found today: an
  orphaned invariants.sh + 4 zombies from a dead session at the 32-process cap.
- fuzzd: the stop file only ends the loop; runsv restarts it — `sv down $PREFIX/var/service/fuzzd`
  is the real stop (it touches the file via the TERM trap and stays down). `sv up` to resume.

## Next
The ordering lives in ONE place: ROADMAP.md "Critical path" (sorted 2026-09-06). Step 1
there first (fuzz batches of 5000 in the background, the >65536-word fn trap, the
shrinker as the probe ladder, the per-call n^2 term), then the PARTIALs (T104b, T96, T90
step 2). Untracked repros in bench/fuzz/repros (BPREF-TIMEOUT-32137, TRAP-81-32006) are
classified, not bugs (heavy nested loops; trap 81 is by design) — delete or keep.

## Pitfalls that cost hours this session (also in the memory file)
- An 8-deep parenthesised chain of calls `((((f(a)*16+f(b))*16+...` hits exit 95 in the
  compiler — write a loop. A `zeros` inside a while body is L8 (the T80 draft had one).
- `pgrep -f <pattern>` matches the shell that runs it; never pipe it to kill.
- Timing-flag gates (lcjit) miss under three parallel batteries; rerun standalone first.
- AGENTS.md L18: never a wait-only turn while a shell runs — do the next independent item.
- TASKS.md is GENERATED (tools/roadmap_check.sh from HISTORY.md headers): edit the header, not the table.
- OPT-G1-class bugs: any "is this register used" scan must cover the words between the prologue and the body (param stores), not the body alone.
- Derive every instruction word with python int(hex,16) and objdump the emitted .bin: a
  hand-typed clz became `rev`; the first entry stub wrote into the RX code image.
- A clone-spanning fn must keep <= 8 live symbols; workers exit with
  sys_exit_thread_guard (svc 93 since T127).
- Never prefix a runner with `S=...` in this shell ($S is the scratchpad variable); the
  runners use NSRC. `&&`-lists followed by `&` background the whole list.
- Nested `if` as a call argument is the AGENTS.md #6 trap: bind it to a let first.
- st_open ftruncates to the preallocated size: report logical size = arena_used*8.
- The store's crc is crc32x over raw words; crc32(cells,n) is byte-per-cell.
