# Ethical / scope decisions — stage21-reconciliation (B1)

## RK-2 — courier earnings asymmetry (debt modeled, pay not)

**Raised by:** Counsel + Architect (NEEDS-HUMAN). The money model tracks what the courier OWES (COD cash
collected, which **bundles the `delivery_fee`** — verified `orders.ts:499` `total = subtotal + delivery_fee
+ tax − discount`, treated 100% owner revenue with **no courier-pay portion in code**) but models nothing
about what the courier is PAID. Counsel: honest for an owner/family courier; a fairness edge for a hired one.

**Human decision (operator, 2026-06-29): OWNER/FAMILY COURIER ONLY at launch.**
- At launch the courier is the owner or family reconciling their own till → a debt-only model is honest
  (the ledger mirrors their own pocket; "out-of-band pay" = their own shop's money).
- **RK-2 DISCHARGED** with a recorded **SEGMENT CONSTRAINT**: before onboarding ANY non-owner/hired courier,
  an earnings representation is REQUIRED — minimally the read-only **expected-pay-before-accept** consent
  floor Counsel carved out, ideally a full earnings model — authored as its own named council. Shipping the
  debt-only model to a hired courier WITHOUT that is a charter dignity breach (re-opens this STOP).
- **LATENT-STOP-2** (courier-visible own-ledger — couriers currently can't read their own holds) is gated to
  the SAME trigger: a courier-visible debt must ship with a courier-visible own-ledger.
- **Trigger-watcher:** operator owns watching for "first hired/non-owner courier"; recorded so it can't pass
  silently.

**Status:** DISCHARGED (owner/family-only segment) + segment-constraint + latent-stops recorded.

## Note — design NOT yet APPROVED
This ADR is round-1-resolved (2 CRITICAL + 4 HIGH dispositioned structurally) but **PENDING a re-attack
round** before STOP-DESIGN-B. No code gate cleared. Open technical residuals: the refund *caller* is unbuilt
(the residual-guard trigger blocks over-reversal, but the owner-refund *recording* is a contract until that
path ships with tests); already-paid snapshots can't be back-corrected (forward owner-refund + review flag);
multi-currency deferred (single-currency invariant); B3 NOBYPASSRLS is an external dependency (GUC-readiness
only prevents the self-DoS when it lands).
