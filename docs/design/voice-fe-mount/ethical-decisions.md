# Ethical decisions — voice-fe-mount (STOP-DESIGN-B)

Recorded per the Triadic Council skill. Counsel is advisory; the human/operator is final. Every
open ETHICAL question has a recorded human decision below before code was written.

- **Date:** 2026-07-03
- **Owner / decider:** operator (sviatoslavsyniak@gmail.com)
- **Council outcome:** APPROVED — 0 unresolved CRITICAL/HIGH, 0 live ETHICAL-STOP. Artifacts:
  proposal.md · breaker-findings.md · counsel-opinion.md · resolution.md (r1+r2).

## Decision 1 — GO to build the mount now (dark)
**Decision:** BUILD the voice mount this session, dark behind `VITE_VOICE_ENABLED` (default OFF), to
the full 17-X-blocker red→green bar in `resolution.md §FINAL`. Merge dark; do not launch.
**Rationale:** design is hardened and converged; building dark is the sanctioned "deploy dark to
verify" — launch remains a separate, explicit act.

## Decision 2 — Counsel open question: flip-ON condition (the honest-demand gate)
**Counsel's ask:** pre-commit NOW, while unattached, the condition that moves voice from dark → ON,
so dark-mounting does not quietly turn the still-open demand decision (ADR-0015 §5 / R-J) into a
rubber stamp.
**Human decision:** **flip-ON at operator discretion later — NOT pre-committed.**
**Recorded caveat (counsel, advisory):** counsel named this exact choice as the momentum-ratchet /
rubber-stamp risk — a mounted 95%-built feature converts voice from *demand-gated* to *in-flight*,
and byte-reversibility ≠ momentum-reversibility. The human consciously accepted this risk; counsel
does not override a conscious human. **Accepted-risk owner: operator.**

## Standing constraint carried into the build (non-negotiable)
**B1a′ honest-UI is a must-pass exit criterion.** The closed-venue units ("closed add → no chip / no
"Done" / no `addItem`") and the fail-closed `createVoiceGate` machine gate MUST be red→green before
merge. If B1a′ is ever descoped, deferred, or its point-of-action liveness dropped, that becomes a
**live ETHICAL-STOP on honest-UI** (counsel WATCH-LINE) requiring a fresh human decision. Owner:
implementer + counsel re-review if descoped.
