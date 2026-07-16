# Adversarial Mirroring Deliberation (operator protocol, 2026-07-16)

## Rule (binding, for all research / reason / plan agents)

Every research, reason, and plan agent MUST, before finalizing an idea, explain
its idea to a **critically aligned mirror agent** (a research / reason / plan
agent of the same kind, tasked to be adversarial). The mirror challenges the
explanation and the reasoning. Author and mirror then reconcile their
explanations in dialogue until **BOTH agree** the reasoning is sound.

- The dialogue is capped at **2 laps** maximum.
  - Lap 1: author proposes → mirror critiques → (if no open objections) AGREE.
  - If objections remain, author reconciles → Lap 2: mirror critiques revised →
    (if no open objections) AGREE.
- If agreement is **not** reached within 2 laps, the **least-friction version**
  is adopted automatically: the candidate carrying the **fewest open objections**.
  A failed reconcile that did NOT reduce objections does NOT win — the original
  stands. No third lap is ever run (no hang, no infinite dialogue).
- The full transcript is retained (mirror it through the event-log upstream) so
  the deliberation is auditable.

## Why

A single agent's reasoning is fragile to its own blind spots. A critically
aligned mirror is the cheapest high-value check against confirmation bias,
unsupported claims, and logic gaps — without the cost of a full external review.
The 2-lap cap + least-friction tiebreak keeps the protocol bounded and
non-blocking: it can never deadlock a pipeline.

## Implementation

`bebop2/core/src/deliberate.rs` — `deliberate(initial, mirror, reconcile)`:

- `Mirror` trait: `fn critique(&self, proposal: &str) -> Vec<String>` — returns
  the open objections; empty `Vec` = agreement.
- `reconcile`: `FnMut(&str, &[String]) -> String` — author's revision for lap 2.
- Returns `Conclusion { adopted, outcome, laps, transcript }` where `outcome` is
  `Agreed` or `LeastFrictionAdopted`, and `laps ∈ {1,2}` (structurally capped).
- Enforced by tests: lap1-agreement, lap2-reconciliation, cap→original,
  cap→revision-when-better, never-exceeds-two-laps.

This is a decision-protocol gate. It does NOT mutate kernel state (cf.
`self_mod` effector, which is separately authorized).
