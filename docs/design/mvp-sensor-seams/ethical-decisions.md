# Decisions — mvp-sensor-seams (Triadic Council)

Date: 2026-06-25 · Owner: human (SyniakSviatoslav) · Conductor: Council

## ESTOP-1 (Counsel) — immutable promised_window vs client honesty → RESOLVED in design (no human gate needed)
A set-once `promised_window` would show the customer a frozen number that could be wrong and
uncorrectable through the channel that lied. **Resolved by SPLIT** (ADR-0009 v4): frozen
`promised_window_{lo,hi}` = promise-as-made, read ONLY by owner/§8 measurement; mutable `live_eta_{lo,hi}`
= the customer truth channel, recomputed per stage with the width-floor. Counsel re-examined and confirmed
the customer read-path points at the mutable column → red line lifted. No standing ESTOP.

## STRATEGIC — stock counter runtime (§2.3/§3.2): A vs B
**Decision: OPTION B — ship the inert `products.stock_remaining` column-seam only; DEFER the per-unit
decrement/restock RUNTIME to a focused follow-up.**
**Why:** the atomic decrement/restock broke in THREE consecutive council rounds (C1 lifecycle-after-COMMIT →
R2-C1 raw status-writers → R3-C1 FORCE-RLS firing-context), each fix hitting a new context the layer below
doesn't satisfy (the customer-cancel raw-pool path is the recurring villain). The brief's own "schema full,
runtime later" doctrine applies. The §2.3 limited-special need is already met by the shipped binary
`is_available` toggle. Under B the §4 per-unit DoS surface disappears (nothing decrements). The follow-up
starts ahead: a tenant-scoped SECURITY-DEFINER restock fn (location from the order row, abuse-safe) + an
anti-cheat-green DoD that runs against the REAL empty-context cancel handler under FORCE-RLS.
Deferred-with-runtime: R3-C1 (restock RLS context), R3-H1 (ON DELETE SET NULL line — accept-risk: a deleted
product has no counter to restock). Owner of the follow-up: Product + backend.

## Counsel open-Q §5 — band-centering decision → DEFERRED (measurement built now)
WHERE inside the honest ETA band the promise sits (the owner's OTP/conservativeness knob) is an
autopilot-time runtime decision (owner: Product + North-Star lead). **But** the symmetric customer-cost
signal — `late_within_band_rate` (`delivered_at` vs `promised_window_hi`/`live_eta_hi`) — is NAMED as a
collection output of the M1 sensor contract NOW (no new seam), so the autopilot cannot be built OTP-skewed
before that signal exists. Decision deferred; measurement not.

## In-batch tail accept-risks (named owners)
- Distributed botnet beyond per-IP velocity → Ops. · session_ref timing-correlation → Ops. · privileged/
  migration write bypassing the set-once trigger (intended escape hatch) → Architect. · BOM intermediate-node
  introduction = one named owner-driven backfill (NOT migration-free — honestly scoped) → North-Star.
