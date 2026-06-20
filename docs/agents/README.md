# dowiz / DeliveryOS — Agentic System Index

> **Single entry to the meta-layer.** What agents we have, what goes where, in what order to stand it up, and how a request flows through the system. This is the *human* source of truth; `loops/registry.md` (once built) is the *machine* one. Product reality (ADR, red-lines, scope) lives in `Context-Handoff-v4_5`, not here.

This directory is the **documentation corpus** — one doc per role, per loop, and per governance change. **Nothing here is active yet.** Activation is the [Integration Plan](./INTEGRATION-PLAN.md), which moves these into live `.claude/agents`, `.claude/commands`, `.claude/hooks`, and `loops/`.

## 30-second map

```
DESIGN PLANE — harden a serious change before code
  ├─ System Architect  — "will it work / scale?"      (engineering truth)
  ├─ System Breaker    — "how does it break?"          (adversarial truth)
  └─ Counsel           — "is it worth / good / wise?"  (good·beauty·wisdom) + agent physician
        mechanism: design-convergence loop · trigger /council
LOOP PLANE — loop mechanics
  ├─ Loop-Architect    — builds/improves/CERTIFIES loops (M1–M11)   (quality)
  └─ Loop-Orchestrator — matches/runs/supervises/harvests           (usage)
        nothing uncertified is dispatched
OBJECT PLANE — the work
  ├─ worker            — does exactly enough
  └─ error-fix + variants — verifies code live (Playwright)  · trigger /converge-loop
```

**Backbone:** three independent design stimuli (build / break / weigh) prevent collapse into convenient untruth. **Usage↔quality split** (Orchestrator tunes parameters, Architect certifies structure). **Human is final** (Counsel advisory; ETHICAL-STOP = friction, not veto). **Cost discipline:** the triad/loops cost tokens — serious work convenes the council, small work does not.

## The seven roles

| Plane | Role | Axis | Doc | Model |
|---|---|---|---|---|
| Design | System Architect | will it work / scale | [roles/system-architect](./roles/system-architect.md) | opus |
| Design | System Breaker | how it breaks | [roles/system-breaker](./roles/system-breaker.md) | opus |
| Design | Counsel | worth / good / wise; agent health | [roles/counsel](./roles/counsel.md) | opus |
| Loop | Loop-Architect | is the loop sound (M1–M11) | [roles/loop-architect](./roles/loop-architect.md) | opus |
| Loop | Loop-Orchestrator | which loop, for whom, when | [roles/loop-orchestrator](./roles/loop-orchestrator.md) | inherit |
| Object | worker | do exactly enough | [roles/worker](./roles/worker.md) | — |
| Object | error-fix / variants | every flow green | [roles/error-fix-runtime](./roles/error-fix-runtime.md) | — |

## The loop library (three families)

- **Design (1):** [design-convergence](./loops/design-convergence.md) — the Triad Council.
- **Error / diagnostics (7, reactive):** [error-fix-convergence](./loops/error-fix-convergence.md) (canonical) · [backend-contract-convergence](./loops/backend-contract-convergence.md) · [investigation-triage](./loops/investigation-triage.md) · [regression-hunt](./loops/regression-hunt.md) · [incident-recovery](./loops/incident-recovery.md) · [refactor-convergence](./loops/refactor-convergence.md) · [performance](./loops/performance.md).
- **Build / delivery (3, proactive pipeline):** [build-stage](./loops/build-stage.md) → [audit-gate](./loops/audit-gate.md) → [exit-audit](./loops/exit-audit.md).

Registry/catalog: [loops/REGISTRY](./loops/REGISTRY.md). Governance change: [governance/require-classification](./governance/require-classification.md).

## Control flow (how a request moves)

```
any request
  └─ /loop-orchestrator → CLASSIFY (4-condition test)
       ├─ not a loop (one-shot)         → do it once, stop
       ├─ loop exists + CERTIFIED       → REUSE / ADAPT-params → DISPATCH worker
       └─ no loop / "sick" / structural → Loop-Architect (BUILD/IMPROVE → M1–M11 → CERTIFY) → register → DISPATCH
                                            └─ error-fix → /converge-loop (Playwright to green) → HARVEST memory

  SEPARATE BRANCH — SERIOUS change before code:
    /council → design-convergence (Architect ↔ Breaker ∥ Counsel)
      → hardened plan + ADR + threat-model + ethical decisions → THEN code → error-fix verifies
      (accepted threat-model items → X-blockers in the error-fix matrix)
```

## Bring-up order (summary; full steps in the Integration Plan)

0. Prereqs: governance hooks live, repo healthy, Claude Code authed.
1. Loop skeleton: `loops/registry.md`, `loops/_templates/*`, empty `reports/`+`memory/`, seed two cards.
2. Loop-Architect (quality keystone).
3. Core commands: `/loop-orchestrator`, `/build-verify-loop`, `/converge-loop`.
4. Triad: `system-architect`, `system-breaker`, `counsel` + `/council`.
5. Restart `claude` → verify `/agents` + `/help`.
6. (rec.) OpenRouter cross-opinion bridge (M11).
7. (next) Hook automation: `require-classification` patch + `route-request`.
8. Proof-of-life: `/converge-loop` narrow scope · `/council` on the next real serious change · first Counsel health-pass.

## Invariants that hold everything (🔴 cross-cutting)

- **Nothing uncertified is dispatched.** No loop runs without M1–M11.
- **Orchestrator never mutates loop structure** — parameters only; structure/new loops via Loop-Architect + re-cert.
- **Breaker proposes no fixes** (breaks only, demonstrably); **Architect never self-marks "resolved"** without a Breaker/Counsel round.
- **Counsel advisory; human final.** ETHICAL-STOP only on a grounded red-line, with a recorded human decision.
- **Serious → council BEFORE code.** Small does not convene.
- **No fake-green** everywhere; test = spec; server read-only in error/UI loops.
- **Product red-lines** (from v4.5): human-in-loop/zero-autoban; friction-not-verdict; integer money; RLS FORCE; zero cookies; JWT RS256; zero PII in AI; claim-check; anonymize-not-delete; a11y WCAG-AA; "schema rich, runtime minimal"; trigger = first real paid order.
- **Governance hooks** (`post-edit-gates`, `require-classification`) sit above all agents.

## Reading order for a new session

- **Always:** this index + `Context-Handoff-v4_5` (ADR/red-lines/reality).
- **Building/running loops:** + Loop-Orchestrator-Spec-v2 + Loop-Core pack.
- **Serious change / design:** + System-Architect-Breaker-Spec-v1 + Counsel-Spec-v1 + Triad-Council pack.
- **Running error-fix:** + Convergence-Playwright-Loop-Prompt + component inventory + contracts.
