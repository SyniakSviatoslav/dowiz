# Checkout "Communication" overhaul — Council grounding brief

Owner request (checkout item 2). Serious/red-line → council before code.

## The ask
1. Rename "Messenger" → **"Communication"**; make it a **REQUIRED** selector + a **REQUIRED** adjacent input.
2. Expand kinds: keep **Viber/Telegram/WhatsApp**, add **Signal, SimpleX, Google Meet, Microsoft Teams**.
3. **Conditional input** by kind:
   - phone-kinds **(Viber, Telegram, WhatsApp, Signal)** → a **phone** input (E.164).
   - link-kinds **(SimpleX, Google Meet, Microsoft Teams)** → a **link** input (placeholder changes; validate it's a URL).
4. **Remove the standalone phone field** — phone lives inside the handle for phone-kinds.
5. Selector shows the real **logos** (not just names).
6. **"Deliver to someone else"** — a same-receiver checkbox (default CHECKED); if unchecked, a separate
   receiver contact must be specified.
7. i18n (sq/en/uk) for all new strings.

## Grounding (live code — the constraints)
- `messenger_kind` is a DB **`CHECK (messenger_kind IN ('telegram','whatsapp','viber'))`** on **customers,
  couriers, orders** (mig `1790000000038`). New kinds need a **forward-only migration** expanding all three
  (operator-gated; `ALTER TYPE` n/a — it's a CHECK, so DROP+ADD constraint). 🔴 red-line.
- **Phone is load-bearing in the order contract** (`apps/api/src/routes/orders.ts`):
  - **OTP** — `require_phone_otp` + `hashPhone` (`:121,145,155`); OTP is phone-based.
  - **Throttle** — per-phone `phone_hash` hard block (`:79,247,257`).
  - **Customer dedup** — `INSERT customers … ON CONFLICT (phone)` upsert (`:511`).
  - Courier↔customer contact + failure-fallback also assume phone.
  ⇒ "Remove phone" really means: **for phone-kinds the handle IS the phone** (extract+normalize → still feeds
  OTP/throttle/dedup); **for link-kinds there is NO phone** → OTP, per-phone throttle, and the
  `ON CONFLICT(phone)` dedup must all behave correctly with a **phone-less order** (the hard part).
- **No `receiver`/`recipient` field exists** — order create destructures only `customer` + `delivery`
  (`:92`). "Deliver to someone else" needs a new receiver model (columns vs delivery-notes).
- Order-create currently **requires phone**; must become **"communication (kind+handle) required, phone
  derived/optional"** — a Zod + server-validation contract change.
- Messenger **deep-link** today maps phone-kinds → `wa.me`/`t.me`/`viber://`. Link-kinds (Meet/Teams/SimpleX)
  are **customer-supplied URLs** shown to the courier → **open-redirect/phishing surface** (validate scheme +
  host allowlist; never auto-open untrusted).
- **Logos:** Tabler has brand-telegram/whatsapp/viber/signal; **Google Meet / Microsoft Teams / SimpleX**
  likely need vendored SVGs (brand-asset + trademark care).

## Questions for the council (the load-bearing ones)
- **Phone-less orders** (link-kinds): how do OTP, per-phone throttle, and `ON CONFLICT(phone)` dedup behave?
  (throttle → fall back to IP? OTP → skip when no phone? dedup → key on (kind,handle) when phone null?)
- **Receiver model**: new `orders.receiver_*` columns (migration) vs fold into delivery instructions? RLS?
- Is "remove phone" truly *no phone column on the order*, or *phone-as-handle + no separate UI field*?
- **Link validation/safety**: scheme + host allowlist for Meet/Teams/SimpleX; how the courier opens it safely.
- v1 scope: ship the rename + required + new phone-kinds first, link-kinds (Meet/Teams/SimpleX) behind a flag?
- Trademark/logo usage for Google Meet / MS Teams / Signal / SimpleX.

## Red-lines to respect
Forward-only operator-gated migration; RLS-FORCE on any new columns; the phone-keyed OTP/throttle/dedup must
not silently break (auth/anti-abuse surface); no open-redirect via customer links; i18n hand-translated.
