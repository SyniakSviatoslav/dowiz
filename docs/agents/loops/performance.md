# Loop · performance

**Family:** Error · **Status:** DRAFT (→ verify) · **Trigger:** `/perf <metric/symptom>` · **When built →** `loops/performance.yaml`.

**When:** latency/response/load beyond budget. **Measure before optimizing**, against a defined budget (not by feel); zero correctness regression; no architectural change for speed without the Council.

```yaml
id: performance
version: 0.1
status: DRAFT
intent: "bring a speed metric within budget WITHOUT functional regression"
problem_signature: "latency/response-time/load beyond budget; speed degradation"
trigger: "/perf <metric/symptom>"
role_mindset: "perf engineer: measure before optimizing; against budget, not by feel"
preconditions: ["a defined budget (load-time / p95 / query-time)", "a reproducible way to measure"]
execution_skills: [profile-measure, bottleneck-bisect, targeted-optimization, regression-perf-test]
goal: "measured metric <= budget (reproducible); zero functional regression"
verification: "metric <= budget (median of N runs, not one-off); functional flows stay green; perf-budget gate/test added"
iron_principles: [measure-before-optimize, budget-defined-first, no-correctness-regression, no-premature-optimization, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: measured against budget (reproducible); within budget; functional flows green; perf-budget/test added as a guard; 0 flaky"
gates: [STOP-BASELINE, STOP-BUDGET]
proof_artifacts: [baseline-measurement, bottleneck-evidence, after-measurement, perf-test-budget-gate]
out_of_scope: [optimize-without-measuring, behavior/contract-change-for-speed-without-council]
escalation: "architectural change for perf → serious change → /council"
skills_required: [repo-access, profiler, test-runner]
memory_file: loops/memory/performance.md
verification_report: loops/reports/performance-0.1.md
```
