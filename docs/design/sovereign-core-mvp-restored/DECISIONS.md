# Sovereign Core MVP — Locked Decisions (2026-07-06)

Operator directives + architect (lead) rulings that bound the grand plan. Grounded in three analysis
briefs (build-state, error-history, token-economy) captured 2026-07-06.

## D1 — MVP scope (operator, AskUserQuestion 2026-07-06)
**Own-channel hub + read-only aggregator view.** The owner controls THEIR OWN data (menu, direct orders,
customers) across their own channels (web/QR/social/messaging), one direct 0%-commission checkout.
Aggregator (Wolt/Glovo) orders appear LATER as a READ-ONLY unified dashboard (status/data visible;
money + intake stay on the owner's checkout). **NOT** marketplace order-ingestion — that breaks the
single-money-surface invariant and needs a money council. Honors the escape-aggregators thesis.

## D2 — Decentralization = design invariant, not an MVP feature
"No central server" is reachable for FREE from an immutable, deterministic, replayable, WASM-pure event
log. **Bake NOW** (cheap seams): event-sourced core (`state = fold(events)`), pure side-effect-free core
→ WASM, every mutating event carries content-hash (`request_hash`) + a signature slot, a
transport-agnostic sync PORT (server-DB today = one impl; libp2p peer = another). **DEFER Phase 2+:**
libp2p/mesh transport, CRDT merge (Automerge/Yjs — only needed for concurrent offline multi-writer edits
of the same entity; MVP reconciles per-source), per-actor Ed25519/PQC as auth root, full offline-first.

## D3 — The "grail" (energy-aware consent compute) = Phase 3, NOT in the pure core
`ResourcePolicy`/battery/idle/mesh are the ultimate platform vocabulary → they violate the core's purity
law (`clippy.toml` bans clocks/RNG/env; platform words stay in the shell). A decision that reads battery
is non-deterministic/non-replayable. Reachable for free: consent + contribution become signed
`ProofOfContribution` events in the same log. The core never learns what a battery is.

## D4 — Model routing for this arc (operator override of standing "Fable off")
**Fable 5 authors plans (critical reasoning) · Opus (lead) adversarially reviews · Haiku executes.**
"Top models only for planning/critical reasoning; cheap models do." Applies for this arc only; the
`[[model-routing-policy]]` "Fable off for lanes" still holds for cheap doer-lanes.

## D5 — Verification spine (from the error-history brief — THE load-bearing rule)
The #1 recurring failure root across 81 ledger rows + 19 reflections is **"the live/deployed/routed/bound
reality ≠ the edited/assumed/literal model"** (proxy passes, real surface fails). Therefore every step's
DoD MUST include a **deterministic gate that drives the REAL deployed/bound/un-bypassed surface, with
proof the gate can go RED on the actual defect.** No mirror-oracles, no psql-literal "proofs", no
"the call returned" — independent oracles + goal-state re-reads + bound-path parity + NOBYPASSRLS
behavioral tests. This is the manifesto's "invariants outside the agent / adversarial-not-confirmatory"
made mechanical.

## D6 — Space-stack is roadmap, hard-gated behind MVP
PQC (Kyber/Dilithium), Coq/Aeneas formal verification, Canvas/vello UI, mesh/P2P — each Red-Teamed by the
operator himself as premature. Marked Phase 2+; not MVP work. Seams only where free (D2/D3).

## D7 — Step 0 = token/harness optimization BEFORE architecture build (operator)
"Maximum optimization to cut token usage / avoid rot / remove bottlenecks" is the first execution step,
measured before/after. Ranked targets in the token brief (biggest: `enableAllProjectMcpServers:true`
loads every MCP connector into every lane ≈ 28%/session of pure plumbing).

## Current state (build-state brief, terse)
- `dowiz-core` = pure crate `rebuild/crates/domain` (renamed). 10-status order machine (100-pair sweep
  green), integer money `Lek(i64)` (no `From<f64>`), idempotency/request_hash, codec. wasm32 + clippy
  disallowed-methods proven. Step 1+2 done; Step 3 PARTIAL.
- NOT closed: `pricing.rs` (884-line) not extracted; `decide` enforces only the state machine, not the
  money/actor/conservation corridors (they exist as separate pure fns, not composed behind one door);
  sovereign gate NOT in CI (`.github` protect-path; manual proposal); no `deny.toml`.
- Hub = a doctrine + ONE primitive: channel *attribution* (`routes/orders/channel.rs`, write-only
  `orders.metadata.channel`). No `sales_channel` entity, no adapters, no cart-token (money-council-gated).

Full phased plan: `GRAND-PLAN.md` (authored by Fable 5, adversarially reviewed by lead).
