# CLAUDE.md — agent operating discipline (rewritten for Hermes self-upgrade)

## Memory-first protocol (operator, 2026-07-11)
1. **Update living memory FIRST.** Before writing/planning any code, record new state to the
   canonical corpus. The corpus is the source of truth, not chat history.
   - dowiz (product) → `/root/dowiz/MEMORY.md`.
   - bebop/bebop2 (protocol) → `.../bebop-repo/` corpus.
2. **Push plans to remote FIRST.** Any plan/roadmap/decision doc is committed and pushed to
   `origin` before execution begins — so it can never be lost to a crashed session or stale context.
3. **Ground truth outranks plans.** Re-verify code claims with `grep`/`git`/tests before trusting a
   pasted "verified" status. A plan describes the *desired* state; the live repo is what *is*.
   Record both separately: DONE (verified) vs PLANNED. Never let a stale plan silently overwrite
   ground truth.

## Self-upgrade discipline (operator, 2026-08-12)
1. **Oracle-first ETA.** Before any task, invoke `kernel/src/oracle.rs` to estimate ETA and complexity.
2. **Spec → Blueprint → TDD → parallel agents → verify.** Spec and blueprint MUST precede code.
   Tests are RED before GREEN. Verification is by a different model/agent — never self-verify.
3. **Parallel-first, swarm when parallel-safe.** Frame independent work as parallel-safe and dispatch
   via the orchestrator. Sequential gates (operator decisions, external validation, red-lines) run
   single-threaded and block.
4. **Kernel + Rust over Python/bash.** Prefer `kernel/src/` primitives (json, parse, event_log,
   spectral, markov, predict, spool, fsm, ritual, retrieval). Replace shell/grep/cat with
   `fd`/`rg`/`tsx` and kernel-native search when speed matters.
5. **Vector navigation for code lookup.** Use BM25 + trigram + PPR (in-memory indices) for semantic
   code search, not just textual rg. Maintain `navigation.json` and refresh when the tree changes.
6. **Live memory.** Keep `live_memory.json` current for the active session: recent facts, decisions,
   BLOCKERS, WORK_IN_PROGRESS, NEXT. Persist at natural checkpoints.
7. **Prompt/skill enrichment always on.** When a task arrives, run intent detection + enrichment
   (prompt_enrich.rs) and select/reuse skills from the library before writing new code. Never write
   a skill that already exists.
8. **No silent adoption.** Any new dependency/crate/API/transport/backend swap must pass a decart
   comparison and leave a decart report in the change. Modern/Rust-native is the default and tiebreak;
   older tech is kept as a bridge, not purged.

## Tool Use
- **Read before edit**: Never edit a file without reading the relevant section first. No blind writes.
- **Existing files win**: Edit rather than create. Never make a new file when an existing one can be extended.
- **One edit per turn**: Don't batch multiple file edits in a single step — confirm each before the next.
- **Don't over-tool**: If the answer is already known or the task is trivial, respond directly without calling tools.
- **Investigate before escalating**: Use search/read tools exhaustively before asking the user for information
  they didn't volunteer. Only ask when the information genuinely can't be found.
- **Parallel when truly independent**: Batch tool calls only when they have zero ordering dependency.
  If B depends on A's result, run sequentially.

## Planning
- **Think before critical actions**: Pause before git commits, deployments, schema changes, or declaring a
  task complete. State what you're about to do and why.
- **Todos for 3+ step tasks only**: Don't create task lists for simple work. Exclude linting and type-checking
  from todos — they're verification, not tasks.
- **One task in_progress at a time**: Serialise execution; context thrashing from parallel active tasks causes mistakes.

## Error Recovery
- **Test failures = code is wrong**: When tests fail, assume the implementation is wrong unless explicitly
  told otherwise. Don't rewrite tests to pass.
- **Route around environment issues**: If a local tool is broken, use alternatives (CI, remote, different command)
  rather than blocking on a fix. Report the environment issue separately.
- **Fix before proceeding**: Any script, hook, or shell error stops the current task. Fix the root cause, then resume.

## Code Standards
- **Match the project's conventions**: Read existing patterns before generating new code. Don't impose your own style.
- **No output of code unless requested**: Use edit tools silently. Keep chat focused on intent and decisions, not diffs.
- **Non-interactive flags**: Always pass `--yes`, `--non-interactive`, etc. for automation-context commands.
  Never assume a human can respond to a prompt.

## Verification rules
- Compile the touched crate, run its tests, run clippy — fresh evidence before claiming done.
- Use a different model/agent to verify; self-verification is banned for correctness.
- Record verification evidence in MEMORY.md (what ran, what passed, what failed).

## Session closing
- End every work session with: git status, staged/unstaged summary, git log --oneline -3,
  origin/main comparison, and a short MEMORY.md note saying what changed and what is still open.
- If blocked, record the blocker in MEMORY.md with the exact failure and what was tried.

Source: derived from AGENTS.md operating spine + DECISIONS.md invariants + operator directives 2026-07-11/14/16
+ pasted rule sets (Planning, Error Recovery, Code Standards, Tool Use) folded in as binding rules.
