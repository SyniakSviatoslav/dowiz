# Token-circuit guard — manual apply (`.claude/` is a protected zone)

Mechanically enforces the AGENTS.md HARD TOKEN THRESHOLDS (operator directive 2026-07-05): a
sub-unit (subagent / `/loop` iteration / workflow worker / council round) that crosses **80K** tokens
gets a deterministic recycle directive; the lead session at **300K** gets a save+push+`/clean`+fresh
directive. Proven locally 2026-07-05 (4 cases: session≥300K, sub-unit≥80K, small no-op, missing
transcript fail-open — all correct; jq-absent → python3 fallback exercised).

## 1. Install the hook script

```bash
cp docs/operating-model/proposed-token-guard/token-circuit-guard.sh .claude/hooks/token-circuit-guard.sh
chmod +x .claude/hooks/token-circuit-guard.sh
```

## 2. Register it in `.claude/settings.json`

Add to the existing `hooks.PostToolUse` array (alongside `post-edit-gates.sh` / `loop-detector.sh`),
and add a `SubagentStop` entry so an over-budget lane is caught even if PostToolUse does not fire
inside subagents in this harness:

```json
"PostToolUse": [
  { "matcher": "Bash|Edit|Write|MultiEdit|Read|Grep|Glob|Task|Agent",
    "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/token-circuit-guard.sh\"" } ] }
],
"SubagentStop": [
  { "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/token-circuit-guard.sh\"" } ] }
],
"Stop": [
  { "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/session-recycle-guard.sh\"" } ] }
]
```

Also install the Stop-hook: `cp docs/operating-model/proposed-token-guard/session-recycle-guard.sh
.claude/hooks/ && chmod +x .claude/hooks/session-recycle-guard.sh`.

## AUTOMATIC restart (the launcher)

`session-recycle-guard.sh` (Stop) writes `.claude/state/RECYCLE` when a session ends ≥300K;
`scripts/claude-recycle-loop.sh` watches that signal and relaunches a FRESH session (the
SessionStart context-primer resurfaces the memory index + handoff). Run Claude through it:

```bash
bash scripts/claude-recycle-loop.sh              # interactive
bash scripts/claude-recycle-loop.sh -p '<task>'  # headless/autonomous — fully automatic recycle
```

Headless/autonomous sessions recycle fully automatically. Interactive sessions get the guard's
save+push directive; when that session ends, the loop relaunches. Proven locally (mock: 2 recycles →
3 sessions). `CLAUDE_MAX_RESTARTS` (default 20) is the runaway backstop.

(Merge the PostToolUse entry with what's already there — don't replace the array.)

## 3. Tune (optional, env)

- `TOKEN_LANE_MAX` (default `80000`) — per-agentic-unit threshold.
- `TOKEN_SESSION_MAX` (default `300000`) — lead-session threshold.

## What it counts

Cumulative **new** tokens = Σ over assistant turns of `input_tokens + cache_creation_input_tokens +
output_tokens`, read from the unit's `transcript_path`. `cache_read_input_tokens` is **excluded** on
purpose — counting the re-read prefix every turn would trip 300K in ~3 turns; "new tokens" is the
honest measure of work and grows at a sane rate. Main-vs-sub is a transcript-path heuristic
(`*/agents/*`, `agent-*`, `task-*`, `subagent*`).

## Honest scope / limitations

- **Signal, not SIGKILL.** A hook cannot forcibly terminate a running subagent in this harness. The
  guard emits a high-friction `systemMessage` + `additionalContext` recycle directive (matching the
  fail-open, friction-not-block philosophy of `loop-detector.sh`). The *enforcement* is that plus the
  per-unit token budget carried in every dispatch brief (AGENTS.md). To make it a HARD stop, change
  the sub-unit branch to `exit 2` (blocking) — but test first: a hard block mid-tool can strand WIP.
- **Mid-lane firing depends on the harness.** If project PostToolUse hooks do not fire inside
  subagent contexts, the 80K circuit lands at `SubagentStop` (post-hoc — the lane already finished);
  the mid-flight signal then only fires for `/loop`/workflow units whose calls run in the main
  transcript. The `SubagentStop` registration guarantees at least post-hoc detection + telemetry.
- **Fail-open.** Any parse/IO error → `{}` (no-op). It never crashes a tool call.
