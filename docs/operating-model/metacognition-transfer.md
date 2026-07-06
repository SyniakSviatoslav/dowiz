# Metacognition transfer — how expensive-model reasoning becomes a cheap model's checklist

> **Thesis (extends [player-roles.md](./player-roles.md) "system-fit over individual cleverness" +
> [KNOWLEDGE-AS-CIRCUITS.md](./KNOWLEDGE-AS-CIRCUITS.md) + AGENTS.md MODEL ROUTING/TOKEN ROUTER):**
> a model's useful metacognition — "what could break?", "did the plan match reality?", "is this guard
> real?", "who should verify this?" — is not talent to be re-run each time. It is COMPILED into shared
> artifacts (a plan's DoD, a deterministic gate, a pre-flight checklist, a circuit) that a cheaper model
> EXECUTES. When the thinking lives in the contract, a Haiku/Sonnet doer + the contract ≈ an expensive
> reasoner. "Make metacognition universal" = keep compiling reasoning into those artifacts, and make the
> two mechanical meta-moves below standard for every lane. Worked, RED-proven evidence: the 0b-2 session
> (`e3e30ac1`) + reflection `2026-07-06-metacognition-transfer-0b2`.

## The cross-pattern: planner (Fable-5) ↔ executor (Opus) ↔ doer (Haiku/Sonnet)

In the Sovereign-Core arc the Fable-5 planner authored the GRAND-PLAN. Its highest-value output was **not
the steps** — it was, per step, the **DoD + deterministic gate + red-line flag**. That is the planner's
reasoning already *compiled*: it says exactly what "done" means and how to prove it, so the executor does
not re-derive it. The executor's own value-add reduced, in practice, to **three mechanical moves** — none
of which requires being smart, all of which a cheap doer can run:

1. **Execute the DoD** (the compiled plan).
2. **Reconcile the plan against reality** — grep every file/symbol the step names; the plan is a hypothesis
   about a codebase that has since moved (0b-2: the plan's `codec/request_hash.rs` was wrong — the hash
   lives in the shell; blind obedience would have pulled a hasher into the sovereign core).
3. **RED-prove every gate** — inject the exact defect the gate targets and watch it fail (0b-2: a
   `#[deny(clippy::…)]` compiled clean but guarded *nothing*; only injecting the dodge exposed the dead gate).

The metacognition transfers because (1) is the plan, and (2)+(3) are cheap checklist steps. Talent is not
in the loop; the contract is.

## Transferable moves → their compiled artifact (author once = expensive · run every time = cheap)

| Metacognitive move (the "thinking") | Compiled artifact a cheap model executes | Authored by | Run by |
|---|---|---|---|
| "What could break? What's done?" | **Exit-list / DoD** ([task-exit-rule.md](./task-exit-rule.md)) — authored BEFORE code | planner / lead (once) | doer (every step) |
| "Does the plan match the code that exists now?" | **Reconcile-vs-reality pre-flight** — grep each named file/symbol; record divergences | — (universal step, below) | doer |
| "Is this guard a real gate or decoration?" | **RED-prove requirement** — the guard must be observed failing on its target defect (generalizes the Mandatory Proof Rule from fixes to *guards*) | — (universal step, below) | doer |
| "Will a wrong value slip a happy-path check?" | **Deterministic gate that can go RED** (rustc exhaustiveness, a circuit, a self-test, a hook) | lead (once) | CI / pre-commit (every commit) |
| "Am I trusting a compromised verifier?" | **Decorrelation rule** — red-line/money verified by a fresh reviewer that didn't write the code ([player-roles.md](./player-roles.md) fresh-eyes) | — (standing rule) | lead dispatches |
| "Am I sending the model work a script can finish?" | **INVERSION OF CONTROL / TOKEN ROUTER** (AGENTS.md rule −1) — deterministic-first, LLM only for the irreducible decision | — (standing rule) | every lane |
| "Should this repeated mistake need a model at all?" | **Circuit** ([KNOWLEDGE-AS-CIRCUITS.md](./KNOWLEDGE-AS-CIRCUITS.md)) — a pattern fires deterministically; reasoning is the last line, not the first | librarian (on promotion) | runner / hook |

The right-hand columns are the point: expensive cognition is paid **once** (to author the artifact); cheap
execution runs it **forever**. A doer never re-reasons what a gate already decides.

## The universal doer pre-flight + exit (the two new meta-moves, added to the Task-Exit Rule)

Any lane — on any model — runs these. They are mechanical, cheap, and carry the executor's metacognition:

**PRE-FLIGHT (before writing code)**
- [ ] **Reconcile-vs-reality:** grep every file/symbol the plan/step names; if reality diverges, reconcile
      explicitly and record why (the plan is a hypothesis). Blast-radius grep the shape of any shared type
      you will change.
- [ ] **Author the exit-list/DoD** before code (Task-Exit Rule) — each item checkable, with its proof.

**EXIT (before "done")**
- [ ] **RED-prove every guard you added** — inject its target defect, watch it fail, revert. A
      lint/`#[deny]`/assert/hook/gate that was never observed going red is **unverified** and may be dead.
      If it will not go red, it is not a gate — replace it with one that does.
- [ ] **Decorrelate the verifier** for red-line/money — a fresh reader that didn't write the code.
- [ ] Each exit-list item marked PASS with pasted proof.

> Why these two specifically: both 0b-2 near-misses (the wrong `codec.rs` assumption; the dead `#[deny]`)
> were caught by a mechanical move, not by cleverness — and both belong to failure families the ledger
> already knows: "live/edited-model ≠ reality" (#77 family) and "certified-green / mirror verifier" (#56,
> reflection `2026-07-06-harden-gates-first`). Making the moves standard closes them for every model.

## Status

- **APPLIED (agent-editable docs):** this doc + the reflection `2026-07-06-metacognition-transfer-0b2`
  (INBOX). The two meta-moves are stated here as the universal pre-flight/exit.
- **PROPOSED (operator-gated — not applied):** promote "RED-prove every guard" to a Tier-1 lesson/guardrail
  via the normal librarian/critics flow (it is a generalization of the Mandatory Proof Rule); add a one-line
  pointer to this doc from the CLAUDE.md Task-Exit section (CLAUDE.md is operator-cp-applied, token-gated).
  No hook/settings change is made here (gated). The critics/librarian retro decides whether the reflection
  ratchets into a circuit — this doc does not self-promote (separation of powers: worker reflects, Council
  deliberates, librarian enacts).
