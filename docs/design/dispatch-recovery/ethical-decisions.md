# Ethical / scope decisions — dispatch-recovery (B2+B5)

## ETHICAL-STOP-1 — honest-failure tail (courier shortage)

**Raised by:** Counsel + Breaker. Today the system FALSELY logs "re-offered" while orders silently strand;
the round-1 fix wires the `ORDER_DISPATCH_FAILED` consumer (owner alert) and an honest customer state via a
new `orders.dispatch_exhausted_at` held-marker, so the customer is NOT left on a false "on its way." The
remaining human call (R-NEEDS-HUMAN-1) was whether, after max re-dispatch + owner alert, an un-actioned
order should AUTO-transition (grace → cancel + refund) so the customer's truth never depends on a human.

**Human decision (operator, 2026-06-29): OWNER-ONLY, NO AUTO-TRANSITION.**
- The `ORDER_DISPATCH_FAILED` consumer (owner Telegram/push alert) **stays** — the owner IS told.
- The honest customer state **stays**: on exhaustion the customer sees a truthful "delayed / finding a
  courier" state (via `dispatch_exhausted_at`), **NOT** a false "on its way" — this is the minimum that
  discharges the Counsel ETHICAL-STOP and is independent of the declined auto-cancel.
- **DECLINED:** the automated grace-window → auto-`CANCELLED` + refund. The owner resolves a stranded order
  MANUALLY. (`DISPATCH_OWNER_GRACE_ENABLED` stays OFF / the grace tail is not built.)
- **Accepted residual (operator-owned):** if the owner never acts, the customer sits in an honest-but-
  unresolved "delayed" state indefinitely. Operator owns the manual resolution; this is a human-dependent
  tail, but NOT a dishonest one (the floor — no false "on its way" — holds).

**Status:** DISCHARGED — owner alerted + customer-honest state preserved; auto-cancel automation declined by
operator with the residual owned. (If implementation drops the honest customer state too, that re-opens the
STOP — the honest "delayed" push is load-bearing, the auto-cancel is not.)

## Note — design NOT yet APPROVED
Round-1-resolved (3 HIGH dispositioned) but **PENDING a re-attack round** before STOP-DESIGN-B. No code gate
cleared. Net schema = one additive migration (`orders.dispatch_exhausted_at`, nullable — no `order_status`
enum ripple). Accept-risks owner-recorded (Option-C fold-in guarded by a standing regression test; slow-
courier re-pick; Recon M1 scan). A6 now watches all 8 workers truthfully (added the 4 missing heartbeats,
incl. `backup-hourly`) — not trimmed.
