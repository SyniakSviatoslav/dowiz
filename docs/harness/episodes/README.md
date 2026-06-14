# Episode Store

> Per `.agents/rules/harness-self-improvement.md` Phase A1. Each agent run on a stage/flow emits an episode package.

## Format

Each episode is a dated markdown file: `YYYY-MM-DD--short-description.md`

## Required fields

- **model**: model id + version used
- **task**: the task spec / prompt
- **actions**: tool calls made (summary, not full log)
- **diffs**: files changed, line counts
- **gate_results**: which gates ran and their outcomes
- **interventions**: `flag-only` or inline-fix, with reason
- **diagnose**: root-cause category from the failure-mode ledger
- **health**: files re-edited, loop iterations, tokens consumed
- **verdict**: passed / failed / flaky

## Example

```markdown
# Episode: 2026-06-09--subdomain-asset-404

- **model**: deepseek-v4-flash-free / opencode
- **task**: Fix subdomain static assets returning 404 on tenant subdomains
- **actions**: grep → read server.ts → edit subdomain condition → build → deploy → verify
- **diffs**: 1 file, +1/-1 lines (server.ts:198)
- **gate_results**: health check green, assets return 200
- **interventions**: none
- **diagnose**: systemic — subdomain middleware missing exclusion for file extensions
- **health**: 3 tool calls, 1 edit, 2 deploys
- **verdict**: passed
```
