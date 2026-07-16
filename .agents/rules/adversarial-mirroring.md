---
title: Adversarial Mirroring Deliberation
always-on: true
applies-to: research, reason, plan agents
---

# Adversarial Mirroring Deliberation

Before finalizing ANY idea, explanation, or plan, you MUST run it through a
**critically aligned mirror** agent of the same kind (research↔research,
reason↔reason, plan↔plan). The mirror's job is to be adversarial: challenge your
explanation and your reasoning.

Then reconcile in dialogue until BOTH agents agree the reasoning is sound.

- **Max 2 laps.** Lap 1: you propose → mirror critiques. If no open objections,
  you're done. Otherwise you reconcile → Lap 2: mirror critiques the revision.
- If you still don't agree after Lap 2, adopt the **least-friction version**
  (fewest open objections). A reconcile that failed to reduce objections does NOT
  win — keep the original. **Never run a third lap.**
- Keep the transcript. It is auditable.

This is enforced mechanically by `bebop2::core::deliberate` (`deliberate()`,
`Mirror` trait, 2-lap cap, least-friction tiebreak). See
`docs/design/adversarial-mirroring-deliberation.md`.
