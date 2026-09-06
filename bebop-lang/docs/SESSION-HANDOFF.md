# SESSION HANDOFF — 2026-09-06 (session 16; resume in ONE read)

Status: 2026-09-06 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are
- Compiler: fixpoint e14dd55e (T96 P3 cmp_try). bebop.bin 74804 words, stub 131, 199 fns.
  Battery GREEN; docs/PERF.md is generated at the end of every tools/chain.sh (perf.py, D12-A).
- Session 16 landed (all pushed to origin main): 01185e7 D12-D hygiene, 94cf3fc D12-C
  (TRAP-81/82 split, fuzz_trap82 = ALERT), e97d301 docs/DECISIONS-RESEARCH-2026-09-06.md,
  67855f5 D13 (all 12 retro proposals mechanised: PROC_CAP gate, FREEZE-on-codegen,
  check_words.py, self-copy exec, serialised lcjit, journal linter, commit-msg hook, fuzzd
  pause|resume) + D14 recorded (all 12 research recommendations accepted).
- Roles (AGENTS L22): the main session (Fable) plans/specs/orchestrates; Sonnet agents do the
  roadmap items; Opus agents write research. Agent spawns are gated by ~/.claude/hooks/agent-gate.sh
  (reads > 30 = idle+~4, or 2 agents already) -- max 2 agents.
- Box calibration: a chain adds +15 procs (ps -e 26 -> 41, box alive); PROC_CAP default 30 =
  idle box; `tools/fuzzd.sh pause` before chains/agents (it parks after the batch in flight; pair
  it with `tools/fuzzd.sh stop` the first time so runsv restarts the new file), `resume` at close.
- In flight when this was written: a Sonnet agent on the D14 ledger side (T53/T54 DELETED, PARKED
  heading, LMDB out of the thesis, critical path B1 -> B2 -> B5 -> tag-stack IR -> K8 -> B4) and
  a Sonnet agent on B1 (prologue/epilogue + call-site x15/x14 right-sizing from planning-pass
  facts; expected K2H 3.8x -> ~2.7x, bin_words -3 %). If the tree holds an uncommitted bebop.bp
  diff, that is B1: check docs/exp.journal for its last line before touching it.

## Next
The ordering lives in ONE place: ROADMAP.md "Critical path" (D14, 2026-09-06): B1 -> B2 (value in
x0 at if/while joins) -> B5 (loop rotation), each a single-variable `tools/chain.sh --codegen`
commit with constructs re-frozen and an honest.sh row; then the operand-tag-stack IR rung (D12-B
amended; report §3.3) -> K8 (branchy honest kernel; T52 conditional on its row) -> B4 computed
frames; store: profile the 45-90 s CSR build (B8) before any store change; after the IR rung a
24 h codegen freeze so fuzz_seeds_on_bin reaches 10^5 on one md5. Operator step still pending:
~/adbfix.sh from Termux after `adb pair` (removes the 32-phantom cap).

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
