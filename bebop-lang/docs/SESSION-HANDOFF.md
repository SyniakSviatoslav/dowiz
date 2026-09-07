# SESSION HANDOFF — 2026-09-08 (session 27; resume in ONE read)

Status: 2026-09-08 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are
- Compiler: fixpoint **42ce19e5**, bebop.bin 40418 words, census `bebop 40418 1143 126 0 867 1702 275`.
  Battery GREEN on the promoted bin, invariants GREEN, construct parity 75/75.
- Session 27 landed four commits, each chain-gated and each with its evidence in the message:
  - `056ad43` A13 + A15 — compile-time diagnostics: exit 100 `fn with more than 14 parameters`
    (the two silent `brk #8` clamps in emit_call/compile_fn are DELETED), exit 101 `unbound symbol`
    at five sym_lookup sites, exit 104 `too many fns (cap 512)` split off 89's text. The unbound
    diagnostic exposed a real parser bug fixed in the same commit: **emit_let_stmt now consumes its
    own `in`** — it used to leave it, so emit_body classified `in <body>` as a fresh item and read
    `in` as an identifier (pre-A15 a dead `reg == -1` variable the item pop discarded).
  - `b335a27` A9 `scan` builtin (session-26 Lane D, rebased): 46 as+objdump-verified words, bound is
    the caller's `pos[1]`, `scan` is now a reserved word, bpref stub mirrors bound/class/write-back.
  - `abaab38` B1 step 1 `sys_fsync` (session-26 Lane C, rebased): syscall 82 derived from
    asm-generic/unistd.h, `st_fsync_dir` fsyncs the DIRECTORY after st_compact's publishing rename.
    scrash_torn TRIALS=50 = 0 invalid reopens.
  - `4a81b6b` A14b part 2 spec — root cause of the last corpus exit 89 (a pre-arm kind-3 `SYM`
    window entry relocated inside one if-arm; site is bebop.bp:1855 arm_spanning, and this CORRECTS
    part 1's journal attribution to :1861).
- Lane merge protocol, as actually exercised: `git diff` the worktree's CODE files only,
  `git apply --3way`, then re-derive every gate-table number from the new battery output. Lane
  census_allow / word_budget lines cut against the old base are ALWAYS stale — never copy them.
- Box: `fuzzd` and `boxguard` permanently REMOVED (operator 2026-09-07); fuzz is foreground batches
  only. Heavy jobs (chain, battery, sweep, fuzz batch) go through `tools/slot.sh <label> <cmd>` —
  3 slots, slot 1 = A78 cores 4,5,6 for the main session's merge chain (`SLOT_PREFER=1`), slots 2/3
  = 0,1 and 2,3. Light work (editing, python, oracles, single probe compiles under
  `nice -n 10 taskset -c 0-3`) takes no slot. The one hard refusal is MemAvailable < 600 MB.
- Operator 2026-09-08 (session 27), BINDING: **one main agent + one worker, no more.** The
  four-parallel-lane protocol in docs/blueprints/PARALLEL-LANES-2026-09-08.md describes how the
  session-26 lanes were run and how they were merged; its §3 lane list is history, not a standing
  instruction.
- Three stale worktrees remain at `.claude/worktrees/agent-*` (all at 21e92aa). Their content is
  fully merged; they are untracked and can be dropped with `git worktree remove --force`.
- Free-LLM routing live: `tools/llm_route.sh` (groq, openrouter, mistral, gemini; keys chmod 600
  under ~/.config). Rust toolchain 1.93.1, graphify 0.9.56 at /usr/local/bin/graphify.
- Blocked: `git push` (https remote asks a password, no SSH key authorized). Everything from 21e92aa
  onward is LOCAL ONLY.
- The store trap-82 RED is CLOSED: it was environmental. store.bp hardcodes /tmp/opencode for its
  atomic publish and the box restore had removed the directory; std_golden.sh now mkdir -p's it.

## Next
1. A14b part 2 codegen — the card `docs/blueprints/A14b-corpus-leftovers.md` is executable:
   vs_span_to_slots must also demote kind-3 SYM entries, no new instruction word. Repro
   `c95_symspan` (bpref 17, rc 89 at 6:146 on 42ce19e5) is HELD OUT of bench/parity_constructs
   until the fix lands — construct_parity.sh globs `*.bp`, so an EXPECT-less COMPILEFAIL there
   turns the battery RED.
2. Then A2b / A3 / A10 in their original order: **A12 is REFUTED** by its own measure-first probe
   (Lane B, session 26 — rung 1 buys 0-2 % of K5, not the >= 5 % its gate required) and has left the
   critical path.
3. A5 / A6 next in Phase A; A6 also owns A14b's two trap-81 seeds (100671, 100828), which is why
   A14b part 2 targets seed 100744 only.
4. A4's corpus/fuzz window runs LAST, in foreground 10^4-seed batches on whatever md5 is promoted then.

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
