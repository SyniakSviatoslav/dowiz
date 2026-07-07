# Fable one-shot harness audit — decorrelated read that feeds §0·GP

**Status:** PENDING operator override (human-only gate). This is a *sanctioned, bounded, single-use
proxy exception* under §0·GP: a decorrelated cross-model read of the harness, run once, human-gated,
whose findings are compiled into the model-agnostic playbook. It does not become standing machinery.

## Why Fable, and why one-shot
Operator 2026-07-07: examine the existing tools/skills/loops/systems/agents/subagents usage with a
model decorrelated from the Opus lead, and apply the findings to the agnostic meta-cognition layer
so it acts as the ultimate harness for any model. Fable is the decorrelation. It runs ONCE because a
standing proxy is exactly what §0·GP removes; a one-shot audit that produces *deterministic
follow-ups* (checks/gates) is ground-truth-generating, not proxy-standing.

## To enable (HUMAN ONLY — the agent is structurally blocked from writing this)
`agent-dispatch-gate.sh` denies `model: fable` unless a non-expired line exists in
`.claude/state/fable-override` (format `<slug>|<unix-expiry>`), and `guard-bash.sh` + `protect-paths.sh`
block any agent from writing that file. Operator, run:

```bash
echo "harness-audit-oneshot|$(date -d '+3 hours' +%s)" > .claude/state/fable-override
```

Then tell the session to dispatch the audit. The line expires in 3h (fail-closed; the gate re-arms
itself — no cleanup step needed).

## The dispatch (once override is active)
Model `fable`, read-only (`Explore`-grade grants), one comprehensive prompt. Ground-truth discipline
is MANDATORY in the prompt: every finding must cite `file:line` or a measured number, and must end in
a *deterministic follow-up* (a check/gate/test to write, or a process/agent to delete), never a bare
opinion. The audit covers:

1. **Agents/subagents** (`.claude/agents/*`, dispatch patterns): which remaining agents earn their
   keep by a ground-truth check on their output vs. which are unverified proxies. The 0-tool-use
   degenerate-return failure (2/4 dispatches this arc returned 0 tool_uses + echoed an injected
   system-reminder as their "result") — root-cause it and propose the deterministic checker
   (SubagentStop/return-integrity guard that reds on 0-tool-use + reminder-echo signature).
2. **Hooks** (`.claude/hooks/*` + registration): confirm each surviving hook is a deterministic
   ground-truth gate (not a re-grown advisory proxy); check for gate-bypass lanes.
3. **Skills** (`.claude/skills/*`, the SKILL registry): which are used, which are dead, which encode
   proxy reasoning vs. deterministic procedure.
4. **Loops** (`loops/*`, registry, router): certification honesty (CERTIFIED vs report-only vs lost
   report), dead loops, loops that still reference removed council/critics.
5. **Circuits & the EYE** (`docs/operating-model/circuits/`, proposed-eye): are they genuinely
   deterministic ground truth, and are they wired?
6. **Token/measurement substrate** (`.claude/logs/harness-events.jsonl`): per-hook fire counts +
   hit-rate; flag any surviving proxy whose measured value/cost is net-negative.

Output = a ranked list of deterministic follow-ups (checks to write / processes to delete), each with
`file:line` evidence. Those follow-ups are the next work items and the additions to
`model-agnostic-playbook.md` §0·GP.

## After the audit
Compile the ground-truth findings into the playbook; turn each actionable follow-up into a
deterministic gate/circuit (the ratchet) or a deletion. The Fable read never decides — it signals;
the checks/gates/human decide (§0·GP).
