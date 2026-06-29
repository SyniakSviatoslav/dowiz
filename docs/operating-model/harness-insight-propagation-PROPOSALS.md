# Harness-insight propagation — STAGED proposals (protect-paths → operator/librarian)

The reverse-engineered insights (`meta-loop-reverse-engineering-insights.md`) propagate to a SMALL set of
high-value targets. `.claude/` (agents, skills, CLAUDE.md/AGENTS.md) is protect-paths, so these are staged
proposals, each justified. **No mass edit** — the project already embodies the patterns; these are targeted
reinforcements. Reject any that don't earn their keep (ponytail).

## P1 — Token-efficient read habit (CLAUDE.md/AGENTS.md pointer + pre-edit lesson)
**Insight:** `tools/ccc` (AST symbol search) + repowise skeleton/range reads cut read-tokens vs grep/full-file
— directly serves the standing token-efficiency goal. It's built but under-used by agents.
**Proposal:** add one line to the Agent-Discipline / Tool-Use section: *"Prefer `ccc`/repowise `get_context`
skeleton + range reads over grep sweeps and full-file reads when locating code."* + a `docs/lessons/` entry
(TRIGGER: broad code search) so the `pre-edit-lessons` hook surfaces it. **Owner:** librarian (lesson) + operator (CLAUDE.md).

## P2 — loop-architect M11 independent cross-review (residual from the v0.2 cert)
**Insight:** the v0.2 certification flagged M11 (independent-model cross-review) unavailable — the OpenRouter
free slugs are dead (harness memory). Decorrelated cross-review is the STORM/council insight; a single-model
self-cert is weaker.
**Proposal:** when an independent-model bridge is restored (or via a different local model), route the
loop-architect CERTIFIED verdict through one independent reviewer before "released." Until then the
decorrelated-lens design + adversarial mutant test are the documented substitute. **Owner:** operator (bridge) + loop-architect.

## P3 — Reuse the decorrelated-lens pattern (pointer, not rewrite)
**Insight:** the v0.2 `evaluateLenses` (security · reversibility · perf, all-must-pass) is a reusable
validation primitive. The converge/error-fix/review loops + review agents (security-sentinel,
invariant-guardian) already do council-style decorrelation, so this is a **pointer**, not new code: their
docs can reference the lens primitive as the canonical shape. **Owner:** loop-architect docs. *(Do NOT
refactor working agents onto it speculatively.)*

## P4 — skillspector as the standing scan-before-install gate
**Insight:** `tools/skillspector` = the EvoMap "validate-before-inherit" / scan-before-install control;
already adopted (skill-adoption memory). **Proposal:** confirm the `find-skills`/skill-install flow's docs
name skillspector as the mandatory pre-install scan (a one-line reinforcement if missing). **Owner:** operator.

## Explicitly NOT proposed (would be bloat / against governance)
- Rewriting the ~80 skills or the agent definitions with "insights" — the patterns are already present;
  speculative edits degrade working tools (ponytail / anti-slop).
- Touching product code for these meta-insights — out of scope; the insights are about the *harness*.
- Loosening any firm boundary or widening autoupgrade auto-apply — the v0.2 cert preserved both; keep it.

## Status
P1–P4 staged for operator/librarian. The doc updates (this + the insights doc) + the v0.2 meta-loop upgrade
are the applied portion this round.
