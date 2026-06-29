# Ethical decisions — admin-platform-authz (B4)

## ETHICAL-STOP-1 — audit legibility floor (self-watched cross-tenant admin audit)

**Raised by:** Counsel (counsel-opinion.md). **Class:** friction, not veto — the gate is a net *reduction*
in capture risk (≤5 named, audited platform principals replacing "every owner can read fleet-wide data")
and ships regardless. The STOP only sets the legibility floor.

**Concern (grounded in the charter line "never captured… never turned against the people it was learned
from"):** platform-admin holds audited cross-tenant access, but the audit log is self-watched
(operator = admin = auditor = sole reader) and invisible to the affected restaurants — no notification,
no appeal. Counsel's single question: *at which named, owned trigger does an out-of-band, append-only
audit mirror become mandatory, and who watches for that trigger so it cannot pass silently?*

**Human decision (operator, 2026-06-29):** **Ratify Counsel's recommended floor.**
- Ship the platform-admin gate + the `platform_admin_audit_log` table NOW.
- The **out-of-band, append-only audit mirror becomes mandatory at the FIRST of**: (a) the first
  non-founder ops hire, OR (b) the first external/paying tenant onboarded.
- **Trigger-watcher:** the operator (sole founder) owns watching for that trigger; recorded here so it
  is a decided deferral, not a silent default.
- Until that trigger, the self-watched audit (`platform_admin_audit_log`, actor_id on every action) is
  accepted as the interim floor.

**Status:** DISCHARGED — recorded human decision + named trigger + named watcher. Council may exit.
Residuals R8 (audit-reader isolation) / R9 (out-of-band mirror) remain tracked to this same trigger.
