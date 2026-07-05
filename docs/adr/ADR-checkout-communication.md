# ADR-0016 — Checkout "Communication" channel overhaul

- Status: **APPROVED** (Triadic Council converged + operator decisions 2026-06-30). Build behind a flag;
  migration operator-applied FIRST. See `docs/design/checkout-communication/resolution.md`.
- Date: 2026-06-30
- Seat: System Architect → Council (Architect · Breaker · Counsel) → operator ratification

## Decision (ratified)
1. **Phone (call/SMS) is a first-class Communication kind, and the phone field STAYS.** Dissolves the Breaker
   CRITICAL (Fastify per-phone throttle keeps seeing `customer.phone`), the derivation-seam HIGH, and the
   Counsel exclusion ETHICAL-STOP (courier keeps `tel:`; phone-only customers not excluded).
2. **Receiver ("deliver to someone else") SHIPS in v1** — same-receiver checkbox default checked; if off, a
   receiver name + channel. Hardened: privacy notice on the form, `receiver_*` covered by the Phase-5
   anonymizer in the SAME release, receiver-DSAR keyed on receiver_handle, anonymize-on-delivery retention.
3. **SimpleX = text-only** (shown as copyable text, courier never auto-opens; no host-allowlist needed).
   **Google Meet + Microsoft Teams CUT** (category error for food delivery).
4. **Logos** vendored from web brand assets (nominative use); WCAG-AA text labels retained beside each.
5. v1 kinds: **Phone, WhatsApp, Viber, Telegram (username), Signal (phone), SimpleX (text-only link)**.
6. Migration applied (operator-gated) BEFORE the code; link-kind input validated server-side (exact-host
   equality + https for any clickable link; SimpleX rendered text-only). Telegram = username (`t.me`), never
   a `tel:`/phone.

(original DRAFT context retained below)

# ADR-0016 (context)

- Date: 2026-06-30
- Seat: System Architect
- Supersedes/relates: mig `1790000000038` (messenger deep-link, 3-kind CHECK); ADR-0005 (door-handover
  parity / server-authoritative total); the OTP/velocity anti-abuse seam (migs `…054`/`…057`).
- Full design: `docs/design/checkout-communication/proposal.md`

## Context

Checkout contact today = required phone + optional 3-kind messenger (telegram/whatsapp/viber, mig …038).
Owner wants contact reframed as a **required "Communication" channel** (kind + handle), 4 new kinds
(Signal, SimpleX, Google Meet, MS Teams), conditional input (phone vs link), the standalone phone field
removed, brand logos, and a "deliver to someone else" recipient.

Phone is load-bearing for **three independent subsystems**: phone-keyed OTP (`require_phone_otp`,
`hashPhone`), the per-phone velocity throttle (`phone_hash`), and customer dedup
(`ON CONFLICT (location_id, phone)`). Three of the new kinds (SimpleX/Meet/Teams) carry **no phone**, and a
fourth (Telegram, via `@username`) yields no phone either. Critically, `customers.phone` is **nullable** and
under a `UNIQUE (location_id, phone)` constraint where Postgres treats NULLs as distinct → the existing
`ON CONFLICT(phone)` upsert **never fires for phone-less rows** (would mint a new customer per order).

## Decision (proposed — pending council)

1. Rename to **"Communication"**, required selector + required adjacent input.
2. Expand `messenger_kind` CHECK to 7 kinds on `customers`/`couriers`/`orders` via one forward-only,
   operator-gated migration (DROP auto-named inline CHECK → ADD named CHECK).
3. Model the receiver as **dedicated nullable `orders.receiver_*` columns** (name + kind + handle + phone),
   not a free-text fold — for structured courier contact, clean RLS, and per-column GDPR anonymization.
4. **Phone-less orders create no `customers` row** (`customer_id` NULL); contact snapshot rides on the
   order. (Avoids the NULL-distinct UNIQUE mint-every-time bug.)
5. **Derive phone from the channel.** Phone-yielding kinds (WhatsApp/Viber/Signal) feed OTP/throttle/dedup
   unchanged; phone-less kinds (Telegram-username, link-kinds) have explicit per-subsystem behavior:
   OTP **blocked by policy** when a location requires it; throttle falls to the **unconditional IP floor**
   (+ optional contactHash); dedup **skipped**.
6. Link-kinds are validated server-side against a **scheme + host allowlist** (open-redirect control); the
   courier never auto-opens an untrusted link (explicit tap, `rel="noopener noreferrer"`, fail-closed
   null-builder).
7. **Phased**: ship rename + required + phone-yielding kinds (+ Telegram) live; ship SimpleX/Meet/Teams +
   URL allowlist dark behind `COMM_LINK_KINDS_ENABLED` (default off). Schema expands to all 7 now.

## Invariants honored

- Forward-only, operator-gated migration; down() is a no-op (red-line `packages/db/migrations/`).
- New columns are ALTER on an already ENABLE+FORCE'd `orders` (RLS + grant-mirror preserved); verify:rls.
- Integer-money untouched (no monetary columns).
- No open-redirect (server-authoritative scheme+host allowlist).
- No new PII→AI egress; handles/links/receiver added to the redaction + Phase-5 anonymizer sets.
- i18n hand-translated, sq/en/uk via the parity gate.
- Server authoritative over the derived phone and all gates.

## Consequences

- (+) Contact becomes a first-class, validated, structured field with a real courier deep-link per kind.
- (+) The phone-less state is explicit and handled, not silent breakage.
- (−) CRM dedup is lost for phone-less repeat customers (accepted v1; handle-dedup deferred).
- (−) Phone-less orders have no courier "Call" channel (Telegram-username + link-kinds) — owner must accept.
- (−) Anonymizer must be extended in the **same release** as the receiver columns (GDPR gate).

## Open questions (must close before ratification)

R1 trademark/logo rights · R2 OTP-required vs phone-less policy · R3 Telegram phone-or-username ·
R5 owner acceptance of no-callable orders · R6 anonymizer-coverage gate · R7 exact link allowlist hosts ·
R8 IP-throttle-only floor for phone-less. See proposal §13.
