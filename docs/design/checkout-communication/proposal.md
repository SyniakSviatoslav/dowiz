# Design Proposal — Checkout "Communication" overhaul

Status: DRAFT for Triadic Council. Author: System Architect seat. Design-time only — **no production
code, no migration files** in this PR. See `docs/adr/ADR-checkout-communication.md` (decision pending).

Grounding read: `docs/design/checkout-communication/research.md`, `apps/api/src/routes/orders.ts`,
`packages/db/migrations/1790000000038_messenger-deeplink.ts`,
`packages/db/migrations/1780310074262_orders.ts` (customers DDL),
`packages/db/migrations/1780421100060_anonymization-seam.ts` (phone NULLability),
`apps/web/src/pages/client/CheckoutPage.tsx`, `apps/web/src/lib/messenger.ts`,
`apps/web/src/pages/courier/DeliveryPage.tsx`, `packages/shared-types/src/legacy.ts` (CreateOrderInput).

---

## 0. Load-bearing facts discovered in the live code (these drive every decision)

1. **`customers.phone` is NULLABLE** today. Mig `1780421100060` ran `ALTER COLUMN phone DROP NOT NULL`
   (to allow anonymization). The table still carries `UNIQUE (location_id, phone)` from mig
   `1780310074262`. **In Postgres a UNIQUE constraint treats NULLs as distinct** → `ON CONFLICT
   (location_id, phone)` (orders.ts:513) **never fires when phone is NULL**. A phone-less order therefore
   creates a *brand-new* `customers` row every single time — no dedup, unbounded row growth. This is the
   central correctness fork, not a corner case.
2. **Telegram is NOT a phone-kind.** `messengerLink()` (messenger.ts:23) maps a Telegram handle to
   `t.me/<username>` — a `@username`, never a phone. The research brief groups Telegram with the
   phone-kinds, but a Telegram username yields **no E.164**, so it behaves like a link-kind for
   OTP/throttle/dedup. Treating it as "E.164 input" would break the existing `t.me` deep-link.
   True phone-yielding kinds = **WhatsApp, Viber, Signal** (digits/E.164). Telegram = hybrid.
3. **Phone-throttle is already conditional** (`if (phoneHash)` at orders.ts:242). **IP-throttle is
   unconditional** (orders.ts:267, 20 orders/(location,IP)/15min). So the phone-less path already
   degrades to IP-only throttle — by accident, not design. We make it deliberate.
4. **OTP is globally dark** (`OTP_ENABLED='false'`, orders.ts:18) but the per-location
   `require_phone_otp` toggle and the hashed-phone OTP machinery exist. Any phone-less policy must be
   correct for the day OTP is switched on, even though it's inert now.
5. **CHECK constraints are inline-anonymous.** Mig `…038` used `ADD COLUMN … CHECK (… IN (…))` → Postgres
   auto-named them `customers_messenger_kind_check`, `couriers_messenger_kind_check`,
   `orders_customer_messenger_kind_check`. Expansion must DROP those exact auto-names then ADD named
   constraints (so the *next* expansion is clean).
6. **No receiver/recipient field exists.** Order-create destructures only `customer` + `delivery`
   (orders.ts:92). `orders.customer_id` is already nullable (anonymous orders), so a phone-less /
   recipient-only order is representable without new nullability work on the FK.

---

## 1. Problem + non-goals

**Problem.** Checkout's contact model is "phone (required) + optional messenger (Telegram/WhatsApp/Viber)".
The owner wants contact reframed as a **required "Communication" channel** (kind + handle), more kinds
(add Signal, SimpleX, Google Meet, MS Teams), conditional input (phone vs link), the standalone phone
field removed, real brand logos, and a "deliver to someone else" recipient. The hard part is that phone is
**load-bearing for three independent subsystems** (OTP auth, velocity anti-abuse, customer dedup/CRM) and
some of the new kinds carry **no phone at all**.

**Non-goals (this proposal):**
- Not changing the OTP *mechanism* (hash, session, single-use) — only its *applicability* when phone is null.
- Not building an in-app chat for link-kinds; the courier still opens the customer's link externally.
- Not migrating historical orders' contact channel; forward-only, new orders only.
- Not redesigning the courier delivery screen beyond the "open link safely" affordance.
- Not enabling OTP (`OTP_ENABLED` stays false); we only ensure the phone-less policy is *correct* for it.

---

## 2. Back-of-envelope (sizing the blast radius, not new infra)

This is a **schema + contract + FE** change, not a capacity change. Sizing confirms it needs **zero new
runtime topology** ("schema rich, runtime minimal").

- Scale today: tens of locations, peak ~ single-digit orders/min/location (memory: demo + a handful of
  live venues). Order-create is one short write-transaction (`statement_timeout=4500ms`, orders.ts:114).
- New columns: at most **+4 on `orders`** (`receiver_name`, `receiver_messenger_kind`,
  `receiver_messenger_handle`, `receiver_phone`) + CHECK expansion on **3 existing** columns. No new tables.
- Connection budget unchanged: still one connection per order-create from the operational pool
  (max:8, packages/db/src/index.ts). No new worker, no new queue topic, no analytics fan-out. Migrations
  run via the operator-gated path; the CHECK swap is metadata + a one-pass validation scan over
  `customers`/`couriers`/`orders` (small tables) — fast, but **operator-gated** because it touches
  red-line `packages/db/migrations/`.
- Write amplification: the existing single-statement customer upsert may become a *branch* (upsert when
  phone present, skip when null) — same or fewer statements, never more. No N+1.

Conclusion: no horizontal scaling gate, no new pool. The risk is **correctness** (phone-less subsystems),
not throughput.

---

## 3. Options (≥2 with tradeoffs)

### 3A. The receiver model — columns vs notes fold

| Option | Concept | Tradeoffs |
|---|---|---|
| **A1: dedicated `orders.receiver_*` columns** (recommended) | Normalized per-order snapshot of the recipient's name + channel + (optional) phone. | + Courier gets a *structured* "call/message the receiver" button (tel:/deep-link), same as the customer rail. + Cleanly RLS'd (inherit `orders` tenant_isolation, ENABLE+FORCE+grant-mirror). + Phase-5 anonymizer can null specific columns (it operates per-column). − 4 new nullable columns; a forward-only migration. |
| **A2: fold into `delivery_instructions` free text** | Append "Deliver to: <name> <contact>" to the existing courier-visible notes string (orders.ts:519). | + Zero schema change. − Unstructured → courier cannot deep-link/`tel:` the receiver, only read it. − Pollutes the "how to find you" field the FE already crams two things into (notes + dropoff chip). − Anonymizer can't surgically erase a receiver's PII embedded in free text → GDPR hazard. − No validation (E.164/URL allowlist) possible on free text. |

**Decision: A1.** A receiver is a *per-order fact* (a gift recipient, a colleague), not a reusable identity,
so it belongs on `orders`, not `customers`. The deciding factor is the **GDPR/anonymizer red line**: PII
folded into free text cannot be erased per-column; dedicated columns can. Cost is 4 nullable columns.

### 3B. The phone-less dedup behavior — `ON CONFLICT(phone)` with NULL

| Option | Concept | Tradeoffs |
|---|---|---|
| **B1: phone-less orders create NO customer row** (recommended for v1) | `resolvedCustomerId = NULL` (anonymous-order path already exists, orders.ts:521). Contact lives only on the order's snapshot columns. | + Single-statement hot path preserved; no new unique index; no NULL-conflict surprise. + Matches existing anonymous handling. − No CRM dedup for repeat link-kind/Telegram-username customers (accepted risk — these are the minority and have no stable phone key anyway). |
| **B2: partial unique on `(location_id, messenger_kind, messenger_handle) WHERE phone IS NULL` + second upsert branch** | A second `ON CONFLICT` target keyed on the handle when phone is null. | + Preserves CRM dedup for handle-stable kinds. − Two conflict targets can't share one statement → the upsert forks into two code paths. − A handle is mutable/typo-prone → weaker dedup key than a phone; risks both false-merge and false-split. − New index on a low-cardinality nullable combo. |

**Decision: B1 for v1, B2 deferred behind a flag.** The NULL-distinct UNIQUE behavior is exactly why a
naive "just let it conflict" is wrong. B1 sidesteps it by not minting a customer when there's nothing to
dedup on. CRM-by-handle (B2) is a real feature but a separate, lower-priority decision.

### 3C. v1 scope — big-bang vs phased

| Option | Concept | Tradeoffs |
|---|---|---|
| **C1: ship all 7 kinds + remove phone at once** | One release. | − Largest phone-less surface live immediately (OTP/throttle/CRM all degraded for link-kinds before the policy is proven). − Trademark clearance for 4 logos blocks the whole release. |
| **C2: phased — rename + required + phone-yielding kinds first; link-kinds behind a flag** (recommended) | Ship the rename, the required selector, and WhatsApp/Viber/Signal (handle→phone, everything keeps working). Telegram stays as today (username). Link-kinds (SimpleX/Meet/Teams) ship dark behind `COMM_LINK_KINDS_ENABLED`. | + The phone-less path lands behind a flag we can validate before launch. + Trademark clearance for Meet/Teams/SimpleX/Viber doesn't block v1's core. + Schema is expanded once (all 7 kinds in the CHECK) so no second migration — "schema rich, runtime minimal". − Two-step launch. |

**Decision: C2.** Expand the schema to all 7 kinds now (one migration), gate the *runtime exposure* of the
phone-less link-kinds behind a flag. This is the project's standing pattern (seams into schema, runtime off).

---

## 4. Decision + rationale (ADR summary — full stub in docs/adr/)

1. **Rename** "Messenger (optional)" → **"Communication" (required)**: a required kind selector + a required
   adjacent input. (FE + i18n + the order-create contract.)
2. **Expand `messenger_kind`** CHECK to `('telegram','whatsapp','viber','signal','simplex','google_meet','ms_teams')`
   across `customers.messenger_kind`, `couriers.messenger_kind`, `orders.customer_messenger_kind` — one
   forward-only, operator-gated migration (DROP auto-named CHECK → ADD named CHECK). Same set on the new
   `orders.receiver_messenger_kind`.
3. **Receiver = dedicated columns** (3B/A1): `orders.receiver_name`, `orders.receiver_messenger_kind`,
   `orders.receiver_messenger_handle`, `orders.receiver_phone` — all nullable, RLS-FORCE inherited,
   anonymizer-covered.
4. **Phone-less orders create no customer row** (B1); contact snapshot rides on the order.
5. **Phased rollout** (C2): phone-yielding kinds + rename in v1; link-kinds dark behind a flag.
6. **Phone is derived, not removed from the data model.** The *standalone UI field* goes away; a derived
   `phone` still flows into OTP/throttle/dedup **whenever the chosen kind yields one**. Phone-less is an
   explicit, handled state, not silent breakage.

**Why not "just remove phone"?** Phone underpins three subsystems. Removing it for kinds that *can* supply
it (WhatsApp/Viber/Signal) is safe — we extract it from the handle. Removing it for kinds that *can't*
(Telegram-username, link-kinds) creates a phone-less order; we make each subsystem behave correctly in that
state rather than silently no-op. See §6/§7.

---

## 5. Data / migrations (forward-only, operator-gated, RLS FORCE, integer-safe)

**Migration is design-described only — NOT written in this PR.** It is a single forward-only migration,
flagged 🔴 red-line (`packages/db/migrations/`), operator-gated (staging DB first, boot-guard FATAL on drift).

**5.1 CHECK expansion (3 existing columns).** For each of `customers.messenger_kind`,
`couriers.messenger_kind`, `orders.customer_messenger_kind`:
```
-- design intent (not committed code):
ALTER TABLE <t> DROP CONSTRAINT IF EXISTS <t>_<col>_check;     -- the auto-named inline CHECK from mig …038
ALTER TABLE <t> ADD CONSTRAINT <t>_<col>_kind_chk
  CHECK (<col> IN ('telegram','whatsapp','viber','signal','simplex','google_meet','ms_teams'));
```
- `DROP … IF EXISTS` keeps it idempotent. Re-add **named** so the next expansion is a clean rename, not an
  auto-name hunt.
- Validation scan is cheap (small tables) but still operator-gated.

**5.2 Receiver columns (`orders`).**
```
ALTER TABLE orders
  ADD COLUMN IF NOT EXISTS receiver_name             text,
  ADD COLUMN IF NOT EXISTS receiver_messenger_kind   text CHECK (receiver_messenger_kind IN (<7 kinds>)),
  ADD COLUMN IF NOT EXISTS receiver_messenger_handle text,
  ADD COLUMN IF NOT EXISTS receiver_phone            text;
```
- All nullable; default NULL = "deliver to me" (the same-receiver checkbox is checked).
- **RLS:** `orders` already has `tenant_isolation` ENABLE+FORCE; columns inherit it — **no new policy
  needed**, but the migration must NOT create the table fresh (it's ALTER, so FORCE state is untouched).
  Reviewer check: confirm `orders` is ENABLE+FORCE'd and grant-mirrored (verify:rls gate).
- **Anonymizer (Phase-5):** the receiver columns + `receiver_phone` are PII → must be added to the
  per-column anonymization set (alongside `customers.phone`, `customer_messenger_handle`). This is a
  **required follow-up**, not optional — a receiver's phone left un-nulled is a GDPR regression.
- **Money:** N/A — no monetary columns touched (integer-money invariant unaffected).

**5.3 Down:** no-op (forward-only discipline, matches mig …038).

**5.4 Contract change (`CreateOrderInput`, packages/shared-types).**
- `customer.messenger_kind`: `z.enum([...7])` (was 3).
- `customer.messenger_handle`: still `min(1).max(120)`, now **required when a kind is present** (the pair
  is jointly required at the form + server level; see §3 validation).
- `customer.phone`: stays `optional()` at the schema (the *derived* phone may be absent for link-kinds);
  **server makes it conditionally-required by kind** (phone-yielding kind ⇒ a valid E.164 must be
  derivable; else 400). Keeping the Zod field optional but enforcing the rule server-side avoids a brittle
  cross-field Zod refine that the FE can't easily mirror, and keeps the server authoritative.
- New optional `receiver` object: `{ name, messenger_kind, messenger_handle, phone }`, `.strict()`,
  present only when "deliver to someone else" is unchecked.

---

## 6. Consistency + idempotency (the phone-less fork, resolved per subsystem)

The single rule: **derive `phone` from the chosen channel; if the channel yields a phone, every existing
subsystem works unchanged; if it doesn't, each subsystem has a defined phone-less behavior.**

Kind → phone derivation:
- **whatsapp, viber:** handle is digits → strip to E.164 → `phone`. (Mirrors messenger.ts:28/32.)
- **signal:** input is E.164 → `phone`. Deep-link `https://signal.me/#p/<E164>`. (See NEEDS-HUMAN: Signal
  username links exist; v1 = phone only.)
- **telegram:** handle is `@username` → **no phone** (phone-less unless the customer also typed a number;
  see §4 NEEDS-HUMAN). Behaves as link-kind for the three subsystems.
- **simplex, google_meet, ms_teams:** URL → **no phone** (phone-less).

| Subsystem | Phone present (whatsapp/viber/signal) | Phone-less (telegram-username / link-kinds) |
|---|---|---|
| **OTP** (`require_phone_otp`) | Unchanged: hashPhone(phone) → OTP session. | **No SMS target → OTP cannot run.** Policy: **if `require_phone_otp` is ON, the storefront must NOT offer phone-less kinds** (only phone-yielding kinds selectable) — otherwise the order silently bypasses the gate. Since OTP is globally dark today this is dark too, but the gate must be coded now. (NEEDS-HUMAN: confirm "block phone-less when OTP required" vs "force a fallback phone".) |
| **Per-phone throttle** (orders.ts:242) | Unchanged: `phoneHash` computed → 5/(loc,phone)/15min. | `phoneHash` undefined → per-phone throttle **skips** (already the code's behavior). **IP-throttle (orders.ts:267, 20/(loc,IP)/15min) remains the floor** — deliberate, documented. Optional hardening: when phone is null, compute `contactHash = sha256(kind+':'+normalizedHandle)` and store it in the existing `velocity_events.phone_hash` slot (it's an opaque hash; no schema change) to bound per-identity abuse for link-kinds too. Recommended but flag-gated with the link-kinds. |
| **Customer dedup** `ON CONFLICT(location_id,phone)` (orders.ts:513) | Unchanged: upsert. | **B1: no customer row** — `resolvedCustomerId = NULL`, contact snapshot lives on the order. (Avoids the NULL-distinct UNIQUE that would mint a new row every time.) |
| **Idempotency** (`buildRequestHash`, idempotency_key) | Unchanged — independent of phone. | Unchanged — request-hash + idempotency_key already don't depend on a customer row. |
| **Courier contact** | `tel:` + deep-link both work. | No `tel:`; deep-link/link only. Courier "Call" button hidden when no phone (FE must not render `tel:` with empty/undefined — DeliveryPage.tsx:459 currently assumes a phone). |

**Money:** untouched. **Server authority:** the server re-derives the phone from the handle and re-runs all
gates server-side — it never trusts a client-sent `phone` that contradicts the handle.

---

## 7. Failures + degradation (failure-first; every external call timeout+fallback, zero cascade)

- **Link-kind deep-link is a customer-supplied URL → open-redirect / phishing surface.** This is the new
  external-trust boundary. Mitigation (server-side, authoritative):
  - Validate at order-create: link-kind handle must parse as a URL with `scheme === 'https'` (except
    SimpleX which uses `https://simplex.chat/...` invite links — confirm scheme) **and host in an
    allowlist**: `signal.me`, `simplex.chat`, `meet.google.com`, `teams.microsoft.com` (+ `teams.live.com`?
    NEEDS-HUMAN). Reject (400) anything else. Never store an un-validated URL.
  - **Courier never auto-opens** an untrusted link. The "Message" button (DeliveryPage.tsx:464) already
    uses `rel="noopener noreferrer"` + explicit tap — keep that, and additionally render link-kinds as a
    *visible href the courier chooses to open*, never a redirect/auto-navigate. No `window.open` on load.
  - The deep-link builder (`messenger.ts`) must **return null for any kind+handle that fails the
    allowlist** so the button simply doesn't render (fail-closed), matching its current null contract.
- **OTP unavailable (no phone):** not a 5xx — it's a *policy* outcome. If a location requires OTP and the
  customer picked a phone-less kind, the FE blocks selection (no silent bypass). No cascade.
- **Migration failure (operator):** boot-guard FATAL on drift (existing). The CHECK swap is reversible by
  re-adding the old constraint manually if validation fails on dirty data — but we expect no dirty data
  (only the 3 legacy kinds exist). Operator runs on staging DB first.
- **Anonymizer not yet updated:** if the receiver columns ship before the anonymizer covers them, that's a
  PII-retention regression → **the migration and the anonymizer update must ship together** (gate).
- **No new external dependency** is introduced (no SMS provider, no link-preview fetch). Degradation
  surface is bounded to "button doesn't render".

---

## 8. Security + tenant isolation

- **RLS:** new `orders.receiver_*` columns inherit `orders` tenant_isolation (ENABLE+FORCE+grant-mirror).
  Migration is ALTER (not CREATE) → FORCE state preserved. Reviewer asserts via `verify:rls`.
- **No new PII egress to AI:** receiver/handle/phone are menu-unrelated PII; they must never enter the
  menu-only AI path (claim-check / anonymizer rules unchanged; add receiver columns to the redaction set).
- **Open-redirect:** §7 allowlist is the control. Scheme + host both checked server-side; client validation
  is UX-only, never authoritative.
- **Anti-abuse:** phone-less orders keep the IP-throttle floor; optional contactHash hardening (§6).
- **Logging:** handles/links are PII → must be redacted in logs the same way phone/WS-token are (memory:
  WS-token log-redaction P1). Reviewer checks the order-create log lines don't dump `messenger_handle`/URL.
- **No secrets, no cookies, JWT RS256 unchanged.**

---

## 9. Operability

- **Health:** no new dependency → no new health signal. The flag `COMM_LINK_KINDS_ENABLED` is the only new
  operational knob (default OFF). Scaling-gate: none (sizing §2).
- **Observability (<1 min):** add a counter dimension on order-create for `communication_kind` and a
  `phone_less=true|false` boolean so we can see the phone-less share before/after enabling link-kinds.
  Surface "phone-less orders that hit an OTP-required location" as an alert (should be zero by policy).
- **Rollback:** flag OFF instantly hides link-kinds (FE + server reject). The schema (7 kinds in CHECK)
  stays — harmless, since the FE won't emit the dark kinds and the server rejects them when flagged off.
  Forward-only migration is not rolled back; the runtime is.
- **Flag/scaling-gate:** `COMM_LINK_KINDS_ENABLED` (default off) gates SimpleX/Meet/Teams end-to-end
  (FE selector options + server enum acceptance + allowlist). Telegram/WhatsApp/Viber/Signal ship live.

---

## 10. FE design (Communication selector, conditional input, receiver)

- **Selector** replaces the 3-option `<Select>` (CheckoutPage.tsx:790). Required. Shows brand logos:
  - **Tabler-available:** `brand-telegram`, `brand-whatsapp` (confirm `brand-signal` exists in the pinned
    Tabler version; if not, vendor it).
  - **Vendored SVG (brand-asset + trademark care):** **Viber, Google Meet, Microsoft Teams, SimpleX** —
    Tabler core does not reliably carry these. Vendored under the project's icon assets, monochrome-able,
    with a documented trademark-usage note (NEEDS-HUMAN, §11).
- **Conditional input** by kind:
  - phone-yielding (whatsapp/viber/signal) → phone input, `inputMode="tel"`, placeholder `+355 6X XXX XXXX`,
    normalized via the existing `normalizeAlbanianPhone` + `PHONE_E164_REGEX`.
  - telegram → `@username` (or phone) — placeholder `@username`.
  - link-kinds → URL input, placeholder per kind (`https://meet.google.com/…`, `https://teams.microsoft.com/…`,
    `https://simplex.chat/…`), client-side scheme+host check mirroring the server allowlist (server is
    authoritative).
- **Required validation:** kind chosen ⇒ handle required and valid-for-kind (block submit, inline error,
  matching the existing `phoneError` pattern at CheckoutPage.tsx:782-784).
- **Remove the standalone phone field** (CheckoutPage.tsx:778-785) — its role is absorbed by the
  Communication input for phone-yielding kinds; for phone-less kinds the order is phone-less (§6).
- **Same-receiver checkbox** (default **checked**) under contact. Unchecked → reveal receiver fields:
  receiver name (required) + receiver Communication (same selector component, required). Maps to the new
  `receiver` object in the contract (§5.4).
- **i18n (sq/en/uk), hand-translated**, via `scripts/i18n-add.ts` into the key-major catalog (never edit
  derived messages; parity gate enforces all three). New keys (illustrative):
  `checkout.communication`, `checkout.communication_required`, `checkout.communication_kind`,
  `checkout.communication_handle_label`, `checkout.communication_link_placeholder`,
  `checkout.communication_link_invalid`, `checkout.deliver_to_someone`, `checkout.receiver_name`,
  `checkout.receiver_name_required`, `courier.open_link` (+ per-kind labels reuse `messengerLabel`).

---

## 11. Deeplink / notification rendering

- **`messenger.ts` extends** `MessengerKind` to 7 and `messengerLink()`:
  - whatsapp → `wa.me/<digits>` (unchanged), viber → `viber://chat?number=+<digits>` (unchanged),
    telegram → `t.me/<username>` (unchanged).
  - signal → `https://signal.me/#p/<E164>`.
  - simplex/google_meet/ms_teams → the stored URL **only if it passes the host allowlist**, else `null`
    (fail-closed → button hidden). This is the same null-contract the courier UI already depends on.
- **Courier UI (DeliveryPage.tsx:459-470):**
  - "Call" (`tel:`) renders **only when a phone exists** (today it assumes `task.customer.phone`; must
    guard against null for phone-less orders — currently `tel:` + empty = broken affordance).
  - "Message" renders the deep-link/link via the null-safe builder, keeping `target="_blank"
    rel="noopener noreferrer"` and explicit tap (no auto-open).
  - If a **receiver** is present, the courier rail shows the receiver's contact (name + the same
    call/message affordances) in addition to (or instead of) the customer's — UX decision for the FE seat;
    contract supports both.
- **Telegram-notify path (owner notifications):** the owner's order-notification includes the customer's
  communication channel. For phone-less orders the notification must not template an empty phone; it shows
  the kind+handle/link instead. Low blast radius (notification body only); no contract break.

---

## 12. v1 scope recommendation

**Ship in v1 (live):** rename → "Communication" required; selector + required input; **WhatsApp, Viber,
Signal** (handle→phone, all subsystems intact) + **Telegram** (unchanged username behavior); remove the
standalone phone field (phone derived); the same-receiver checkbox + receiver columns; full i18n.

**Ship dark behind `COMM_LINK_KINDS_ENABLED` (default off):** **SimpleX, Google Meet, MS Teams** (the
phone-less link-kinds) + the URL allowlist validation + the optional contactHash throttle hardening.

**Schema expands to all 7 kinds now** (one migration) so enabling the flag is runtime-only.

---

## 13. Open / accepted risks + NEEDS-HUMAN

| # | Item | Class | Owner |
|---|---|---|---|
| R1 | **Trademark/logo usage** for Google Meet, MS Teams, Signal, SimpleX, Viber (brand-asset guidelines, monochrome rendering rights). | NEEDS-HUMAN | Owner / legal |
| R2 | **OTP-required + phone-less kind**: block selection (recommended) vs force a fallback phone. OTP is dark today but policy must be set. | NEEDS-HUMAN | Council + owner |
| R3 | **Telegram is not a phone-kind** — username yields no phone. Accept phone-less Telegram, or require a phone alongside the username? | NEEDS-HUMAN | Council |
| R4 | **CRM dedup loss** for phone-less repeat customers (B1). Accepted for v1; B2 (handle dedup) deferred. | Accepted risk | Architect |
| R5 | **Removing the standalone phone field** means phone-less orders lose the courier "Call" channel for Telegram-username + link-kinds. Confirm the owner accepts no-phone-callable orders, or keep an optional phone field. | NEEDS-HUMAN | Owner |
| R6 | **Anonymizer must cover** `receiver_*` columns in the same release — else GDPR regression. Hard gate, not optional. | Must-fix gate | Backend seat |
| R7 | **SimpleX/Signal link scheme/host** exact allowlist values (e.g. `teams.live.com`, SimpleX `https://simplex.chat` vs custom-server hosts). | NEEDS-HUMAN | Architect + research |
| R8 | **IP-throttle as the only floor** for phone-less orders is weaker than per-phone. contactHash hardening (§6) is recommended; accept the gap if deferred. | Accepted-risk-if-deferred | Architect |
