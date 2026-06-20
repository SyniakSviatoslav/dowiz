# Loop · audit-gate

**Family:** Build · **Status:** DRAFT (→ verify) · **Trigger:** `/audit-gate [scope]` · **When built →** `loops/audit-gate.yaml`. Follows `Frontend-Audit-Polish-Gate`.

**When:** prove quality/unification/integrations/polish **live in the browser**; each section A–F PASS with an artifact (screenshot/recording — "from reading code" does not count). Cosmetics/states/tokens → fix inline; logic/contract/security → **flag-only** (resolved via `/council` or `backend-contract-convergence`). This is the discipline this session used for the launch UI proofs.

```yaml
id: audit-gate
version: 0.1
status: DRAFT
intent: "prove quality/unification/integrations/polish live; each section A–F PASS with an artifact"
problem_signature: "need a quality gate: single design system, states, live integrations, polish, rare states"
trigger: "/audit-gate [scope]"
role_mindset: "auditor: PASS only with a browser artifact; cosmetics fixed inline, logic flag-only"
preconditions: ["dev server + backend boot", "shared layer and tokens extracted", "headed browser access"]
execution_skills: [live-browser-audit, visual-regression, inline-fix-cosmetic, flag-only-logic, contract-parity-check, engineer-review]
goal: "sections A–F PASS with proof; a random screen is not guessably 'from another team'"
verification: "A single tokens (grep 0 hex) · B unified states/buttons/forms · C integrations live (Zod parse, WS, idempotency, error codes) · D polish/animation/responsive 390/768/1280 · F rare states reproduced — each PASS with screenshot/recording; flag-only carved out"
iron_principles: [proof-by-artifact-not-words, inline-fix-cosmetic-only, flag-only-for-logic-contract-security, single-design-system, live-browser-not-code-reading, no-cookies, no-fake-green]
loop_body: [SENSE, DIAGNOSE, ACT, VERIFY, REPEAT]
exit_conditions: "all: sections A–F PASS with artifacts; visual-regression baseline 390/768/1280; flag-only carved out; 0 hex in packages/ui; 0 cookie"
gates: [STOP-AUDIT-A, STOP-VERDICT]
proof_artifacts: [per-section-screenshots-recordings, visual-regression-baselines, flag-only-list, grep-clean]
out_of_scope: [fix-server/contracts/logic-here, new-features]
escalation: "logic/contract/security finding → flag-only list (not a fix); serious resolution → /council or backend-contract-convergence"
skills_required: [headed-browser, repo-access]
memory_file: loops/memory/audit-gate.md
verification_report: loops/reports/audit-gate-0.1.md
```
