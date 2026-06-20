# Loop · exit-audit

**Family:** Build · **Status:** DRAFT (→ verify) · **Trigger:** `/exit-audit <phase>` · **When built →** `loops/exit-audit.yaml`.

**When:** before crossing between phases — **adversarially** prove (or disprove) a whole phase is complete. **Does not fix** (holes go back to build-stage/error-fix); it proves only. For Phase 5 the reality criterion is the **first real paid order**, not "all stages green." This is the spirit of the `/reliability-gate` skill already in the repo.

```yaml
id: exit-audit
version: 0.1
status: DRAFT
intent: "adversarially prove (or disprove) a WHOLE phase is complete before transition"
problem_signature: "before crossing between phases: is the phase really complete?"
trigger: "/exit-audit <phase>"
role_mindset: "exit auditor (adversarial): assume the phase is NOT ready; try to break the completion claim"
preconditions: ["phase stages claimed complete", "system boots", "phase-wide red-lines defined"]
execution_skills: [phase-coverage-check, adversarial-probe, e2e-run, contract-parity, security-tenant-probe, verdict-author]
goal: "proven/disproven: all stages green, phase red-lines hold, critical-path E2E green"
verification: "every phase stage has a green checkpoint; phase-wide red-lines proven by evidence; three-role critical path E2E green; cross-tenant=0; zero secrets; (Phase 5) real-paid-order criterion addressed"
iron_principles: [adversarial-completion, red-lines-proven-not-claimed, evidence-over-assertion, launch-trigger-reality, audit-does-not-fix, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: every stage checkpoint green; phase red-lines proven; E2E critical path green; security/tenant probes clean; verdict recorded; (Phase 5) real paid order = criterion"
gates: [STOP-COVERAGE, STOP-VERDICT]
proof_artifacts: [phase-coverage-matrix, red-line-probe-evidence, e2e-run, security-probe, exit-verdict]
out_of_scope: [fix-here]
escalation: "FAIL → list of holes back to build-stage/error-fix; serious architectural hole → /council"
skills_required: [repo-access, test-runner, headed-browser]
memory_file: loops/memory/exit-audit.md
verification_report: loops/reports/exit-audit-0.1.md
```
