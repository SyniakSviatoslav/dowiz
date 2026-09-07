# SESSION HANDOFF — 2026-09-08 (session 25; resume in ONE read)

Status: 2026-09-08 CURRENT (rewritten at every session close; task bodies live in HISTORY.md,
the ledger in TASKS.md, one line per experiment in docs/exp.journal)

Repo: /root/dowiz (git@github.com:SyniakSviatoslav/dowiz.git, branch main) — bebop-lang is a
SUBDIRECTORY of that repo: `git show HEAD:./bebop.bin`, commits carry `bebop-lang/` paths.
HEAD: `git log --oneline | head -3`; every commit message carries the gate evidence.

## Where we are
- Compiler: fixpoint 7d8262a1 (ROADMAP A14 `let` binding restriction lifted). bebop.bin 39688 words (160588 B), stub 131, 268 fns in bebop.bp. A14 LANDED: 327/330 UNSUPPORTED-89 corpus repros now compile and match bpref
  (was 0/330 before A14). The 3 leftovers are A14b (1 non-arm-spanning exit-89) and A6 (2 frame-heap
  trap-81). Corpus-driven fuzz not running (operator rule: foreground batches only, no daemon).
- Box state (restored 2026-09-07): `fuzzd` and `boxguard` permanently REMOVED per operator decision;
  fuzz runs foreground-only in 10^4-seed batches (not 24 h daemon at 10^5). Process cap 30; idle box
  ~26 procs, chain adds ~15. One writer owns bebop.bp at a time; up to 4 workers can run in parallel.
- Newly installed this session: Rust toolchain (rustc 1.93.1, cargo 1.93.1 via apt — fixed `money`
  and `ordfsm` oracle ERR). graphify 0.9.56 at `/usr/local/bin/graphify` (pip-installed, no
  per-tool-call hook). mempalace memory-palace plugin v0.1.0 user-scoped enabled (MCP, talks to
  https://m.cuer.ai). Free-LLM routing live: `tools/llm_route.sh` has working keys for groq,
  openrouter, mistral, gemini (keys at ~/.config/llm/groq.key, ~/.config/llm/mistral.key,
  ~/.config/llm/gemini.key, ~/.config/openrouter/key, all chmod 600).
- Blocked: `git push` (https remote asks password, no SSH keys authorized for GitHub; commit 21e92aa
  is LOCAL ONLY). All roadmap work accumulates locally until operator supplies token/key.
- Known pre-existing RED: std_golden gate `store` trap-82 (SIGSEGV/SIGBUS) traps identically on OLD
  committed bebop.bin (box-restored 2026-09-07); environmental, not compiler regression. Worker
  root-causing it under docs/blueprints/STORE-GATE-TRAP82.md.

## Next
ROADMAP.md "Critical path" unchanged: A2b/A3 follow-ups, then A4 (corpus sweep at fuzz-freeze scale),
then the rest of Phase A (A5-A15). Operator decisions 2026-09-08 reordered A12 BEFORE A2b/A3/A10 (flat
IR + graph RA replaces window allocator those rows polish). Specs/blueprints for every row written
before worker starts it (A2b blueprint exists; A12 pending). A4's fuzz window is now foreground
batches with 10^4-seed target, not 24 h daemon.

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
