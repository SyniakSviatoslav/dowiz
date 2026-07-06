# Lead Adversarial Review of the Grand Plan (2026-07-06)

Opus (lead) review of `GRAND-PLAN.md` (authored by Fable 5) — "find 3 ways it breaks," per the
Manifesto's adversarial-not-confirmatory principle. The plan is strong, grounded, and self-critical;
these are the load-bearing amendments to fold in before/while executing.

## Grand-plan structure (Fable's exec summary, for reference)
- **Phase 0a** — 6 ranked token/harness cuts, cheapest-first (MCP allowlist ~28%/session, Explore-default,
  3 hook narrowings, ledger compact index, context-budget-guard, red-line-gate relaxation); 5 of 6 are
  protect-path → staged + operator-apply; does NOT block 0b.
- **Phase 0b — seal the core** (6 steps): extract `pricing.rs` (finding: `distance_km` f64 haversine stays
  in shell, distance crosses as integer meters — a wasm/replay float hazard); `Envelope{seq,at,cause}` +
  `Priced/RefundObligated/BindingTerminalized` events with an exhaustive-fold compile gate; corridors
  composed behind ONE `decide` door (`CorridorBreach`); Hard-Truth L1–L3 proptests; **0b-5 shell-flip**
  (the D5 keystone — deployed staging route provably executes `decide`, RED proof = injected corridor
  refusal visible on the live route + `x-dowiz-cutover` asserted); CI wiring + `deny.toml`/cargo-deny.
- **Phase 1 — hub core:** `sales_channels` registry (FORCE RLS, NOBYPASSRLS behavioral gate, council);
  append-only `order_events` log + replay-parity job vs real staging DB; ONE sync-PORT trait + contract
  suite on 2 impls (D2 seam); content-hash per event (independent pgcrypto oracle), signature slot dormant.
- **Phase 2 — Trojan-horse MVP (flagged OFF):** QR/link distribution + attribution e2e gate; **2.2 checkout
  through the sealed core** (cart-token v0, server-priced, money council, independent hand-computed money
  oracles + tamper/idempotency adversarial battery — the centerpiece/longest step); owned customer data +
  erasure goal-state re-reads; aggregator view = trait + empty-state stub only.
- **Phase 3+ roadmap:** each deferred item (ingestion, libp2p, CRDT, PQC, ProofOfContribution, Canvas,
  Coq) mapped to the seam that already makes it a swap, with its unlock gate.

## Lead amendments (F1–F5)

**F1 — REUSE the existing cutover shadow-diff; do NOT reinvent a leaner flip. [strongest]**
0b-5's "shell-flip" and Phase-1's replay-parity propose bespoke mechanisms; Fable flags 0b-5 *skips the
shadow-compare window* as a leanness bet. But the rebuild ALREADY built shadow-diff → flip → parity →
sub-second rollback → auto-degrade (`docs/ops/rebuild-cutover-h_t.json`, cutover harness). Skipping it
re-opens the project's #1 failure root — the exact cutover-stack mismatch that shipped the OG bug on
2026-07-06. **Ride the existing shadow-diff harness for the shell-flip and replay-parity.**

**F2 — Add an `f64`-in-core-state ban to the sovereign gate. [cheap, high-value]**
The `distance_km` finding proves platform floats leak during extraction; the gate bans clocks/RNG/env but
NOT `f64` in `Event`/`OrderState`/fold-state. One leak = silent native↔wasm replay divergence, invisible
until it diverges in prod. Add a clippy/lint (or a structural test) banning `f64` in core state types.

**F3 — Decouple Phase 0a; operator-apply latency must not gate anything.**
Run the protect-path harness cuts as an async operator batch. In parallel I do the AUTONOMOUS items
(REGRESSION-LEDGER compact index, wiring VSA codec/blind-orch into the dispatch loop) + all of 0b (code).
0a's staged items never block 0b.

**F4 — STRATEGIC FORK (operator's call — OPEN):**
The plan is **core-first**: owner value doesn't appear until Phase 2 (weeks of sealing + a money council).
The operator's own #1 risk is *"over-engineering kills the startup before owners onboard,"* and the live
revenue path (demos + claim→owner acquisition motion) already exists.
- **(a)** Strict sealed-core-first (the current plan), OR
- **(b)** *True Trojan-horse:* ship a thin owner **"control your data" dashboard on the current stack in
  parallel** — familiar front, harden the sovereign core underneath — so owners onboard + give feedback
  WHILE the deterministic foundation is built.
**Lead recommendation: (b)** — matches the Manifesto, de-risks the startup, and the demos are the front
door already open. **Not yet decided by operator — resolve at the start of the execution session.**

**F5 — Execute in fresh sessions, one phase at a time.**
The planning session (2026-07-06) ran well past the anti-rot threshold. Continuing to *build* in a bloated
context IS the token-waste/rot the operator asked to eliminate. Each phase starts fresh, loading
`MANIFESTO.md` + `DECISIONS.md` + `GRAND-PLAN.md` + this review.

## Verdict
Plan APPROVED to execute with amendments F1–F3 folded in and F4 resolved first. All red-line steps (0b
corridors, `sales_channels` RLS, 2.2 money/cart-token) carry council gates in the plan — the serious-gate
policy is satisfied. Phase 0 is the entry point.
