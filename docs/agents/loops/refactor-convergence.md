# Loop · refactor-convergence

**Family:** Error · **Status:** DRAFT (→ verify) · **Trigger:** `/refactor-converge [scope]` · **When built →** `loops/refactor-convergence.yaml`.

**When:** duplicated logic/component/direct-fetch-past-shared; desync; goal = single source of truth. **Behavior-preserving** (same outputs); full flow run must stay green; no new features along the way.

```yaml
id: refactor-convergence
version: 0.1
status: DRAFT
intent: "consolidate dupes/desync to a single source WITHOUT behavior change; all flows stay green"
problem_signature: "duplicated logic/component/direct fetch past shared; state desync; goal = single source"
trigger: "/refactor-converge [scope]"
role_mindset: "refactor engineer: structure changes, behavior does not"
preconditions: ["flows have tests (or a harness)", "shared layer (apiClient/useWebSocket/ThemeProvider) exists", "design tokens extracted"]
execution_skills: [duplication-scan, consolidate-to-single-source, full-flow-run, engineer-review]
goal: "dupes consolidated to a single source; zero direct fetch/WS past shared; zero hardcoded tokens; all flows green"
verification: "grep clean (0 hex in packages/ui; 0 direct fetch/new WebSocket past shared; 0 local price recompute); FULL flow run green (not only touched); 0 behavior regression"
iron_principles: [behavior-preserving, full-run-after-refactor, single-source-of-truth, no-new-features-during-refactor, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: dupes removed to single source; grep clean; full flow set green; zero behavior change (same outputs); 0 flaky"
gates: [STOP-MAP, STOP-FULLRUN]
proof_artifacts: [duplication-map, grep-clean-output, full-green-run, before-after-equivalence]
out_of_scope: [new-features, behavior-changes, contract-changes, improve-along-the-way]
escalation: "refactor exposes a needed contract/behavior change → separately via /council"
skills_required: [repo-access, test-runner]
memory_file: loops/memory/refactor-convergence.md
verification_report: loops/reports/refactor-convergence-0.1.md
```
