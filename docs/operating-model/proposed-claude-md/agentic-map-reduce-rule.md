# PROPOSED (protected-path) — Agentic Map-Reduce universal rule

> Operator applies. Two always-loaded governance files get the short form; the long form is
> `docs/operating-model/agentic-map-reduce.md`. Numbers cited come from
> `docs/research/2026-07-04-harness-token-audit.md`.

## Insert into `.claude/CLAUDE.md` → §"Agent Discipline (…)" → "Tool Use", after the existing
## "Spawn parallel subagents…" bullet:

```markdown
- **Map-Reduce every decomposable execution (universal — lead loop AND every subagent).** Before
  non-trivial work, CLASSIFY: does it split into ≥2 independent, collision-free, order-free units?
  YES → **Map** (fan out concurrently in one message) → **Reduce** (synthesize/dedup/integrate).
  NO (one hot file, dependency chain, trivial edit) → solo. PARTIAL → map the independent part, keep
  the shared integration point for the reducer. Two token rules make this a saving, not a cost:
  (1) **narrow tool grants** — give each mapper the smallest agent/tool set for its slice (a
  read-only search gets `Explore`, not `general-purpose`; the ~42K/lane dispatch floor is mostly
  tool-schema overhead from broad grants); (2) **distilled returns** — a mapper's final message is
  DATA (conclusion + `file:line` pointers + counts), never a raw dump; its large context stays in
  the mapper, and the reducer TRUSTS the distillate without re-reading. Bound map width to the unit
  count; bound recursive fan-out to depth ≤2. Full spec: docs/operating-model/agentic-map-reduce.md.
```

## Mirror line into `AGENTS.md` (root), under a new "Execution shape" heading near Agent Discipline:

```markdown
# Execution shape — Map-Reduce (universal)

Decomposable work runs Map→Reduce: classify (≥2 independent/collision-free/order-free units?) → map
(concurrent, narrow-tool subagents returning DISTILLED results, not dumps) → reduce (synthesize,
dedup, integrate the shared point, trust the distillate — don't re-read). Non-decomposable work runs
solo. Never fan out what doesn't split; never serialize what does. Depth ≤2. Spec:
docs/operating-model/agentic-map-reduce.md.
```

## Why protected-path / why propose not force
`.claude/CLAUDE.md` and `AGENTS.md` are governance files loaded by every agent — a change here is a
harness-wide behavior change, exactly the class the protect-paths gate reserves for human sign-off.
The spec + this insert are ready; applying the two blocks above makes the rule universal (all
existing agents inherit it via the shared load, no per-agent edit).
