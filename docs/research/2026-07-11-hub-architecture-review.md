# Hub Architecture Review — "One hub for food vendors: many order sources, vendor-owned couriers" — 2026-07-11

> **Operator question (verbatim intent):** "The one hub architecture for food vendor owners — which
> can allow processing the orders from multiple sources web/messengers/bots/etc and couriers. So
> this is an actual delivery hub system for the vendors with own couriers." Find it, reconstruct it
> end-to-end, and review it.
>
> **Method:** read-only research session (nothing in the tree modified; the only file created is
> this report). Grounds: the full-project audit
> `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` (§1, §2.1, §4.2, §5.1–5.3) and the 13
> gap blueprints `docs/design/gap-blueprints-2026-07-11/` (esp. G06/G03/G11 + MASTER-EXECUTION-PLAN)
> — cited, not redone; the design corpus (channel-hub, sovereign-core-mvp, rebuild-plan,
> order-channel-attribution, EXPANSION-PLAN, customer-distribution-channels research); the living
> memory corpus; and fresh code review + live read-only GET probes of dowiz.fly.dev /
> dowiz-staging.fly.dev performed for this review. Every fresh claim is labeled **VERIFIED**
> (code/live endpoint checked this session), **CLAIMED-UNVERIFIED** (from docs/memory, not
> independently re-checked), or **CONTRADICTED**.

---

## 0. Executive summary

**The architecture the operator asked about exists, is unusually well designed, and is roughly
one-third real.** The "one hub" is documented across MANIFESTO/DECISIONS/GRAND-PLAN (the sovereign
core), REBUILD-MAP §6 + 07-channel-hub-adoption (the channel doctrine), and EXPANSION-PLAN (entry
doors): one deterministic kernel as the single decide door; channels as thin, write-only
attribution wrappers around ONE web checkout; a signed cart-token as the only handoff for
channels that pre-build intent; aggregators read-only by doctrine, never intake. It is coherent,
adversarially self-reviewed, and economically disciplined (§1).

**What is real today for a vendor on prod (VERIFIED):** exactly ONE order source — the web
storefront `/s/:slug` (plus its subdomain/QR-link variants) feeding one canonical, transactional,
integer-money, idempotent order pipeline; a Telegram bot for owner notifications and
confirm/reject actions; a complete and genuinely hardened vendor-owned-courier backend
(invite → shift → assign → honest dispatch → deliver-v2 cash-as-proof → journal redispatch, with
per-frame WS authz); an owner console with realtime orders, menu, settings; and a customer
tracking page (§§2-5).

**Where the claim fails today (ranked):**
1. **"Many order sources" is attribution-true but transport-false.** Every messenger/bot/agent
   intake is paper or dark scaffold; even the built wrappers (QR kit page, TMA) sit behind
   default-off flags, and nothing reads the attribution that IS captured — zero readers of
   `metadata->>'channel'` in prod (§2, §5.4).
2. **The one working door 400-fails 3 of its 6 advertised contact options** and every "deliver to
   someone else" order (G03, prod-live since 07-03), and the vendor acquisition gate is broken
   (`/claim` 404, 11/12 demos staging-only, prod worker stopped — G11).
3. **Couriers are deaf outside the app**: zero courier push/Telegram anywhere in the system; a
   locked phone silently loses dispatch after the 5-min sweep; the courier UI is a design
   generation behind its own backend (offer-handshake unflippable, refusal tails unreachable);
   "earnings" is actually cash-owed-to-venue — no compensation model (§4).
4. **There are two half-hubs on one spine.** Production is 100% Node; the Rust kernel is
   staging-dark — and this review found its checkout **bypasses `kernel::decide` entirely**
   (no `Command::PlaceOrder` exists in the api crate; same math, different door), on top of G06's
   findings that `hub_checkout` gates nothing, replay-parity is a placeholder, `cause_hash` is
   the literal string "placeholder", and the validation suites cannot fail (§3).

**The verdict:** the hub is a correct architecture wrapped around one live door, with a courier
loop that is operationally real but humanly leaky, and a second (better) hub that is not yet
honest with itself. Nothing about "one hub, many sources, own couriers" is blocked on the Rust
rewrite. The fastest path to making the operator's sentence true for one real vendor this month
(§7): ship the Wave-0/1 fixes (G03 + /claim + worker + GDPR trio, adding this review's three
small fixes), hand one claimed venue the already-built QR kit with a one-card
attribution reader, and give couriers an out-of-app beep — then, and only then, spend on the Rust
exit gate (G06 Option B, amended: route the Rust checkout through `decide` before any prod flip),
and defer every messenger transport until the G7 vendor survey says which one earns its keep.

---

## 1. The vision — what "one hub" means in this project

The hub architecture is real, written down in unusual detail, and internally coherent. It is spread
across four load-bearing documents plus one binding decision record:

| Doc | Role |
|---|---|
| `docs/design/sovereign-core-mvp/MANIFESTO.md` | Operator's vision: "a modular hub that lets a food-business owner control their own data across their own channels from one module" — escape aggregators, 0% commission, own the customer (§2) |
| `docs/design/sovereign-core-mvp/DECISIONS.md` | D1 scope lock: own-channel hub + ONE direct checkout + owned customer data; aggregator orders **read-only, later**; marketplace ingestion explicitly rejected ("breaks the single-money-surface invariant") |
| `docs/design/sovereign-core-mvp/GRAND-PLAN.md` | The execution roadmap: 0b (seal the kernel) → Phase 1 (channels registry, event log, sync port, signed envelope, owner channel surface) → Phase 2 (distribution artifacts, kernel checkout, owned customers, aggregator stub) with an explicit MVP exit gate |
| `docs/design/rebuild-plan/REBUILD-MAP.md` §6 + `07-channel-hub-adoption.md` | The channel-hub doctrine: ONE commerce core + 4 thin head families, channel taxonomy v1, cart-token spec v0, per-head governance cards, ROI-ranked 90-day sequence |
| `docs/research/2026-07-04-customer-distribution-channels.md` (742 lines) | The evidence base: every channel grounded in code (file:line) + live 2026 web sources, ranked Tier 1/2/3 for the Albania market |
| `docs/design/dowiz-brand/EXPANSION-PLAN.md` | The "hybrid multi-layered hub" frame: "The kernel is the cathedral; every entry point is a write-only door that carries commands to it" — entry-point sequencing QR → Telegram Mini App → WhatsApp Cloud API |

### 1.1 The five load-bearing ideas (reconstructed faithfully)

1. **One kernel, one door.** All business mutation flows `Command → kernel::decide → Vec<Event>`;
   state is `fold(events)`; forbidden transitions are compile/refusal errors. `decide` composes, in
   order: state machine → actor gate → CC-1 courier-strand guard → pricing/LC1 conservation
   corridors (GRAND-PLAN 0b-3). VERIFIED at `rebuild/crates/domain/src/kernel.rs` (see §3.2). The
   EXPANSION-PLAN's decision rule for every entry point: *"does this door decide anything, or only
   carry? Doors carry a Command to kernel::decide… Doors NEVER price, transition state, or invent
   money. Coherence by construction, not vigilance."*

2. **Channels are attribution + entry doors, never decision inputs.** The channel taxonomy v1
   (`07-channel-hub-adoption.md` §2) is a 13-value lowercase-kebab allowlist —
   `web-direct | qr | nfc | gbp | apple-maps | instagram | facebook | whatsapp | telegram-tma |
   kiosk | widget | agent | other` — additive-only, immutable, first-touch-per-cart, never-null.
   The binding invariant (order-channel-attribution proposal §8, ADR'd): channel is **write-only
   metadata** — "never read by pricing, the order-status state-machine, dispatch/courier-assignment,
   notifications, or any authz/RLS decision." Attribution is analytics-grade, not money-grade.

3. **Single money surface — the cart-token doctrine.** REBUILD-MAP §6 invariant 1 (operator-set,
   standing): *"every conversational/social/agentic channel terminates in a signed, TTL'd,
   idempotent cart token → the ONE web checkout. Channels build carts; only checkout touches
   money."* In-chat payments are per-channel 🔴 council-gated indefinitely. Pure-link channels
   (QR/NFC/GBP/Apple/Instagram links) never need a token — they enter at the menu. The cart-token
   spec v0 (`07-channel-hub-adoption.md` §3): signed compact token `{slug, items, channel, iat,
   exp≤15min, nonce}`, **no prices, no totals, no PII**, single-use nonce, server re-validates and
   re-prices every line. This one rule is why N channels ≠ N money surfaces ≠ N fiscalization
   integrations (the Albania fiskalizimi blocker attaches to exactly one checkout).

4. **Thin heads, four families.** REBUILD-MAP §6: one commerce core + head families **render**
   (Astro storefront, kiosk, TMA) · **conversational** (ONE dialogue FSM + per-platform transports)
   · **feed/discovery** (JSON-LD, GBP/Apple links, Meta feed) · **agentic** (MCP/UCP stub). Heads
   contain translation only, import the client contract never the domain, budget ≤~500 lines,
   per-head governance card (authz principal, idempotency keys, rate quota, kill switch, monitoring
   line) before any taxonomy value is valid. "Channel death is a flag flip."

5. **Aggregators: read-only view, never intake.** D1 locks it: Wolt/Glovo orders appear later as a
   READ-ONLY unified dashboard tab (`aggregator_view` flag, one `AggregatorSource::
   fetch_orders_readonly` trait, zero impls in MVP — GRAND-PLAN 2.4). Money and intake stay on the
   owner's checkout. This is a deliberate anti-Deliverect stance: the hub ingests attribution and
   visibility, not marketplace order flow.

Underneath these sit the decentralization seams (DECISIONS D2 — event-sourcing, content-hash +
signature slot per event, transport-agnostic sync port) and the reliability philosophy (MANIFESTO
§4: fail-fast, corridor refusal over workaround, invariants outside the agent). The
`docs/design/dowiz-agent-cli/CORE.md` bridge maps the same kernel shape onto the bebop CLI ("where
the Grand Plan says dowiz-core, read Bebop kernel") — a documented seam, explicitly NOT an MVP
dependency (G06 §2.6).

### 1.2 Intended order flow per source (diagrams-in-text, per the design docs)

**A. Web direct / QR / NFC / social link (pure-link channels — the only class fully designed AND
built):**

```
customer device
  │  GET /s/:slug?ch=qr            (?ch= read ONCE at landing, sessionStorage per-slug key)
  ▼
storefront (React SPA today / Astro+Svelte target)
  │  menu → cart (server-side) → checkout form
  │  POST /api/orders  { items, customer, … , channel:'qr' }   ← Zod-validated 13-value enum
  ▼
order core (Node today / Rust kernel target)
  │  validate → server-price (integer Lek) → idempotency-key dedupe
  │  → INSERT orders (+ metadata.channel='qr', write-only)  [+ order_events append, Rust path]
  ▼
fan-out: owner Telegram/webpush/email notification → owner accepts → courier loop (§4)
```

**B. Conversational/bot/agentic channels (designed, NOT built — the cart-token class):**

```
bot / DM flow / MCP agent
  │  builds intent (item IDs + qty ONLY — no prices)
  │  mints signed cart-token {slug, items, channel, iat, exp≤15m, nonce}
  ▼
/s/:slug/checkout?ct=<token>
  │  verify sig/exp/slug → server re-validates EVERY line → prices fresh from DB
  │  → same ONE web checkout as flow A, order stamped with the channel
```

**C. Telegram Mini App (wave-1 "wrap", dark):** `t.me/<bot>/<app>?startapp=<slug>` → Telegram
WebView loads `/s/:slug?ch=telegram-tma` → identical to flow A. Adapter = theme mapping + MainButton
proxy + (Phase-2) `initData` HMAC auth. Behind `TG_STOREFRONT_ACTION`/`TMA_ENABLED`/
`VITE_TMA_ENABLED`, all default OFF.

**D. Aggregator (Wolt/Glovo) — read-only later:** `AggregatorSource::fetch_orders_readonly` →
dashboard "All orders" tab. Never intake, never money.

### 1.3 The MVP exit gate (the design's own definition of "the hub exists")

GRAND-PLAN §"MVP exit gate", verbatim: *an owner can (1) register channels and print QR/links,
(2) receive a real direct order end-to-end at 0% commission through the sealed core, (3) see it
attributed in their dashboard, (4) own and erase the customer record — with the full money battery
green, `/reliability-gate` GO, replay-parity green, NOBYPASSRLS suites green, and the sovereign CI
check required on `main`.*

**Verdict on the vision itself:** coherent, unusually disciplined, and honest about its own failure
modes (GRAND-PLAN's adversarial self-critique names dual-write rot, the 0b-5 flip risk, and the
over-engineering magnets by name; LEAD-REVIEW adds five amendments). Two things the vision does
**not** cover well, which matter for the operator's exact question: (a) **the courier loop is not
part of the hub docs at all** — GRAND-PLAN/channel-hub/EXPANSION-PLAN treat "hub" as
inbound+checkout+data; couriers live in a separate, older design lineage (deliver-v2, ADR-0013) that
predates the sovereign-core arc and was never re-integrated into the hub definition; (b) the
**conversational head family** (the "messengers/bots" half of the operator's sentence) is entirely
demand-gated behind a vendor survey (G7) that has never been run.

---

## 2. Inbound channels scorecard — what can actually deliver an order TODAY

**Ground rule discovered in code (VERIFIED):** the Node app has **exactly one** order-creating
endpoint (`POST /api/orders` via `order-persistence.ts:79`; the only other `INSERT INTO orders` is
a dev seed fail-closed in prod, `dev/mock-auth.ts:449` + `server.ts:409-418`). The Rust app
likewise has exactly one (`orders/mod.rs:675`). So every "channel" below is either (a) a wrapper
that lands customers on the same web checkout with an `x-channel` tag, (b) an owner/ops surface,
or (c) paper. That is faithful to the design (§1 — "every channel is a different wrapper around
the same link") — but it means "processing orders from multiple sources" is today true only in
the attribution sense, not the transport sense.

| # | Order source | Design status | Code status | Prod status TODAY | Key evidence |
|---|---|---|---|---|---|
| 1 | **Web storefront `/s/:slug`** (`web-direct`) | done (the anchor) | LIVE, full pipeline §3.1 | **LIVE — the only real order source**; but carries the G03 contract bugs: Phone/Signal/SimpleX pick → 400; "deliver to someone else" → 400 | `orders.ts:93`, `legacy.ts:48`, G03 §1 (prod-probe VERIFIED) |
| 2 | **Subdomain white-label** `<slug>.dowiz.org` | done | LIVE rewrite → `/s/:slug` (reserved www/api/app) | LIVE (same checkout) | `apps/api/src/lib/subdomain-rewrite.ts:17-32`, wired `server.ts` (research §0.1); file present VERIFIED |
| 3 | **QR / NFC kit** (`?ch=qr`, `?ch=nfc`) | done (research §1; GRAND-PLAN 2.1) | Attribution plumbing LIVE end-to-end (`ClientLayout.tsx:114` → sessionStorage → `x-channel` → `orders.metadata`); owner **QR-kit page BUILT but DARK** behind `VITE_CHANNEL_KIT_ENABLED` (`AdminRoutes.tsx:23-24`, `QRKitPage.tsx:38-39` builds `/s/<slug>?ch=qr|nfc`) | Attribution works today for any hand-made QR; the owner-facing kit is flag-off; **nothing reads the attribution** (no dashboard) | VERIFIED code; audit §6.2 (`VITE_CHANNEL_KIT_ENABLED` dark) |
| 4 | **Telegram bot** (owner ops) | done (telegram-notifications council) | LIVE: `/start` connect, order confirm/reject callbacks, `/open` courier shift; `/store` + `/settings` dark behind `TG_STOREFRONT_ACTION`/`TG_CATEGORY_GATING` | LIVE for owner notifications + order actions. **NOT a customer ordering channel** — no order intake via chat | `telegram-webhook.ts:161-536, 539-776` VERIFIED |
| 5 | **Telegram Mini App** (`telegram-tma`) | done, council-APPROVED as a dark "distribution probe" | Scaffolded 3-part: bot menu-button (`TMA_ENABLED`, owner self-preview → `${APP_BASE_URL}/s/<slug>?ch=telegram-tma`, `notifications/telegram-mini-app.ts:38-43`); FE detection (`VITE_TMA_ENABLED`, `tma.ts`); **CSP blocks `telegram-web-app.js`** (spa-shell.ts script-src) and the Dockerfile ARG for the client flag is missing | DARK. Even flipped ON it is owner-self-preview, not a customer channel (audience question R1 explicitly OPEN) | `TMA-VALIDATION.md` (whole doc); `tma.ts:4-12`; VERIFIED flags absent from EnvSchema (raw env reads) |
| 6 | **WhatsApp** | Cloud API door specced (EXPANSION-PLAN 1.C: webhook + HMAC + interactive-list→SKU; open-wa banned for prod) | **PAPER-ONLY as intake.** As a *notification* channel it was deliberately REMOVED for privacy/ToS (migration `1790000000043`); survives as wa.me contact deep-link + attribution value | Nothing on prod beyond the contact deep-link | EXPANSION-PLAN 1.C VERIFIED read; migration + `config/index.ts:44` VERIFIED |
| 7 | **Instagram / Facebook** | done (research §6: link-wrapper only; Meta killed native checkout) — `instagram` 7th messenger-kind + story kit = wave 1 | **NOT built**: `MESSENGER_KINDS` still 6 kinds, no `instagram` (`messenger.ts:8`); attribution values exist in the allowlist; story kit absent | Attribution value would stamp if a link carried `?ch=instagram`; no owner kit, no ig.me contact option | VERIFIED `messenger.ts:8`; research §0.1 gap G9 |
| 8 | **GBP / Apple Maps ordering links** | done (research §2 — playbook + concierge, zero code) | n/a by design (ops artifact = the URL) | Possible today by hand; no playbook executed, no owner surface | research §2, §15 Tier-1 #2 |
| 9 | **Voice** | ADR-0015, council-approved, client-side read-only scope | Phase-0 safety core BUILT (`packages/voice`, 43/43 tests; ConfirmationGate fail-closed; no checkout write path — `voice/handlers.ts:50-52`); **MicFab not mounted anywhere**; flags dark | DARK, and by design voice can never place an order — it is navigation/cart-assist | voice-control-council memory + audit §6.2 VERIFIED-by-grep (no JSX usage) |
| 10 | **Phone / manual owner entry** | **not designed anywhere** | **does not exist** — no owner manual-order endpoint; a phoned-in order has no representation in the system | Absent | VERIFIED (route-surface sweep, `bootstrap/routes.ts:87-159`) |
| 11 | **MCP / UCP / conversational-agent heads** | designed (Phase C wave 1 stub; `agent` taxonomy value reserved; cart-token doctrine ready) | zero code (grep: `agent` channel value only) | Absent | `07-channel-hub-adoption.md` §4; research §12 |
| 12 | **Embeddable widget** | done (research §7) | BUILT (loader + SRI + iframe resize: `apps/api/src/client/widget/loader.ts`, built artifacts in `public/dist/`) — productization (docs + admin copy-snippet) not done | Deployable today by a motivated owner; no admin surface | file presence VERIFIED; research §0.1 |
| 13 | **Kiosk / counter mode** | designed Tier-2 (research §10) | value reserved in allowlist only; no kiosk mode code | Absent | VERIFIED grep (kiosk appears only in channel allowlists + landing copy) |
| 14 | **PWA / push retention surface** | designed (research §8) | manifest BUILT but single-tenant-flavored (hardcoded `/s/dubin-sushi` shortcut, `apps/api/public/manifest.json`); webpush vertical built server-side; sw.js build discrepancy flagged | Partially live; not per-tenant | research §0.1 (file VERIFIED present) |
| 15 | **Aggregator ingest (Wolt/Glovo/Baboon)** | **deliberately rejected** (D1: read-only view later; intake never) — 2.4 stub unbuilt (zero artifacts) | zero code (grep: rhetorical comments only) | Absent, by doctrine | DECISIONS D1; G06 §2.1; agent-A grep VERIFIED |

**Scorecard verdict.** For a vendor on prod **today** there is one order source (web storefront,
incl. its subdomain/QR-link variants), one owner ops channel (Telegram bot), and a checkout whose
Communication step 400-fails 3 of its 6 advertised options plus every "deliver to someone else"
order (G03). Every messenger/bot/agent *intake* channel is paper or dark scaffold — consistent
with the thin-head doctrine (they were all designed as wrappers around the one URL), but the
wrappers themselves (QR kit page, Instagram kit, GBP playbook, TMA flip) have not been handed to
any owner. The design's own Tier-1 list (research §15: QR kit, GBP/Apple links, Instagram pack,
WhatsApp free-tier playbook, attribution/shortlink hub — all ~€0, all S-effort) remains unshipped
11 weeks of work-time later, while two deeper layers (Rust kernel, event log) were built beneath
it.

---

## 3. Order processing core — one canonical order, or two half-hubs?

### 3.1 The Node production path (what every real order rides today) — VERIFIED

**Exactly one canonical intake.** Grep for `INSERT INTO orders` across `apps/` returns two non-test
sites: the canonical checkout persistence (`apps/api/src/lib/order-persistence.ts:79`) and a
dev-only visual-seed order (`apps/api/src/routes/dev/mock-auth.ts:449`, fail-closed 404 in prod —
`server.ts:409-418` + boot assert `packages/config/src/index.ts:246-260`). Every real channel today
— direct web, QR, NFC, GBP/social links, TMA WebView — is the *same* `POST /api/orders` endpoint,
differentiated solely by the write-only `x-channel` header.

**The pipeline** (`apps/api/src/routes/orders.ts`, one DB transaction with
`SET LOCAL statement_timeout=4500` at `:124`):

1. **Schema**: `CreateOrderInput.parse` at `orders.ts:93` — `.strict()` Zod
   (`packages/shared-types/src/legacy.ts:40-77`): locationId, type delivery|pickup, items (qty
   1-99, modifier_ids), customer{phone/name/messenger_kind/handle}, delivery pin,
   `payment{method: z.literal('cash')}`, tip, `idempotency_key` (required uuid). Parse failure →
   400 VALIDATION_FAILED (`orders.ts:94-97`).
2. **Location gates**: 409 NOT_PUBLISHED for drafts (`:145-148`); VENUE_CLOSED behind
   `ENFORCE_VENUE_HOURS` (default off).
3. **Idempotency**: canonical request fingerprint `buildRequestHash`
   (`apps/api/src/lib/order-canonical.ts:30-52` — sha256 over locationId/type/sorted items/rounded
   pin/address/cash/currency/menu_version/customerId); tenant-scoped composite PK
   `(location_id, key)` (migration `1790000000029`); same key + different hash → 422
   IDEMPOTENCY_KEY_REUSED; same key + same hash → replay 200 (`orders.ts:394-410`); race → 409.
4. **Anti-abuse**: 5/min rate limit keyed phone-or-IP (`orders.ts:74-88`); per-phone velocity
   5/15min; per-(location,IP) 20/15min; preflight signal machine hard_block/soft_confirm/clean
   (`orders.ts:363-392`).
5. **Money**: integer minor units end-to-end; prices from the in-transaction MVCC snapshot, never
   the client (`orders.ts:414-423`); BigInt half-up tax ("RED LINE: zero float arithmetic on
   money", `apps/api/src/lib/money.ts:9-24`); DB `integer CHECK >= 0` columns; currency from the
   location row.
6. **Writes**: customers upsert, orders (29 columns incl. `metadata` jsonb with channel),
   velocity_events, order_items(+modifiers), idempotency_keys, customer_track_grants, plus
   **transactional** pg-boss enqueues ORDER_TIMEOUT + NOTIFY_TELEGRAM_SEND
   (`order-persistence.ts:78-182`). Post-commit: MessageBus `order.created` (zero customer PII,
   `orders.ts:606-645`), customer tracking JWT, optional dark crypto-charge fork.

**State machine**: canonical in `packages/domain/src/order-machine.ts:18-40` — the same 10 statuses
and transition relation as the Rust kernel (§3.2), byte-frozen across the port. Enforcement is
centralized in `updateOrderStatus` (`apps/api/src/lib/orderStatusService.ts:53-124`):
`assertTransition` → status-guarded UPDATE (`WHERE status=$3`, 409 on race) → courier-assignment
terminalization fold (`:139-150`) → refund_due obligation fold (`:165-219`) →
`order_status_history` audit row (`:226-237`) → bus fan-out. Route-layer authz on top: owner PATCH
with membership-JOIN tenant check, `assertOwnerTargetAllowed` blocking owner-driven SYSTEM-only
cancel edges (`apps/api/src/lib/orderAuthz.ts:11-27`), DELIVERED/PICKED_UP anti-strand guards
(`orders.ts:929-955`).

**Attribution today (phases "1.1/1.2/1.5" in the Node reality):**

- **Capture** — VERIFIED live-path: storefront reads `?ch=` once on mount
  (`apps/web/src/routes/ClientLayout.tsx:114` → `captureChannel`), persists per-slug in
  sessionStorage `dos_channel:<slug>` (`apps/web/src/lib/channel.ts:36,66-74`), sends
  `x-channel: getOrderChannel(slug)` at checkout (`CheckoutPage.tsx:317`); server normalizes via
  the 13-value allowlist (`apps/api/src/lib/channel.ts:19-51` — never throws, unknown → `other`,
  missing → `web-direct`) and folds it into `orders.metadata` (`order-persistence.ts:103`).
  The allowlist is deliberately **triplicated** (api lib, web lib, DB CHECK in migration
  1780350000000) with a TODO to collapse into shared-types — because shared-types is a
  protect-paths-blocked zone (`channel.ts:12-18`).
- **No reader.** Zero code reads `metadata->>'channel'` for any purpose — not pricing/dispatch
  (by design), but also **no analytics or dashboard surface** (grep-verified). The only owner
  exposure is the raw metadata passthrough in the dashboard response noted by the attribution
  proposal (`docs/design/order-channel-attribution/proposal.md` §8). Attribution is captured and
  then dark.
- **Registry dormant.** `sales_channels` (migration `1780350000000`) and `order_events`
  (`1780350000001`) exist in the shared DB but have **zero runtime references anywhere in
  `apps/`** (grep-verified) — they are Rust-only tables (§3.2). The Node app's live audit trail is
  `order_status_history` (migration `1780338982015`).

**The frozen-contract defect cluster (the messenger-kind incident is 1 of 4).** The `.strict()`
`CreateOrderInput` in `packages/shared-types/src/legacy.ts` has drifted from both the FE and the
DB — all VERIFIED against src and dist:

1. `messenger_kind: z.enum(['telegram','whatsapp','viber'])` (`legacy.ts:48`) vs the FE's six
   mandatory kinds (`apps/web/src/lib/messenger.ts:8`; selector
   `ContactInfoSection.tsx:85-87,122-124`; required at `CheckoutPage.tsx:234`) → Phone/Signal/
   SimpleX customers get 400 at `orders.ts:93`. DB already admits all 6 (migration
   `1790000000074_checkout-communication.ts:12`). This is G03's bug; prod-live since 07-03.
2. `receiver{}` absent from the strict schema, yet the FE sends it whenever "deliver to someone
   else" is unchecked (`CheckoutPage.tsx:332-334`) → 400 for those orders across ALL 6 kinds
   (G03 bug #2); downstream code already consumes it (`orders.ts:592-594`,
   `order-persistence.ts` receiver_* columns).
3. `payment.method: z.literal('cash')` vs the FE's `method:'crypto'` path (`CheckoutPage.tsx:60,
   345`) → crypto checkout is 400-rejected before the dark Plisio fork (`orders.ts:666`) can ever
   run.
4. `client_menu_version` read at `orders.ts:579` but not a schema field → always null.

One root cause: the wire contract lives in a protect-paths-frozen file while FE, DB, and the Rust
port (which fixed all of this — `rebuild/.../orders/dto.rs:40,74-82` with RED tests) moved on.
This is the strongest single piece of evidence that "channel definitions scattered across
surfaces" is a systemic hazard, not a one-off (see §6).

### 3.2 The Rust kernel path (staging-dark) — the intended "one door", not yet one door

The sovereign core is real and rigorous **as a library** (all VERIFIED by fresh code review this
session):

- **The Law.** `kernel.rs:3-5`: *"Two functions and nothing else is the truth: `decide`:
  `(&OrderState, Command) -> Result<Vec<Event>, DomainError>` — the ONE door every business action
  passes through. Pure, total, side-effect-free."* `decide` at
  `rebuild/crates/domain/src/kernel.rs:297-305` composes, in live-handler order: state machine
  (`assert_transition`, `kernel.rs:327-328`) → actor gate (Owner barred from SYSTEM-only cancel
  edges, `:330-339`) → CC-1 courier-strand guard (→DELIVERED/PICKED_UP refused while a live courier
  binding exists, `:341-347`) → event emission incl. `BindingTerminalized`/`RefundObligated`
  (`:349-371`). (Precision note: the phrase "every entry point is a write-only door" is
  EXPANSION-PLAN language; the kernel's own doc comment says "the ONE door" — the "write-only"
  wording in code belongs to channel attribution, `modules/channel_attribution/mod.rs:6-8`.)
- **Commands** (`kernel.rs:89-110`): `Confirm, Reject, StartPreparing, MarkReady, Dispatch,
  MarkDelivered, MarkPickedUp, RevertToReady, Cancel, PlaceOrder{at, actor, cart}` — the create
  command carries item/qty data only; price fields are unrepresentable by type. Every command
  carries caller-supplied `Ts` (the core never reads a clock) and an `Actor{Owner,System}`.
- **Events** (`kernel.rs:188-206`): `StatusChanged, Priced{subtotal,delivery_fee,tax_total,total},
  RefundObligated{amount}, BindingTerminalized`, wrapped at the persistence boundary in
  `Envelope{seq, at, cause: CommandHash, event}` (`kernel.rs:214-220`).
- **State machine** (`order_status.rs:19-30, 57-62`): identical 10-status relation to Node's
  `order-machine.ts`. `fold` is total with a source-reading test forbidding wildcard arms
  (`kernel.rs:604-633`, test `:1106-1124`); `replay`/`replay_envelopes` at `kernel.rs:637-647`.
- **Money**: integer-only `Lek(i64)`, non-negative, no `From<f64>`, checked arithmetic
  (`money.rs:27-58`); float banned from the core by clippy disallowed-types + the wasm32 build
  gate (`rebuild/scripts/sovereign-gate.sh` Gates 1-2).
- **Idempotency**: pure decision `Proceed/Replay/Reuse422/DeleteAndRecreate`
  (`kernel/idempotency.rs:18-46`), enacted against the shared `idempotency_keys` table
  (`rebuild/crates/api/src/routes/orders/pg.rs:172-232`).

**But the shell does not yet honor the Law.** Five findings from this session's code review; the
first is NEW (sharper than anything in G06):

1. **Checkout bypasses `decide` entirely — CONTRADICTED vs the design.** The Rust
   `POST /api/orders` handler (`checkout.rs:105-176`) builds a *shell-level*
   `CreateOrderCommand{input, channel, customer_sub}` (`checkout.rs:151-155`) — not a
   `domain::Command` — and `PgOrdersRepo::create_order` (`pg.rs:91-442`) calls the pure pricing
   functions **directly** (`compute_order_pricing` `pg.rs:291-292`, `delivery_fee_for_order`
   `:326`, `apply_tax` `:337`, `charged_tax`+`compose_total` `:345-346`). `Command::PlaceOrder` is
   never constructed anywhere in the api crate (grep: comments only, `checkout.rs:13`, `pg.rs:808`);
   no `Event::Priced` is ever produced or persisted at checkout. The "MVP centerpiece" (GRAND-PLAN
   2.2: "order placement flowing `Command::PlaceOrder` → `decide`") is
   same-math-different-door.
2. **The door serves two of four transition callers.** Owner transitions go through `decide`
   (`pg.rs:491`, `:567`) — with state synthesized from the mutable `orders.status` column
   (`pg.rs:487-490`), not from replaying the log. Customer-cancel (`pg.rs:688-695`) and the S7
   courier surface (`pg.rs:772-800`) still use the pre-kernel `apply_transition`/
   `transition_effects` path.
3. **The event log is advisory and partial.** Only the two owner transition paths dual-write to
   `order_events` (`apply_events`, `pg.rs:814-885`); `cause_hash` is the literal string
   `"placeholder"` (`pg.rs:863-864`); `content_hash` hashes `serde_json::to_vec(event)` rather
   than codec canonical bytes; `at` is stamped `Utc::now()` at write time (`pg.rs:852`), not the
   command's `Ts`. Replay-parity is a self-described placeholder
   (`scripts/replay-parity-check.sh:59-61`: "just verify the event log is not empty") that also
   selects a nonexistent `binding` column from `orders` (`:41-43`); the `sovereign_core_e2e.rs`
   "replay parity" test folds an **empty** event vec and asserts nothing (`:264-289`), and most of
   that suite is comment-bodied. (Confirms and extends G06 §2.3b-f.)
4. **`hub_checkout` gates nothing** — read only inside a `tracing::debug!` line
   (`checkout.rs:68-70`, `:125-133`) despite its own doc comment calling it "the launch gate"
   (`:21-22`). The one real behavioral guard on the cutover path — `reject_client_price_fields`
   refusing bodies carrying `subtotal/tax_total/delivery_fee/total/discount_total`
   (`checkout.rs:48-54`, `:77-87`) — fires only on the `x-dowiz-cutover` *request* header, which
   **no client sends** (grep across `apps/web/src` + `rebuild/web/src`: zero hits; the Node
   front-door sets it only as a *response* header, `apps/api/src/lib/cutover/front-door.ts:243-244`).
   The `.strict()` DTO parse is the actual backstop. (Confirms G06 §2.3a, adds the dead-guard
   precision.)
5. **The Rust side owns two tables and no pipeline.** `sales_channels` + `order_events` are the
   only Rust-owned tables; both migrations live in the Node package
   (`packages/db/migrations/1780350000000/1/2`) and are applied by the **Node** release command
   (`fly.toml:15-16`). `rebuild/` contains no fly.toml, no Dockerfile, no CI job building the Rust
   binary (find/grep-verified; `.github/workflows/ci.yml` has zero cargo steps). Staging serves it
   from a separate `dowiz-rust-staging` Fly app behind the Node front-door (CLAIMED-UNVERIFIED per
   G04; prod `dowiz-rust` does not exist).

**One database, no sync problem — one data hub, two code half-hubs.** There are not two order
stores: the Rust api connects to the same Postgres via `DATABASE_URL_OPERATIONAL/SESSION`
(`rebuild/crates/api/src/config.rs:5-6`, `:184-185`) and reproduces Node's RLS GUC discipline
(`db.rs:1-60`). Node remains the single public ingress: the cutover front-door
(`apps/api/src/lib/cutover/front-door.ts`) matches each request to one of 10 surfaces, consults the
`cutover_flags` DB table (`flags.ts:111`), and either handles in Node or streams to
`CUTOVER_RUST_UPSTREAM` (inert when unset — prod's state; `front-door.ts:319-326`), with
fail-closed-to-Node for unmapped routes, auto-degrade for non-money surfaces, truthful 503 for
money surfaces, and the `x-dowiz-cutover` response header as the served-by oracle
(`front-door.ts:243-244`, `:377-424`). At any moment a surface is served by exactly one stack
against one database. That is genuinely good cutover engineering — but "one hub" is true at the
data layer only; at the code layer the kernel is the law for owner transitions on staging, and for
nothing in production.

### 3.3 Rust entry-points inventory (VERIFIED from `main.rs` + routers)

Public S1 (mounted unconditionally, `main.rs:631-699`): health, OpenAPI, public menu/info/theme,
storefront HTML `/s/{slug}[/cart|/checkout|/order/{id}]`, per-slug manifest, images/media,
voice-config, VAPID key, rates, robots/sitemaps. Auth-gated (`main.rs:125-282`): S2 auth; S3 owner
catalog (products, categories, **channels**, customers, couriers, invites, locations, menus,
themes, GDPR, onboarding); S5 orders (`orders/mod.rs:674-693`); S6 `/ws` + PgListener fan-out; S7
courier (shifts/assignments/me/settlements/dispatch); S8 Telegram webhook (**auth-gate-only** — it
200s without processing the update body; command dispatch is an unclosed follow-up,
`routes/telegram_webhook.rs:40-46`); S10 admin/internal.

**Inbound order sources in the Rust hub: exactly one** — `POST /api/orders` (web checkout).
`telegram-tma`, `kiosk`, `widget`, `agent` exist only as attribution labels on that one door. The
e2e test's `?channel=` routing and `orders.sales_channel_id` (`sovereign_core_e2e.rs:294-346`)
describe a design that does not exist in code — attribution is header→metadata only. Attribution
counting joins `orders.metadata->>'channel' = sales_channels.kind`
(`routes/owner/channels/pg.rs:62-69`): per-**kind**, not per-channel-row — two same-kind channels
at one location would double-count each other's orders, and the generated 256-bit per-channel
`token` (`channels/pg.rs:252-257`) is never consulted at order time (2.1 unbuilt). The
channel_attribution module also carries a stale doc note ("there is NO sales_channel table… the DB
is frozen", `mod.rs:10-11`) now false since migration 1780350000000 — a small but telling doc-drift
marker.

### 3.4 Honest answer: is there ONE hub today?

**At the data layer, yes; at the code layer, no — and in production, the hub is the Node app,
full stop.** Production runs 100% Node (0% cutover, audit §8); every prod order flows §3.1. The
Rust kernel is the *designated* hub and is a better one on paper (typed commands, corridor
refusals, event log, exhaustive fold), but today it is: staging-only, serving checkout without its
own front door (§3.2 finding 1), logging events for a subset of transitions with a placeholder
cause chain, gated by a flag that gates nothing, validated by a parity job and an e2e suite that
cannot fail, and deployed by artifacts that live outside its own tree. Meanwhile both stacks share
one Postgres, one state machine (byte-frozen), one idempotency table, and one channel taxonomy —
which is why the honest phrase is **two half-hubs on one spine**. The single most important
architectural fact for the operator: **nothing that matters about "one hub" is blocked on Rust** —
the Node hub already has the one-intake + write-only-attribution shape; what it lacks is readers
(dashboard), doors (channels beyond web), and the courier/notification loop polish (§4-5).

---

## 4. The courier side — the vendor-owned-courier loop

This is half the operator's question, and the honest headline is: **the courier backend is the
most complete, most council-hardened vertical in the product — and it is live in prod — but the
loop leaks at three human touchpoints: couriers get no out-of-app notifications, the courier UI
lags the backend by a full design generation, and there is no courier compensation model.** It is
also, notably, absent from every "hub" design document (§1 verdict).

### 4.1 Onboarding / invite — VERIFIED, live

- Couriers are a **separate auth universe**: own `couriers` table with encrypted PII + argon2id
  (`packages/db/migrations/1780421029538_couriers.ts:5-18`), per-location membership
  `courier_locations`, rotating-refresh `courier_sessions` with reuse-detection →
  whole-family revocation (`routes/courier/auth.ts:354-476`), append-only audit log.
- Owner mints invites in the console (`CouriersPage.tsx:144-168`): `POST
  /api/owner/locations/:id/courier-invites` — argon2-hashed 16-hex code, 48h TTL, role
  allowlisted to `'courier'` so an invite can never mint an owner
  (`owner/courier-invites.ts:27-84`, F4 fix). Courier redeems at `/courier-invite/:inviteId` →
  `POST /api/courier/auth/invites/:inviteId/redeem` (`courier/auth.ts:23-156`), gets a 14d JWT
  `{sub, role:'courier', activeLocationId, jti}`.
- **Smell #1 (VERIFIED): a duplicate LEGACY invite system is still registered** — un-prefixed
  `POST /couriers/invites` (`routes/couriers.ts:8-53`, wired `bootstrap/routes.ts:96`) redeemed
  via `POST /api/auth/courier/activate` → SECURITY DEFINER `activate_courier()` creating a
  **different identity** (`users` row + `memberships`, mig `1790000000082:8-28`) whose JWT is
  incompatible with every `/api/courier/*` route. No FE calls it. Dead-but-reachable duplicate
  onboarding surface.
- **Smell #2 (VERIFIED, same failure class as G11's `/claim` 404):** `/courier-invite` is missing
  from `SPA_ROUTES` (`server.ts:858`); invite deep links are saved only by the `accept: text/html`
  OR-branch — any client not sending that header (curl, some in-app webviews, unfurlers) gets 404.

### 4.2 Assignment / dispatch — VERIFIED, live, owner-driven

**Model: the owner picks (or the system auto-picks); couriers do not self-claim.** (The G11
"/claim 404" is the *vendor acquisition* funnel, not a courier flow — grep confirms no courier
order-claim route exists.)

- **Manual assign**: `POST …/orders/:orderId/assign-courier` (`owner/dashboard.ts:215-376`) —
  status-guarded, terminalizes any existing binding first (C-3), displaces a busy courier via a
  state-machine revert (D2), auto-opens a shift if none. Flag fork: `COURIER_OFFER_HANDSHAKE_
  ENABLED` ON → insert `status='offered'` + `task_offered` WS + TTL; OFF (today) → force
  `'accepted'` and drive the order to IN_DELIVERY.
- **Honest auto-dispatch**: owner PATCH → IN_DELIVERY runs `attemptHonestDispatch`
  (`lib/dispatch.ts:11-57`): freshest-heartbeat available on-shift courier; **no courier → the
  order does not advance** (the "no-trap" F1 red-line).
- **Durable redispatch**: journal `courier_dispatch_queue` (mig `1780421100044`) written on every
  reject/decline/cancel/abort + offer/accept expiry; `CourierOfferSweepWorker` (1-min cron,
  4 passes incl. 5-min accept-timeout and flag-dark grace-cancel,
  `workers/courier-offer-sweep.ts:9-84`) pumps into pg-boss `COURIER_DISPATCH`;
  `CourierDispatchWorker` executes with constraint-race handling and exhaustion →
  `dispatch_exhausted_at` + owner alert + honest customer DISPATCH_DELAYED push
  (`workers/courier-dispatch.ts:29-174`).
- **Runtime home matters**: all courier workers run in the **API web process**
  (`server.ts:346-351` → `bootstrap/workers.ts:60-92`), not the separate Fly worker (which owns
  only `order.timeout`, `apps/worker/src/handlers.ts:15`). So the stopped prod worker machine
  (G11 S5) does **not** kill courier dispatch — it kills PENDING-order auto-cancel.

### 4.3 Delivery lifecycle + deliver-v2 cash-as-proof — VERIFIED, live, design honored

- **Assignment state machine, DB-enforced** (mig `1790000000073:8-34`): `offered →
  assigned/accepted → picked_up → delivered | cancelled | rejected | offered_expired`, partial
  unique ≤1 active binding per order (the C-1 "0% redispatch" fix) + one active assignment per
  courier.
- **Courier endpoints** (`routes/courier/assignments.ts`): list/accept/reject/picked-up/
  delivered/cancel (5-min regret window)/abort (en-route, no time gate)/decline. Cancel and abort
  share one rail `releaseBindingAndReoffer` (`lib/bindingRelease.ts:18-56`): post-pickup → honest
  CANCELLED; pre-pickup → revert READY + re-enqueue (closes the "customer trapped on stale
  IN_DELIVERY" bug).
- **Cash-as-proof shipped as designed** (memory `deliver-v2-cash-as-proof-2026-06-28` verified
  item-by-item in code): single completion primitive `completeDelivery`
  (`lib/deliveryCompletion.ts:52-133`) with first-class `payment_outcome` enum (`paid_full |
  delivered_prepaid | refused_goods | refused_payment | customer_cancelled_on_door`;
  `paid_partial` structurally unrepresentable); `paid_full` requires `cash_amount === total` else
  422 CASH_AMOUNT_MISMATCH; refusal tails → honest CANCELLED. Every paid_full writes an
  append-only `courier_cash_ledger` **'hold'** row (`'release'/'settle'` reserved-unwritten, mig
  `1790000000028:4-8`); immutable `delivery_trace` crumb with GPS/route/price snapshots + 7-day
  anonymizer + retention cron. Owner-proxy `/deliver` and `/pickup` use the same primitive
  (parity). Drift guardrail `scripts/guardrail-deliver-v2.mjs` in verify:all.
- **Who records COD payment:** the courier (`/delivered`) or the owner (proxy). **Settlement of
  the holds is design-only**: Stage-21 reconciliation ADR is explicit "NO production code —
  awaiting human sign-off" (`docs/adr/ADR-stage21-reconciliation.md:3-13`), guarded by a
  red-on-disk NO-AUTO-DEDUCT / NO-COURIER-SCORING invariant test.

### 4.4 Realtime + authz (ADR-0013) — VERIFIED, the DoD is real in code

Single tri-state authz predicate (`lib/courier-room-authz.ts:32-66` — never throws; read-binding
includes `'offered'`, send-binding excludes it; `location:*` rooms denied to couriers). WS
subscribe gate: courier may join only `courier:<self>` and `order:<id>` with a live binding
(`websocket.ts:408-429`). C1 fan-out guard re-validates each courier's binding on **every**
order-room frame (TTL cache + `binding_revoked` eviction, `websocket.ts:242-262`,
`lib/courier-relay-guard.ts`), enforced by a custom ESLint rule `local/no-raw-courier-ws-send`.
Customer GPS relays to courier members only, guard-revalidated. E2E isolation spec exists
(`e2e/tests/courier-room-authz-isolation.spec.ts`; "6/6 green on staging v195"
CLAIMED-UNVERIFIED). Known accepted gap: no cache-bust on owner-reassign (≤TTL eviction window).

### 4.5 Location/tracking — VERIFIED, privacy-maximalist

GPS stored **only during an active delivery** (P0-1 hard gate, `courier/shifts.ts:365-413`),
rounded coords, 1/10s rate limit, 50km geofence sanity, venue-geofence sensor event; owner live
map + per-order breadcrumbs (masked PII, `owner/couriers.ts:147-248`); customer gets
`order.courier_updated` + one-per-leg road polyline (ORS with haversine degrade,
`workers/courier-events.ts:129-246`) and an honest ETA **range** (`lib/etaService.ts:1-38`);
GPS purge cron + stale-heartbeat owner alert. Gap: the courier's own task ETA is a hardcoded
`'~15 min'` string (`assignments.ts:35`).

### 4.6 The three loop-breaking gaps (ranked, all VERIFIED)

1. **Couriers get NO out-of-app notifications.** Every courier-lifecycle event notifies the
   *owner* (Telegram) or the *customer* (webpush); there is no courier device/push registration
   anywhere (push tables are customer-only, migs `1780421100059`/`1780348982033`). A courier
   learns of an assignment only via an open WS tab + in-app sound (`TasksPage.tsx:26,89`).
   Combined with the 5-min accept-timeout sweep, **a courier with a locked phone silently loses
   assignments**. For "a delivery hub with own couriers" this is the single largest product gap.
   (The bot infra to fix it exists — couriers even have `/open` in the owner bot already.)
2. **Courier FE lags the backend by a design generation.** `DeliveryPage.tsx:240-244` always
   sends `payment_outcome:'paid_full'` with `cash_amount = task.total` — the refusal tails and
   `/abort` have **no courier UI** (grep: zero `abort`/`decline` calls under
   `apps/web/src/pages/courier/`); `task_offered` and `binding_changed` WS events have **no FE
   handler**; `/me/assignments` excludes `'offered'`. Consequence: the offer-handshake flag is
   backend-complete but **cannot be flipped** without FE work, and the cash-as-proof edge cases
   are exercisable only via the owner proxy.
3. **No courier compensation model.** Grep for courier fee/rate/compensation across api +
   migrations is empty. `courier_payouts.total_earned = Σ cash_amount` — i.e., **cash the courier
   owes the venue**, mislabeled "earnings" (`mig 1790000000078:160-191`, daily 02:00 cron);
   owner approve/pay flow exists (`owner/settlements.ts:109-160+`). Real pay math is unbuilt
   Stage-21. For vendor-owned couriers this may be acceptable (salaried staff), but the hub
   cannot claim a courier *economy* — only courier *operations*.

Plus two latent defects: **`c.name` schema mismatch** — `notifications/workers/index.ts:623,648`
select `c.name` from `couriers`, which has only `full_name_encrypted` (no migration adds `name`;
grep-verified) → the `courier.assigned` and `shift.*` owner-Telegram enrichment jobs should fail
at runtime with `column c.name does not exist` (NEW finding, needs a live-log confirm);
and `cash.reconcile_discrepancy` is renderable but has **no emitter** anywhere in src (dead
wiring).

---

## 5. Owner/vendor console — what the vendor gets to run the hub

### 5.1 What a vendor can RUN today (all VERIFIED, prod code paths)

- **Order console** (`/admin`, `/admin/orders` — `pages/admin/DashboardPage.tsx`): live order list
  (`GET /api/owner/orders` with ratings/OTP/reputation signals, `spa-proxy.ts:393-449`) + WS
  realtime (owner rooms `location:<id>:dashboard`, `location:<id>:couriers`, `order:<id>` —
  `apps/api/src/websocket.ts:191-531`, subscribe + per-frame relay re-authz with 10s-TTL
  revocation-evicted cache `:35-171,266-284`). Status actions via `PATCH /api/orders/:id/status`
  (`DashboardPage.tsx:290`) with "honest dispatch" — sending for delivery with no courier on shift
  does NOT advance the order (toast, `DashboardPage.tsx:308`). Owner-scoped
  confirm/reject/assign-courier/pickup/deliver/verify endpoints
  (`routes/owner/dashboard.ts:193,203,215,379,447,539`), courier live map, preset customer
  messages.
- **Menu management** (`MenuManagerPage.tsx`, 1405 lines): product/category CRUD (14+8 endpoints),
  menu import preview/commit, availability/stop-list, modifier groups, media, translation,
  confirm gate.
- **Settings** (`SettingsPage.tsx`): store details/fees/hours/pause; **Telegram connect** (deep
  link + QR + Send-Test, `:236-253`); notification target enable/disable; category
  preference-centre UI **dark** until `VITE_TG_CATEGORY_GATING` (`:577-613`); customer-facing
  fallback phone + dead-channel degradation report (`owner/fallback.ts:21-111`).
- **Activation/publish gate**: a storefront cannot go live without ≥1 active notification target
  (`routes/owner/activation.ts:12-49`) — a genuinely hub-coherent invariant (no silent vendor).
- **Plus**: Couriers page + invites, Promotions (UI-only — `discountTotal=0` hardcoded, the audit's
  "POTEMKIN" item), Analytics (revenue/top-products/heatmap — **no channel dimension**), CRM
  (customers), Branding/theme, Supplies, GDPR/refunds/settlements surfaces.
- **Customer-side tracking page exists** (`/s/:slug/order/:id`, `OrderStatusPage.tsx`, 798 lines):
  status timeline, 15s poll + WS room `order:<id>` with live courier position (masked phone) and
  road polyline; opaque track-token (sha256-hashed, 14-day) minted transactionally at order
  creation and exchanged for a customer JWT (`order-persistence.ts:148-160`,
  `routes/customer/track.ts:28-86`).

### 5.2 The owner's Telegram surface (the "run it from your pocket" half)

LIVE today: `/start <token>` connect (notification target upsert,
`telegram-webhook.ts:605-644`), `/start login_<token>` full Telegram web login (`:574-603`),
order **confirm / reject-with-reason** inline buttons executing the real state machine with
tenant GUC (`:322-387,453-491`), `/open` (courier shift), `/stop`. DARK behind default-off flags:
`/store` open/close toggle (`TG_STOREFRONT_ACTION`), `/settings` category toggles
(`TG_CATEGORY_GATING`), Mini-App menu button (`TMA_ENABLED`). Whether any flag is ON in prod is
CLAIMED-UNVERIFIED (Fly secrets not readable this session; fly.toml carries no `[env]` for them).
There is **no `/claim` bot command** — claiming is the web flow, currently 404-broken at its entry
(G11).

### 5.3 Notification fan-out (the outbound half of the hub) — VERIFIED

```
order event ──(same txn)──▶ pg-boss NOTIFY_TELEGRAM_SEND (dedup singletonKey)
MessageBus channels ──▶ bootstrap/messaging.ts bridges bus→queues
worker ──▶ SELECT owner_notification_targets WHERE location & channel='telegram' & active
        ──▶ prefs/category gate → quiet-hours → circuit breaker → rate limit → adapter
        ──▶ every decision audited to notification_outbox_audit (no_target/prefs_disabled/
            held/quiet_tz_fallback/circuit_open/delivered/failed)
```

- **21 owner events** in the registry (`notifications/event-registry.ts:16-167`): order.created
  (Confirm/Reject buttons), pending_aging, confirmed/rejected/delivered/timeout_cancelled/
  dispatch_failed/dwell_escalation/ready_for_pickup, cash.reconcile_discrepancy,
  delivery.flag_raised, rating.low_received, courier.assigned (Track button), shift.started/
  closed/close_reminder, ops.* liveness/backup/degradation, test.
- **Category gating** (dark behind `TG_CATEGORY_GATING`): transactional never suppressible;
  categorization principle "reversibility of consequence" (`event-registry.ts:175-184`);
  timezone-aware quiet hours with held-once-then-deliver semantics ("zero silent drops",
  `workers/index.ts:241-263`); flag OFF → legacy hardcoded UTC 22:00-08:00 drop (`:265-271`).
- **Adapters**: Telegram (auto-disable target on 401/403, honor 429 retry-after,
  `adapters/telegram.ts:14-53`); WebPush owner+customer (`adapters/webpush.ts:7-85`); Email is
  **deliberately not a tenant channel** (platform-ops only via Resend; the DB CHECK allows only
  `('telegram','push')`, migration `1780348982032:8`); `adapters/push.ts` is a dead fake-ack
  scaffold.
- **Customer pushes** are a separate path (`customer_devices`, events CONFIRMED/IN_DELIVERY/
  DELIVERED + delay/cancel variants, `workers/index.ts:43,99-200`).
- **Couriers receive no notifications at all** — only WS `courier:<id>` room messages (see §4).
- **Latent defect (NEW, VERIFIED):** `bootstrap/messaging.ts:9-27` enqueues `'backup.failed'` and
  `'settlement.disputed'` — event names that do not exist in the registry (`ops.backup_failed` is
  the real name); the renderer's default arm returns a raw string, the adapter destructures
  `{text, reply_markup}` → `text: undefined` → Telegram 400. These two ops alerts are **broken by
  construction**, and the boot-time `assertVocabulary()` does not check event names. Also the
  `messaging.ts:19` comment says "notify courier" but the handler only targets owner chats —
  comment-level evidence of the courier-notification gap.

### 5.4 The attribution dashboard — the hub's missing mirror

**CONTRADICTED nuance vs the "1.5 endpoint-only" shorthand:** in the **Node prod app there is no
attribution read surface at all** — grep for `->>'channel'`/aggregation across `apps/api` finds
zero readers; `AnalyticsPage.tsx` consumes only revenue/products/heatmap. The Phase-1.5
`GET …/channels/with-attribution` endpoint exists **only in the Rust staging tree**
(`rebuild/crates/api/src/routes/owner/channels/mod.rs:131-150`), counting per-kind (§3.3), with no
UI anywhere (G06 bonus finding: no channels UI in `apps/web/src` or `rebuild/web`). So the honest
status ladder is: prod = capture-only, dark QR-kit page; staging = capture + dormant registry +
endpoint; nowhere = a screen a vendor can look at. The exit-gate clause "(3) see it attributed in
their dashboard" is unsatisfiable by any UI today.

### 5.5 Live spot checks (read-only GETs, 2026-07-11, this session) — VERIFIED

| Path | dowiz.fly.dev (prod) | dowiz-staging.fly.dev |
|---|---|---|
| /health · /livez | 200 · 200 | 200 · 200 |
| /s/sushi-durres (SPA shell) | 200 | 200 (shell 200s for any slug) |
| /public/locations/sushi-durres/menu · /info | 200 · 200 | **404 · 404** |
| /public/locations/demo/info | 200 | 200 |
| /public/locations/test-slug/info | **200** (test data live in prod DB) | 404 |

Combined with G11's probes (11 of 12 outreach demos 404 on prod; `GET /claim` 404 on BOTH prod and
staging; prod worker machine stopped since 07-03): the prod hub is healthy as a server and broken
as a funnel.

---

## 6. Architecture review proper

### 6.1 Strengths (genuine, evidence-backed)

1. **The minimal hub shape is already correct in production.** One canonical intake, one
   transactional pipeline, server-authoritative integer pricing, tenant-scoped idempotency,
   write-only channel attribution (§3.1). Most "omnichannel" systems get this wrong by growing N
   intakes; dowiz's doctrine ("every channel is a wrapper around the same link") means adding a
   channel never adds a money surface. This is the single best architectural decision in the
   project.
2. **The single-money-surface / cart-token doctrine** (REBUILD-MAP §6 inv.1, research §14) is
   both principled and market-correct: Meta killed native checkout, Order-with-Google is dead,
   Telegram exempts physical goods — the industry converged on merchant-checkout link-out. One
   checkout also means one fiscalization point (the Albania legal blocker).
3. **Byte-frozen shared invariants across the two stacks**: identical 10-state machine
   (`order-machine.ts:18-40` ≡ `order_status.rs:57-62`), integer money both sides, one Postgres,
   one idempotency table, one channel taxonomy. The split (§6.2.1) is real but it is a split
   above a genuinely shared spine.
4. **The kernel as a library is excellent**: pure `decide/fold/replay`, corridor refusals over
   workarounds, exhaustive-fold compile discipline, wasm/clippy purity gates, typed
   `PlaceOrder` that cannot carry a price. When the shell finally honors it, it will be worth
   having.
5. **Deliver-v2 is council-grade domain modeling**: `payment_outcome` as a first-class enum with
   `paid_partial` structurally unrepresentable; cash HOLD ledger rows as proof-of-delivery;
   honest CANCELLED over false DELIVERED; one completion primitive shared by courier and owner
   proxy (§4.3). The courier realtime authz (per-frame re-validation + lint-enforced guard usage)
   is beyond what most production systems do.
6. **Operational honesty as a design value**: honest dispatch (no courier → order doesn't
   advance), honest customer ETA ranges, notification audit ledger with zero-silent-drop quiet
   hours, activation publish-gate requiring a live alert channel, truthful 503 on money surfaces
   in the cutover front-door.
7. **Economic discipline in channel sequencing**: attribution before investment; every Tier-1
   channel €0-recurring; paid channels (Viber €100/mo) demand-gated behind a vendor survey;
   "channel count = ops budget" with per-head kill switches and governance cards.
8. **Privacy as a differentiator**: WhatsApp notification channel removed on ToS/PII grounds;
   GPS only during active delivery; courier PII encrypted; FORCE-RLS everywhere; per-slug
   sessionStorage attribution that cannot become a tracking cookie.

### 6.2 Weaknesses / risks (ranked by expected damage)

1. **The two-half-hub split, and the verification theater around it.** The declared hub (Rust
   kernel) is not the operating hub (Node), and the bridge between them is decorated with proofs
   that cannot fail: a flag that gates nothing, a parity job that only detects orphaned logs, an
   e2e suite whose tests are comment-bodied or assert on empty vectors, a "launch gate" guard on
   a header no client sends (§3.2). The GRAND-PLAN's own adversarial critique predicted exactly
   this ("if replay-parity ever goes advisory… the event log decays into decoration… or 1.2 was
   theater"). Worse, the Rust checkout bypasses `decide` entirely — so even a successful staging
   cutover of S5 would NOT put orders through the one door. Risk: the kernel rots as a parallel
   truth while consuming red-line review budget. The Rust core also runs in **zero CI**.
2. **Scattered channel/contact definitions — the messenger-kind incident is a symptom, not a
   bug.** The kind list exists in 4 TS sites + 2 Rust sites + 2 SQL CHECKs (G03 §2.1); the
   channel allowlist is triplicated (api lib / web lib / DB CHECK) with a TODO; the Rust
   attribution module carries a stale "no sales_channel table" note; notification event names
   drift (`backup.failed` vs `ops.backup_failed` — broken by construction, §5.3). Root process
   cause: the wire contract lives in a protect-paths-frozen file (`packages/shared-types`) while
   FE, DB, and Rust moved on. Until there is ONE source of truth per vocabulary with a
   mechanical parity gate (the 1.1 CHECK-mirror test exists in Rust but runs in no CI), every
   14th value added anywhere will break somewhere else.
3. **Three different things are all called "channel", and they interact confusingly.**
   (a) inbound attribution taxonomy (13 values, write-only); (b) owner notification channels
   (`telegram|push` in owner_notification_targets); (c) customer contact messenger kinds
   (6 values, ADR-0016). `whatsapp` exists in (a) and (c) but was deliberately removed from (b);
   `telegram` means an ops bot in (b), a contact deep-link in (c), and a Mini-App storefront
   wrapper in (a). The conflation has already produced doc errors ("notify courier" comment on an
   owner-only path) and makes the operator's question ("orders from messengers") ambiguous inside
   the codebase itself. The hub vocabulary needs one page that separates intake transports,
   attribution labels, contact preferences, and notification targets.
4. **Attribution is captured and then dark — the measurement loop is open.** Zero readers of
   `metadata->>'channel'` in prod (§5.4). The design's whole economic argument ("attribution
   before investment", "the measurement layer that makes every ranking falsifiable") is
   inoperative until one screen shows counts-by-channel. Also, counsel's warning stands: a
   broken QR silently becomes `other`/`web-direct` — with no reader, nobody would ever notice.
5. **The courier loop leaks at the human edges** (§4.6): no courier push/Telegram (locked phone =
   silently lost dispatch after the 5-min sweep), courier FE a generation behind the backend
   (offer-handshake unflippable; refusal tails unreachable from the courier UI), no compensation
   model, `c.name` enrichment bug, hardcoded courier ETA. The backend quality here makes the
   FE/notification gap look like an oversight of sequencing, not skill.
6. **Prod bugs sit at exactly the revenue edges while the engine idles**: checkout 400s for 3/6
   contact options and all "deliver to someone else" orders (G03, live since 07-03); `/claim`
   404 (G11); 11/12 demos staging-only; prod worker machine stopped (PENDING auto-cancel dead);
   GDPR trio unmerged. None of these is hub *architecture* — all of them make the hub *claim*
   false for a real vendor today.
7. **At 10 venues** the design mostly holds (per-location RLS, per-location notification
   targets, per-location couriers, WS rooms, journal dispatch are all location-scoped;
   "channel count = ops budget" is the right governor), with four foreseeable pressure points:
   (a) no cross-venue owner view (each location is its own console; a 10-venue restaurateur has
   10 dashboards — the `aggregator_view` stub is about foreign platforms, not own-fleet
   aggregation); (b) courier dispatch workers live in the API web process (`bootstrap/
   workers.ts:60-92`) — fine now, a noisy neighbor under real load; (c) owner-driven manual
   dispatch doesn't scale past a handful of simultaneous orders without the offer-handshake +
   courier push actually shipping; (d) the notification fan-out is per-target sequential with
   circuit breakers — adequate, but nobody has load-tested 10 venues × dinner rush (the
   rate-limiter/E2E conflict on staging suggests limits will need tuning).
8. **Claim/reality drift is a cultural risk the repo knows about** (audit §7.2, G13): "MVP is
   SHIPPING-READY" vs 5/12 phases + unverifiable gates; "1.5 ✅" while no UI exists and the Node
   prod has no endpoint at all; stale code comments asserting the opposite of reality. The gap
   blueprints have started correcting this; this review's §2/§3 tables are written to be
   re-checkable.

### 6.3 Mission fit: "one hub, many order sources, vendor-owned couriers" — scored

| Claim component | Status today (prod, real vendor) | Verdict |
|---|---|---|
| **One hub** | One intake, one pipeline, one DB — but the *designated* hub (kernel) is a staging shadow and the operating hub carries the G03 contract rot | **TRUE in shape, split in implementation** |
| **Orders from web** | Live, with a 400-bomb under 4 of its checkout permutations | **TRUE with a live leak** |
| **Orders from messengers/bots** | Zero intake transports. Telegram = owner ops; TMA = dark owner self-preview; WhatsApp = paper; Instagram = absent even as a contact kind; voice = cart-assist only; MCP/agent = paper | **FALSE today; designed, not built** |
| **Orders from QR/physical** | Plumbing 100% live; owner kit dark behind a flag; no reader to prove it | **LATENT — days from true** |
| **Aggregators** | Deliberately excluded from intake (D1); read-only view unbuilt | **N/A by doctrine (correct call)** |
| **Vendor-owned couriers** | Invite→shift→assign→deliver→cash-proof loop live and hardened; but couriers are deaf when the app is closed, the courier UI can't express half the backend, and "earnings" = cash owed | **TRUE operationally, leaking at the edges** |
| **Vendor runs it from one place** | Owner console + Telegram actions live; attribution mirror missing; channels UI missing; multi-venue view missing | **TRUE for one venue, minus the channel lens** |

### 6.4 What is genuinely MISSING, ranked by distance-to-value

(1 = shortest distance, highest value for making the operator's sentence true for a real vendor
this month)

1. **The checkout contract fix** (G03: 6-kind enum + `receiver{}` + the `payment.method` widening
   while at it) — ~15 LOC in one operator-gated file; unblocks 3/6 contact choices + gift orders
   on the ONLY working channel. Without it every other channel investment leaks at the same
   funnel step.
2. **The funnel repair** (G11 Wave 0/1: `/claim` in SPA_ROUTES + `/courier-invite` same fix +
   demos to prod + restart worker + rotate PROVISION_OPS_SECRET) — the hub is unreachable by new
   vendors until this lands.
3. **Courier out-of-app notification** — a courier Telegram target (the bot infra, deep-link
   connect flow, and even the courier `/open` command already exist) or courier webpush. This is
   the missing half of "own couriers" and it is NOT in any existing gap blueprint — the one net-new
   build item this review adds. Effort M; reuses `owner_notification_targets` pattern (add
   `courier` audience or a `courier_notification_targets` table + `task_assigned`/`task_offered`
   events). Fix the `c.name` bug in the same pass.
4. **An attribution reader** — one "Orders by channel" card in AnalyticsPage reading
   `metadata->>'channel'` (the sanctioned reader per the ADR), + flip `VITE_CHANNEL_KIT_ENABLED`.
   This closes the measurement loop, makes the QR kit shippable to a real venue, and satisfies
   exit-gate clause (3) in its Node form. Effort S.
5. **The Tier-1 channel kits** (research §15, all S): Instagram 7th messenger-kind + story kit;
   GBP/Apple links playbook; WhatsApp free-tier wa.me playbook. All are ops/link artifacts around
   the one URL — no new transport, no council.
6. **Rust exit-gate honesty** (G06 Option B): real `hub_checkout` gate, real replay-parity, real
   `cause_hash`, non-vacuous Playwright, then 2.1 distribution artifacts + the channels UI —
   **plus, from this review: route the Rust checkout through `Command::PlaceOrder → decide`
   before any S5 prod flip**, otherwise the flip ships a hub that still bypasses its own law.
7. **The first true second intake transport** (WhatsApp Cloud API webhook per EXPANSION-PLAN 1.C,
   or a Telegram customer bot) — only AFTER the G7 vendor survey provides demand evidence, and
   only through the cart-token council. Distance-to-value is long (weeks + council + Meta
   verification) and the evidence says the market may not need it before QR does its work.
8. **Courier compensation (Stage-21)** — needed for courier *economy* claims; correctly gated on
   the NO-AUTO-DEDUCT ethics ratification; not this month.

---

## 7. Recommendation — making the hub claim true for one real vendor

The gap blueprints' MASTER-EXECUTION-PLAN already sequences most of what matters; this review
**adopts its waves rather than inventing new ones**, and adds exactly two hub-specific items the
blueprints missed (courier notifications; the attribution reader) plus one red-line correction to
G06 (kernel checkout routing). The sequencing principle: *make the ONE working hub honest before
adding doors; close the courier loop before adding channels; add measurement before adding
transports.*

### 7.1 Sequenced path

**Now (Wave 0/1 per MASTER-EXECUTION-PLAN — days):**
- Ride the Wave-1 prod PR as planned: GDPR trio + anonymizer DI fix + **G03** (6-kind enum +
  `receiver{}`; consider widening `payment.method` in the same diff) + **G11** `/claim` in
  SPA_ROUTES + OG cards + demo provisioning; Wave-0 ops: restart the prod worker, rotate
  `PROVISION_OPS_SECRET`. **Add from this review:** `/courier-invite` to SPA_ROUTES (same
  one-line class as `/claim`, §4.1), and the 2-line event-name fix `backup.failed`→
  `ops.backup_failed` / `settlement.disputed` (§5.3), and the `c.name`→`full_name_encrypted`
  courier-enrichment fix (§4.6).
- **Validation week (Wave 2, operator-personal)**: concierge one venue exactly per G11 days 3-7 —
  and hand that venue the QR kit: flip `VITE_CHANNEL_KIT_ENABLED` in the same build and print the
  A6 tents. GREEN metric unchanged: a real order row from a non-operator customer on a claimed
  venue.

**Next (1-2 weeks, parallel to validation):**
- **Attribution reader** (S): "Orders by channel" in AnalyticsPage over `metadata->>'channel'` —
  the sanctioned reader; closes the measurement loop; makes the QR kit falsifiable (an `other`
  spike = broken QR).
- **Courier notification slice** (M): courier Telegram connect (reuse the deep-link
  connect-token pattern + the existing bot) or courier webpush; deliver `task_assigned` /
  `task_offered` / READY out-of-app. This is the highest-leverage courier item and the
  prerequisite for ever flipping `COURIER_OFFER_HANDSHAKE_ENABLED`.
- **Courier FE catch-up** (M): offered-task list + `/decline` + `binding_changed` handling +
  non-`paid_full` outcomes in DeliveryPage — then the offer handshake can actually be flipped on
  staging.
- **Tier-1 channel kits** (S each, per research §15): Instagram messenger-kind + story kit,
  GBP/Apple link playbook, wa.me playbook. All link artifacts, no transports, no councils.

**Then (the Rust hub, per G06 Option B — with one amendment):**
- Execute G06 Phase 1 (real `hub_checkout` gate, real replay-parity, real `cause_hash`,
  repaired Playwright) and Phase 2 (2.1 distribution artifacts + channels UI completing 1.5).
- **Amendment (red-line, from §3.2 finding 1):** before any S5 prod flip, route the Rust checkout
  through `Command::PlaceOrder → kernel::decide` and dual-write `Priced` — otherwise the cutover
  ships a hub that bypasses its own law. Add the customer-cancel and courier transition callers
  to the same door in the following pass. PARK 1.3/2.4 with dated markers exactly as G06
  recommends.
- Put the sovereign gate + cross-stack vocabulary parity checks (channel allowlist, messenger
  kinds, event names) into CI (0b-6) — the mechanical fix for weakness §6.2.2/3.

**The single next channel that earns its keep (evidence-based):** the **QR/NFC kit** — research
§15 Tier-1 #1: lowest friction of any channel (scan→menu ~3-5s), perfect for a dine-in/walk-in
cash-first market, immune to the unresolved Albania messenger-penetration question (R9), already
demanded by the trust research, and 95% built (plumbing live, page built, flag off). The first
*messenger transport* (WhatsApp Cloud API per EXPANSION-PLAN 1.C) is third in line at best, and
only after the G7 10-15-vendor survey — the research is unambiguous that betting engineering on
any specific messenger is currently unfounded.

### 7.2 What to explicitly NOT build (each has a named re-entry trigger)

- **Aggregator ingestion** (Wolt/Glovo intake) — doctrine-excluded (D1); even the read-only stub
  (2.4) is parked per G06. Re-entry: operator + money council.
- **Viber bot** — €100/mo floor against LOW-MED evidence. Re-entry: G7 survey names it.
- **Conversational dialogue FSM / Telegram customer ordering bot / WhatsApp Flows** — no
  transport before survey demand; cart-token council first. Re-entry: G7 + money council.
- **Cart-token implementation** — no channel that pre-builds intent exists yet; the spec is the
  council packet, not a build license (07-channel-hub §3). Re-entry: first conversational/agent
  head approved.
- **1.3 sync port, CRDT, libp2p, signing/bebop2 coupling, PQC** — Phase-3 seams; G06 §2.6 is
  explicit that bebop2's unaudited crypto must not guard money/identity.
- **Voice ordering, kiosk mode, custom domains, App Clips** — Tier-2/3 or dead per research;
  none moves the first-real-order metric.
- **A second dashboard for channels in the Rust/Astro FE before the Node reader exists** — the
  vendor lives in the React console today; ship the mirror where the vendor is.

### 7.3 The one-sentence verdict

The "one hub" exists as a disciplined doctrine and a live single-intake production system with a
genuinely hardened vendor-owned-courier backend; but today it is a hub with **one door, no
channel mirror, deaf couriers, and a broken front gate** — and the fastest path to making the
operator's sentence true is not the Rust kernel or any new messenger, it is: fix the gate (G03 +
/claim), hand one venue a QR kit with a channel counter, and give couriers a beep.

---

## Appendix — evidence index

- **Fresh code review this session (VERIFIED):** apps/api order/channel/notification/courier
  paths; apps/web checkout/console/courier pages; rebuild kernel + api routes + modules;
  packages/db migrations; packages/config flags; live GETs against dowiz.fly.dev and
  dowiz-staging.fly.dev (status codes only). Key anchors are inline throughout.
- **Inherited and cited, not redone:** audit `2026-07-11-full-project-audit-dowiz-bebop.md`
  (§1, §2.1, §4.2, §5.1-5.3, §6.2-6.4, §7.2, §7.8, §8, §9); gap blueprints G03 (checkout
  contract), G06 (sovereign exit gate), G11 (validation week), MASTER-EXECUTION-PLAN (waves);
  design corpus per §1 table; memory corpus (sovereign-core-mvp-handoff, deliver-v2-cash-as-proof,
  adr-0013, telegram-notifications-council, voice-control-council).
- **New findings first reported here:** Rust checkout bypasses `kernel::decide`
  (§3.2.1); `x-dowiz-cutover` price-field guard is dead code for real traffic (§3.2.4);
  Node prod has NO attribution endpoint (the 1.5 endpoint is Rust-only) (§5.4);
  `backup.failed`/`settlement.disputed` notifications broken by construction (§5.3);
  couriers have zero out-of-app notifications + `c.name` schema mismatch in courier
  enrichment (§4.6); `/courier-invite` missing from SPA_ROUTES (§4.1); legacy duplicate
  courier onboarding still reachable (§4.1); attribution counts join per-kind, not per-token
  (§3.3); `courier_payouts.total_earned` is cash-owed, not earnings (§4.6).

*Prepared 2026-07-11 by a read-only review session. The only file created by this session is this
report; the working tree, branches, staging, and prod were left exactly as found; all probes were
read-only GETs. (Observed but not touched: a concurrent plan-auditor lane wrote
`docs/design/plan-audit-dowiz-2026-07-11.md` and `docs/design/plan-audit-memory-2026-07-11.md`
into the same tree during this session's window — different topic, not this review's output.)*
