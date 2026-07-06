# Reflection — the executor's metacognition is a few mechanical moves, not talent; compile them so cheap models reproduce them

**Date:** 2026-07-06 · **Slug:** metacognition-transfer-0b2
**Qualified because:** red-line money-type change (Sovereign Core `Event`/`fold`) + 3 files + ≥3 iterations
(the clippy-belt investigation took ~6 attempts) + stage-close (GRAND-PLAN 0b-2).

## CONTEXT
Executed GRAND-PLAN 0b-2 (event vocabulary + `Envelope`) autonomously — a red-line change that grows the
sovereign core's money/binding event alphabet and the accumulating `fold`. The GRAND-PLAN was authored by
the Fable-5 planner; I (the Opus integrator/executor/reviewer) ran it, gated it, and shipped `e3e30ac1`.
Operator then asked to examine my own reasoning/pathfinding patterns — and the cross-patterns with the
Fable-5 planner — document them, and transfer the metacognition into the harness so CHEAPER models reproduce
it universally.

## DECISIONS
1. **Reconciled the plan against reality before coding.** The plan said "`codec/request_hash.rs` (expose
   `CommandHash`)"; a grep showed `build_request_hash` lives in the `api` SHELL, which the pure core cannot
   reach. → made `CommandHash` an OPAQUE core newtype the shell SUPPLIES (like `Ts`), never computed in-core.
2. **Blast-radius grep before a shape change.** Enumerated every `OrderState`/`Event`/`fold` construction &
   match site before growing them (contained to the `domain` crate; the api shell imports only `ErrorCode`)
   → grew the types confidently, ~4 literal sites to touch, zero shell regression.
3. **Scope discipline — deferred the tempting adjacent work.** Grew the alphabet + fold but left `decide`
   UNCHANGED; EMITTING the new facts is corridor wiring = 0b-3, not 0b-2.
4. **Rejected a false-green guard.** `#[deny(clippy::wildcard_enum_match_arm)]` on `fold` compiled clean and
   LOOKED like a gate; RED-proving it showed it never fires (clippy 1.96 doesn't lint `fold`'s `&Event`/deref
   match). Replaced it with a deterministic source self-test that IS RED-proven, and documented the rejection.
5. **Decorrelated verification** for the money red-line: a fresh opus `invariant-guardian` read the diff
   independently (PASS/high/no-flags) — a reviewer that didn't write the code.

## WHERE
`rebuild/crates/domain/src/kernel.rs` (`fold` + the `#[deny]`-that-guarded-nothing + the CommandHash
placement) and the plan's `codec/request_hash.rs` assumption (`docs/design/sovereign-core-mvp/GRAND-PLAN.md`).

## WHY (causal)
1. **A plan is a HYPOTHESIS about a codebase that has since moved; its named files/symbols must be verified
   against reality before execution, not trusted.** The `codec/request_hash.rs` line pre-dated `request_hash`
   landing in the shell. Following it literally would have pulled a hasher into the sovereign core (a Law
   violation the wasm/entropy gate exists to forbid). What caught it was not cleverness — it was a MECHANICAL
   move (grep the symbol the plan names, compare to reality, reconcile). Same shape as the standing rule for
   recalled memories ("if it names a file/function/flag, verify it still exists") — generalized to plans.
2. **A guard you did not watch go RED is decoration, not a gate — the same root as the mirror-oracle /
   "certified green" class (#56).** The `#[deny]` compiled and read like enforcement; it guarded nothing.
   Only injecting the exact defect it targets (`_ => *state`) and watching it FAIL to fail surfaced the dead
   gate. This generalizes the Mandatory Proof Rule from FIXES to GUARDS: a guard is unverified until its RED
   is observed. Shipping the dead `#[deny]` would have been a self-inflicted false-green.
3. **The expensive planner's reasoning is transferable BECAUSE it is already compiled into the per-step
   DoD + gate + red-line.** A cheap executor reproduces expensive-model output by three mechanical moves:
   (a) execute the DoD, (b) reconcile the plan's named files/symbols vs reality, (c) RED-prove every gate.
   None requires the executor to be smart — the metacognition lives in the shared contract (plan + gates +
   checklist), which is exactly the player-roles thesis ("system-fit over individual cleverness… each player
   executes independently of talent"). "Make metacognition universal" = keep compiling reasoning into those
   artifacts, and make moves (b) and (c) standard doer pre-flight/exit steps so a Haiku/Sonnet doer inherits
   them for free.

## CONFIDENCE
HIGH on (1) and (2) — (2) is directly evidenced by this session's dead `#[deny]` AND #56 (mirror oracle) AND
reflection 2026-07-06-harden-gates-first (rotted reviewer), one certified-green family. HIGH on (3) as the
transfer mechanism: it's the player-roles contract with a worked, RED-proven example.

## NEXT-TIME
- Before executing a plan step, grep every file/symbol it names; reconcile divergences explicitly (the plan
  is a hypothesis, not ground truth).
- RED-prove every guard/gate you add — inject its target defect and watch it fail — BEFORE claiming it is in
  place. A `#[deny]`/lint/assert/hook that was never observed going red is unverified and may be dead.
- Keep encoding expensive-model reasoning as DoD + gate + red-line (its compiled form); add the two
  meta-moves (reconcile-vs-reality, RED-prove-the-guard) to the universal doer pre-flight/exit so cheap
  models inherit the metacognition. See `docs/operating-model/metacognition-transfer.md`.

## LINK
commit `e3e30ac1` (0b-2) · `rebuild/crates/domain/src/kernel.rs` `fold` + `fold_stays_exhaustive_no_wildcard_arm`
· `docs/operating-model/metacognition-transfer.md` (the universal distillation) · [[player-roles]] ·
`docs/operating-model/KNOWLEDGE-AS-CIRCUITS.md` · REGRESSION-LEDGER #56 (mirror-oracle) ·
reflection `2026-07-06-harden-gates-first-and-rotted-reviewer` (same certified-green family) ·
[[sovereign-core-mvp-handoff-2026-07-06]].
