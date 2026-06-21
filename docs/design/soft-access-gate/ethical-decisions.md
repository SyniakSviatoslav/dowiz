# Ethical Decisions — Soft Access Gate

Recorded at STOP-ETHICS gate. Decision owner: **Sviatoslav (sviatoslavsyniak@gmail.com)**, date **2026-06-20**.
Counsel is advisory; the conscious human overrides. Each ETHICAL-STOP below has a recorded decision.

## STOP-1 — Narrative must not sell exclusivity that does not exist
**Context:** owner onboarding is currently OPEN self-serve (`apps/api/src/routes/auth.ts:112-118,138` Google; `:184-188,213` Telegram → first login mints `role:'owner'`; `routes/owner/onboarding.ts:30` only `requireRole(['owner'])`). A "waitlist/approved" framing would be a dark pattern while anyone can self-provision.

**Decision: INVITE-GATING FIRST.** The human chose to make the gate *true* rather than soften the copy. Owner-onboarding invite-gating ships **before** the waitlist CTA goes live. This promotes the former defer-flag (`owner-onboarding-invite-gating`) to a **blocking prerequisite** for this feature.
- Consequence: owner-invite-gating is itself a serious auth-flow change → requires its **own** Triad Council / design pass (token issuance path, allowlist/invite storage, who may invite, first-owner bootstrap, RS256 token claims). It is NOT designed in this pass.
- Until invite-gating ships, the access-request CTA must NOT be launched with "waitlist/approved/queue position" language. Sequencing: invite-gating council → invite-gating shipped → soft-access-gate shipped.

## STOP-2 — PII collected now; retention + erasure must exist day one
**Decisions:**
1. **Lawful basis: EXPLICIT CONSENT.** A mandatory consent checkbox precedes submit (not legitimate-interest). This changes the frictionless surface (one field + checkbox + button) and requires capturing consent (consent timestamp + privacy-notice version) on the row.
2. **Retention: 12 months**, then auto-erase. Day-one manual erasure path remains in scope (`scripts/erase-access-request.ts <email>` + DELETE grant + runbook); automated 12-month TTL sweep folded into the design (not deferred to Stage-30 as the only mechanism — Stage-30 automates on top).
3. `user_agent` column dropped (no named use). `ip_hash` retained for abuse-forensics with named purpose.

## STOP-2 follow-on — /privacy page
**Decision: BUILD minimal `/privacy`** (sq/en) as part of this change. The GDPR microcopy links to it; today the page does NOT exist in `apps/web` (only a string in `CheckoutPage.tsx`). A link-to-404 is itself a GDPR failure, so the page is in scope.

## Net scope impact
- **New blocking prerequisite (separate change):** owner-onboarding-invite-gating — needs its own council.
- **Folded into this design:** consent checkbox + consent capture (consent_at, privacy_version columns); 12-month TTL auto-erase sweep; minimal `/privacy` page (sq/en).
- **Dropped:** `user_agent` column.
- Lawful basis flips legitimate-interest → consent throughout proposal/ADR/privacy copy.
