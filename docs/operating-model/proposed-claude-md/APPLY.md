# APPLY — Task-Exit Rule into `.claude/` (operator action required)

`.claude/` is a protected zone (`protect-paths.sh` blocks Edit/Write → manual approval). This agent
**cannot** write the rule into `.claude/CLAUDE.md` or `.claude/agents/*.md`. Below is the exact text
to paste, mirroring the [Agent Operating Model](../agent-operating-model.md) precedent
(`proposed-hooks/`). Full rationale: [task-exit-rule.md](../task-exit-rule.md).

---

## 1) Append to `.claude/CLAUDE.md` (new top-level section)

```markdown
## Task-Exit Rule (universal — every agent, every task, every change, no exception)

Before executing ANY task — independent of spec completeness or clarifications — in this order:

1. **Enrich conditions.** Derive the full definition of "done", not just the literal task text.
   Enrich by fixed dimensions: states (loading/empty/error/success); error matrix
   (401/403/404/422/429/5xx/network); rare/edge states; regression radius (what this could break);
   design system/tokens; i18n (al/en); security (no cookie/secret/PII, Zod-parse); api/ws contract
   parity. A non-applicable dimension → mark `N/A — why`, never skip silently.
2. **Author task-exit.** Write a checkable exit checklist for *this* change — one item per enriched
   condition, each with the proof that confirms it (file:line / test name / command output / artifact).
   **Write it BEFORE touching code.**

Then make the change. Then:

3. **Verify against the pre-written exit.** Walk each item → PASS/FAIL with proof. "Looks fine" = FAIL.
   No item is credited by intent — only by observed proof.

Don't declare a task done until every item is green or raised as an explicit flag. Flag class:
**inline-fix** (cosmetic/states/tokens — fix now) vs **escalate** (contract / price-status business
logic / security — don't touch, raise separately). **No size exemption:** the smaller the change, the
likelier a missed detail. Full spec: docs/operating-model/task-exit-rule.md.
```

## 2) Add to each subagent header (`.claude/agents/*.md`) — one line near the top of the body

```markdown
> **Task-Exit Rule applies** (docs/operating-model/task-exit-rule.md): before any change, enrich
> "done" + author a checkable exit checklist (with proof) BEFORE code; verify artifact-not-intent
> after. No size exemption.
```

Apply to worker/builder agents that produce code (e.g. `loop-architect`). Pure read-only critics
(`cause-critic`, `pattern-critic`, `ratchet-critic`, `invariant-guardian`, `security-sentinel`,
`test-scout`) already verify-by-artifact; the line is optional there.

## 3) (Optional) Presence-only enforcement hook

If guaranteeing the rule wasn't "forgotten" ever matters: a tiny PreToolUse hook that checks only the
**presence** of an EXIT block for the task (not content/results) — like `require-classification`. This
is a presence-check, **not** a verification loop. Not needed by default; the rule rides on step order.

---

**Verification after applying:** none beyond a re-read — these are governance text inserts, no code
surface. The rule's own teeth are the existing mechanical floor (`post-edit-gates` / `lint:gates` /
`typecheck`), which is unchanged.
