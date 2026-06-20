# Loop · incident-recovery

**Family:** Error · **Status:** DRAFT (→ verify) · **Trigger:** `/incident <what's failing>` · **When built →** `loops/incident-recovery.yaml`.

**When:** a live outage/degradation right now. **Stabilize first with reversible actions** (fallback/flag/scaling-gate — zero lost orders, zero destructive ops, zero autoban; human is authority), *then* root-cause, permanent fix, detection, post-mortem. A permanent fix on a serious surface goes through `/council` **after** stabilization.

```yaml
id: incident-recovery
version: 0.1
status: DRAFT
intent: "live outage: stabilize (no lost orders) → root → fix → detection → post-mortem"
problem_signature: "live outage/degradation now; users/orders at risk"
trigger: "/incident <what's failing>"
role_mindset: "on-call engineer: stop the bleeding with reversible actions first, then find root"
preconditions: ["access to live signals (health/errors/alerts)", "fallback/degradation and flag/scaling-gate available"]
execution_skills: [triage-stabilize, observability-read, root-cause-trace, minimal-fix, postmortem-author]
goal: "service stabilized (degraded-not-down, zero lost orders) → root → permanent fix → detection/alert → post-mortem"
verification: "health back to green/degraded; zero lost orders (fallback holds); full green after fix; controlled rollback proven; alert/test added; post-mortem recorded"
iron_principles: [stabilize-before-root-cause, no-data-loss-order-survives, human-authority-no-autoban, reversible-actions-first, blameless-postmortem, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: stabilized (degraded-not-down, orders intact); root proven; permanent fix landed; detection/alert or test added; controlled rollback verified; post-mortem in docs/incidents/"
gates: [STOP-STABILIZED, STOP-FIX]
proof_artifacts: [incident-timeline, stabilization-evidence-health-orders, root-evidence, added-alert-or-test, postmortem]
out_of_scope: [risky-changes-during-stabilization, destructive-actions, autoban/auto-punish-courier]
escalation: "permanent fix on a serious surface → AFTER stabilization via /council (post-incident)"
skills_required: [repo-access, observability, flag-control]
memory_file: loops/memory/incident-recovery.md
verification_report: loops/reports/incident-recovery-0.1.md
```
