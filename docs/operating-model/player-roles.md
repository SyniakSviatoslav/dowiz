# Player roles & the pre-agreed stop/pivot condition (§4)

> Governed by [agent-operating-model.md](./agent-operating-model.md). System-fit over individual
> cleverness — "chemistry" is a **shared contract** (classification + hooks + design system + shared
> layer) each player executes independently of talent.
>
> **How the expensive players' metacognition transfers to the cheap ones →
> [metacognition-transfer.md](./metacognition-transfer.md)** — the planner's reasoning is compiled into
> each step's DoD+gate+red-line; the executor's is two mechanical moves (reconcile-vs-reality,
> RED-prove-the-guard) any doer runs. Talent is not in the loop; the contract is.

## Roles
| Role | Who | Job |
|------|-----|-----|
| **Drafter** | cheap rotation model (OpenRouter: Nemotron→Qwen→DeepSeek→Gemma→Mistral) | fast first code / draft |
| **Adversarial reviewer** | a DIFFERENT model than the drafter | *breaks* the draft (anti self-confirmation) |
| **Integrator-orchestrator** | Claude Code (main loop) | converges, runs the gates, holds coherence |
| **Naive / fresh-eyes** | a context-free agent (no Mem0/Repowise/handoff) | sees what the expert system is blind to (§6) |

In this codebase the council subagents (`system-architect`, `system-breaker`, `counsel`) are the
adversarial-reviewer instantiation for serious changes; `Explore`/`general-purpose` with zero memory
are the fresh-eyes instantiation.

## 🔴 Pre-agreed stop/pivot condition (closes v4.5 §9 open accountability item)
> "Patience and attachment are indistinguishable from the inside." The system subordinates its own
> coherence-genius to a condition agreed BEFORE the work, so ego/sunk-cost can't masquerade as
> diligence.

**Standing stop/pivot triggers (any one → stop & re-decide, blameless):**
1. **Loop:** N=3 failed attempts on the same target/signature (override `DOUBT_LOOP_N`) → escalate.
2. **Budget:** a spike exceeds its written time-box, or a build exceeds 3 iterations without a green
   checkpoint → stop, write a blameless retro, pick the cheaper path.
3. **Reality contradicts expectation:** a pilot / telemetry / real paid-order signal disagrees with
   the plan → `challenge` channel, not "push harder".
4. **Red-line / irreversible** in reach → gate to human (never auto-proceed).
5. **Green checkpoint reached** → the right to stop without guilt is explicit; handoff docs are the
   burnout insurance.

A stop at a green checkpoint or a well-proven NO-GO is **the net working**, not failure.
