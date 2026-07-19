# GAP-A1 — Disposition Audit: the 9 Un-homed Arc Sub-units (2026-07-19)

> Follow-up to `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md`, which found this is the ONE genuine
> gap in the entire project roadmap. Per its own recommendation, this is a disposition/triage
> pass — most items resolve without new blueprints. Read `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE
> -2026-07-16.md:536-541` (§10.0) for the original flag, and
> `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md` + `mesh-real/BLUEPRINTS-MESH-REAL.md` for
> each item's real source content (read directly this pass, not inherited from memory).

## Disposition table

| ID | What it's about | Disposition | Evidence |
|---|---|---|---|
| **MESH-14** | Resolve stale/contradictory mesh docs + one RED-gate + "status only from a live test" CI-lint rule (5 sub-parts: D3-DTN reconcile note, migration-plan diagram, ADR-0007/0008 ratify, workspace Cargo.toml + README fix, CI-lint) | **NEEDS A SMALL BLUEPRINT** — real, doable, docs+CI-lint only ("НЕ ЧІПАЄМО — продакшн"). Part 5 (CI-lint requiring a live-test citation for any "CLOSED" claim) is functionally the SAME mechanism as today's **Q1 claim-verification checkpoint** (`BLUEPRINT-Q-SERIES...md`). Recommend: fold part 5 into Q1's scope rather than duplicate it; the remaining 4 parts (reconcile note, diagram, ADR ratify, workspace/README fix) are small enough for one short blueprint. | `mesh-real/BLUEPRINTS-MESH-REAL.md:184-194` |
| **IP-01** | KernelFacade anti-corruption layer — the single allowed path into the kernel for any integration adapter | **ALREADY COVERED, not cross-referenced.** `BLUEPRINT-P42-mcp-agent-skills.md` explicitly names this as one of "three instances, one pattern" (KernelFacade / P40's ToolPort / MCP layer) per the master roadmap's own §10.3 item 5. Fix: add the IP-01 cross-reference to P40/P42's index rows, no new blueprint. | `BLUEPRINT-P42-mcp-agent-skills.md:57,362` |
| **IP-02** | Additive `scope.rs` capability-dictionary extension for new integration Resources/Actions | **PREREQUISITE-SHAPED, not standalone.** Only becomes real work when the first new port (P42/P43) actually needs a new Resource/Action variant — happens inside whichever port blueprint builds it, not as its own document. No new blueprint; note as an implementation step inside P42/P43. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:26-37` |
| **IP-03** | `InboundPort`/`OutboundPort` trait contract | **PREREQUISITE-SHAPED, not standalone** — same reasoning as IP-02; the trait gets defined when the first port consuming it is actually built (P42/P43). No new blueprint. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:39-48` |
| **IP-04** | Route every inbound frame through `HybridGate` (wire→Law→money) with no exceptions | **ALREADY SATISFIED, no-op.** The item's own text says "НЕ ЧІПАЄМО — hybrid_gate.rs (RequireBoth уже правильний)" — it's validating already-correct existing behavior, not proposing new work. No blueprint needed; note as closed-by-inspection. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:50-58` |
| **IP-05** | Operator-as-API 8-parameter reactive core (all input signals reduce to Δ on `M·Ü+Γ·U̇+c²·L·U=S`) | **ALREADY ASSIGNED, not cross-referenced.** P42 itself states "IP-05's superposition of intents stays with DELIVERY's [scope]" — assigned, just not ID-tagged in the index. Fix: cross-reference only. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:64-75`; `BLUEPRINT-P42...md:123` |
| **IP-06** | `QualityGovernor` — graceful hardware-degradation ladder (Q3 WebGPU-compute → Q2 fragment-shader → Q1 CSS-springs → Q0 static), money never tweens at any tier | **GENUINE FUTURE-BLUEPRINT CANDIDATE — flagged, not written here.** No existing P-blueprint covers this specific degradation-ladder mechanism; it's real, separable, DELIVERY/engine-scoped, medium complexity. Recommend a dedicated follow-up blueprint when engine rendering work is next active — not urgent today (no live GPU pipeline exists yet per today's own RGB-GPU research, so a degradation ladder has nothing to degrade *from* yet). | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:77-86` |
| **IP-07** | Multimodal input superposition (touch/voice/gesture/gaze → one summed `Intent`) | **ALREADY ASSIGNED, not cross-referenced** — grouped with IP-05 under the same P42 DELIVERY-scope note. Cross-reference only. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:88-96`; `BLUEPRINT-P42...md:123` |
| **IP-09** | Corpus port — embedded/vector/RAG/agentic-RAG retrieval integration | **OVERLAPS TODAY'S P95, not a separate gap.** Today's living-memory index-persistence research (`OPUS-TOKENIZATION-LIVINGMEMORY-RECHECK-2026-07-19.md`, `BLUEPRINT-P95...md`) already covers this exact retrieval-port territory in more current detail. Recommend: cross-reference IP-09 → P95 rather than write a second, competing document. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:100+` (header only, this pass) |
| **IP-17** | `SeedPool` + `EntropySource` trait — "mix never replace" entropy pooling, OS floor mandatory, QRNG additive-only | **REAL, WELL-SPECIFIED, BUT CRYPTO RED-LINE — human-gated by its own text** ("🔴 crypto red-line — human gate"). Directly relevant to the README's "quantum-sourced entropy... further injection is open research" line just added. Correctly NOT actioned without explicit operator sign-off — flagging for operator attention, not writing a blueprint unrequested for red-line crypto work. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:237-249` |
| **IP-18** | ANU QRNG 3-step human-friendly onboarding UI | **BLOCKED ON IP-17** — registers a new `EntropySource`, meaningless before the pool exists. Same disposition: real, but stays with IP-17 pending operator decision. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:251-265` |
| **IP-21** | Core-untouched + capability-isolation RED-suite (R0-R8), aggregating tests from IP-01/02/04/08/10/11/12/14/15/19 | **VERIFICATION SCAFFOLDING, inherently downstream.** It's a test-aggregator over the OTHER IP items — cannot be meaningfully written until more of them land. Same shape as this session's Q1 checkpoint: grows incrementally as referenced work ships, not a one-time document. No blueprint needed now; revisit when ≥3 of its referenced items (IP-01/02/04 already effectively satisfied) actually get implementation work. | `integration-ports/BLUEPRINTS-INTEGRATION-PORTS.md:302-311` |

## Summary by category (12 items total: MESH-14 + 11 IP-*)

- **Needs a real small blueprint (1):** MESH-14 — recommend folding its CI-lint sub-part into Q1 rather than duplicating.
- **Already covered / already satisfied, cross-reference-only fix (6):** IP-01, IP-02, IP-03, IP-04, IP-05, IP-07 — no new blueprint, just index hygiene.
- **Genuine future-blueprint candidate, flagged not written (1):** IP-06 (`QualityGovernor`) — real and separable, but has nothing to degrade from yet (no live GPU pipeline exists per today's RGB-GPU research); revisit when engine rendering work is next active.
- **Overlaps existing work, redirect (1):** IP-09 → P95.
- **Correctly stays operator-gated, flagged not actioned (2):** IP-17, IP-18 — crypto red-line, by their own text.
- **Verification scaffolding, inherently downstream (1):** IP-21.

Net: zero new large blueprints required right now. One small blueprint (MESH-14, docs+CI-lint
scope only, folding part 5 into Q1) and one index cross-reference cleanup close out the gap that
can be closed today; IP-06 is named for a future pass, IP-17/18 correctly wait on the operator.
