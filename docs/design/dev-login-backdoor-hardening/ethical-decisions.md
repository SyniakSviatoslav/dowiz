# Ethical Decisions — dev-login-backdoor-hardening

Recorded human decisions on the council's ETHICAL-STOPs. Counsel is advisory; the
human operator is final. (Council protocol STOP-ETHICS gate.)

| Field | Value |
|-------|-------|
| Decided by | Operator (sviatoslavsyniak@gmail.com) |
| Date | 2026-06-22 |
| Change | `dev-login-backdoor-hardening` (ADR-0003) |

---

## STOP-1 — Forensics before declaring "closed"

**Counsel's red line:** the design closes the hole and rotates the key, but never
asks whether the backdoor creds were *used*. Because no auth/login audit table
exists, absence of evidence is not evidence of absence; the rotation-surviving
fingerprint is rows created by the self-escalation chain
(`organizations` / `locations` / `memberships`) plus `auth_refresh_tokens` and any
real users/paid orders in the exposure window.

**Decision — TREAT AS NEAR-MISS (zero victims).**
Production was never launched: the storefront is gated behind the soft access /
invite-gating CTA (STOP-1 invite-gating blocks prod launch, per project memory),
so there are no real tenants, customers, or paid orders on prod. The confirmed
exploit is therefore a **CRITICAL defect with zero realized consequence (near-miss)**,
not a breach. Severity-of-defect ≠ severity-of-consequence (counsel's open question).

**Conditions attached to this decision:**
1. **`kid:1` rotation remains MANDATORY** (R-6, Operator-owned). The near-miss
   ruling does not waive it — the leaked owner token (kid:1, ~1d TTL) must be
   invalidated by rotating the prod JWT signing key regardless of victim count.
2. **Confirmatory counts are still owed when prod DB access is available.** Run the
   schema-verified BYPASSRLS queries from `resolution.md` (RESOLVE round 2/3) to
   confirm zero junk orgs/locations/memberships were created by the self-escalation
   path during the exposure window. Run as a **BYPASSRLS/superuser** role — under
   FORCE-RLS the app role returns zero by policy and a real breach would read clean.
   **Forensics before any deletion** — `memberships` and `auth_refresh_tokens`
   CASCADE from `users`; "delete `empty@`/`test@`" means the **code literal only**,
   never the prod user row, until counts are taken.

## STOP-2 — Disclosure obligation

**Counsel's red line:** if real PII was in the blast radius during the live window,
the disclosure duty is independent of the hole now being closed.

**Decision — NO DISCLOSURE DUTY TRIGGERED**, contingent on STOP-1: with prod dark
(no real users/PII processed), there is no data subject to whom disclosure is owed.
This decision is **revisited if** the confirmatory counts (STOP-1 condition 2) reveal
any real user/order in the exposure window.

---

## Open question (counsel) — was prod dark?

Answered above: **yes, prod was dark / never launched.** This collapses the
uncertainty: zero-victim near-miss. The cheap confirmatory `SELECT count(*)` is
deferred (not skipped) to whenever prod DB access is at hand.
