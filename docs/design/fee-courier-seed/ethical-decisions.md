# Ethical decisions — fee-courier-seed (Triadic Council)

Date: 2026-06-25 · Owner: human (SyniakSviatoslav) · Conductor: Council

## ETHICAL-STOP-1 (Counsel) — cash-on-delivery: amount SHOWN == amount COLLECTED at the door
**Decision: PROCEED.** The hardened design satisfies the red line: the customer sees the
**server-authoritative total** in a before-commit review step (estimate-hint → review server total →
confirm cash → submit), the courier screen shows the same "collect: X", and the FE handles
`422 CASH_AMOUNT_TOO_LOW`. **Door-handover parity is the PRIMARY acceptance criterion**, enforced by a
red→green parity guardrail + a door-handover E2E (Mandatory Proof Rule + money red-line). No ship without it.

## Item 3 (encrypted dev-seed) — strategic decision
Architect + Counsel both recommended **(a) drop** on proportionality (impersonation expansion = same
class as the prior `dev-login-backdoor` CRITICAL). **Human decision: BUILD HARDENED (b)** — accepted
ONLY with ALL FOUR constraints, each a ship-blocker:
1. `/dev/mock-auth` mints a token ONLY for the single synthetic seeded courier id (a constant) — NEVER
   an arbitrary `body.courierId`.
2. Seed is idempotent (DELETE-before-insert; no duplicate shift/assignment on re-run).
3. `.test`-TLD emails are rejected by real registration (synthetic PII can never shadow a real courier).
4. ON CONFLICT touches only synthetic-owned rows (a namespaced sentinel hash that cannot match a real
   `email_hash`).
Residual risk owner: human. Partial (b) (any constraint missing) recreates the backdoor shape → NO-GO.

## Branch / scope (Counsel §5)
**Decision: keep on `fix/design-system-consistency`**; scope/naming drift acknowledged in this decision log.
