# Ethical decisions — authz-state-hardening (B7+N)

## ETHICAL-STOP-N5b — owner-marked no_show reputation strike

**Raised by:** Counsel + Architect (NEEDS-HUMAN). **Class:** friction, not veto. The manual `no_show`
mark (`owner/signals.ts:224-250`) writes a raw, unattributed, non-dismissible counter on the
`customers` row, BYPASSING `customer_signals` — whose own contract comment is "Owner acknowledge/dismiss
only" (`1780421100057:104`). The system contradicts its own dignity contract. Counsel verified the
strike's ONLY effect today is an acknowledgeable `soft_confirm` that already discloses the count to the
customer (`evaluatePreflight.ts:127-134`) — there is NO auto-deny engine. The audit's "cross-tenant
reputation" framing was FALSE: `customers` is location-scoped (RLS FORCE, per `(location_id, phone)`).

**Human decision (operator, 2026-06-29): REQUIRE 6b-1 (attributable + dismissible record).**
- Every owner-marked strike MUST be written as an attributable, dismissible `customer_signals` record
  (`owner_id` + reason + timestamp) — reusing the existing FORCE-RLS `customer_signals` table
  (architect-verified **zero-migration**) — before/as it touches the reputation counter. The strike
  becomes auditable (who marked it, why) and dismissible (the owner contract the raw counter bypassed).
- **6b-2 (subject contest channel)** — DEFERRED to a NAMED trigger, recorded so it can't silently become
  permanent: *the first time `no_show` is consumed by anything stronger than an acknowledgeable
  `soft_confirm`* (i.e. the moment the signal gains real consequence, disclosure/contest becomes
  mandatory). Operator owns watching for that trigger.
- **Courier-as-witness** (Counsel's open question — attach the courier's delivery-attempt attestation as
  the strike's evidentiary ground, since the witness is the courier at the door, not the owner pressing
  the button): noted as a future hardening, NOT required for the 6b-1 floor.
- Paired with **6a** (block a strike unless the assignment reached `picked_up` — a real delivery attempt),
  this discharges N5.

**Status:** DISCHARGED (6b-1 floor) + 6b-2 named-trigger deferral + courier-attestation noted. The ADR
remains DRAFT pending the Breaker re-attack on the corrected proposal before any code.

## Note — design NOT yet APPROVED
B7+N banks at round-1-resolved (2 HIGH + 2 MED dispositioned). PENDING a re-attack round before
STOP-DESIGN-B, then implementation. All five fixes are zero-migration and independent. No code gate cleared.
