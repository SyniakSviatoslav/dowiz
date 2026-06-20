# Governance change · `require-classification` patch + `route-request`

> The "change" that closes the loop automatically. **When built →** `.claude/hooks/require-classification.sh` (extends the existing one) + new `.claude/hooks/route-request.sh` + a `.claude/settings.json` merge + one line added to `/council`. **Source:** require-classification pack.

⚠️ **This is behaviour-changing governance and a protected-zone (`.claude/`) edit — it is the highest-caution item in the whole system and is sequenced LAST in the [Integration Plan](../INTEGRATION-PLAN.md).** It must not be installed casually: a mis-tuned gate can `deny` the assistant's own edits.

## What it does
- **Gate (PreToolUse `Edit|Write|MultiEdit`):** before editing a file on a "serious surface" (schema/migrations/contracts/money/RLS/auth/state-machine/WS/integrations), checks whether the Council already produced an APPROVED plan for the current change. If not → `permissionDecision:"deny"` with an actionable directive ("run `/council` first"). Always passes council/loop/config artifacts (else deadlock).
- **Router (UserPromptSubmit):** scans each prompt and **injects context** (never blocks) — serious → "run `/council` before code"; recurring + DoD → "run `/loop-orchestrator` (4-condition test)".
- **Friction, not verdict:** a human bypasses via `.claude/state/serious-override`. The Council clears the gate on GO by writing `.claude/state/serious-cleared`.

## Design constraints (🔴, honest)
- **`deny`, not `exit 2`** (bug #24327 — `exit 2` can read as a user refusal and halt). `deny` + a clear "this is an automatic gate, not a user refusal — act, don't stop" directive.
- **Fail-open:** unparseable input or missing `jq`/`python3` → pass (warn only), never brick the session. Human-final beats gate-strictness.
- **Heuristic, tunable:** seriousness is regex over path (gate) and text (router) — tune `SERIOUS`/`REPEAT` per repo; false positives have the instant `serious-override`.
- **Coexists** with the existing `post-edit-gates` (PostToolUse) and the current `require-classification` (Stop) — the patch *extends*, it doesn't delete; the Stop classification hook stays.

## Council linkage (one added line to `/council` step 8)
On STOP-DESIGN-B GO: `echo "<slug>" >> .claude/state/serious-cleared`; tell the human the gate is open for `<slug>`; after shipping, re-arm: `: > .claude/state/serious-cleared`.

## `.gitignore` additions
```
.claude/state/
.claude/logs/
```
(The hooks/commands/agents themselves are committed; local state/logs are not.)

## Acceptance (proof-of-life)
Gate denies an uncleared serious edit · `serious-override` bypasses (logged) · `/council` GO writes `serious-cleared` → edit passes · router nudges on serious/recurring prompts · a normal UI/text edit passes with zero friction.
