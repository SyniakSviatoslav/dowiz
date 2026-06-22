# Reflections — worker self-reflection + Council retro (bottom-up "why")

Part of the self-improvement loop. **Advisory, not authority** — a reflection never changes
behaviour on its own; it feeds the Council retro, which terminates in a deterministic
ratchet artifact (a guardrail / lesson / prune-revision / pointer) **or** an explicit no-op.

## When a worker writes a reflection
After a **qualified** fix/failure (the Council threshold): the fix touched ≥3 files **OR**
took ≥3 iterations/retries **OR** closed a stage **OR** touched a 🔴 red-line (auth/money/RLS/
migrations). Trivial reversible edits below the threshold → **no reflection** (anti-ceremony).

## Worker reflection format
`docs/reflections/INBOX/{yyyy-mm-dd}-{slug}.reflection.md`, atomic + distilled (NOT a raw
transcript):

```
---
CONTEXT:   <what you were doing>
DECISIONS: <key decisions / forks taken>
WHERE:     <symptom — where it went wrong>
WHY:       <CAUSAL ROOT — which assumption / reasoning step was wrong; what info was
            ignored; what should have been verified. Not just the symptom.>
CONFIDENCE: high | med | low   # on the WHY (falsifiable; Council challenges it)
NEXT-TIME: <what you'd do differently>
LINK:      <file:line / commit>
---
```
The worker only files the reflection in `INBOX/`; it does **not** itself make systemic changes
(separation of powers: worker reflects, Council deliberates, librarian enacts).

## Council retro (triggered, time-boxed, max N reflections/retro)
Roster (cheap models, isolated contexts): **cause-critic** (challenge each WHY: cause vs
correlate/coincidence/parallel-deploy → confirm/downgrade CONFIDENCE), **pattern-critic**
(cross-reflection structural root), **ratchet-critic** (each confirmed root → a deterministic
output: Tier-1 guardrail / Tier-2 lesson / prune-revision / CLAUDE.md pointer — or "no
actionable artifact + reason"). **Executor = librarian** enacts (writes candidates red→green,
updates the ledger, prunes the store, moves processed reflections to `ARCHIVE/`).

Output: `docs/reflections/RETRO-{date}.md` — every line is **→ artifact** OR **→ explicit
no-op with reason**. Reflection that doesn't terminate in a ratchet artifact (or explicit
no-op) is a no-op — change for its own sake is forbidden. Recurrent doubt/bug → **promote to a
guardrail / tighten the spec** so it stops recurring.
