# Layer Blueprint — Channel Adapters / Multichannel Ingestion — 2026-07-11

> **The hub's defining feature: many funnels → one `kernel::decide` door.** This blueprint specs the
> channel-adapter layer so the hub genuinely accepts orders from many funnels AT ONCE, each as a thin
> adapter into the ONE door — without ever growing a second money surface.
>
> **Method:** read-only design session. Repos untouched (a parallel session is changing code; all
> file:line anchors are as-read 2026-07-11 and may drift). The only file created is this one. Fresh
> web research on 2026 messenger bot APIs performed and cited (§Sources). Labels: **VERIFIED** (code/
> primary read this session) · **VERIFIED-in-repo** (a cited repo doc verified it, not re-checked) ·
> **VERIFIED-secondary** (reputable web secondary) · **UNVERIFIED** · **DESIGN** (this doc's judgment,
> falsifiable by building).
>
> **Standing decisions honored throughout (binding):** local-first ratified; **COD mandatory** (no
> in-chat payments, ever — council-gated indefinitely); **NO courier scoring** (attribution never
> touches courier metrics); anonymity a value (hub-guarantee + per-channel pass-through, doc 04);
> **multichannel / NO dedicated app**; hybrid-only crypto (G09); **storefront sovereignty** (`/s/:slug`
> is one channel but its branding is the vendor's).
>
> **Grounds:** hub architecture review `docs/research/2026-07-11-hub-architecture-review.md` (§2
> scorecard, §3.1–3.3, §5.4, §6); G03 `docs/design/gap-blueprints-2026-07-11/G03-checkout-422-
> messenger-kinds.md`; `SYNTHESIS.md` + `04-anonymity-mesh-messenger-revision.md` (this dir);
> `docs/research/2026-07-11-MAX-EV-SYNTHESIS.md`; EXPANSION-PLAN §1.C; live channel code (all read
> this session): `apps/api/src/lib/channel.ts`, `apps/web/src/lib/{channel,messenger,tma}.ts`,
> `apps/api/src/routes/telegram-webhook.ts`, `rebuild/crates/api/src/modules/channel_attribution/mod.rs`,
> migrations `1780350000000/1`.

---

## 0. Ground truth — the honest baseline this layer builds on

From the hub review §2 (VERIFIED there, spot-re-verified here):

- **Prod has exactly ONE order intake**: `POST /api/orders` (`order-persistence.ts:79`). Every
  "channel" is a wrapper around that door, differentiated only by the write-only `x-channel` header.
  "Many order sources" is **attribution-true, transport-false** today.
- **Attribution capture is live end-to-end** (`?ch=` → per-slug sessionStorage → `x-channel` →
  `orders.metadata.channel`, normalized by a never-throwing 13-value allowlist — VERIFIED
  `apps/api/src/lib/channel.ts:19-51`, `apps/web/src/lib/channel.ts:17-87`) — **and nothing reads
  it**. Zero readers of `metadata->>'channel'` in prod (review §5.4).
- `sales_channels` + `order_events` exist in the shared DB (VERIFIED migrations `1780350000000/1`),
  have **zero Node runtime references**, and the Rust reader counts per-**kind**, not per-channel-row
  (double-counts same-kind channels; the 256-bit per-channel `token` is never consulted — review §3.3).
- The one live door **400-fails 3 of 6 advertised contact options + every receiver{} order** (G03).
- The Rust checkout **bypasses `kernel::decide`** (review §3.2.1). This layer therefore binds adapters
  to the **wire contract** (the door's HTTP surface / cart-token), never to kernel internals — so the
  layer is valid under Node-today, Rust-later, and local-first-eventually.

**The doctrine this layer implements** (EXPANSION-PLAN, REBUILD-MAP §6 — VERIFIED read): *"does this
door decide anything, or only carry? Doors carry a Command to `kernel::decide`… Doors NEVER price,
transition state, or invent money."* Adding a channel must never add a money surface, a fiscalization
point, or a PII store.

---

## 1. The adapter contract

### 1.1 First: the three vocabularies (the confusion this contract ends)

The review (§6.2.3) found three different things all called "channel." The contract names them apart;
every adapter spec below uses these words exactly:

| Vocabulary | Values | Role | Read by |
|---|---|---|---|
| **`OrderChannel`** (attribution taxonomy) | 13-value kebab allowlist (`web-direct, qr, nfc, gbp, apple-maps, instagram, facebook, whatsapp, telegram-tma, kiosk, widget, agent, other`) — VERIFIED triplicated: api lib / web lib / `sales_channels` CHECK + Rust port | **WRITE-ONLY** acquisition attribution: *which funnel produced this order* | analytics/dashboard ONLY. Never pricing, state machine, dispatch, authz, courier anything |
| **`MessengerKind`** (contact preference) | 6 values `phone, whatsapp, viber, telegram, signal, simplex` (FE, ADR-0016) vs 3 in the validator (**the G03 bug**) | *how the courier/vendor coordinates with THIS customer on THIS order* | courier deep-link rendering (`messengerLink`), receiver contact |
| **Notification target** (owner/ops alert transport) | `telegram \| push` (DB CHECK, migration `1780348982032`) | *how the VENDOR hears about orders* | notification workers only |

`whatsapp` appearing in two of these and `telegram` in all three is why the enums leak. **Contract
rule V-1:** each vocabulary gets ONE source-of-truth module in `packages/shared-types`
(`messenger.ts` per G03 Phase 4a; `channel.ts` collapsing the triplicated allowlist per the TODO
already in both copies), plus a **mechanical cross-stack parity gate in CI** (TS ↔ Rust ↔ SQL CHECK —
the 14th-value-breaks-somewhere hazard, review §6.2.2). No adapter may declare a private copy of any
vocabulary. (Gate: `packages/shared-types` is protect-paths — operator applies, per G03.)

### 1.2 Three adapter classes (and only three)

Every channel in §2 is an instance of one class. A proposed channel that fits none is refused at
design time.

**Class L — LINK adapters** (pure-link: web, QR, NFC, GBP/Apple, social links, widget, subdomain).
The adapter *is a URL*: `/s/:slug?ch=<value>` (or a short-link that 302s to it). No parsing, no
token, no per-channel auth — the customer lands on the vendor-branded storefront and the ONE checkout
does everything. Cost of a new Class-L channel ≈ printing artifacts + one attribution value.

**Class I — INTENT adapters** (conversational/bot/agent: Telegram customer bot, WhatsApp Cloud API,
Instagram DM, SimpleX bot, MCP/agent heads). The adapter runs a dialogue in the platform's idiom,
builds an **order intent = item IDs + quantities ONLY** (prices unrepresentable), then mints the
**signed cart-token** `{slug, items[], channel, iat, exp≤15min, nonce}` — no prices, no totals, no
PII (spec: `07-channel-hub-adoption.md` §3, VERIFIED-in-repo) — and hands the customer the ONE web
checkout at `/s/:slug/checkout?ct=<token>`. The server verifies sig/exp/slug, re-validates every
line, re-prices from the DB, burns the nonce. **In-chat payment is forbidden by the COD ruling and
the single-money-surface invariant — every Class-I flow terminates on the web checkout, cash on
delivery.** (This is also market-correct: Meta retired in-app checkout; Telegram exempts physical
goods from its Stars mandate — physical goods may use ordinary flows/link-out, VERIFIED
core.telegram.org/bots/payments.)

**Class M — MIRROR adapters** (the `.onion` web mirror; a future LAN/kiosk mirror). Not a new body
at all: the SAME storefront + checkout served over a different transport by the vendor node. Zero
new parsing, zero new money surface; the adapter is a listener + an attribution value.

**Contract rule C-1 (the load-bearing one):** *no adapter, of any class, ever writes an order.* The
only order-creating call in the system remains the one door (`POST /api/orders` today;
`Command::PlaceOrder → kernel::decide` when the Rust shell is made honest — review §3.2 amendment).
Class-I adapters that try to "help" by inserting rows are refused in review; the grep
`INSERT INTO orders` must keep returning exactly one non-test site per stack (the review's ground
rule — this is the layer's standing falsifiable proof).

### 1.3 The contract interface (spec — descriptive, not code)

```
ChannelAdapter (one per taxonomy value that has a transport; Class L needs only rows 1, 6-8)
  1  kind:            OrderChannel                    — exactly one allowlisted value
  2  class:           'link' | 'intent' | 'mirror'
  ── inbound ──────────────────────────────────────────────────────────────────
  3  verifyTransport(request) -> Principal | REFUSE   — platform authenticity, fail-closed:
       WhatsApp: X-Hub-Signature-256 HMAC (timing-safe) · Telegram: secret-token header +
       (TMA) initData HMAC + auth_date freshness · IG: Meta webhook signature ·
       web/QR/onion: none (anonymous principal). A failed verify is a 401 + audit line,
       NEVER a half-ingested order.
  4  parseIntent(update) -> CartIntent | REFUSE       — items (IDs from interactive list/
       button ids → SKU, never free-text auto-execute — EXPANSION-PLAN §1.C rule) + qty
       1..99 + slug. NO prices, NO totals. Malformed/partial → REFUSE whole, echo a
       channel-idiom error; nothing persisted.
  5  handoff                                          — the ONLY two legal exits:
       Class L/M: a URL into /s/:slug (?ch= stamped once at landing)
       Class I:   mintCartToken(intent) → /s/:slug/checkout?ct=…  (single-use nonce)
  ── outbound ─────────────────────────────────────────────────────────────────
  6  renderStatus(orderEvent, locale) -> Message|null — status back in the channel's idiom
       (Telegram: edit/inline message · WhatsApp: free-form/utility template inside the
       24h service window — free since the customer initiated · web/onion: the existing
       tracking page /s/:slug/order/:id · SimpleX: bot message). PII-minimal: status +
       ETA range only; never address, never other-party contact.
  ── governance card (REBUILD-MAP §6 — MANDATORY before the taxonomy value is live) ──
  7  authPrincipal · idempotencyKeys · rateQuota · killSwitch(flag, default OFF) ·
     monitoringLine (one counter: intents minted / tokens redeemed / refusals)
  8  anonymityLabel                                    — the honest pass-through label per
       doc 04 §2.2, rendered wherever the channel is offered ("Convenient, NOT anonymous —
       Telegram knows your number" / "Network-anonymous by default").
```

**Contract rule C-2 (identity mapping):** each transport's native identity (Telegram `chat_id`,
WhatsApp `wa_id`, IG-scoped user id, SimpleX queue address) is **PII**. It lives only in the
adapter's own coordination store and — for orders — in the per-order PII envelope (§3.3). It is
NEVER written to `orders.metadata`, the attribution log, or `order_events`. The one sanctioned
coupling between a channel and the contact vocabulary: an adapter MAY prefill
`customer.messenger_kind` + `handle` from its native identity (§1.5 map) — through the same
`CreateOrderInput`, subject to the same validation, erasable by the same anonymizer.

**Contract rule C-3 (storefront sovereignty):** the brand at every customer touchpoint is the
**vendor's**. Class L/M land on the vendor-themed `/s/:slug` (already per-tenant themed — VERIFIED-in-
repo). Class-I adapters must carry vendor identity in the platform's own terms: the WhatsApp number
is the *vendor's* number/WABA; a Telegram customer bot is registered *per vendor* (bot token a
`sales_channels`-row credential, §3.2), not one global dowiz bot fronting all venues; templates and
dialogue copy are vendor-named. dowiz appears, if at all, as fine print. (The existing owner-ops bot
is dowiz-operated — acceptable because its audience is the vendor, not the customer.)

**Contract rule C-4 (channel-death):** killing a channel is a flag flip + deactivating its
`sales_channels` rows. No adapter may hold state whose loss breaks orders already placed.

### 1.4 Channel-independent vs channel-specific — the split

| CHANNEL-INDEPENDENT (the spine — behind the door, identical for every funnel) | CHANNEL-SPECIFIC (the adapter — thin, ≤~500 lines/head) |
|---|---|
| `kernel::decide` / the one intake; 10-status machine; server pricing in integer Lek; tenant-scoped idempotency; anti-abuse velocity gates | transport auth (HMAC/signature/initData), webhook plumbing, platform rate-limit etiquette |
| the order record + `order_events` append | dialogue/parsing → CartIntent; interactive-list id→SKU maps |
| **attribution write** (normalized allowlist token → metadata + registry row, §3) | which attribution value + (later) which channel-row token it stamps |
| **the per-order PII envelope** (hash in the log, crypto-shred — §3.3) and the hub anonymity guarantee (no profile, COD cash, vendor-node data) | the honest anonymity **label** (pass-through, doc 04) |
| customer coordination via `MessengerKind` + courier deep-link rendering | the prefill mapping native-identity → (kind, handle) |
| notification fan-out to the vendor; tracking JWT | `renderStatus` in the channel idiom |
| GDPR erasure path (anonymizer NULLs contact/receiver fields — VERIFIED both lineages, G03 §2.1) | the adapter's own coordination store + its erasure hook |
| fiscalization (attaches to the ONE checkout only) | — (never; that is the point) |

### 1.5 The MessengerKind unification — how the enum stops leaking (G03 mapping)

The G03 fix (6-kind enum + `receiver{}` + `messenger_handle` max 500 in `legacy.ts` — operator-gated,
~15 LOC) is **entry precondition for this whole layer** (Phase 0): today the one live channel rejects
half its own contact vocabulary, so any new funnel would leak at the same step. On top of it:

- **SSOT** `packages/shared-types/src/messenger.ts` (G03 Phase 4a; the recovered IG-lane draft minus
  instagram) — FE, validator, courier surface, Rust dto import ONE list. Parity gate in CI (V-1).
- **Adapter prefill map** (the only channel→kind coupling, rule C-2):

| Adapter | Prefill `messenger_kind` | handle source | Note |
|---|---|---|---|
| web / QR / links | none — customer picks from 6 | customer-typed | as today |
| Telegram bot / TMA | `telegram` | username from initData/chat (customer-confirmed, editable) | phone-less kind — already tolerated by the door (VERIFIED-in-repo G03 §2.4) |
| WhatsApp Cloud API | `whatsapp` | `wa_id` (E.164) → feeds `customer.phone` (throttle/OTP/dedup intact) | |
| Instagram DM | `instagram` — **requires the 7th kind** (G03 Phase 4b: red-line migration 085 staged from the recovered draft; kind must never outrun its DB CHECK) | IG username | gated |
| Viber (contact deep-link) | `viber` | phone | already live as a contact kind |
| `.onion` mirror | none by default — per-order pseudonym channel; customer MAY volunteer a kind | — | phone-less path exists (telegram/simplex precedent) |
| SimpleX contact/bot | `simplex` | invite link (needs the 500-char cap — the G03 length trap, VERIFIED) | text-only, never clickable (existing `messengerLink` rule) |

**Rule:** a new intake adapter whose platform implies a contact kind not in the SSOT is BLOCKED until
the kind lands (SSOT + DB CHECK first, FE last) — the exact anti-pattern the lost IG worktree
correctly HELD on (G03 §2.2).

---

## 2. Per-channel adapter specs

### 2.0 Summary table

| # | Channel | Class | `OrderChannel` value | Anonymity (honest label, doc 04) | Status | Buildable NOW? | Effort (sessions) |
|---|---|---|---|---|---|---|---|
| 1 | Web storefront `/s/:slug` (+subdomain) | L | `web-direct` | hub guarantee ✓ · network-anon ✗ (IP to relay/ISP) | **LIVE** (G03 leak) | fix is Phase 0 | S (G03: ~15 LOC, operator gate) |
| 2 | QR / NFC kit | L | `qr` / `nfc` | same as web | plumbing LIVE; kit page DARK (`VITE_CHANNEL_KIT_ENABLED`) | **YES** — flag flip + reader | S (≤1) |
| 3a | Telegram owner bot | ops surface, not intake | — | n/a (vendor-facing) | LIVE | already | — |
| 3b | Telegram Mini App (storefront wrap) | L | `telegram-tma` | ✗ phone-bound; Telegram sees metadata | scaffolded DARK; CSP blocks `telegram-web-app.js`; Dockerfile ARG missing (VERIFIED-in-repo) | YES technically; **audience question R1 open** → demand-gate | S–M (1–2) |
| 3c | Telegram customer ordering bot | I | needs `telegram-bot` value (additive) | ✗ phone-bound | paper | NO — G7 survey + cart-token council | M–L (3–5) |
| 4 | WhatsApp Cloud API | I | `whatsapp` | ✗ Meta holds identity+metadata | paper (open-wa BANNED for prod — EXPANSION-PLAN 1.C) | NO — survey + Meta Business Verification + council | M–L (3–5 + 2–10 biz-days verification) |
| 5a | Instagram link/story kit | L | `instagram` | ✗ Meta | attribution value exists; kit absent | **YES** | S (≤1) |
| 5b | Instagram DM intake | I | `instagram` | ✗ Meta | paper | NO — App Review (weeks), 200 DM/hr cap; park behind WhatsApp | L (4+ incl. review wait) |
| 6 | Viber bot | I | needs `viber` value | ✗ phone-bound | paper | NO — €100/mo/bot floor (VERIFIED help.viber.com) | park; contact deep-link already free |
| 7 | `.onion` web mirror | M | needs `onion` value (additive) | **✓ network-anonymous by default, relay-free** | design (doc 03/04) | NO — needs the vendor node (local-first P2+) | M (2–3) once node exists |
| 8 | No-phone messenger contact (SimpleX) | contact kind now; I later | needs `simplex` value if intake | **✓** no identifiers at all | kind live in FE (G03 unblocks) | contact: YES (rides Phase 0) · bot intake: earn-it | contact S · bot M |

### 2.1 Web storefront `/s/:slug` — the anchor (Class L, LIVE)

- **Inbound flow (as-built, VERIFIED via review §3.1):** land (QR/link/direct) → menu → server-side
  cart → checkout form → `POST /api/orders` with `x-channel` — one transaction, integer money,
  idempotent, velocity-gated.
- **Contract work:** the G03 fix (Phase 0) is the only change; the channel already IS the reference
  implementation of the door.
- **Identity/anonymity:** no account, no forced phone (kind-dependent); hub guarantee applies; label:
  *"No profile, no card — but NOT network-anonymous."*
- **Sovereignty:** already vendor-themed per-tenant; subdomain `<slug>.dowiz.org` live. A
  vendor-custom-domain variant is a parked Class-L extension (no new intake).

### 2.2 QR / NFC (Class L — the max-EV first door)

- **Flow:** printed QR/NFC → `/s/:slug?ch=qr|nfc` → identical to §2.1. Attribution plumbing 100%
  live (VERIFIED); owner **QR-kit page BUILT, DARK** behind `VITE_CHANNEL_KIT_ENABLED`
  (`QRKitPage.tsx` builds the URLs — VERIFIED-in-repo).
- **Buildable now:** flip the flag in the staging/prod build + ship the attribution reader (§3.1) so
  a broken QR (typo'd `ch`) is *detectable* as an `other`-spike. Print A6 tents for venue #1 (rides
  the MAX-EV walk-in plan).
- **Upgrade path (Phase 3):** per-channel short-links `/c/<token>` → 302 → `/s/:slug?ch=qr` binding
  the scan to a `sales_channels` **row** (table QR #4 vs door sticker), enabling per-artifact counts
  and killing the per-kind double-count (§3.2). Dynamic-QR TTL nonce (EXPANSION-PLAN idea) is NOT
  needed for Class L — there is no intent to sign; the menu is public.
- **Anonymity:** as web. NFC adds no identity (tag is a passive URL).

### 2.3 Telegram — three distinct things (keep them named apart)

**(a) Owner ops bot — LIVE, not an order channel.** `/start` connect, confirm/reject callbacks,
`/open` shift (VERIFIED `telegram-webhook.ts:146-763`). Stays the vendor-alert surface; also the
cheapest home for the courier out-of-app beep (review §7 — different layer, noted as a dependency
consumer of this bot, not of this blueprint).

**(b) Mini App storefront wrap (Class L) — scaffolded, dark.** `t.me/<bot>/<app>?startapp=<slug>` →
WebView loads `/s/:slug?ch=telegram-tma` → flow §2.1. Blockers (all VERIFIED-in-repo, review §2#5):
CSP `script-src` doesn't allowlist `telegram-web-app.js`; the client-flag Dockerfile ARG is missing;
`TMA_ENABLED`/`VITE_TMA_ENABLED` default off; and the open product question R1 — as flipped it is
*owner self-preview*, not a customer distribution channel. `lib/tma.ts` (theme mapping, BackButton,
allowlisted untrusted-bridge attrs) is honest groundwork (VERIFIED read). Auth upgrade when it goes
customer-facing: `initData` HMAC-SHA256 (`secret = HMAC("WebAppData", bot_token)`) + `auth_date`
freshness verified server-side on EVERY write (EXPANSION-PLAN 1.B/1.C; Telegram now also offers
third-party Ed25519 `signature` validation — VERIFIED-secondary docs.telegram-mini-apps.com).
- **Verdict:** technically S–M; hold behind the same demand gate as (c) — the MAX-EV skip-list
  explicitly parks TMA intake. Do the CSP + Dockerfile ARG hygiene whenever the file is next touched
  (it also blocks unrelated features — EXPANSION-PLAN notes).

**(c) Customer ordering bot (Class I) — paper; the template Class-I spec.** Dialogue in chat
(inline-keyboard menu; ids→SKU; no free text execution) → CartIntent → cart-token → web checkout
link back into the chat (opens in-app browser; COD). Telegram Bot API is free; physical-goods flows
are exempt from the Stars in-app mandate, link-out is compliant (VERIFIED core.telegram.org). Webhook
auth: `X-Telegram-Bot-Api-Secret-Token` (the Rust S8 webhook is auth-gate-only today — VERIFIED-in-
repo). Per C-3, the bot identity should be per-vendor (bot token in the channel registry row).
- **Attribution:** needs an additive `telegram-bot` value — `telegram-tma` is a different funnel and
  must not be overloaded (taxonomy governance §3.5).
- **Anonymity label:** *"Convenient, NOT anonymous — Telegram requires a phone number and its servers
  see your messages (bot chats are not E2EE)."* (VERIFIED-secondary.)
- **Gate:** G7 vendor survey demand + cart-token council. Effort M–L (3–5 sessions: token mint/verify
  + dialogue FSM + governance card + ingress security review per EXPANSION-PLAN standing track).

### 2.4 WhatsApp Cloud API (Class I — the likely first messenger transport, still demand-gated)

- **Standing verdicts honored:** open-wa/wa-automate **BANNED for prod** (unofficial, 2–8wk ban
  timelines, permanent number loss, ~4GB RAM/session — EXPANSION-PLAN 1.C, VERIFIED read; allowed
  only as a throwaway prototype). WhatsApp as a *notification* channel was deliberately REMOVED on
  privacy/ToS grounds (migration `1790000000043` — VERIFIED-in-repo); this spec does not resurrect
  it — outbound here is only in-session replies to a customer who messaged first.
- **2026 platform facts (fresh research):** Meta switched to **per-message pricing 2025-07-01**;
  inside the **24h customer-service window (customer-initiated), free-form messages AND utility
  templates are FREE** — so a pure inbound ordering flow costs ≈ €0 (VERIFIED-secondary respond.io).
  Utility templates outside the window ≈ $0.008–0.012 (VERIFIED-secondary). **Business Verification**
  (legal docs via Business Manager) takes ~2–10 business days; unverified caps at 250 conversations/
  24h — fine for prototyping on a Meta test number (VERIFIED-secondary; matches EXPANSION-PLAN's
  "2–5 days" within error).
- **Inbound flow:** customer messages the vendor's WhatsApp number (from the wa.me deep-link the
  storefront already renders) → webhook (verify `X-Hub-Signature-256` HMAC-SHA256, App Secret,
  timing-safe) → interactive **list/button ids → SKU** (never free-text execution) → CartIntent →
  cart-token → checkout link in-chat → COD web checkout. Status back via free in-window replies;
  post-window status uses a *utility* template (cheap class) only if the vendor opts in.
- **Identity/anonymity:** `wa_id` = phone → prefills `whatsapp` kind and `customer.phone` (full
  throttle/OTP/dedup path). Label: *"Convenient, NOT anonymous — Meta holds your identity and
  metadata."* Hub guarantee still applies (no dowiz profile; envelope + shred).
- **Sovereignty:** the WABA + number are the **vendor's**, onboarded via the channel registry;
  dowiz never owns the customer relationship. Per-vendor verification is the real adoption cost —
  surface it in the G7 survey.
- **Gate:** G7 survey names it + money-adjacent ingress review + cart-token council. Effort M–L
  (3–5 sessions + verification wall-clock).

### 2.5 Instagram (two funnels, opposite gates)

- **(a) Link/story kit (Class L) — buildable NOW, S.** Meta retired native checkout (Sep 2025 —
  VERIFIED-in-repo research §6), so the correct door is a link: `?ch=instagram` on the bio/story
  link + an owner story-kit page (mirrors the QR kit). The `instagram` attribution value already
  exists (VERIFIED allowlist). Rides Phase 2 with zero transport, zero council.
- **(b) DM intake (Class I) — gated, park behind WhatsApp.** 2026 platform facts (fresh research,
  VERIFIED-secondary): Business/Creator account + Meta App Review for
  `instagram_business_manage_messages` (weeks; 25 test users pre-review), webhook events
  (`messages`, `messaging_postbacks`), **200 automated DMs/hr/account** + 1-DM-per-user-per-24h on
  comment/story triggers, mandatory bot disclosure, 24h response window; API itself free. Flow =
  same Class-I shape as §2.4 (ids→SKU → cart-token → checkout link). Requires the `instagram`
  MessengerKind (7th kind, red-line migration — §1.5) for coordination prefill.
- **Anonymity label:** *"Convenient, NOT anonymous — Meta holds your identity."*

### 2.6 Viber (park the bot; keep the contact kind)

- The **contact deep-link** (`viber` MessengerKind) is live and free — nothing to build.
- A Viber **bot** (Class I) carries a **€100/month/bot maintenance fee** under the commercial model
  effective 2024-02-05, plus per-message fees for bot-initiated messages (session replies to
  customer-initiated chats are free) — **VERIFIED** help.viber.com (Rakuten's own FAQ). Per C-3
  sovereignty that is €100/mo *per vendor bot* — an impossible floor pre-revenue.
- **Park.** Re-entry trigger: G7 survey shows Viber-dominant demand (it is #1 in several Balkan
  markets — VERIFIED-secondary) AND a vendor accepts the fee in their own name. If ever built:
  needs an additive `viber` attribution value; same Class-I shape.

### 2.7 The vendor-exposed `.onion` web mirror (Class M — the anonymous tier)

- **What it is (doc 03 §3.2 / doc 04 §5, inherited):** the vendor node runs as a **Tor v3 onion
  service** serving the SAME storefront + checkout; the QR card prints a second line
  (`http://<56-char>.onion/s/:slug`). Zero new client, zero new money surface, no relay in path —
  the €6 SNI relay is bypassed entirely on this leg. Customers who already run Tor Browser/Orbot
  reach it; nobody is required to.
- **Adapter body:** a Tor listener + the `onion` attribution value + the honest label. Checkout
  defaults phone-less (per-order pseudonym; the telegram/simplex phone-less path already exists —
  VERIFIED-in-repo G03 §2.4). Status: keep-tab-open live updates (push-free); backgrounded push is
  honestly labeled as the APNs/FCM residual (doc 04 §4.3 — the floor survives).
- **Gate:** requires the vendor node (local-first rung P2+ minimum for a read-mirror; P4 for
  full-writes-on-node). Fronting the *Fly* app with an onion is technically possible but dishonest
  (traffic still terminates centrally) — REFUSED as theater; ship it when the node is real.
  Effort M (2–3 sessions) once the node exists.
- **Anonymity:** the one funnel dowiz itself exposes with **✓ network anonymity by default**. The
  honest residuals stand: delivery address, courier SIM, physical/cash correlation, push wall
  (doc 04 §5.3).

### 2.8 No-phone messenger contact (SimpleX; Session assessed)

- **Now (rides Phase 0):** `simplex` is already one of the 6 contact kinds; the G03 fix (enum + the
  **500-char handle cap** — real invite links are 200–400 chars, the VERIFIED length trap) makes it
  actually work. This is a *coordination* channel (courier copies the text link), not intake — cost
  ≈ 0.
- **Later (earn-it):** a **SimpleX order bot** (Class I) on the vendor node via the TypeScript bot
  SDK (AGPL; SMP relays self-hostable — VERIFIED-in-repo doc 04 §2.3) for customers who already run
  SimpleX. Needs an additive `simplex` attribution value. ≈0 Durrës install base → never default,
  purely additive anonymous option.
- **Session: do NOT adopt as a trust-critical channel** — seven vulns incl. realistic
  network-takeover (Yu & Haines, eprint 2026/773 — VERIFIED-in-repo doc 04). Re-entry: fixes +
  independent audit.

### 2.9 Explicitly NOT channels (doctrine, restated so this layer can refuse cleanly)

Aggregator intake (Wolt/Glovo) — D1 doctrine, read-only-later only. Voice — cart-assist, structurally
cannot place an order (ConfirmationGate). Phone-in/manual owner entry — real gap, but it is an
*owner console* feature (an owner-authenticated form hitting the same door), not a channel adapter;
hand to the console layer. Widget/kiosk — Class L variants already reserved in the taxonomy; widget
is built-not-productized; kiosk parked.

---

## 3. The attribution + anonymity spine

### 3.1 Make attribution READ on prod (today it is dark)

- **A-1 The sanctioned reader (Phase 1, S):** one "Orders by channel" card in `AnalyticsPage`
  reading `orders.metadata->>'channel'` grouped by day/range — the ONE reader the write-only ADR
  permits (VERIFIED comment `apps/api/src/lib/channel.ts:8-10`). Endpoint in the Node prod app
  (the Rust 1.5 endpoint is staging-only — review §5.4); owner-scoped, tenant-JOIN like the
  existing analytics.
- **Falsifiability built in:** the card renders `other` and `web-direct` as first-class rows — a
  broken QR (typo'd `ch=`) shows as an `other` spike; a QR that silently drops its param shows as
  `web-direct` inflation. Counsel's dark-failure warning becomes a visible signal.

### 3.2 Bring `sales_channels` alive — per-TOKEN attribution (Phase 3, M)

The registry exists (VERIFIED migration: per-location rows, 13-kind CHECK, unique 256-bit `token`,
`active` flag, FORCE RLS) and is dead in Node. Spec:

- **Short-link redemption:** `GET /c/<token>` → look up active row → 302 to
  `/s/:slug?ch=<kind>&cid=<row-id>`; FE carries `cid` beside `ch` (same sessionStorage pattern);
  door folds `metadata.channel_id` beside `metadata.channel`. Fixes the Rust per-kind double-count
  (review §3.3) and gives per-artifact counts (table-QR #4 vs door sticker).
- **Registry as credential home (C-3):** Class-I adapters store per-vendor transport credentials
  (bot token ref, WABA id, webhook secret ref) on their channel row — secrets in the secret store,
  the row holds references + the kill switch (`active=false` = channel death by flag).
- **Owner surface:** the QR-kit page grows into the channels page (create row → mint token → print
  artifact / connect transport). This satisfies exit-gate clause (1) "register channels and print
  QR/links" in its Node form.

### 3.3 The PII envelope — anonymity is channel-INDEPENDENT (the hub guarantee)

Per docs 03/04 (inherited, binding): regardless of the funnel —

- **Per-order PII envelope:** delivery address + coordination contact (incl. any platform identity
  the adapter learned — rule C-2) encrypted under a per-order data key; envelope lives on the vendor
  node (never gossiped); **only `H(envelope)` enters the append-only log**.
- **Crypto-shred:** destroy the per-order key after the dispute window (~30 days); ciphertext rots
  to noise on every replica; the PII-free signed skeleton (amounts, status, PoD, channel kind)
  survives for books/analytics. (EDPB 02/2025 key-destruction-as-erasure — VERIFIED-in-repo doc 03.)
- **Node-today approximation (honest):** prod Postgres holds PII in `orders` columns; the existing
  anonymizer/GDPR erasure (NULLs contact/receiver fields — VERIFIED both lineages) is the interim
  shred. The envelope becomes literal at local-first P2/P4; **this layer's contract is written
  against the envelope so no adapter ever needs rework** — adapters hand PII to the door and never
  store it, today or later.
- **Attribution never carries PII:** `metadata.channel`/`channel_id` are allowlisted tokens/UUIDs by
  construction (`normalizeChannel` can only emit allowlist members — VERIFIED both impls + the Rust
  property test). `order_events` payloads are PII-free skeleton events; the envelope hash is the only
  bridge, and it dies with the key.

### 3.4 Red-lines this spine enforces (standing)

- **Write-only stands:** channel is never read by pricing, the state machine, dispatch, notifications
  routing, or authz. The reader whitelist is: analytics card, channels page, exports. Enforce with a
  grep-guardrail (`metadata->>'channel'` allowed only under an allowlisted path set) — RED case: add
  a read in `dispatch.ts` → guardrail fails.
- **NO COURIER SCORING:** no attribution-derived, channel-derived, or outcome-derived courier metric,
  ranking, or dispatch weight — ever. Channel analytics aggregate by channel/venue, never by courier.
  (Also legally load-bearing — the couriers-as-venue-staff posture, VERIFIED-in-repo launch-legality
  research.)
- **COD:** no adapter renders, links, or requests any payment instrument. `payment.method` stays the
  door's business (cash today; the crypto fork is a separate dark lane).
- **No dedicated app:** no adapter may require an install of anything dowiz ships. Adapters ride
  channels the customer already has.

### 3.5 Taxonomy governance (adding the values this blueprint needs)

Additive values needed: `onion` (§2.7), `telegram-bot` (§2.3c), `viber` + `simplex` (only if their
Class-I adapters are ever unparked). Process (the anti-§6.2.2 rule): (1) land the CI **parity gate**
comparing the four lists (api TS, web TS, Rust array, `sales_channels` CHECK) FIRST — RED case: add
a value to one list only → CI fails; (2) each addition = one migration (red-line, operator) + the
SSOT module + the same-commit parity-green proof; (3) a value is only valid once its adapter's
governance card exists (REBUILD-MAP rule). Values are never removed or renamed (additive-only,
first-touch-per-cart, never-null — the ADR'd invariants stand).

---

## 4. Sequencing — which doors earn their keep (max-EV-governed) + VbM per phase

Governing findings: **QR/web first; messenger transport NOT until demand shows** (MAX-EV §7
skip-list + hub review §7 — the first messenger transport is "third in line at best," after the G7
survey). The July walk-in IS the survey. Sessions ≈ one focused agent session.

### Phase 0 — Unbreak the one live door (the precondition for everything)

- **Entry:** now; rides the Wave-1 prod vehicle (G01/G03/G11 per MASTER-EXECUTION-PLAN).
- **Modules:** `packages/shared-types/src/legacy.ts` (operator-applied G03 draft: 6-kind enum,
  handle max 500, `receiver{}`); no adapter code.
- **VbM (from G03, adopted):** GREEN — `request.post('/api/orders')` with kind ∈ {phone, signal,
  simplex-with-real-300-char-invite} → 201; telegram + receiver{simplex} → 201. RED — `kind='icq'`
  → 400 `VALIDATION_FAILED`; the same spec run against the unfixed build MUST fail (proves
  falsifiability).
- **Effort:** S (≤1 session + operator apply). **Deps:** operator protect-paths authorization.

### Phase 1 — QR/NFC live + the attribution mirror (first REAL second funnel, measured)

- **Entry:** Phase 0 merged; venue #1 claim scheduled (G11).
- **Modules:** build-arg flip `VITE_CHANNEL_KIT_ENABLED`; `apps/api` owner analytics endpoint
  (channel GROUP BY); `AnalyticsPage` "Orders by channel" card; printed A6 kit for venue #1.
- **VbM:** GREEN — an order placed via `/s/:slug?ch=qr` creates **exactly ONE** order row with
  `metadata.channel='qr'` AND increments exactly the `qr` bucket in the card (count asserted
  before/after). RED-1 — `?ch=qrr` (malformed) lands in `other`, NOT `qr`, and creates exactly one
  order (malformed attribution never blocks or duplicates an order). RED-2 — replaying the same
  `idempotency_key` from a QR session returns 200-replay and the bucket count does NOT increment
  twice. RED-3 — the card run against a venue with zero tagged orders shows zero (no phantom
  attribution).
- **Effort:** S–M (1–2 sessions). **Deps:** Phase 0; none on Rust.

### Phase 2 — Link-kit fast-follows (Class L breadth, zero transports, zero councils)

- **Entry:** Phase 1 card live (so every kit is measurable from day one).
- **Modules:** Instagram story/bio kit page (`?ch=instagram`) · GBP/Apple ordering-link playbook
  (ops artifact + owner copy-paste surface) · wa.me contact playbook · widget productization
  (admin copy-snippet for the already-built loader) · `/courier-invite`-class SPA_ROUTES hygiene
  for any new deep link.
- **VbM:** GREEN — one order per kit-channel on staging shows in its own bucket. RED — removing the
  `?ch=` from a kit link demonstrably shifts the order to `web-direct` (the mirror detects funnel
  breakage, per-channel).
- **Effort:** S per kit (~2 sessions total). **Deps:** Phase 1.

### Phase 3 — The adapter contract hardened (registry, SSOT, token; still no new transport)

- **Entry:** first real non-operator order exists (G11 GREEN) — the same trigger as local-first P2;
  before this, contract formalization is speculative overhead.
- **Modules:** shared-types SSOT `messenger.ts` + collapsed `channel.ts` (operator; G03 4a) · CI
  vocabulary parity gate (§3.5) · `sales_channels` Node runtime: `/c/<token>` redirect + `cid`
  capture + channels/QR-kit owner page v2 (§3.2) · cart-token spec → council packet (build license
  only with Phase 4's gate) · TMA hygiene (CSP allowance + Dockerfile ARG) WITHOUT flipping
  audience — the flip stays demand-gated.
- **VbM:** GREEN — two same-kind channels (two QR rows) at one location attribute separately by
  `cid` (kills per-kind double-count — assert two distinct counts). RED-1 — a value added to only
  one vocabulary copy fails CI (run it: parity gate goes red on a scratch branch). RED-2 — a
  deactivated channel row's `/c/<token>` 302s to the storefront WITHOUT `cid` (dead channel stops
  attributing but never blocks the order).
- **Effort:** M (2–3 sessions). **Deps:** G11 GREEN; operator (shared-types + migration if the
  redirect table needs anything — expected NO new migration: `sales_channels` suffices).

### Phase 4 — First intent-class transport (ONLY if the survey says so)

- **Entry (hard gate):** G7 10–15-vendor survey names the messenger AND ≥1 venue commits to
  operating it in their own name (number/bot per C-3) AND cart-token council approves AND ingress
  security review (HMAC verify, rate-limit, kill switch, monitoring line) passes. Expected pick:
  WhatsApp Cloud API (§2.4) or Telegram bot (§2.3c); Viber only on survey evidence + fee acceptance.
- **Modules (WhatsApp shape):** `apps/api/src/routes/whatsapp-webhook.ts` (signature verify +
  update router) · `lib/cart-token.ts` (mint/verify/burn; single-use nonce store) · checkout
  `?ct=` redemption path (re-validate, re-price, prefill) · dialogue map (list-ids→SKU from the
  menu snapshot) · `renderStatus` in-window replies · governance card + flag (default OFF) ·
  `sales_channels` credential row.
- **VbM:** GREEN — an order initiated in-chat terminates as **ONE** canonical order via the web
  checkout, stamped with the channel; the customer's checkout shows server prices (assert a
  deliberately stale in-chat menu still yields DB prices). RED-1 — a webhook with a bad
  `X-Hub-Signature-256` → 401, **zero DB writes** (assert row counts unchanged). RED-2 — a cart
  token with a tampered item id / expired `exp` / reused nonce → checkout refuses; no order.
  RED-3 — a token minted carrying a price field is unconstructible (schema has no price slot —
  compile/parse-level RED). RED-4 — kill switch OFF → webhook 200-ACKs (platform etiquette) but
  mints nothing.
- **Effort:** M–L (3–5 sessions + Meta verification wall-clock 2–10 biz-days). **Deps:** Phases
  0–3; G7; council; per-vendor Meta Business Verification.

### Phase 5 — The anonymous tier (rides local-first, not before)

- **Entry:** vendor node real (local-first P2 for read-mirror; P4 for node-authoritative writes);
  anonymity labels shipped on all channels (they're copy — do them in Phase 3).
- **Modules:** Tor onion-service listener on the vendor node serving `/s/:slug` (+`onion` value) ·
  phone-less checkout default on that funnel · second QR line on the kit · (earn-it) SimpleX bot
  adapter via the TS SDK on the node.
- **VbM:** GREEN — an order over the `.onion` creates the same ONE canonical order (skeleton
  identical to a clearnet order except `metadata.channel='onion'`); the vendor's clearnet relay
  logs show ZERO traffic for it (relay-free assertion). RED-1 — envelope check: the append-only log
  for that order contains no address/contact plaintext (grep the payloads); only `H(envelope)`.
  RED-2 — crypto-shred: after key destruction the envelope fails to decrypt (assert decrypt error)
  while the skeleton still folds to the correct terminal state. RED-3 — the onion funnel with a
  volunteered phone kind still passes the same velocity gates (anonymity ≠ abuse bypass).
- **Effort:** M (2–3 sessions) + SimpleX bot M if unparked. **Deps:** local-first ladder; taxonomy
  addition (§3.5).

### Parked (each with its re-entry trigger)

| Item | Why parked | Re-entry trigger |
|---|---|---|
| Viber bot | €100/mo/bot floor (VERIFIED) vs LOW-MED evidence | G7 names it + vendor accepts fee |
| Instagram DM intake | App-Review weeks + 200 DM/hr; dominated by WhatsApp for the same audience | WhatsApp adapter live + IG-heavy vendor demand |
| Telegram TMA customer flip | audience question R1 open; MAX-EV skip-list | demand signal from the owner-preview / survey |
| Session adapter | eprint 2026/773 network-takeover findings | fixes + independent audit |
| MCP/agent head (`agent`) | no demand; cart-token doctrine ready | first agent-commerce partner; council |
| Aggregator intake | doctrine-excluded (D1) | operator + money council (read-only view only) |
| In-chat payments, any channel | COD ruling + single-money-surface invariant | council, indefinitely 🔴 |

**Standing layer-wide proof (all phases):** the review's ground rule stays the falsifiable spine —
`grep 'INSERT INTO orders'` = exactly one non-test site per stack, and every live funnel's order is
distinguishable ONLY by attribution metadata. Any phase that breaks either assertion is RED by
definition.

---

## Sources

**Repo (read-only, this session):** files and docs cited inline; primary anchors:
`apps/api/src/lib/channel.ts`, `apps/web/src/lib/{channel,messenger,tma}.ts`,
`apps/api/src/routes/telegram-webhook.ts`, `apps/api/src/notifications/*`,
`rebuild/crates/api/src/modules/channel_attribution/mod.rs`,
`packages/db/migrations/{1780350000000_sales-channels,1780350000001_order-events-log}.ts`,
`docs/research/2026-07-11-hub-architecture-review.md`,
`docs/design/gap-blueprints-2026-07-11/G03-checkout-422-messenger-kinds.md`,
`docs/design/local-first-hub-2026-07-11/{SYNTHESIS,04-anonymity-mesh-messenger-revision}.md`,
`docs/research/2026-07-11-MAX-EV-SYNTHESIS.md`, `docs/design/dowiz-brand/EXPANSION-PLAN.md` §1.C.

**Web (fetched/searched 2026-07-11 this session):**
- WhatsApp: per-message pricing since 2025-07-01; free-form + utility templates free inside the 24h
  customer-initiated service window; verification 2–10 biz-days; 250 conv/24h unverified —
  [respond.io/blog/whatsapp-business-api-pricing](https://respond.io/blog/whatsapp-business-api-pricing)
  (fetched; VERIFIED-secondary), corroborated by
  [chatarmin.com](https://chatarmin.com/en/blog/whatsapp-cloudapi),
  [intelliconcierge.com](https://www.intelliconcierge.com/blog/whatsapp-cloud-api-pricing).
- Instagram Messaging API: App Review for `instagram_business_manage_messages`, 25 test users
  pre-review, 200 automated DM/hr, 24h window, bot disclosure, API free —
  [developers.facebook.com Instagram messaging docs](https://developers.facebook.com/docs/instagram-platform/instagram-api-with-instagram-login/messaging-api/),
  [zernio.com/blog/instagram-messaging-api](https://zernio.com/blog/instagram-messaging-api),
  [creatorflow.so IG DM compliance 2026](https://creatorflow.so/blog/instagram-dm-compliance-meta-rules/)
  (VERIFIED-secondary).
- Viber: €100/month/bot maintenance fee + per-message bot-initiated fees; session (customer-initiated
  24h) messages free; model effective 2024-02-05 —
  [help.viber.com bot commercial model](https://help.viber.com/hc/en-us/articles/15247629658525-Bot-commercial-model),
  [help.viber.com commercial model FAQ](https://help.viber.com/hc/en-us/articles/15383950711197-Rakuten-Viber-chatbot-commercial-model-FAQ)
  (VERIFIED — platform's own support pages).
- Telegram: Bot API/Mini Apps free; Stars mandatory only for DIGITAL goods — physical goods use
  ordinary provider/link-out flows; initData validation incl. third-party Ed25519 `signature` —
  [core.telegram.org/bots/payments-stars](https://core.telegram.org/bots/payments-stars),
  [core.telegram.org/bots/payments](https://core.telegram.org/bots/payments),
  [core.telegram.org/bots/webapps](https://core.telegram.org/bots/webapps),
  [docs.telegram-mini-apps.com init-data](https://docs.telegram-mini-apps.com/platform/init-data)
  (VERIFIED — platform docs).
- SimpleX / Session / Tor / push-wall facts: inherited from doc 04's same-day verified research
  (not re-fetched; labels carried as VERIFIED-in-repo).

*Produced 2026-07-11. Read-only session; the only file created is this blueprint. Code anchors may
drift — a parallel session is actively changing the tree.*
