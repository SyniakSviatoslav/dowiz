# Loop · investigation-triage

**Family:** Error · **Status:** DRAFT (→ verify) · **Trigger:** `/investigate <symptom>` · **When built →** `loops/investigation-triage.yaml`.

**When:** symptom present, root unknown, repro unstable. Repro-first, hypothesis-bisect, fix the cause (not the symptom), add a regression test. Root in server/contract → escalate MISSING.

```yaml
id: investigation-triage
version: 0.1
status: DRAFT
intent: "find root of an unknown bug: stable repro → root → fix → regression test"
problem_signature: "symptom present, root unknown; reproduction unstable/unclear"
trigger: "/investigate <symptom>"
role_mindset: "diagnostician: reproduce first, then treat the cause, not the symptom"
preconditions: ["a symptom/report exists", "access to logs/traces/repo"]
execution_skills: [repro-harness, hypothesis-bisect, log-trace-analysis, minimal-fix, regression-test-author, engineer-review]
goal: "deterministic repro + proven root + minimal fix + regression test that catches it"
verification: "repro 100% before fix; green after; new regression test RED on old code / GREEN on new; 0 flaky"
iron_principles: [repro-before-fix, root-not-symptom, add-regression-test, server-read-only-unless-root-is-server, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: deterministic repro captured; root proven by evidence (not guess); fix minimal; regression test RED→GREEN; 0 flaky; 0 scope-creep"
gates: [STOP-REPRO, STOP-ROOT]
proof_artifacts: [failing-repro-trace, hypothesis-bisect-log, root-evidence, regression-test, green-run]
out_of_scope: [fix-adjacent-too, change-contracts-without-escalation, close-without-regression-test]
escalation: "root in server/contract → MISSING + separate dialog (serious fix → /council)"
skills_required: [repo-access, logs-access, test-runner]
memory_file: loops/memory/investigation-triage.md
verification_report: loops/reports/investigation-triage-0.1.md
```
