# DeliveryOS — Agent Context

## What this project is
HTML mockup of DeliveryOS: SaaS delivery platform for restaurants.
Three roles: Client (orders food), Owner (manages), Courier (delivers).
Market: Albania, mobile-first, 77% cash payments.

## What we are building
Static HTML screens — no build step, no framework, vanilla JS + Tailwind CDN.
5 screens in priority order — see screen-builder skill for full specs.

## Non-negotiable rules
1. All colors via CSS variables — see DESIGN.md and tokens.css
2. No cookies anywhere — localStorage/sessionStorage only
3. No position:fixed in embed mode (?embed=true)
4. Theme switcher on every screen — cycles 6 presets via :root vars only
5. Dark mode mandatory on every screen

## Active skills
- deliveryos-theme — for any color/CSS/brand work
- component-builder — for individual UI components
- screen-builder — for full page screens

## Phase 4 Anti-Fake Rules (P26)
1. 🔴 **No auto-ban.** Signals are advisory only. No signal blocks order placement.
2. 🔴 **Reputation decays exponentially** (30-day half-life). Counter alone = no signal.
3. 🔴 **OTP = owner toggle, off by default.** Zero friction. Rate-limited (3/15min sends, 5/15min verifies).
4. 🔴 **Human-in-loop.** Acknowledge/dismiss are manual only. No auto-acknowledge.
5. 🔴 **Velocity = privacy-first.** Only `*_hash` (sha256), never raw phone/IP. 24h retention, tenant-scoped.
6. 🔴 **OTP token = short-lived (15min), order-scoped, single-use, not JWT** (opaque 32B base64url).
7. 🔴 **0 PII** in `customer_signals.evidence`, `velocity_events`, MessageBus events.
8. 🔴 **Cross-tenant signal query → 404**, not 403.
9. 🔴 **Zod `.strict()` on all endpoints.** RS256 JWT only. 0 cookies.

## Phase 5 Anonymizer Rules (P30)
1. 🔴 **Single mechanism — two triggers.** `AnonymizerService.anonymize(scope, subject)` is the only anonymization function. `RetentionTrigger` (cron) + `GdprErasureTrigger` (request) both call it. No divergent paths.
2. 🔴 **Anonymize, NOT delete.** PII fields → NULL or anon-token. Business fields (totals, counters, snapshots) remain. FK references preserved (`customers.id` unchanged). No `DELETE FROM customers/orders`.
3. 🔴 **Storage + R2 coverage.** Customer avatar cleanup on anonymization. R2 manifest is PII-free (only row counts + checksums). R2 lifecycle ≤ DB retention. `docs/phase5/retention-policy.md` documents the window.
4. 🔴 **Idempotent.** `anonymized_at IS NOT NULL → skip`. Re-running on already-anonymized subjects is a no-op.
5. 🔴 **Audit append-only, PII-free.** `anonymization_audit_log` — no PII in metadata. RLS + FORCE.
6. 🔴 **GDPR dedup.** UNIQUE partial index prevents multiple in-flight requests per customer. Rate-limit 1/customer/24h.
7. 🔴 **0 PII** in pg-boss payload, MessageBus events, logs, audit, error messages.
8. 🔴 **Cross-tenant → 404**, not 403. Owner-only RBAC.
9. 🔴 **Zod `.strict()` on all GDPR endpoints.** RS256 JWT only. 0 cookies.

## Reference documents
Full product context: docs/DeliveryOS-Context-v3.md
Full technical summary: docs/DeliveryOS-Full-Summary-v2.md
Phase 4 anti-fake: docs/phase4/anti-fake.md
OTP flow: docs/phase4/otp.md
Signals UI: docs/phase4/signals-ui.md