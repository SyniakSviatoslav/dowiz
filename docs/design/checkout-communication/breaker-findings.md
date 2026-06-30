# Checkout "Communication" overhaul — Breaker findings (Triadic Council)

Returned inline by the system-breaker (harness blocks its file write); persisted here. Verified against
`orders.ts` (rate-limit keyGenerator :67-83, velocity phoneHash :237-261, OTP :140-176/297-322, customer
upsert :507-524), `preflight.ts:64-168`, `messenger.ts:1-38`, mig `1790000000038`, mig `…060`.
**8 findings — 1 CRITICAL · 3 HIGH · 3 MED · 1 LOW.**

## CRITICAL
- **C1 · Removing the phone field defeats the per-phone rate-limit for ALL traffic.** The 5/min throttle's
  `keyGenerator` runs at the **Fastify layer, before the handler** and reads `req.body?.customer?.phone`
  literally (`orders.ts:72-74`). The handle→E.164 derivation lives in the handler → **structurally can't run
  in the keyGenerator**. Remove the phone field → keyGenerator falls back to `req.ip` for every order of every
  kind → the per-phone control is gone (one number can order across rotating IPs) AND CGNAT/office users
  false-positive throttle. Proposal §6/§12 ("phoneHash unchanged / all subsystems intact") is false.

## HIGH
- **H1 · Derivation-seam underspecified** (`orders.ts:237` `cust?.phone`, `:509` `if (cust && cust.phone)`).
  Don't-inject → `cust.phone` undefined for ALL kinds → velocity skips AND no customer row ever created for
  any order (total CRM/dedup loss, not just link-kinds). Inject-early → dedup works but keyGenerator (C1) still
  can't see it. Internally inconsistent; the injection point vs the 3 read-sites is never pinned.
- **H2 · Receiver = non-consenting third-party PII with an un-fulfillable DSAR.** `gdpr_erasure_requests` is
  keyed on the *customer's* id/phone; `receiver_phone` is free-text on orders with no subject key/index → a
  **receiver-initiated** DSAR can't be located or erased before the parent order's 365-day clock. R6 only
  covers the customer's retention erasure, not lawful basis / receiver DSAR — wider GDPR gap.
- **H3 · Open-redirect: the §7 allowlist has no pinned exact-host equality + no server code site.**
  `messenger.ts` has zero URL validation today. `host.includes()`/`endsWith()` pass
  `teams.microsoft.com.evil.ru` / `evilmeet.google.com` / userinfo `@evil.com` / IDN homoglyphs → courier taps
  "open link" → phishing. **SimpleX self-hosting** (arbitrary invite hosts) can't be exact-allowlisted at all,
  yet SimpleX ships in the schema set. Control semantics + insertion point undefined.

## MEDIUM
- **M1 · CHECK auto-name assumption unverified.** Proposal asserts the mig-`038` auto-names
  (`customers_messenger_kind_check`, …) as fact without a `pg_constraint` read across prod/staging/dev. If a
  column pre-existed (`ADD COLUMN IF NOT EXISTS` skips the CHECK) or a re-add made `…_check1`, the targeted
  `DROP CONSTRAINT IF EXISTS` misses → **old 3-kind CHECK survives** → `signal` order → 500, order lost.
- **M2 · Deploy-ordering: Signal is v1-LIVE but the enum migration is separately operator-gated.** Code
  offering Signal deployed before the migration lands on that DB → `messenger_kind='signal'` hits the
  un-expanded CHECK → 500. The flag gates only SimpleX/Meet/Teams, not Signal; rollback doesn't cover the
  code-before-migration window.
- **M3 · Telegram username/phone ambiguity.** `messengerLink` telegram → `t.me/<u>` (public usernames only).
  Customer types a phone for Telegram → dead `t.me/%2B…` link + no `tel:` (phone-less) → courier has zero
  contact. Also: Telegram phone-less → per-phone velocity skips → handle-rotation abuse hole.

## LOW
- **L1 · Proposal mischaracterizes the OTP-phone-less risk as "silent bypass" — it's fail-CLOSED**
  (`preflight.ts:150-167`: `otpSatisfied=false` can never flip to clean; `:297` needs `phoneForSignals`).
  Real risk = a phone-less customer at an OTP-required venue is **permanently locked out** (self-DoS / lost
  sale), not a security hole. (Low — OTP globally dark today.)

## Top criticals to resolve first
C1 (per-phone throttle defeated), H1 (derivation seam), H2 (receiver DSAR), H3 (open-redirect/SimpleX).
