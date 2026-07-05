# KNOWLEDGE-AS-CIRCUITS wiring — manual apply

Makes the circuit registry fire automatically (spec: `docs/operating-model/KNOWLEDGE-AS-CIRCUITS.md`).
The runner + registry are already committed and usable now (`node scripts/run-circuits.mjs --staged`).

## 1. Hard block at commit — add to `.husky/pre-commit` (not protected; edit directly)

Add near the other guardrail steps:

```bash
echo "1.6: KNOWLEDGE-AS-CIRCUITS (registry — error-patterns/lessons/design-rules/best-practices)..."
node scripts/run-circuits.mjs --staged || exit 1
```

`run-circuits.mjs` exits 2 on any red-line circuit (money=float, missing RLS FORCE, …) → the commit
is blocked. This is the mechanical enforcement; no reliance on reasoning.

## 2. Immediate signal on edit — install the PostToolUse hook (`.claude/` is protected → cp)

```bash
cp docs/operating-model/proposed-circuit-wiring/circuit-guard.sh .claude/hooks/circuit-guard.sh
chmod +x .claude/hooks/circuit-guard.sh
```

Register in `.claude/settings.json` `hooks.PostToolUse` (merge, don't replace):

```json
{ "matcher": "Edit|Write|MultiEdit",
  "hooks": [ { "type": "command", "command": "bash \"$CLAUDE_PROJECT_DIR/.claude/hooks/circuit-guard.sh\"" } ] }
```

## 3. Growing the registry (loop-work)

Per `docs/operating-model/KNOWLEDGE-AS-CIRCUITS.md`: every qualified lesson/repeat-error/red-line →
one new circuit (red→green), same session. Seed the existing lesson store via `/loop-orchestrator`
(one lesson → one circuit per pass). Library best-practices → `docs/libraries/<name>.md` + a
`require_together` circuit gating the cached doc exists before the dependency is added.

Proven locally 2026-07-05: money-float-in-code → RED-LINE exit 2; f64-in-comments → clean; whole
`dowiz-core` passes all 4 seed circuits.
