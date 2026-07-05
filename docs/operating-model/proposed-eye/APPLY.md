# THE EYE — parallel real-time monitor — manual apply (`.claude/` is protected)

A cheap, deterministic monitor (operator directive 2026-07-05): watches signals in real time and HALTS
the agent for inspection ONLY at **≥3 bad signals or ≥1 critical** — silent below that (forbidden to
interrupt). No LLM, pure signal counting → near-zero token cost.

- CRITICAL (1 stops): a hard block / red-line trip — `RED-LINE` (circuit/doubt-gate), `BLOCKED`
  (protect-paths / guard-bash).
- BAD (3 stops): a plain tool failure — `is_error:true` / `success:false`.

Halts via `{"continue":false}` + a stopReason telling the agent to inspect before continuing; resets
the tally after firing.

## Install

```bash
cp docs/operating-model/proposed-eye/eye-guard.sh .claude/hooks/eye-guard.sh
chmod +x .claude/hooks/eye-guard.sh
```

Register in `.claude/settings.json` `hooks.PostToolUse` (merge, don't replace) — put it LAST so it
observes the other guards' output in the same batch:

```json
{ "matcher": "Bash|Edit|Write|MultiEdit|Read|Grep|Glob|Task|Agent",
  "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/eye-guard.sh\"" } ] }
```

## Tune / scope

- `EYE_BAD_MAX` (default 3) — bad-signal threshold. Critical is always 1.
- Session tally lives at `.claude/state/eye/<session_id>.tally` ("bad critical"); resets on fire.
- Sources today: the current event's tool_response (failures + block strings the other guards emit).
  Extend by having circuit-guard/loop-detector append to the same tally for more signal sources.
- Proven locally 2026-07-05: 2 bad → silent; 3rd bad → STOP; 1 critical → STOP; jq-absent → python3
  fallback exercised.
