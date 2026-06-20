# Loop · regression-hunt

**Family:** Error · **Status:** DRAFT (→ verify) · **Trigger:** `/regression-hunt <what broke>` · **When built →** `loops/regression-hunt.yaml`.

**When:** worked before, broke after a change; approximate "last-known-good" exists. git-bisect to one culprit commit, then minimal fix / clean revert, plus a regression test. Don't refactor along the way.

```yaml
id: regression-hunt
version: 0.1
status: DRAFT
intent: "find culprit commit (worked before) → minimal fix/revert → regression test"
problem_signature: "worked before, broke after a change; approximate 'last good' state known"
trigger: "/regression-hunt <what broke>"
role_mindset: "regression hunter: bisect to a single commit, then minimal action"
preconditions: ["deterministic repro of the regression", "git history available", "approximate good-state known"]
execution_skills: [repro-harness, git-bisect, diff-analysis, minimal-fix-or-revert, regression-test-author]
goal: "culprit commit found + minimal fix/clean revert + a test that catches the regression"
verification: "bisect narrows to 1 commit; repro deterministic; fix green; regression test RED on culprit / GREEN after; 0 flaky"
iron_principles: [repro-before-bisect, bisect-to-single-commit, prefer-minimal-fix-or-clean-revert, add-regression-test, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: culprit isolated by bisect; deterministic repro; fix or justified revert; regression test added; affected flow green; 0 flaky"
gates: [STOP-CULPRIT, STOP-FIX]
proof_artifacts: [bisect-log, culprit-diff, repro, regression-test, green-run]
out_of_scope: [refactor-along-the-way, revert-that-breaks-the-changes-reason]
escalation: "culprit is a deliberate change with a reason → no blind revert; resolution → /council if serious"
skills_required: [repo-access, git, test-runner]
memory_file: loops/memory/regression-hunt.md
verification_report: loops/reports/regression-hunt-0.1.md
```
