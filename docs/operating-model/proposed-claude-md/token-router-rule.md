# PROPOSED CLAUDE.md change — TOKEN ROUTER bullet (awaiting operator approval)

**Why:** operator directive 2026-07-05 ("always use the most token-optimized approach based on
task criteria, all agents and subagents should use this"). The full binding rule landed in
AGENTS.md ("RULE: TOKEN ROUTER"); CLAUDE.md is hook-protected (governance zone), so this is the
proposal for the human to apply. Measured basis: `docs/research/token-economy-comparison-2026-07-05.md`.

**Change 1 — update the stale figure in the Map-Reduce bullet (Agent Discipline → Tool Use):**
replace `the measured ~42K/lane dispatch floor is mostly tool-schema overhead from broad grants`
with `measured 2026-07-05: floor = 35,753 tok/general-purpose lane vs 16,960/Explore lane → −18.8K
per read-only lane before any work`.

**Change 2 — append this bullet after the Map-Reduce bullet:**

> - **TOKEN ROUTER (universal — every agent and subagent, operator directive 2026-07-05)**: before
>   any non-trivial step, classify the task and take the cheapest ADEQUATE route — deterministic
>   code before any LLM call; `vsa match` before recall; graph-first (`codebase-memory`) for
>   structure (−20% narrow / −53% broad, measured); repowise skeleton before big-file reads (−90%);
>   `repowise distill` on every noisy command (−92%, lossless via expand); `Explore`-grade grants
>   for read-only lanes (−18.8K/lane); `route.mjs`-decided frames for >1KB payloads (−34%);
>   viz/macro only past the ~25-30-entity crossover. **Quality floors override cost**: red-line
>   audits get sweep + an independent critic lane with byte-verified claims; reasoning/design stays
>   plain-prose on the full model; edits always Read real bytes; escalation ladders never skipped
>   for savings. The lead embeds the router line in every dispatch prompt. Table + measured basis:
>   AGENTS.md "TOKEN ROUTER" + docs/research/token-economy-comparison-2026-07-05.md.
