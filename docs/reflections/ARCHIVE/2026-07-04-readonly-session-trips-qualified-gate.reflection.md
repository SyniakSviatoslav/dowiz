# Reflection — the qualified-change Stop gate fires on tree state, not this-session authorship

- **Date:** 2026-07-04
- **Trigger:** Stop hook — "QUALIFIED change (≥3 code files or a red-line surface) with no fresh
  reflection." Fired at the end of a session whose ONLY tool calls were Read + read-only git/bash.
- **Class:** harness / self-improvement loop (advisory-vs-authority; false-attribution).

## CONTEXT

The session task was pure recall/orientation: "retrieve the last session memories, changes and
project update research report." I made **zero** code edits — only Read and read-only `git
log/status/diff`. At Stop, the qualified-change gate fired anyway, because the working tree carries
a large uncommitted change left by the **prior** session: the voice-FE mount (`apps/web/src/lib/voice/`,
`packages/ui/src/voice/`, `packages/voice/src/confirmation-gate.ts`, `i18n-catalog.ts`) — ≥3 code
files AND a red-line-adjacent surface (voice→cart). Council-APPROVED, dark behind `VITE_VOICE_ENABLED`.

## DECISIONS

- Did NOT write a reflection claiming causal ownership of the voice-FE diff — I did not author it,
  and the honest-report rule forbids narrating authorship I don't have. The existing
  `2026-07-03-swarm-mergeback-rot.reflection.md` already carries the causal root of that work rotting.
- Wrote THIS reflection instead: an honest record of the gate's scope behavior, which is the only
  qualified event this session actually produced.

## WHERE

- The qualified-change Stop hook (reads working-tree state / `git status` at Stop, not the
  session-authored diff).
- Working tree on `fix/audit-remediation`: prior session's uncommitted voice-FE mount.

## WHY (causal root, not just location)

The gate measures **tree state**, not **this-session authorship**. So an uncommitted qualified change
left across a session boundary trips the *next* session's gate — even a read-only one — and demands a
reflection from an agent that changed nothing. Root cause upstream: the prior session ended with a
council-approved, ready-to-commit qualified change left **uncommitted** (same session-boundary hygiene
gap as the swarm-mergeback-rot / narrated-readiness family, row #48). The gate isn't wrong that a
qualified change exists un-reflected; it's mis-attributing *whose* it is and *when* it happened. A
naive compliance (write a WHY for the voice diff) would have manufactured a false-causal reflection —
the exact confirmation-bias failure the reflection ritual exists to prevent.

## CONFIDENCE

High on the observation (tool log for this session is Read/read-only-bash only; `git status` shows the
voice-FE files as pre-existing untracked/modified from 2026-07-03 timestamps). Medium on the fix
direction (two viable: scope the gate to session-authored diff, OR enforce commit-before-session-end).

## NEXT-TIME

1. **Clear the tree first.** The real next action independent of this gate: commit the council-approved
   voice-FE mount **dark** (or the operator does), which removes the un-reflected qualified change and
   the gate goes quiet honestly. Leaving a ready qualified change uncommitted across a boundary is the
   avoidable upstream cause.
2. **Don't compliance-fake a reflection.** When a gate fires on work you didn't author, the honest
   reflection is *about the gate firing*, not a fabricated WHY for someone else's diff.

## PROPAGATE (candidate — advisory; librarian/ratchet decides)

- Consider scoping the qualified-change detector to the **session-authored diff** (edits this agent
  made since session start) rather than raw `git status`, so read-only orientation sessions don't
  inherit a prior session's un-committed qualified surface. If kept tree-based, pair it with a
  session-end "commit or explicitly park" step so qualified changes never cross a boundary uncommitted.

## LINK

[[2026-07-03-swarm-mergeback-rot.reflection.md]] (same session-boundary hygiene family, row #48) ·
audit-remediation-orchestration-2026-07-03 (voice-FE = the open uncommitted unit) ·
governance-gates-rot-open reflection (advisory-vs-authority tuning).
