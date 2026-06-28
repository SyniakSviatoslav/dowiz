# Human Decisions — Flow Simplification Patch

> Recorded at STOP-DESIGN-B. The Triadic Council converged (4 Breaker rounds, 3 Counsel examines):
> 0 open CRITICAL/HIGH, **NO ETHICAL-STOP**. No grounded red-line was crossed, so there is no
> ethics-gate override here — these are the two NEEDS-HUMAN product/strategy decisions the council
> deliberately routed to the human. Decided 2026-06-28. Owner: product (user).

## 1. §6 — claim vs go-live sequencing → THREE ACTS PRESERVED

- **Question:** the patch text said "claim → one action → goes live." The shipped P6 council bindings
  (CC2/CC3/H-publish) + Counsel require claim → review → publish to remain three acts (the consent +
  allergen-confirmation gate = PROTECTED FRICTION, distinct from the incidental cart/page friction this
  patch removes).
- **Decision:** **Accept the 3-act model.** Claim = take ownership + bind owner login (one action);
  **go-live stays a separate, gated act.** The patch's "one action goes live" is NOT a build source.
- **Consequence:** `published_at` stays NULL through claim; `claim_transfer` untouched; the consent gate
  is annotated PROTECTED FRICTION as a code-level marker (G-PF1/G-PF2).

## 2. §3 — entrance/apartment field floor → CONTEXTUALLY-REQUIRED (pin-confidence-gated)

- **Question:** is the door-detail (entrance/apartment) hard-required, optional-but-inviting, or
  contextually-required?
- **Decision:** **Contextually-required.** Optional when the map-pin is high-confidence; **required when
  the pin is low-confidence** (multi-unit geocode / pin far from a snapped address). Routes friction to
  exactly where omission causes a failed delivery + a clarifying call the least-served customer cannot
  take; silent for the confident single-house user.
- **Consequence:** server-tolerant, no order-contract change; the FE gates the requirement on pin
  confidence. (Counsel's care-grounded recommendation; Architect concurred.)

---

*Both decisions are now build sources. The §3 gate must land before the checkout field-floor builds.*
