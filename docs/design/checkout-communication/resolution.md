# Checkout "Communication" overhaul — Council Resolution (synthesis)

**Seats:** Architect (`proposal.md` + `docs/adr/ADR-checkout-communication.md` DRAFT) · Breaker
(`breaker-findings.md`, 8 findings) · Counsel (`counsel-opinion.md`, 2 ETHICAL-STOPs). Brief: `research.md`.

## Verdict — NOT BUILD-READY, but one fix dominates the criticals
The intent is good and the Architect's data findings are sharp (phone is NULLABLE under a UNIQUE that never
conflicts → phone-less orders make no customer row; Telegram is a *username* kind, not phone). **But removing
the standalone phone field is the root of the worst findings** — and Counsel's fix dissolves them at once.

### The load-bearing resolution: **Phone (call/SMS) is a first-class Communication kind, and the phone field STAYS.**
- Kills **C1** (the Fastify `keyGenerator` keeps seeing `customer.phone` → per-phone throttle intact).
- Kills **H1** (no fragile handle→phone derivation across 3 read-sites; phone-yielding kinds keep a real phone).
- Discharges **Counsel STOP-1** (courier keeps a `tel:` channel; phone-only/low-tech/elderly customers are not
  excluded — Charter "serve everyone").
- Dissolves the entire "remove the phone field" tension. So: **"Communication" = required channel selector;
  Phone is one of the kinds; the phone input is NOT removed** — it's the Phone kind's input and the universal
  fallback. WhatsApp/Viber/Signal carry a phone handle (feed throttle/dedup); Telegram = username (`t.me`).

## Dispositions
| # | Finding | Disposition |
|---|---------|-------------|
| **C1** per-phone throttle defeated | **FIXED by Phone-first-class + keep phone field** (above). |
| **H1** derivation seam | **DISSOLVED** — phone-kinds keep a phone column; no cross-site derivation. |
| **H2** receiver DSAR / non-consent | **DESCOPE or harden:** v1 either drops "deliver to someone else", OR stores receiver contact with (a) a privacy **notice** on the form, (b) a `receiver_phone`-keyed erasure path + index, (c) **anonymize-on-delivery** (not the 365-day clock). Recommend **defer receiver to v1.1** with a proper consent/DSAR design — it's the least-baked piece. NEEDS-HUMAN. |
| **H3** open-redirect / SimpleX | **Link-kinds need exact-host equality** (`new URL(h).host === allowed && protocol==='https:'`, reject userinfo/IDN), server-authoritative, + courier **never auto-opens** (show as copyable text w/ "unverified link" notice). **SimpleX self-hosting can't be allowlisted → drop SimpleX from v1** (or text-only, no clickable open). |
| **M1** CHECK auto-name unverified | Resolve-round must **read `pg_constraint`** on prod/staging/dev for the real names before DROP; the migration drops by-verified-name then adds **explicitly-named** constraints. |
| **M2** deploy-ordering (Signal live) | **Migration lands BEFORE the code** (operator-gated, like all migs); OR gate Signal behind the same flag until the enum is applied everywhere. No live kind may outrun its CHECK. |
| **M3** Telegram ambiguity | Telegram = **username-only** (no phone for it); if the customer wants a call they pick **Phone**. Placeholder/validation per kind; no `t.me/<phone>` dead links. |
| **L1** OTP framing | Corrected: OTP is **fail-closed** (`preflight.ts:150-167`), not a silent bypass. Real risk = phone-less lockout at OTP-venues → block phone-less selection when `require_phone_otp` (with Phone-first-class this is moot — pick Phone). |

## Counsel — carried (friction, not veto)
- **STOP-1** discharged by Phone-first-class.
- **STOP-2 (receiver consent/minimization):** notice copy + anonymize-on-delivery + minimal retention — folded
  into H2's disposition.
- **Strategy adopted:** **cut Google Meet + Microsoft Teams from v1** (video-conf is a category error for food
  delivery); the required field must **explain why** ("the courier will message you about your order") to
  avoid a dark-pattern read; keep WCAG-AA text labels under the logos; shorten the selector. Counsel's unasked
  question recorded: *the channel chosen at order ≠ the one answered during delivery* — prefer reachable
  channels (Phone/WhatsApp) in ordering/copy.

## v1 scope (recommended)
- **Selector kinds (live):** **Phone (call/SMS)**, WhatsApp, Viber, Telegram, Signal. Rename → "Communication",
  required, with logos + a "why" line. Phone field stays (Phone kind + universal fallback).
- **Dark / deferred:** SimpleX (link-validation unsolved for self-host), Google Meet + MS Teams (category
  error — cut), and the **receiver "deliver to someone else"** (defer to v1.1 with consent/DSAR design).
- **Migration:** expand `messenger_kind` to the live set (+signal, +the deferred ones for schema-rich) with
  **verified** constraint names, forward-only, operator-applied **before** the code.

## NEEDS-HUMAN (gates the resolve→build)
- Keep the phone field + Phone-first-class? (recommend **yes** — it's the whole fix.)
- Receiver "deliver to someone else": defer to v1.1 (recommend) or ship with consent/DSAR hardening now?
- SimpleX in v1 (text-only) or drop? Meet/Teams confirmed cut?
- Trademark/logo usage rights for WhatsApp/Viber/Telegram/Signal (+SimpleX if kept).
- OTP-required + phone-less policy confirm (moot if Phone-first-class).

## Recommendation
Adopt **Phone-first-class + keep the phone field** (the dominant fix), **cut Meet/Teams**, **defer receiver +
SimpleX**, then run **one RESOLVE round** that pins H3 (host-equality + no-auto-open), M1 (verified constraint
names), M2 (migration-before-code), M3 (Telegram=username) → flip the ADR DRAFT→APPROVED. Build behind a flag,
migration operator-applied first. **No code until APPROVED.**
