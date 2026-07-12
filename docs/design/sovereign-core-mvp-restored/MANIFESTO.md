# Dowiz / Sovereign Core — Operator Manifesto (source-of-truth vision)

> Faithful capture of the operator's strategy sessions (2026-07-05 → 2026-07-06), declared by the
> operator as **"the final state of truth for the desired product, MVP, further features, main concerns
> and rules."** This is the vision doc; `DECISIONS.md` = the binding rulings; `GRAND-PLAN.md` = the
> execution roadmap; `ANALYSIS.md` = the grounding evidence. The raw prompt conversation is the primary
> source; this organizes it without loss of substance.

## 0. Role & protocol of operation
Operator = product owner. Lead agent = **Senior Systems Architect + Lead Developer**, building a
deterministic autonomous business-process system (Sovereign Core) to production, with autonomy.
Protocol: **Analysis → Decision-making → Execution → Reporting.** Act autonomously EXCEPT critical
decisions (core-architecture change, external dependency choice, interaction-protocol change) → stop and
ask. Each iteration reports: done / tests passed / critical decisions needing operator.

## 1. Core doctrine (Непорушні — non-negotiable rules)
1. **Determinism > AI.** AI is a tool for writing code/tests in R&D/back-office ONLY. System runtime logic
   is hard-deterministic (Rust/WASM). No probabilistic decisions in business logic. AI in the client
   runtime only when a feature generates margin that exceeds its API cost.
2. **Modular Integrity.** Every module atomic; communication via event bus; a fault in one module must not
   break the core. **Platform vocabulary (clocks, RNG, env, floats, battery, network) never enters the
   pure core** — it stays in the shell.
3. **Testing as Religion.** Every line ships with its test (unit/integration). Untested code = nonexistent.
   Green tests are the authority, not the model's confidence.
4. **Resource Efficiency.** Lean core, not an enterprise monster. Optimize for constrained resources;
   predictable, near-zero runtime cost.

## 2. The MVP
A **modular hub that lets a food-business owner control their own data across their own channels from one
module** — escaping aggregators (0% commission, own-the-customer, own the data). Scope locked (D1):
own-channel distribution (web/QR/social/messaging) + ONE direct checkout + owned customer data;
aggregator (Wolt/Glovo) orders are a **read-only** unified view in a later phase (order-intake through the
hub breaks the single-money-surface invariant).

## 3. Architecture (the "landing" after Red-Teaming the space-stack)
- **Universal Rust:** backend + frontend logic (via WASM) + one shared `dowiz-core` crate → type-safety
  across the stack; kills the "agent decided X, DB thinks Y, UI shows Z" class.
- **Immutable event-sourced state machine = the law:** `Intent → decide → Event`; forbidden transitions
  are compile/runtime errors; `state = fold(events)`; event log = time-travel debug + the primary truth.
- **`dowiz-core` is pure** (no side-effects, no I/O, no clock/RNG). "Formal verification for the poor" =
  Rust enums + ownership + a wasm build gate. Money is integer-only (`Lek(i64)`, no `From<f64>`).
- **Hybrid UI = Trojan-horse:** classic Astro + Svelte 5 baseline (reliability, SEO, accessibility, works
  in the metro / on weak net / for crawlers) + optional Canvas/Intent/voice (Whisper) overlay as premium.
  **A fallback for every "perl."** Ship the familiar thing; harden the deterministic core underneath.

## 4. Reliability philosophy — the "logistics casino" (risk-managed, antifragile)
- **Risk-adjusted gate:** before any money/logistics action, `score = Utility − λ·Risk`; over threshold →
  `ABORT` / Manual-Review. **Fail-fast:** better to reject a good order than let a broken one reach the
  kitchen. Circuit breakers + graceful degradation + safe-mode, never global collapse.
- **Asymmetric / adversarial verification:** cheap deterministic gates (types, JSON/Zod schemas,
  invariants, compiler/linter, timeouts) over expensive reverse-engineering. Ask *"find 3 ways to break
  it,"* not *"explain it"* (avoids confirmation bias — reverse-engineering is also generation). Levels:
  L0 static/schema/invariant (always) · L1 differential/shadow/golden-master (vs prior version) · L2
  adversarial audit (selective, high blast-radius only).
- **Invariants OUTSIDE the agent:** external / out-of-band validators, physical ground-truth anchors; the
  agent is never the sole source of truth (incentive-misalignment defense).
- **Design-for-failure:** blast-radius isolation, retries → reflection-loops (isolated self-correction) →
  rotations (key/model fallback), dead-letter queues for "bad" outputs (quarantine, review weekly).
- **Trigger-gated intelligence (HFT/quant pattern):** 90% cheap deterministic path; a heavy model fires
  ONLY on a telemetry anomaly (latency / entropy / prediction-error spike). Manage execution like a
  portfolio: stop-loss (abort low-confidence/looping runs), take-profit (close as soon as tests pass).
- **Self-harness loop:** telemetry (structured execution traces) → weakness-mining (cluster failures, by
  code not LLM) → harness proposal (patch prompt/tool/policy) → **regression-gated validation** (merge
  only if the full snapshot suite passes) → merge. Never a patch without a test. Guard against
  overfitting-to-failures, drift (keep a change-log + rollback), and flaky tests (mock external inputs).

## 5. Token economy (unit-economics moat)
AI cost is R&D-only; at runtime hard Rust logic runs at predictable near-zero cost. Compress state (VSA /
symbolic IDs + a codebook, not JSON), prompt-cache the stable prefix, route by task-shape, spend heavy
models only on planning/critical-reasoning. Parallel isolated workers (MapReduce), not a swarm —
context-bloat/handoff/consensus loops make swarms cost 3–50×. Large-context "monolith" beats swarm when
the crossover math favors it; keep agency "on-demand" (only on triggers).

## 6. Decentralization (added 2026-07-06) — "no central server" as a design INVARIANT
Formula: **Local-first data × Local execution (WASM/edge) × P2P protocol = decentralized reliability;**
each node (venue, courier, client) an autonomous decision center. Reachable for FREE from an immutable,
deterministic, replayable, WASM-pure signed event log. **Bake seams now** (event-sourcing, WASM-pure core,
content-hash + signature slot per event, transport-agnostic sync port). **Defer machinery** (libp2p/mesh,
CRDT merge, per-actor Ed25519/PQC, full offline-first) to Phase 2+. (D2)

## 7. The grail (added 2026-07-06) — Energy-Aware Logistics
Consent-based edge compute: idle courier/venue devices become compute + mesh nodes (validate signatures,
relay mesh) under an explicit power policy (e.g. battery >40% + charging/idle), earning reputation. No
data centers = unit-economics no aggregator can match. **NOT in the pure core** (battery/idle/mesh =
ultimate platform vocabulary, non-deterministic, non-replayable). Rides the event log as signed
`ProofOfContribution` events. Phase 3. (D3)

## 8. Red-Team discipline (the operator's own guardrail)
The operator explicitly stress-tests his own plans and demands the lead do the same. **Over-engineering is
the #1 enemy** ("it kills the startup faster than anything"). The space-stack — PQC (Kyber/Dilithium),
formal verification (Coq/Aeneas), Canvas/vello UI, mesh/P2P — is **roadmap, hard-gated behind the MVP**
(D6). Ship the Trojan-horse: a familiar, reliable product with a sovereign deterministic core underneath.
Chase antifragility (grow from stress), not perfection.
