# Loop · design-convergence

**Family:** Design · **Status:** CERTIFIED · **Trigger:** `/council <serious change>` · **When built →** `loops/design-convergence.yaml` (+ command `.claude/commands/council.md`, agents architect/breaker/counsel).

**When:** any serious change before code — schema/contracts/money/RLS/tenant/state-machine/WS/integrations/AI-PII/irreversible/stage-feature/shared-refactor. Small (cosmetic, local refactor without contract impact) does **not** convene.

**Body:** `FRAME → PROPOSE → {ATTACK (Breaker) ∥ EXAMINE (Counsel)} → RESOLVE → {RE-ATTACK ∥ RE-EXAMINE} → REPEAT`. Gates: `STOP-DESIGN-A`, `STOP-ETHICS`, `STOP-DESIGN-B`.

**Hard exit (all at once):** 0 unresolved CRITICAL/HIGH (each fixed or accept-risk+owner) · 0 unresolved ETHICAL-STOP (each with a recorded human decision) · aesthetic/strategic advice addressed-or-acknowledged · back-of-envelope converges · invariants intact · artifacts exist (ADR + proposal + breaker-findings + counsel-opinion + resolution + ethical-decisions).

```yaml
id: design-convergence
version: 1.0
status: CERTIFIED
intent: "hardened plan for a serious change via the Triad Council (Architect/Breaker/Counsel)"
problem_signature: "serious change before code: schema/contracts/money/RLS/state-machine/WS/integrations/AI/irreversible"
trigger: "/council <serious change description>"
role_mindset: "three voices: engineering truth / adversarial truth / good-beauty-wisdom"
preconditions:
  - "change classified as 'serious'"
  - "system-architect, system-breaker, counsel subagents exist"
execution_skills: [system-architect, system-breaker, counsel]
goal: "a plan that holds back-of-envelope, no unresolved CRITICAL/HIGH and no unresolved ETHICAL-STOP"
verification: "0 unresolved CRITICAL/HIGH (Breaker) + 0 unresolved ETHICAL-STOP (Counsel) + advice addressed + invariants intact"
iron_principles: [breaker-specific-demonstrable, breaker-no-fixes, counsel-advisory-human-final, ethical-stop-is-friction, architect-no-code]
loop_body: [FRAME, PROPOSE, ATTACK-EXAMINE, RESOLVE, RE-ATTACK-RE-EXAMINE, REPEAT]
exit_conditions: "all: 0 CRITICAL/HIGH + 0 unresolved ETHICAL-STOP + back-of-envelope converges + artifacts ready"
gates: [STOP-DESIGN-A, STOP-ETHICS, STOP-DESIGN-B]
proof_artifacts: [proposal.md, breaker-findings.md, counsel-opinion.md, resolution.md, ethical-decisions.md, ADR]
out_of_scope: [production-code, fixes-from-the-breaker, control-from-counsel]
escalation: "ETHICAL-STOP → recorded human decision; defer → MISSING"
skills_required: [agent-architect, agent-breaker, agent-counsel, openrouter-cross-opinion]
memory_file: loops/memory/design-convergence.md
verification_report: loops/reports/design-convergence-1.0.md
```

**Runtime bridge:** at STOP-DESIGN-B, carry each accepted threat-model item into the error-fix matrix as an X-blocker / Playwright scenario.
