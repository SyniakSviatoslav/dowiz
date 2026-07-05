# Mechanical enforcement hooks — manual apply (`.claude/` is a protected zone)

Makes routing, mapping, memory/context reading, and token reduction **mechanical** (hook-enforced),
so they cannot be forgotten or misused — no reliance on agentic reasoning. Operator directive
2026-07-05. All three proven locally (synthetic-input tests, jq-absent → python3 fallback exercised).

| Hook | Event | What it enforces (deterministically) |
|---|---|---|
| `dispatch-guard.sh` | PreToolUse `Agent\|Task` | Injects the TOKEN ROUTER + MODEL ROUTING v3 + graph-first + memory-query directive into EVERY subagent prompt (`updatedInput`); flags a missing explicit `model:`. Idempotent (skips if already injected). |
| `context-primer.sh` | SessionStart | Surfaces the MAPPING rule (query the graph first, never embed maps), the memory index (`~/.claude/projects/<cwd>/memory/MEMORY.md`), and routing/reduction/circuit rules at session start. |
| `token-reduce-guard.sh` | PreToolUse `Bash` | For a safe whitelist of noisy read-only commands (cargo/pnpm build·test·clippy·check, unbounded `git log`) with no existing pipe/redirect, rewrites to append `2>&1 \| tail -n N` (`updatedInput`). Anything already output-controlled is left untouched. |

## Install

```bash
D=.claude/hooks
cp docs/operating-model/proposed-mechanical-enforcement/dispatch-guard.sh    $D/
cp docs/operating-model/proposed-mechanical-enforcement/context-primer.sh    $D/
cp docs/operating-model/proposed-mechanical-enforcement/token-reduce-guard.sh $D/
chmod +x $D/dispatch-guard.sh $D/context-primer.sh $D/token-reduce-guard.sh
```

## Register in `.claude/settings.json` (merge, don't replace)

```json
"PreToolUse": [
  { "matcher": "Agent|Task", "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/dispatch-guard.sh\"" } ] },
  { "matcher": "Bash",       "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/token-reduce-guard.sh\"" } ] }
],
"SessionStart": [
  { "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/context-primer.sh\"" } ] }
]
```

## Tune (env)

- `TOKEN_REDUCE_TAIL` (default `60`) — lines kept from a rewritten noisy command.
- `CLAUDE_AUTO_MEMORY_DIR` — override the memory dir the primer reads.

## Honest scope

- **`updatedInput` support required.** These rely on PreToolUse hooks being able to rewrite tool
  input. If a harness build ignores `updatedInput`, `dispatch-guard`/`token-reduce` degrade to
  advisory `additionalContext` (still surfaced, not applied). Verify once via `/hooks`.
- **`token-reduce` is conservative by design.** It only rewrites a known-safe whitelist and never
  touches a command that already has a pipe/redirect (`| tail` changes exit code — unsafe to force
  onto arbitrary commands). Widen the whitelist deliberately, not greedily.
- **`dispatch-guard` injects; it does not block.** A missing `model:` is flagged, not rejected —
  flip the branch to `permissionDecision:"deny"` if you want a hard stop (test first).
- Complements the total-usage circuits in `proposed-token-guard/` (80K unit / 300K session).
