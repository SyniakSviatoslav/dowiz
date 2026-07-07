# Reflection — a guard's RED must be proven along the dimension its invariant ranges over, or a fixed example sits in its blind spot

**Date:** 2026-07-07 · **Slug:** 0b3-decide-composition
**Qualified because:** red-line money + order-lifecycle change (Sovereign Core `decide` — the central
door) + 5 code files (`kernel.rs`, `error.rs`, `pricing.rs`, `lib.rs`, `tests/kernel_hard_truth.rs`) +
stage-close (GRAND-PLAN 0b-3).

## CONTEXT
Executed GRAND-PLAN 0b-3 (compose the actor-gate + CC-1 strand guard + pricing/LC1 corridor behind the
single `decide` door) interactively. Red-line; council was removed for the money boundary, so the gate
was a decorrelated `invariant-guardian` read + `sovereign-gate` + RED proofs. Shipped after
guardian PASS/high/no-flags. `decide` grew from `(&OrderState, Command)` to `(&OrderState, Command,
&Context)`; new `Actor`/`Command::PlaceOrder`/`CorridorBreach`; `Command` lost `Copy`.

## DECISIONS
1. **Escalated the signature fork to the operator BEFORE coding** (AskUserQuestion): where do the
   corridor inputs live — observed-`Context` param vs fat-commands vs fold-into-state. Operator chose
   full-0b-3-this-session + observed-`Context`. The code constraints later CONFIRMED the choice:
   `PricingSnapshot<'a>` holds `&HashMap` refs → cannot ride on a serde/`Copy` `Command`, so the
   snapshot HAD to be observed context, not intent.
2. **Ported the pricing assembly (`pg.rs:287-343`) VERBATIM** and had the guardian diff it arg-by-arg
   against both the live assembly AND the persistence side (`pg.rs:401-405`) — the corridor-order
   red-line ("a divergence here IS the mirror-oracle failure mode").
3. **Kept the core inventing no money number** — `RefundObligated.amount` is shell-observed
   (`ctx.refundable_paid`), `Priced` numbers are the unchanged, already-oracle-verified 0b-1 pricing
   fns. So 0b-3 added NO new byte-parity surface in the core.
4. **RED-proved all three new guards** (actor-gate, `BindingTerminalized` emission, LC1) by injecting the
   exact defect each targets and watching the suite go red, then reverting.
5. **Reconciled the plan against reality**: `idempotency_decision` + `needs_honest_dispatch` are listed
   in the plan as "corridors to fold into `decide`", but they RETURN control-flow (replay/422/proceed;
   route-through-honest-dispatch), not events — so they stay companion/pre-door fns, NOT folded.

## WHERE
`rebuild/crates/domain/src/kernel.rs` (`decide`/`price_cart`), the conservation/LC1 proptest
`place_order_priced_fact_satisfies_conservation_and_lc1` vs the concrete
`place_order_prices_the_cart_into_a_priced_fact` (`tests/kernel_hard_truth.rs`, `kernel.rs` tests), and
the plan's "fold idempotency/needs_honest_dispatch into decide" line
(`docs/design/sovereign-core-mvp/GRAND-PLAN.md` 0b-3).

## WHY (causal)
1. **A guard's RED proof must exercise the dimension its invariant ranges over — a concrete example
   fixed elsewhere in the input space is structurally blind to the break.** RED proof 3 injected the
   classic LC1 bug (pass raw `tax_total` into `compose_total` instead of `charged_tax`). My CONCRETE
   unit test used an EXCLUSIVE venue, where `charged_tax == tax_total`, so it stayed GREEN under the
   bug — it literally cannot witness a double-charge because the two values coincide at its fixed point.
   Only the PROPTEST, which varies `price_includes_tax`, hit the inclusive case (`total=4` vs `3`) and
   went red. This is a sharper sub-rule of the harness's "RED-prove every guard": the RED must move
   along the axis the invariant quantifies over (here the inclusive/exclusive flag). A guard proven only
   at one point on that axis is a partial gate that reads as complete — the same false-green family as
   the 0b-2 dead `#[deny]` (a guard that never fires), now for a guard that fires but in the wrong region.
2. **"Compose corridor X behind the door" is only valid when X RETURNS the door's output type; a
   control-flow decision cannot be folded into an event-emitting door without corrupting its law.**
   The plan named `idempotency_decision`/`needs_honest_dispatch` as corridors to fold in. Reconciling
   against what they RETURN showed they yield a routing/branch decision, not an `Event` and not a
   refusal — folding them would force `decide: -> Result<Vec<Event>, _>` to become
   `-> Result<Union<Vec<Event>, Replay, Route>, _>`, breaking "decide returns events". This extends the
   0b-2 root ("a plan is a hypothesis; verify its named symbols against reality") one level deeper:
   verify the SHAPE/RETURN of what the plan says to compose, not merely that the symbol exists — a
   symbol can exist and still be structurally uncomposable into the target.
3. **On a red-line central-door SIGNATURE with ≥2 live interpretations, escalating the fork to the
   human before coding is cheaper than any wrong-shape commit — because the shape compounds downstream.**
   `decide`'s signature is pinned by 0b-4 (determinism tests) and flipped onto by 0b-5 (the live shell).
   A wrong choice would cost two re-touches of the money door. Escalating cost one question; the code
   constraints (the snapshot lifetime) then made the operator's answer provably right. The human may
   pick MORE scope than you'd recommend (they took full-0b-3 over my split proposal) — the job is to
   make the fork legible, not to default to the smallest slice.

## CONFIDENCE
High on (1) — it is mechanically reproducible (I watched the exclusive-only test stay green under the
injected bug) and generalizes cleanly. High on (2) — type-level, not judgment. Medium-high on (3) — it
is a policy heuristic (when to escalate vs decide), bounded by the doubt-escalation ladder already in
the harness; this is a concrete red-line instance, not a new rule.

## NEXT-TIME
- When RED-proving an invariant guard, first identify the axis the invariant quantifies over (a flag, a
  sign, a boundary) and inject the defect at a point that DIFFERS from any concrete fixture — or prefer a
  proptest over that axis outright. Treat a guard proven only by a single concrete example as
  provisional until a varying-dimension RED is shown.
- Before folding a plan's named "corridor/step" into a composing function, check its RETURN type against
  the target's output type; if it returns control-flow, it belongs beside the door, not inside it.
- Candidate Tier-1 ratchet: a lint/checklist item "guard added → RED-proven across its invariant's
  varying dimension, not just one fixture" — for the librarian/ratchet-critic to weigh.

## LINK
Builds on [[metacognition-transfer-0b2]] (plan-is-a-hypothesis; RED-prove-every-guard) — this is the
next-deeper instance of both roots. Same mirror-oracle/false-green family as the harden-gates /
rotted-reviewer reflection (#56). Commit: 0b-3 (`feat(sovereign-core): 0b-3 compose corridors behind the
single decide door`). Live cursor: `docs/design/sovereign-core-mvp/PROGRESS.md`.
