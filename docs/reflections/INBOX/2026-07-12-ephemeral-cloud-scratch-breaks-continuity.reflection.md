# Reflection: ephemeral cloud checkouts break every "local scratch persists" assumption

## CONTEXT
Autonomous plane-maintainer firing, 2026-07-12 (Sunday). SENSE step found 0 hard fails, so most of
the run's effort went into three small continuity failures that turned out to share one root cause.

## DECISIONS
- `node scripts/plane-telemetry.mjs resolve --prediction-id 258cbb9e1b1a ...` failed with
  `prediction 258cbb9e1b1a not found` even though `inbox --json` (which reads the `telemetry/plane`
  branch) showed it clearly. Read the source (`PREDICTIONS_PATH = loops/runs/predictions.jsonl`,
  `.gitignore:80`) instead of assuming a bug — confirmed the file was simply absent on this fresh
  checkout. Resolved the underlying question by direct means (a `curl` probe of the actual claim)
  instead of forcing the CLI call to "succeed."
- `node scripts/new-dep-scan.mjs` printed "no baseline yet" on what the charter treats as a routine
  daily step. Read the source again (`BASELINE = loops/runs/dep-baseline.json`) — same directory,
  same `.gitignore` rule. Ran `--bump` to bootstrap rather than inventing a fake "0 newcomers, baseline
  already existed" claim.
- The Sunday cross-pattern memory-synthesis ritual names a specific memory
  (`memory-corpus-meta-patterns-2026-07-02`) and its `[[link]]` graph. Searched the repo (no
  `memory-corpus` directory, no MCP memory tool connected via `ToolSearch`), then found the actual
  location named in a prior reflection: `/root/.claude/projects/-root-dowiz/memory/...` — a path on a
  *different* Claude Code project root (this session's project dir is `-home-user-dowiz`, confirmed via
  `ls /root/.claude/projects/`). Escalated as blocked rather than fabricating a plausible-sounding
  synthesis of memories I cannot read.

## WHERE
- `scripts/plane-telemetry.mjs:470` (`PREDICTIONS_PATH`), `.gitignore:80` (`loops/runs/*` ignored)
- `scripts/new-dep-scan.mjs:15` (`BASELINE`, same directory, same ignore rule)
- `docs/reflections/INBOX/2026-07-02-plane-maintainer-env-probe.reflection.md:23` (names the
  `-root-dowiz` memory path)
- `docs/governance/plane-status-2026-07-12.md` ("Actions taken this run" — where this was reported)

## WHY-causal
All three tools were designed and, presumably, first exercised on a **persistent** host (the "one box"
`TOOLING-REGISTRY.md` describes — 4 vCPU / 7.6 GiB, with a live Ollama, Repowise index, and a
`/root/.claude/projects/-root-dowiz/memory/` directory that accumulates over weeks). The plane-maintainer
charter (`docs/governance/plane-maintainer-agent.md`) was authored to run there but is *also* fired as
"an Anthropic-cloud scheduled routine, independent of the Hetzner box" — and a cloud firing gets a fresh
git clone with no history, no `node_modules`, no `loops/runs/*` scratch, and no filesystem path to the
Hetzner box's memory store. `predictions.jsonl` and `dep-baseline.json` are correctly gitignored (they're
meant to be ephemeral working state, durable copies live on `telemetry/plane`) — but the *tools that read
them* (`resolve`, newcomer-diff) were never given a "hydrate from the branch first" path, so on a cloud
firing they always look empty rather than degrading to the branch's view. The memory corpus has no
durable copy anywhere reachable from a cloud firing at all — it's the same shape of gap, but with no
`telemetry/plane`-style fallback to degrade to. Three independent-looking failures, one causal root: the
charter's steps were written assuming continuity that the cloud runtime by construction does not provide.
This is the memory-corpus pattern "verify-artifact-not-proxy" one level up — the *runtime environment*
itself is being silently treated as a stable proxy for state that is not actually there.

## CONFIDENCE
High on all three individual facts (each verified by reading source or listing the actual filesystem,
not inferred). Medium on the generalization that this recurs identically on *every* cloud firing —
plausible from the design (gitignore + fresh clone are structural, not one-off), but only directly
observed this once.

## NEXT-TIME
- Before treating a "not found" / "no baseline yet" result from a `loops/runs/*`-backed tool as a real
  finding, check whether the tool has a branch-hydration path; if not, that absence is itself the
  finding, not the thing the tool was asked to check.
- `resolve` and `new-dep-scan` could both take a `--hydrate-from-branch` (or auto-hydrate) path that
  seeds the local scratch file from `telemetry/plane` / a committed baseline before running — turning
  this from "breaks every fresh checkout" into "self-heals on first use." This is a **candidate
  guardrail/tooling fix**, not enacted this run (advisory in, deterministic out — librarian's call per
  the self-improvement loop).
- The memory corpus has no branch-style fallback at all. If the plane-maintainer is meant to run from
  the cloud on a regular cadence (not just the Hetzner box), the corpus — or at least the specific named
  memories the charter depends on (`memory-corpus-meta-patterns-*`, `tooling-decision-patterns-*`) —
  needs a durable, git-reachable mirror, or the Sunday ritual needs to be scoped to "whichever runtime
  fired this week" rather than assuming both runtimes see the same state. This is an operator-level
  architecture decision, flagged here, not decided here.

## LINK
- [[memory-corpus-meta-patterns-2026-07-02]] (verify-artifact-not-proxy, applied one level up)
- [[plane-maintainer-agent-2026-07-02]]
- `docs/reflections/INBOX/2026-07-02-plane-maintainer-env-probe.reflection.md` (same family of bug:
  trusting cached/assumed state instead of re-reading the actual reachable surface)
