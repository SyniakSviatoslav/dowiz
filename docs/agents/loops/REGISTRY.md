# Loop Registry (catalog)

> Human catalog of the loop library. At integration this seeds the machine source-of-truth `loops/registry.md`, and each card below becomes `loops/<id>.yaml`.
> Statuses: **CERTIFIED** (passed M1–M11) · **DRAFT** (pre-built, needs VERIFY) · **REJECTED** · **DEPRECATED**.

| id | intent | ver | status | trigger | doc |
|---|---|---|---|---|---|
| design-convergence | hardened plan for a serious change (Triad Council) | 1.0 | CERTIFIED | `/council` | [design-convergence](./design-convergence.md) |
| error-fix-convergence | UI↔server to full correspondence, every flow green | 1.0 | CERTIFIED | `/converge-loop` | [error-fix-convergence](./error-fix-convergence.md) |
| backend-contract-convergence | server↔contract into compliance | 0.1 | DRAFT | `/converge-server` | [backend-contract-convergence](./backend-contract-convergence.md) |
| investigation-triage | find root of an unknown bug (repro-first) | 0.1 | DRAFT | `/investigate` | [investigation-triage](./investigation-triage.md) |
| regression-hunt | culprit commit of a regression (git-bisect) | 0.1 | DRAFT | `/regression-hunt` | [regression-hunt](./regression-hunt.md) |
| incident-recovery | live outage: stabilize→root→post-mortem | 0.1 | DRAFT | `/incident` | [incident-recovery](./incident-recovery.md) |
| refactor-convergence | dupes→single source, behavior unchanged | 0.1 | DRAFT | `/refactor-converge` | [refactor-convergence](./refactor-convergence.md) |
| performance | metric within budget, no regression | 0.1 | DRAFT | `/perf` | [performance](./performance.md) |
| build-stage | a roadmap stage to its ✅ checkpoint | 0.1 | DRAFT | `/build-stage` | [build-stage](./build-stage.md) |
| audit-gate | sections A–F PASS with artifacts | 0.1 | DRAFT | `/audit-gate` | [audit-gate](./audit-gate.md) |
| exit-audit | whole-phase completeness (adversarial) | 0.1 | DRAFT | `/exit-audit` | [exit-audit](./exit-audit.md) |

## Three families
- **Design (1):** `design-convergence` — the Triad Council before a serious change.
- **Error / diagnostics (7, reactive):** `error-fix-convergence` (canonical) · `backend-contract-convergence` · `investigation-triage` · `regression-hunt` · `incident-recovery` · `refactor-convergence` · `performance`.
- **Build / delivery (3, proactive pipeline):** `build-stage` → `audit-gate` → `exit-audit`.

Pipeline: `build-stage` (code) → `audit-gate` (UI quality) → `error-fix` (flows green) → `exit-audit` (phase proven) → GATE to next phase.

## Health
A Counsel health-pass reads memory/reports and signals "sick" loops (flaky-under-green, training-never-off, no-memory). Orchestrator routes a sick loop to a Loop-Architect improve-request.

## Certification note
Every DRAFT must pass `/build-verify-loop verify <id>` (M1–M11 + anti-cheat dry-run on a real broken fixture) before `/loop-orchestrator` will dispatch it. The cards are pre-built to M1–M11, so VERIFY is a proof step, not a rebuild.
