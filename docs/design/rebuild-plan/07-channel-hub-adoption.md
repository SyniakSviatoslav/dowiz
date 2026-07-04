# 07 — Channel-Hub Adoption Plan (first-adopter wave → Rust hub)

- **Date:** 2026-07-04 · **Status:** PLAN (docs-only) · **Owner lane:** hub-adoption
- **Feeds on:** `REBUILD-MAP.md` §6 (hub invariants) · `docs/research/2026-07-04-customer-distribution-channels.md`
  (channels research, "research §N" below) · `06-complete-rebuild-stack.md` (stack decision)
- **Purpose:** connect the channel-hub doctrine to the rebuild program: what ships NOW on the Node
  stack (parallel lanes today), what those lanes must honor so their artifacts survive the stack swap
  unchanged, and where every later hub element lands in the REBUILD-MAP phases.
- **Standing money rule (operator-set):** every conversational/social/agentic channel terminates in a
  signed cart-token handoff into the ONE web checkout (research §14). No channel gets a payment
  surface. 🔴 In-chat payments are per-channel council-gated, indefinitely.

---

## 1. First-adopter wave — NOW, on the current Node stack

Three heads are being built in parallel lanes today. All three are **wrappers around `/s/:slug`**;
none touches money, none needs a cart token (pure-link entry per research §14), none is blocked on
the rebuild. Each is chosen precisely because it produces **durable artifacts** (printed URLs, bot
registrations, taxonomy values, attribution rows) that carry into the Rust hub byte-identical.

### 1.1 QR/NFC + channel attribution (`?ch=` → order channel stamping)

- **What it is:** owner-side printable kit (QR PNG/PDF, A6 tent + sticker layouts, per-table
  `?t=<table>` variants) of `https://<slug>.dowiz.org?ch=qr` / `?ch=nfc`; NFC = ops, not code
  (pre-encoded NTAG213, ~€0.20-0.30/tag, research §1). Plus the attribution seam: `?ch=` read once
  at landing, persisted server-side with the cart, stamped on the order at creation.
- **Thin-head doctrine:** the purest possible head — it has **no runtime**. The "adapter" is a
  printed URL; the only code is an owner-side kit generator and one query-param stamp. It cannot
  violate the head/domain boundary because it never executes on the order path.
- **Carries into the Rust hub unchanged:** the printed URLs themselves (physical artifacts on
  tables and doors outlive both stacks — this makes URL stability a hard invariant, see §7), the
  `?ch=`/`?t=` param taxonomy, every attribution row written from today onward (values are
  canonical v1, §2), the kit PDFs.
- **Gets rebuilt:** nothing. The Astro rebuild only makes the same URL land faster (research §1:
  scan → menu ~3-5s, 0 taps).

### 1.2 Instagram as 7th messenger kind + story kit

- **What it is:** `instagram` added to the 6-kind `MessengerKind` set in
  `apps/web/src/lib/messenger.ts` (`ig.me/m/<username>` deep link — officially supported, research
  §6), plus an owner story-share kit (pre-sized story image + link-sticker how-to; stickers
  available to all accounts). Closes gap G9 / MAX-ROI #4. No DM automation, no Messaging API.
- **Thin-head doctrine:** pure link translation inside an existing, proven pattern — the head is a
  URL template + a static asset generator. Discovery stays Meta's; every tap lands on `/s/:slug`
  (`?ch=instagram`); every order lands in our system. Meta itself killed native checkout by
  2025-08, so website-checkout IS the only Meta commerce model left (research §6) — the doctrine
  and the platform reality coincide.
- **Carries into the Rust hub unchanged:** the `instagram` taxonomy value, the `ig.me/m/` link
  format, tenants' placed bio/story links (URLs in the wild), the story-kit assets and playbook.
- **Gets rebuilt:** nothing. The `MessengerKind` list ports as data into the Svelte checkout
  island; links keep resolving to the same storefront.

### 1.3 Telegram Mini App wrap (dark)

- **What it is:** register a Mini App on the already-LIVE bot (`telegram-webhook.ts`, poll
  worker); a thin entry route loads `/s/:slug?ch=telegram-tma` inside Telegram's WebView with a
  `TelegramAdapter` (telegram-web-app.js: `themeParams` → tenant palette, MainButton → "View
  cart", `initData` auth per-launch). **Dark** behind the already-reserved `TG_STOREFRONT_ACTION`
  flag (off). Honest AL caveat stands: built because it's nearly free on live infra, not because
  it's the AL growth channel (research §3, §0.2 R9).
- **Thin-head doctrine:** the TMA is literally the storefront URL in a WebView plus one adapter
  island. Adapter contains translation only (theme mapping, button proxying, launch auth);
  budgeted ≤~500 lines per REBUILD-MAP §6 so channel death is a flag flip. Cart stays
  server-side — which also neutralizes the volatile-WebView-storage problem by construction.
- **Carries into the Rust hub unchanged:** the bot + app registration and
  `t.me/<bot>/<app>?startapp=<slug>` deep links (public artifacts, stable), the `telegram-tma`
  taxonomy value, the adapter **contract** (per-launch `initData` auth, server-side cart, theme
  mapping), the TMA shell shape.
- **Gets rebuilt:** nothing structural — the React adapter is throwaway-thin by design and
  becomes one Svelte island on Astro (research §3). The URL it wraps never changes.

---

## 2. Channel taxonomy v1 — canonical `sales_channel` values

Defined NOW so attribution data collected on the Node stack is forward-compatible with the S5
`sales_channel` entity. Stored as lowercase-kebab text.

```
web-direct | qr | nfc | gbp | apple-maps | instagram | facebook |
whatsapp | telegram-tma | kiosk | widget | agent | other
```

| Value | Enters via | Status today |
|---|---|---|
| `web-direct` | typed/shared URL, subdomain, custom domain — no `?ch=` present (default) | live surface |
| `qr` | printed QR (`?ch=qr`, optional `?t=`) | wave 1 |
| `nfc` | NFC tag tap (`?ch=nfc`) | wave 1 |
| `gbp` | Google Business Profile ordering link (`?ch=gbp`) | playbook (research §2) |
| `apple-maps` | Apple Business "Order Food" action link (`?ch=apple-maps`) | playbook (research §2) |
| `instagram` | ig.me / bio / story sticker (`?ch=instagram`) | wave 1 |
| `facebook` | FB page CTA / Messenger link (`?ch=facebook`) | playbook |
| `whatsapp` | wa.me free-tier path (`?ch=whatsapp`) | playbook (research §5) |
| `telegram-tma` | Mini App entry route | wave 1 (dark) |
| `kiosk` | `?kiosk=1` counter mode (implies `kiosk`) | Tier 2 (research §10) |
| `widget` | embedded widget navigation (loader sets it) | built, productization T2 |
| `agent` | MCP/UCP cart-token mint | Phase C wave 1 stub |
| `other` | anything unrecognized | catch-all |

**Attribution rules (bind today's lanes):**
1. `?ch=` is read **once at landing**, persisted server-side with the cart/session, stamped on
   the order at creation. **First-touch wins** per cart; later navigation never overwrites.
2. Unrecognized or absent value → `web-direct` if no param, `other` if an unknown param —
   **never null**, never a free-form passthrough (no unbounded cardinality from URL tampering).
3. Attribution is **analytics-grade, not money-grade**: it never gates price, availability, or
   authz. (Per-channel price overrides are explicitly deferred — research §13.5.4.)
4. Campaign/placement granularity does NOT go into `sales_channel` — it lives in the shortlink
   layer (`dwz.al/<slug>/<tent|story|gbp>`, research §11), which resolves to a `?ch=` URL.

**Extension rules:**
- **Additive-only, immutable.** Values are never renamed, repurposed, or deleted — historical
  rows must stay interpretable forever. A dead channel's value simply stops occurring.
- A new value is added only when its head goes live **with a governance card (§5)** — channel
  count = ops budget (REBUILD-MAP §6 inv. 4). Expected future values, reserved but NOT yet valid:
  `viber` (G7-gated, research §4), `whatsapp-flows` (gated), `telegram-bot` (conversational,
  distinct from `telegram-tma`), `custom-domain` **only if** we ever decide host-based entry is
  its own channel rather than `web-direct` relocated (default: it is `web-direct`).
- On the Node stack this is a text column + app-level allowlist; at S5 it becomes the
  `sales_channel` first-class entity with the same values — data imports unchanged.

---

## 3. Cart-token spec v0 (design only — no code in this program increment)

> 🔴 **This section is a council input packet, not a build license.** The cart token feeds order
> creation → implementation goes through the **money council** (REBUILD-MAP §1 council register)
> before any code lands, on either stack.

Purpose: the ONE handoff primitive for channels that pre-build intent (bot carousels, DM flows,
agentic carts). Pure-link channels (qr/nfc/gbp/apple-maps/instagram-links) never need it — they
enter at the menu (research §14).

- **Format:** signed compact token — **JWT (EdDSA/RS256) or PASETO v4** (council picks; PASETO
  removes alg-confusion by construction, JWT reuses existing RS256 infra and crates).
- **Payload:** `{ slug, items: [{product_id, qty, note?}], channel, iat, exp, nonce }`
  - `exp − iat ≤ 15 min`. `nonce` = ≥128-bit random, single-use.
  - `channel` ∈ taxonomy v1 (§2) — stamps the resulting order.
  - **NO prices, NO totals, NO PII** in the payload — absent, not merely ignored, so nothing
    downstream can be tempted to trust them.
- **Redemption:** `/s/:slug/checkout?ct=<token>` →
  1. verify signature, `exp`, `slug` match;
  2. **server re-validates every line**: product exists, `is_available`, belongs to the slug's
     location, qty sane; **prices computed fresh from the DB** — server-authoritative pricing
     unchanged; invalid lines surface as explicit "item unavailable" diffs, never silent drops
     without display;
  3. **single-use via nonce**: first redemption records the nonce (unique constraint);
  4. **idempotent redemption**: re-presenting the same token after successful redemption returns
     the same hydrated cart (double-tap/WebView-reload safe); a nonce reused for a *different*
     redemption context → 409/410.
- **Ops:** signing key server-side only, `kid`-rotatable; token size capped (line-count cap; if
  carts outgrow URL budgets, fall back to reference form).
- **Open council question (named divergence):** research §14 sketched a *reference* token
  (opaque `cart_id`, no items); this v0 is *value-carrying* (stateless across hostile WebViews,
  no pre-checkout cart storage). The money council resolves value-vs-reference; everything else
  above (TTL, nonce, single-use, server re-validation, channel stamp) is invariant either way.

---

## 4. Rebuild-phase hooks (where each hub element lands)

| Hub element | REBUILD-MAP phase | Notes |
|---|---|---|
| `?ch=` attribution + channel stamp (text column) | **NOW** (Node lanes) | taxonomy v1 is the contract; data carries forward |
| `sales_channel` first-class entity + attribution port | **Phase B · S5** (orders/money port) | already named in REBUILD-MAP §3 S5; 🔴 money council covers it |
| Cart-token implementation | **S5-adjacent, money-council-gated** | spec v0 above is the packet; no earlier build on either stack |
| Feed heads: schema.org `Restaurant`+`hasMenu` JSON-LD, GBP/Apple link ops, Meta feed | **Phase C wave 1** | S on Astro SSR (menu already server-rendered, research §12) |
| MCP read-only menu/cart + `/.well-known/ucp` manifest stub | **Phase C wave 1** | cheap on the OpenAPI SSOT from Phase A; mirrors Shopify Storefront-MCP shape; cart tool mints the §3 token, returns checkout URL — never touches money |
| TMA wrap | **NOW (dark)** → adapter island re-hosted at storefront cutover | shell/registration/deep-links unchanged |
| Dialogue engine (ONE FSM) + per-platform conversational transports (Viber/WhatsApp/TG-bot) | **Phase C wave 2 — demand-gated (G7 survey)** | no transport is built before the 10-15-vendor survey picks it; Viber carries €100/mo/bot floor (research §4) |
| In-chat payments (any platform) | **never without a per-channel 🔴 council** | standing rule; currently also structurally blocked (ACP card-only, no Stripe in AL — research §12) |

---

## 5. Per-head governance card (template, from REBUILD-MAP §6 invariants)

Every head — including today's wave — gets one card before go-live. No card, no channel value, no
launch. Template:

```
HEAD: <name> · family: render|conversational|feed|agentic
AUTHZ PRINCIPAL: <channel principal + minimal scope — NEVER the tenant's credential>
IDEMPOTENCY KEYS: <scheme for every inbound mutation/webhook; n/a for pure-read heads — why>
RATE QUOTA: <per-channel inbound quota + burst>
KILL SWITCH: <flag name, default-off; flip = channel fully dark, zero code>
MONITORING LINE: <the ONE line surfaced in admin: feed freshness | webhook lag | injection success>
BOUNDARY: imports client contract only (dependency-cruiser now / module visibility in Rust); ≤~500 lines
COUNCIL ROW: <🔴 council + row id if the head touches money/auth/PII; else "none — why">
```

Wave-1 cards (initial fill):

| Field | qr/nfc | instagram | telegram-tma |
|---|---|---|---|
| Family | render (physical link) | render (link/social) | render (WebView) |
| Authz principal | none — anonymous public menu entry | none — anonymous public entry | TMA launch principal from `initData` HMAC; scope: session prefill only, never tenant |
| Idempotency | n/a — no inbound mutations | n/a — no inbound mutations (no DM automation in v1) | n/a in wrap v1 (no bot-side mutations); required the day a bot transport lands |
| Rate quota | storefront's existing public quotas | same | same + per-bot webhook quota (infra already live) |
| Kill switch | n/a (print is vendor-owned; `?ch=` ignore-list if abused) | messenger-kind visibility flag | `TG_STOREFRONT_ACTION` (off today) |
| Monitoring line | orders-by-`ch=qr/nfc` attribution trend | orders-by-`ch=instagram` trend | TMA launches vs orders; webhook lag (existing) |
| Council row | none — no money/auth surface | none — link-only | none for the wrap; 🔴 the moment payments/bot-ordering is proposed |

---

## 6. ROI sequencing — next 90 days (grounded in research citations)

| # | Head | Effort | Cost | Expected impact (cited) | When |
|---|---|---|---|---|---|
| 1 | QR/NFC kit + `?ch=` stamp | S | print €5-15/venue; NFC ~€0.20-0.30/tag | Lowest friction of any channel (scan→menu ~3-5s, 0 taps); immune to the R9 messenger uncertainty; demanded by trust research T4/T7 (research §1) | NOW (lanes) |
| 2 | Attribution + shortlink hub | S | ~€0 (Shlink already planned, R3) | The measurement layer that makes every other row falsifiable — attribution before investment (research §11, §13.5.2) | NOW (lanes) |
| 3 | Instagram pack (G9) | S-M | €0 | The ONLY social channel with CONFIRMED ~43% AL population reach (DataReportal via §0.2); Meta's own model is now website-checkout-only (research §6) | NOW (lanes) |
| 4 | GBP + Apple Business links | S (playbook + concierge) | €0 | Highest-intent placement in existence ("pizza afër meje"); merchant-own-link is the official path since Order-with-Google died 2024-07 (research §2) | weeks 1-4 |
| 5 | WhatsApp free-tier playbook | S | €0 | Zero-cost presence on a presumed-dominant messenger without betting engineering on the unverified claim (research §5, §0.2) | weeks 2-6 |
| 6 | TMA wrap (dark) | S-M | €0 | Nearly free on LIVE bot infra; honest weakest-AL-reach caveat — build-because-cheap (research §3) | NOW (dark) → flag decision later |
| 7 | Per-tenant PWA manifest + SW fix | S-M | €0 | Retention surface ("the vendor's app"); iOS 26 opens every home-screen site as a web app; push vertical already built server-side (research §8) | weeks 4-10 |
| 8 | JSON-LD + MCP/UCP stub | S-M | €0 | Agent-visibility hedge while UCP's "local food delivery" vertical matures; no gatekeeper, no fee, no money surface (research §12) | Phase C wave 1 (or cheap Node pre-ship if SSR shell allows) |
| — | Viber bot | M | **€100/mo/bot floor** | 🚧 GATED on G7 survey — real recurring cost vs LOW-MED regional inference (research §4) | not in the 90 days unless G7 resolves |
| — | WhatsApp Platform/Flows | M-L | ≥€49/mo + per-msg | gated on volume + re-litigating the deliberate ToS removal (research §5) | not in the 90 days |

Through-line (research §15): everything shipping in the 90 days is €0-recurring distribution of
one URL — matching 0%-commission economics; every recurring-fee or unverified-assumption channel
sits behind a named gate.

---

## 7. What today's lanes MUST honor (forward-compatibility contract)

1. **URLs are immutable artifacts.** `/s/:slug`, `<slug>.dowiz.org`, `?ch=`, `?t=`,
   `t.me/<bot>/<app>?startapp=` — printed on tables, placed in bios, registered with platforms.
   The Rust/Astro cutover must serve them byte-identically; no lane may mint a URL shape the
   rebuild won't keep.
2. **`sales_channel` values come ONLY from taxonomy v1 (§2)** — lowercase-kebab, allowlisted,
   first-touch, never-null, additive-only. This is what makes today's attribution rows importable
   into the S5 entity without a rewrite.
3. **Stamp server-side at order creation** — `?ch=` is a hint captured at landing, persisted with
   the server-side cart; analytics-grade only (never gates price/availability/authz).
4. **Heads import the client contract, never the domain** — dependency-cruiser-enforced now,
   module visibility in Rust later; adapters ≤~500 lines; every head has its governance card (§5)
   before its taxonomy value is valid.
5. **TMA rules:** authenticate per-launch from `initData`; cart server-side; nothing durable in
   the WebView; everything behind `TG_STOREFRONT_ACTION` until an explicit launch decision.
6. **Nobody touches money.** The cart-token spec (§3) is a money-council input. No lane
   implements token minting/redemption, in-chat ordering, or any payment surface — on either
   stack — ahead of its council.
