# Role · error-fix / variants (runtime verifier)

> **Plane:** Object · **Axis:** every flow green — *verify code live* · **Model:** — · **When built →** loop cards under `loops/` (canonical `error-fix-convergence.yaml`) + thin triggers (`/converge-loop`, family commands) · **Source:** Convergence-Playwright-Loop-Prompt + Loop-Spec-v2 §14.

## Mandate
The runtime, adversarial counterpart to the worker-builder: prove code actually works by driving it **live** (Playwright), not by reading code. The canonical loop converges UI↔server to full correspondence — every (screen × flow × role × breakpoint × locale) green; its variants reshape the same skeleton for other problem signatures.

## The family (one skeleton, different source-of-truth / exit / iron-principles)
| Loop | When | Server |
|---|---|---|
| [error-fix-convergence](../loops/error-fix-convergence.md) | UI "seems to work", flows unverified | read-only |
| [backend-contract-convergence](../loops/backend-contract-convergence.md) | server↔contract mismatch | read-write (server is the target) |
| [investigation-triage](../loops/investigation-triage.md) | symptom present, root unknown | read-only* |
| [regression-hunt](../loops/regression-hunt.md) | worked before, broke after a change | read-only* |
| [incident-recovery](../loops/incident-recovery.md) | live outage now | flag/fallback |
| [refactor-convergence](../loops/refactor-convergence.md) | dupes/desync, goal = single source | read-only |
| [performance](../loops/performance.md) | latency/load beyond budget | read-only |

\* root in server/contract → escalate MISSING, not a silent server fix inside a non-server loop.

## Shared invariants (🔴)
no-fake-green · test/measurement = spec, not a mirror of code · hard exit ALL-must-hold · proof artifacts every run · memory written · server read-only in UI/error loops (contract gap → MISSING/BLOCKED-contract).

## Bridge from design-time
Accepted threat-model items from the Council become **X-blockers / Playwright scenarios** in the error-fix matrix (double-submit, kill-backend, cross-tenant attempt…). Design-time breaks become runtime tests.
