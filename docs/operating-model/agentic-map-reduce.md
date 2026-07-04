# Agentic Map-Reduce — universal execution rule

> **Standing rule (operator directive 2026-07-04). Binds EVERY agent — the lead loop and every
> subagent, new and existing — via the shared CLAUDE.md / AGENTS.md context they all load.**
> It is both a *speed* rule (parallelism) and a *token* rule (the reducer holds conclusions, not raw
> material). The token half is load-bearing: the harness token audit
> (`docs/research/2026-07-04-harness-token-audit.md`) measured the subagent dispatch floor at
> ~42K tokens/lane (~28% of session tokens), most of it tool-schema overhead — so HOW you map
> matters as much as WHETHER you map.

## The rule in one line

Any execution that decomposes into ≥2 independent units MUST run **Map → Reduce**: fan the units
out as concurrent narrow-tool subagents that each return a **distilled** result, then synthesize.
Work that does not decompose runs solo. Never fan out work that doesn't split; never serialize work
that does.

## 1. CLASSIFY — the gate (do this before executing anything non-trivial)

Ask: does this task split into ≥2 units that are **independent** (different files/dirs/surfaces),
**collision-free** (no shared mutable state), and **order-free** (B doesn't need A's result)?

- **YES → Map-Reduce.**
- **NO → solo/sequential.** A single hot file, a strict dependency chain, or a trivial edit is not
  map-reducible. Fanning it out wastes a ~42K-token dispatch floor per lane for nothing.
- **PARTIAL →** map the independent part, keep the one shared integration point (a hot file,
  registration, final wiring) for the reducer to do serially after the fan-out.

The gate is the token-safety valve: it stops reflexive fan-out. Bound the map width to the number of
genuinely independent units — never spawn more mappers than there are disjoint slices.

## 2. MAP — parallel, narrow, distilled

- **Concurrent in one message.** Dispatch the units as multiple tool-uses in a single turn so they
  run in parallel, not sequentially.
- **Disjoint slices.** Partition into collision-free lanes; two mappers must never mutate the same
  file concurrently.
- **Narrow tool grants (token rule).** Give each mapper the *smallest* agent type / tool set that
  does its job — a read-only searcher gets `Explore`, not `general-purpose`. The measured ~33K of
  per-lane overhead beyond CLAUDE.md is tool-schema/MCP-connector cost from broad grants; narrow
  grants reclaim most of it. Reach for `general-purpose` only when the mapper genuinely needs to
  write + build + test.
- **Distilled return (the core token win).** A mapper's final message is DATA for the reducer, not a
  transcript. Return the conclusion + evidence pointers (`file:line`, verdict, counts, a short list)
  — never raw file dumps, never the full text it read. The mapper's large context stays in the
  mapper; only the distillate crosses back. This is "keep the conclusion, not the file dumps" made
  mandatory. Bound the output size; if a mapper must pass through bulk data, it writes it to a file
  and returns the path.

## 3. REDUCE — synthesize, deduplicate, integrate

The lead (or a dedicated **reducer** subagent when the fan-out is large enough that synthesis itself
is heavy) does:

- **Merge + dedup** across mapper outputs into one coherent result.
- **Resolve conflicts.** Two mappers disagree → adjudicate from the evidence pointers, or escalate
  (doubt ladder) if unresolved. Never silently pick one.
- **Integrate the shared point** the map phase deliberately left out (the hot file, the registration,
  the final wiring) — serially, once.
- **Trust the distillate.** The reducer does NOT re-read what a mapper already read; it works from the
  returned conclusions + pointers. Re-reading to "double-check" throws away the whole token win.
  (Re-read only on the standard triggers: an explicit conflict, a low-confidence return, or a
  red-line decision that demands first-hand verification.)

## 4. UNIVERSALITY — old and new agents, bounded recursion

- **All agents inherit this** by loading CLAUDE.md + AGENTS.md — no per-agent edit needed, so
  "existing agents" are covered the moment the rule lands in those two files.
- **Recursive but bounded.** A mapper that hits its own decomposable sub-task applies Map-Reduce
  again — bounded to **depth ≤ 2** (a mapper may fan out once; its sub-mappers do not) unless a
  budget directive explicitly widens it, so recursion can't run away into the 42K-floor × N blowup.
- **The lead is a reducer too.** The top loop's job on a decomposable request is to map then reduce,
  keeping its own context small by holding distillates, not the material behind them.

## 5. Interaction with existing rules

- **Ponytail / YAGNI first.** Map-Reduce is for work that is *already* worth doing and *does* split.
  It is not a license to invent parallel work. If the task is one line, write the one line.
- **Task-Exit Rule** still applies per unit and to the reduced whole: each mapper enriches + proves
  its slice; the reducer proves the integrated deliverable.
- **Red-lines don't relax under fan-out.** Money/RLS/auth/migrations/bulk-edit slices still gate
  (council/SSG/human) — parallelism speeds the work, never the approval.
- **Doubt ladder** is the conflict-resolution path in Reduce.

## 6. Anti-patterns (all observed as token sinks)

- Fanning out a non-decomposable task → N × 42K floor for no parallelism gain.
- `general-purpose` mappers for read-only search → paying the full tool-schema floor to run `grep`.
- Mappers returning raw dumps → the reducer inherits every mapper's context; the win evaporates.
- Reducer re-reading mapper inputs "to be safe" → double-pays the read.
- Unbounded recursive fan-out → floor blowup.

## 7. Proposed wiring (protected paths — operator applies)

The rule is universal only once it sits in the always-loaded governance files. Proposed diffs:
`docs/operating-model/proposed-claude-md/agentic-map-reduce-rule.md` — a tight insert for CLAUDE.md
§"Agent Discipline → Tool Use" and the mirror line in AGENTS.md. Both are protected; this spec is the
long form, that insert is the always-loaded short form.
