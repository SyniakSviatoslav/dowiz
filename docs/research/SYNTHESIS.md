# DeliveryOS Repo-Teardown — SYNTHESIS

> Main actionable output of the 5-repo + Bolt-UX teardown. Every non-trivial claim cites a dossier
> file + section (`repo-N §Rx`) or a teardown section (`teardown §X`). Lens for **every** call below:
> DeliveryOS is **white-label, PER-RESTAURANT** (`/s/:slug`), **not** a marketplace aggregator.
>
> **Stack of record:** Fastify monolith + Vite SSR + pg-boss worker; Supabase PG17; own `ws`
> (Supabase Realtime OFF); React 18 PWA (client/owner/courier, one codebase, role-routed);
> Zustand + TanStack Query v5; shadcn/ui + Tailwind, colours via `var(--brand-*)`; Zod strict shared;
> cash-on-delivery MVP; RLS (`FORCE`) tenant isolation keyed on `location_id`;
> schema `organizations → locations → (menu/category/item/modifier/order/customer/courier)`;
> integer-minor-unit money; Albania mobile-first; brand = logo + colour; fallback-to-venue-phone on
> any failure; no cookies; i18n al/en.

## REAL DeliveryOS order-state machine (verified against source — used in S2/S3)

Source: `packages/domain/src/order-machine.ts`; enum mirrored in
`packages/shared-types/src/legacy.ts:82` and the PG type
`packages/db/migrations/1780310044710_extensions-and-enums.ts:14`
(`CREATE TYPE order_status AS ENUM (...)`).

**10 states:** `PENDING · CONFIRMED · PREPARING · READY · IN_DELIVERY · DELIVERED · REJECTED ·
CANCELLED · SCHEDULED · PICKED_UP`.

**Legal transitions** (`TRANSITIONS` table, enforced by `assertTransition()` *before* the SQL
UPDATE, then re-enforced by a status-guarded `WHERE status = $current` UPDATE — anti-race):

```
PENDING     → CONFIRMED | REJECTED | CANCELLED
CONFIRMED   → PREPARING | IN_DELIVERY
PREPARING   → READY
READY       → IN_DELIVERY | PICKED_UP
IN_DELIVERY → DELIVERED
DELIVERED / REJECTED / CANCELLED / PICKED_UP = terminal
SCHEDULED   = scaffold (ScaffoldDisabledError — pre-order flow not implemented)
```

Terminals: `DELIVERED, PICKED_UP, REJECTED, CANCELLED` (`isTerminal()`).
Per-transition timestamps already stamped on `orders`: `confirmed_at`, `ready_at`, `delivered_at`
(migrations `1780310074262_orders`, `1780695000000_order_timelines`). Append-only audit:
`order_status_history (from_status, to_status, actor, created_at)` (`1780338982015_order_history`).
Two delivery branches exist: courier delivery (`IN_DELIVERY → DELIVERED`) and **pickup**
(`READY → PICKED_UP`, a live terminal — *not* scaffold).

---

## S1 — Conventions catalogue (Bolt teardown A1–A10, B, C, D, F)

Verdict legend: **HONOR** = established expectation; violating it = friction. **MAY-DEVIATE** = the
per-restaurant / COD niche lets us simplify (justified). **N/A** = marketplace-only, irrelevant to a
single white-label venue. Justification is always *user expectation + niche*, never "a competitor does it".

### A — Client app

| Item (teardown §) | Verdict | Why (user expectation + niche) |
|---|---|---|
| **A1** Onboarding/registration (phone/OTP, profile, geo perms, saved addresses) | **MAY-DEVIATE** | A returning eater on `/s/:slug` expects to order with minimum friction; per-restaurant + COD + no-cookies means we capture *phone + delivery address at checkout*, not a full account. OTP only if abuse demands it. Multiple saved addresses, social login → **N/A** (no account store). Keep: GPS-auto address + apt/floor/courier-note fields (real delivery-accuracy need). |
| **A2** Discovery/home (address switcher, Food/Market/DineOut tabs, cuisine search, collections, venue cards) | **N/A** | All marketplace aggregation. There is exactly one venue at `/s/:slug`; cross-venue search/collections/favourites have no surface. Keep only **Delivery↔Pickup toggle** (we model both: `IN_DELIVERY`/`PICKED_UP`). |
| **A3** Venue/menu page (header, sticky category nav, in-menu search, dish cards, **closed/busy/stop-list** states) | **HONOR** | This is *the* storefront. Users expect category nav, photos, prices, and — critically — honest **open/closed/busy + sold-out** signalling. `apps/web/.../MenuPage.tsx` already centres here. Stop-list maps to `products.is_available` (`1780310072731_menu`). In-menu search → MAY-DEVIATE for tiny menus. |
| **A4** Item/customisation (modifier groups required(radio)/optional(checkbox) w/ limits, qty, note, **live price → "Add €X"**) | **HONOR** | Direct expectation; our schema already supports it: `modifier_groups(min_select,max_select,required)` → `modifiers(price_delta)` (`1780338982010_menu_modifiers`). Price-on-the-button is teardown §F#5 — honour. |
| **A5** Cart (line items + modifiers, edit/dup, **transparent price breakdown**, min-order gate, pickup switch, scheduled slot, promo, tip) | **HONOR** (core) / **MAY-DEVIATE** (extras) | Honour: line items, edit/remove, full breakdown (items/delivery/discount/tax/**total**), min-order gate (`locations.min_order_value`), tip (`order-tip` migration exists). Scheduled slot → **MAY-DEVIATE** (`SCHEDULED` is scaffold). Promo/voucher, subscription → **MAY-DEVIATE/later** (no promo engine in MVP). |
| **A6** Checkout (confirm address+instructions, contactless, time ASAP/scheduled, **payment incl. cash**, promo, tip, final breakdown → Order) | **HONOR** w/ **MAY-DEVIATE** on payment | Honour single-screen checkout (address + summary + place). **MAY-DEVIATE:** COD is the *only* method → render it as a confirmed line ("Pay cash on delivery"), drop the payment selector entirely (Enatega `repo-5 §R7`). `cash_pay_with` column lets us prompt "paying with €X". Contactless/leave-at-door → courier note (MAY-DEVIATE). |
| **A7** Order tracking (real-time states, **live map** venue/courier/dropoff + ETA, courier card, masked call, cancel window, pickup variant) | **HONOR** | The single most important retention loop (teardown §D, §F#1). Status stepper driven by our per-transition timestamps; live courier pin on MapLibre + own `ws`. Map our 10 states (above). Masked call → **MAY-DEVIATE** → tap-to-call venue phone fallback (teardown §F#6; MVP constraint "fallback-to-venue-phone"). Cancel window honoured (cancel legal only from `PENDING`). |
| **A8** Post-order (3-way rating, tip-after, review, receipt, reorder, report-a-problem, history) | **MAY-DEVIATE** | `order-ratings` migration exists → a *basic single rating* is honourable; 3-way courier+venue+dish rating is marketplace-grade → **MAY-DEVIATE/later**. Reorder/history depend on an account we don't keep (no cookies) → **MAY-DEVIATE/later**. Receipt + "report problem"→venue-phone honour. |
| **A9** Profile/account (personal data, addresses, payment methods, subscription, promo, lang, notifications, privacy, delete, referral) | **N/A / MAY-DEVIATE** | No persistent account (no cookies) → most of this is **N/A**. Honour only: **language al/en** (i18n is a hard MVP constraint) and **privacy/data-delete** (GDPR; `owner/gdpr` contract exists). Subscription/payment-methods/referral → N/A. |
| **A10** Cross-cutting (push on status, support chat, loading skeleton/empty/error+retry/offline, busy/surge banners) | **HONOR** (states) / **MAY-DEVIATE** (push/chat) | Honour loading/empty/error+retry/offline states — baseline PWA quality. Status push → **MAY-DEVIATE** → PWA push / `ws` + SW notification (Telegram for owner). Support chat → tap-to-call. Surge banner → **N/A** (no surge). |

### B — Courier app

| Item (teardown §B) | Verdict | Why |
|---|---|---|
| Onboarding/verification (docs, vehicle, KYC, training) | **MAY-DEVIATE** | Courier is *invited by the one restaurant* (`courier-invites` migration), not a gig-marketplace applicant. Lightweight invite-accept, not document KYC. |
| Availability (online/offline, zone, **slot/shift booking**) | **HONOR** (online/offline) / **N/A** (slots) | Online/offline is real (`courier/delivery` contract `status: online\|offline`; `courier-shifts`). Zone/slot-booking marketplace mechanics → N/A. |
| **Offer** accept/reject w/ timer (payout, distance, pickup/dropoff) | **HONOR** | Teardown §F#7. Single explicit accept/decline with a timer. Our `courier_assignments (assigned→accepted→…)` + `courier-dispatch-queue` back it. Drop "payout" framing (COD, owner-settled). |
| To-venue (nav→arrived→**pickup confirm**: item check / venue code) | **HONOR** | Pickup-code exists (`orders.pickup_code`). Single-action surface (Enatega `repo-5 §R6/§R8`). |
| To-client (nav→arrived→handoff→**delivery confirm** PIN/photo/sig) | **HONOR** (handoff) / **MAY-DEVIATE** (PIN/photo) | `IN_DELIVERY → DELIVERED` is the handoff. `order-entry-photo` migration exists → photo proof optional. PIN/signature → MAY-DEVIATE/later. |
| Batching (stacked orders) | **MAY-DEVIATE/later** | Low volume per single venue; not MVP. |
| Earnings (per-order, tips, bonuses, surge, **cash/change handling**) | **MAY-DEVIATE** | Drop marketplace earnings/bonus/surge. Keep **cash/change handling** — it is core to COD reconciliation (`settlement-items`, `settlements` contract). |
| Problems (venue closed, client-unreachable timer+photo, reassign, cancel) | **HONOR** | Real operational need; reassign via dispatch queue; unreachable → photo + venue-phone fallback. |
| Demand heatmap, quests/incentives | **N/A** | Pure marketplace gamification. |
| Ratings, stats, account, support | **MAY-DEVIATE** | Minimal; courier is venue-scoped. |

### C — Partner / owner (restaurant) side

| Item (teardown §C) | Verdict | Why |
|---|---|---|
| Onboarding (KYC, menu setup, hours, zones, bank) | **HONOR** (menu/hours) / **MAY-DEVIATE** (KYC/bank) | Menu setup + hours are core owner config. KYC/bank → MAY-DEVIATE (COD, owner-settled, no platform payout). Delivery zone → we have `locations.delivery_polygon`. |
| **Order tablet/dashboard: audible alert → accept/reject → prep-time → "ready"** | **HONOR** | The owner's primary screen (teardown §F#4; Enatega `repo-5 §R6/§R8`). Audible + persistent new-order alert is mandatory (MVP, iOS audio-context fix planned). Maps to `PENDING→CONFIRMED/REJECTED→PREPARING→READY`. `MenuManagerPage`/orders dashboard are existing hotspots. |
| Menu management (items, categories, modifiers, photos, prices, **availability/stop-list, schedules, "86"**) | **HONOR** (items/mods/86) / **MAY-DEVIATE** (schedules) | Items/categories/modifiers/photos/prices/86-toggle all map to real schema (`products.is_available`, `modifiers.available`). **Schedules (breakfast/lunch mealtimes) → MAY-DEVIATE / future** — no schedule entity exists yet (S3 gap). |
| Operational modes (**busy mode ↑time, pause intake, early close**) | **HONOR** | Public menu contract already exposes `status: open\|closed\|busy` (`shared-types/.../public/menu.ts:53`). These 3-level states are teardown §F#3 — honour as a first-class owner toggle. |
| Analytics/money (history, refunds/disputes, sales, ratings, top items, payouts) | **MAY-DEVIATE/later** | Basic order history honour; rich analytics/payouts/disputes → later. |
| Marketing (discounts/promo, sponsored/ads) | **N/A** | Ads/sponsored = marketplace. Promo-builder → later, not MVP. |
| Reviews (view + reply) | **MAY-DEVIATE/later** | Tied to ratings scope. |
| Support | **MAY-DEVIATE** | Minimal. |

### D — Cross-cutting patterns worth conceptually adopting

| Pattern (teardown §D) | Verdict | Why |
|---|---|---|
| One cart = one venue | **HONOR** (free) | We are *structurally* one-venue (`/s/:slug`). The marketplace pain this solves doesn't even arise. |
| Transparent fees; ETA everywhere | **HONOR** | Breakdown columns exist (`delivery_fee/discount_total/tax_total`); ETA on tracking. Teardown §F#2. |
| **Real-time status as the main retention loop** | **HONOR** | Single most important screen (§F#1). Our own `ws` + MapLibre is the chosen transport. |
| Masked call, contactless, leave-at-door | **MAY-DEVIATE** | Masked call → tap-to-call venue-phone fallback (MVP constraint). Contactless/leave-at-door → courier note. |
| Scheduling, pickup, reorder, favourites | **MAY-DEVIATE** | Pickup honoured (`PICKED_UP`). Scheduling scaffold (`SCHEDULED`). Reorder/favourites → later (no account). |
| 3-way rating + tips | **MAY-DEVIATE** | Tips honour (`order-tip`); single basic rating soon; 3-way later. |
| **busy/closed/stop-list/preorder at every level** | **HONOR** | Mandatory (teardown §E, §F#3). Venue (open/closed/busy) + item (stop-list/86) levels exist; category-level → MAY-DEVIATE; preorder → scaffold. |

### F — Top-7 to consciously borrow as ideas

| # (teardown §F) | Verdict | Why |
|---|---|---|
| **F1** Order state machine + live map | **HONOR** | Main retention loop; we already have the 10-state machine + MapLibre + `ws`. |
| **F2** Transparent price breakdown on cart/checkout | **HONOR** | Zero-surprise total; breakdown columns + server-computed (never client-trusted, `repo-2 §R7`). |
| **F3** busy/closed/stop-list at 3 levels | **HONOR** | Operator honesty; venue+item levels real today. |
| **F4** Audible + persistent owner new-order alert | **HONOR** | Critical; iOS audio-context fix planned (teardown §E). |
| **F5** Button with the total price ("Add • €X", "Order • €Y") | **HONOR** | Cheap, high-signal CTA convention. |
| **F6** Masked client↔courier comms *or clear fallback to venue phone* | **MAY-DEVIATE** | Take the fallback branch for MVP (matches "fallback-to-venue-phone" constraint). |
| **F7** Simple courier offer accept/decline w/ timer + explicit per-button action | **HONOR** | Single-action courier UX (Enatega `repo-5 §R8`). |

**Acceptance check:** A1–A10, B (10 items), C (7 items), D (7 items), F1–F7 all classified above. No A–F item left unclassified.

---

## S2 — Triangulation matrix (core flows × repos → recommendation)

Columns: **R1** medusa (`repo-1`) · **R2** supabase-SaaS (`repo-2`) · **R3** MERN (`repo-3`) ·
**R4** TastyIgniter (`repo-4`) · **R5** Enatega (`repo-5`) · **Bolt** (`teardown`) ·
**DOS-now** = DeliveryOS today (verified source). Final = recommendation. No empty cells.

| Flow | R1 Medusa | R2 Supabase-SaaS | R3 MERN | R4 TastyIgniter | R5 Enatega | Bolt-UX | DOS-now | **Recommendation** |
|---|---|---|---|---|---|---|---|---|
| **Menu render** | Product/Variant modules, no storefront menu UI (§R3) | `menu_categories→menu_items`, JSONB sizes/addons (§R3) | menu embedded in Restaurant doc; shadcn cards (§R3) | `Menu/Category` M:N pivot, `isAvailable()` (§R3.2) | category→food, sticky nav, `isOutOfStock` greys out (§R6) | Sticky category nav, dish cards w/ photo/badge, stop-list dimmed (§A3) | `products.is_available`, relational category/product; `MenuPage.tsx` hotspot | **Relational category→product (keep DOS). Borrow Bolt sticky-nav + dish-card + dimmed-stop-list UX; reject R2/R3 JSONB/embedded menus.** |
| **Modifiers / customisation** | ProductVariant options (§R3) | JSONB `sizes/addons` blob (§R3, anti-pattern) | `menuItems[{name,price}]` only (§R3) | **Two-tier** `MenuOption→MenuOptionValue` + per-item `required/min/max/display_type` (§R3.3) | **Two-level** Variation (1-of) + Addon groups w/ `quantityMin/Max` (§R3, §R8) | Required radio / optional checkbox w/ limits, live "Add €X" (§A4) | `modifier_groups(min_select,max_select,required)→modifiers(price_delta)` — already two-tier | **Keep DOS two-tier schema (confirmed by R4+R5). Borrow Bolt bottom-sheet UX (radio/checkbox + live total). Reject R2 JSONB.** |
| **Cart** | Cart/LineItem module, server totals (§R3) | `orders.items` JSONB, client-set totals (§R7 anti-pattern) | embedded cartItems, react-query (§R3) | `Cart` model + `OrderTotal` ledger rows (§R3.5) | Persistent cart bar w/ running total (§R6, §R8) | Line items+mods, breakdown, min-order gate, "Order • €Y" (§A5) | relational `order_items` + integer breakdown cols | **Relational cart + server-computed integer totals (keep DOS). Borrow Enatega/Bolt persistent cart-bar-with-total UX. Reject R2 client-trusted totals.** |
| **Checkout (COD)** | `AbstractPaymentProvider` + manual provider = COD seam (§R7) | free-text `payment_method`, manual `paid` (§R7) | Stripe-only, payment-first (§R7, divergent) | **COD a first-class gateway**; `processPaymentForm`→status+`processed=true` (§R7) | COD = radio that skips gateway (§R7) | Single-screen checkout, cash where available (§A6) | `payment_method='cash'`, `payment_outcome='pending'`, `cash_pay_with` | **Single COD path now (keep DOS). Capture Medusa `PaymentMethod` *seam* (interface only) for future PSP; collapse Enatega's selector to one confirmed cash line. Don't pre-build gateways (YAGNI).** |
| **Status tracking + live map** | SSE on long-running workflow (medusa-eats, §R5) | Supabase Realtime `postgres_changes`+refetch (§R5, we skip) | react-query 5 s polling (§R5, anti-pattern) | AJAX poll; Pusher opt-in (§R5) | 4 Apollo subs: `orderStatusChanged`, `riderLocation` pin, ETA, chat (§R5) | Live map (venue/courier/dropoff)+ETA, stepper, masked call (§A7,§F1) | own `ws` + Redis pub/sub; curated non-PII deltas; MapLibre; timestamps `confirmed_at/ready_at/delivered_at` | **Own `ws` + MapLibre (keep DOS — superior to all). Stepper over the real states: `PENDING→CONFIRMED→PREPARING→READY→IN_DELIVERY→DELIVERED` (delivery) / `…→READY→PICKED_UP` (pickup); `REJECTED/CANCELLED` terminal. Borrow Enatega timestamp-driven filled-step idea + throttled courier-loc channel; reject every repo's polling/Realtime/SSE transport.** |
| **Courier offer / handoff** | medusa-eats driver-claim on workflow (§R-extra) | N/A — no courier model (§R3) | N/A — no courier (§R4) | `assignee_id` on order; no rider app (§R3.5) | Rider OrderDetail = one big status button + map + tap-to-call; offer accept/reject (§R6,§R8) | Offer accept/reject w/ timer; nav→pickup(code)→handoff→delivered (§B,§F7) | `courier_assignments(assigned→accepted→picked_up→delivered)`, `dispatch-queue`, `pickup_code` | **Single-action courier surface (Enatega §R8 + Bolt §F7). Keep DOS assignment table; one explicit accept/decline w/ timer, then advance via the order machine. R2/R3 N/A (no courier).** |
| **Owner order intake (audible alert + accept/prep/ready)** | N/A — admin is generic commerce, no kitchen tablet (§R6) | dashboard, Realtime-driven order list (§R3) | owner advances status, no alert (§R4) | order list AJAX poll; editable statuses (§R4, anti-pattern) | **Restaurant store app: ringer + prep-time limiter + Accept/Reject + online/offline** (§R6,§R8) | **Audible+persistent alert → accept/reject → prep time → ready** (§C,§F4) | orders dashboard (hotspot); `ws` push; `PENDING→CONFIRMED/REJECTED→PREPARING→READY` | **Kitchen-tablet UX from Enatega+Bolt: audible persistent alert, accept/reject, prep-time, ready. Wire to DOS machine + `ws`. iOS audio-context fix. Reject R4 editable statuses.** |
| **States busy/closed/stop-list/preorder** | N/A (no venue-open concept) (§R3) | `is_available` flag, `status='active'` (§R3) | none (§R4) | `Mealtime` schedule + `out_of_stock_override` 86 + status (§R3.4) | `isAvailable`/`isOutOfStock` at item+restaurant level → grey out (§R3,§R8) | **busy/closed/stop-list/preorder at venue+category+item** (§A3,§D,§F3) | venue `status: open\|closed\|busy` (public menu contract); item `is_available` (86) | **Venue (open/closed/busy) + item (86) live today — honour. Borrow R4 schedule engine for *preorder/breakfast-lunch* as future (S3). Category-level state = MAY-DEVIATE. Greyed-out-not-deleted (Enatega §R8).** |

---

## S3 — Schema reconciliation note

Each external finding tagged **confirms** (DOS already does it), **gap** (DOS lacks it, worth MVP/soon),
or **future** (explicitly out of MVP scope — tagged so it is *not* mistaken for a planned feature).
Mapped to a DeliveryOS module. Verified against live migrations in `packages/db/migrations/`.

| External finding (source) | DOS reality (file) | Verdict | Module |
|---|---|---|---|
| **Two-tier modifiers** `MenuOption→MenuOptionValue` + per-item `required/min/max/display_type` (R4 §R3.3); Enatega Variation+Addon `quantityMin/Max` (R5 §R3) | `modifier_groups(min_select,max_select,required)` → `modifiers(price_delta,available)` → `product_modifier_groups` join → `order_item_modifiers` snapshot (`1780338982010_menu_modifiers`) | **confirms** | menu |
| `display_type` (radio/checkbox/select/quantity) on the group (R4 §R3.3) | not stored — UI infers radio vs checkbox from `min/max` | **gap** (small) — add `display_type` enum to `modifier_groups` for explicit radio-vs-checkbox-vs-quantity | menu |
| Per-item modifier **price override** / 4th table `MenuItemOptionValue` (R4 §R3.3) | single `modifiers.price_delta`, no per-item override | **future** — only if menus re-price shared options (R4 explicitly says skip until proven) | menu |
| **86 / out-of-stock boolean** kill-switch on item + modifier-option, separate from quantity (R4 §R3.4, R5 §R8) | `products.is_available` (`1780310072731_menu`) + `modifiers.available` (`1780338982010`) | **confirms** | menu |
| **Mealtime / schedule engine** (`daily\|period\|recurring`) + derived `is_available_now` → breakfast/lunch, preorder (R4 §R3.4) | no schedule entity exists | **future** (R4 calls it "single most actionable steal" but it is explicitly outside the COD MVP; `SCHEDULED` order-state is also scaffold) | menu |
| `order_restriction` / **fulfilment-scope** on item (delivery-only / pickup-only) (R4 §R3.2) | `orders.type` (`order_type`) is per-order; no per-item restriction | **future** — one nullable enum on `products` if mixed delivery/pickup menus appear | menu |
| **OrderTotal ledger rows** `{code,title,value,priority}` — composable breakdown (R4 §R3.5) | fixed integer columns `subtotal/total/delivery_fee/discount_total/tax_total` (`1780310074262`, `1780338982013_money_breakdown`) | **confirms (intentionally simpler)** — fixed columns suffice for COD; ledger is **future** only if coupons/multi-fee compose. **Integer cents already correct (R4's float = rejected).** | orders/checkout |
| **Denormalised order line snapshot** (copy name/price at order time) (R4 §R3.5) | `order_items(name_snapshot, price_snapshot)` + `order_item_modifiers(name_snapshot, price_delta_snapshot)` | **confirms** | orders |
| **Append-only `status_history`** w/ actor + comment + **notify_flag** per transition (R4 §R4) | `order_status_history(from_status,to_status,actor,created_at)` (`1780338982015`) — has actor, **no comment, no notify_flag** | **confirms** (table exists) + **gap** — add `comment` and `notify` columns to feed the Telegram-notification design (R4 §R4) | orders/notif |
| **Per-state `*_at` timestamp columns** (R2 §R4, R5 §R4) | `confirmed_at`, `ready_at`, `delivered_at` on `orders` (`1780310074262`, `1780695000000`) | **confirms (partial)** — `PREPARING/IN_DELIVERY/PICKED_UP` lack dedicated `*_at` (derivable from history) | orders |
| **RLS tenant isolation**: tenant-id FK on every table + tenant-prefixed composite indexes + per-tenant uniques (R2 §R3) | `location_id` FK + `FORCE ROW LEVEL SECURITY` + `tenant_isolation` policy via `app_member_location_ids()`; `orders_location_status_idx`, `orders_location_created_idx`; `customers UNIQUE(location_id,phone)` (`1780310074262`, `1780421100051_force-rls`) | **confirms (stronger)** — DOS uses `FORCE` + own JWT identity, sidestepping R2's `auth.uid()`-vs-custom-users pitfall (R2 §R8 P1) | locations/db |
| Public-read-by-status-flag / owner-write RLS split (R2 §R3) | `public-menu-rls`, `public-locations-rls`, `customer-rls` migrations | **confirms** | db |
| **Payment-provider seam** `AbstractPaymentProvider` / manual provider for COD (R1 §R7) | single COD path: `payment_method` enum + `payment_outcome` + `cash_pay_with` | **confirms intent; seam = future** — capture interface only when adding a real PSP (YAGNI) | checkout/orders |
| COD merchant knobs (`cod_min_order`, `cod_fee`, landing status) + cash **payment-event audit row** (R4 §R7) | `locations.min_order_value`, `delivery_fee_flat`, `tax_rate`, `delivery_polygon` (`1780338982014_location_commerce`); cash reconciliation via `settlement-items`/`settlements` | **confirms (most)** + **future** (per-attempt `order_payment_event` audit row, integer cents) | locations/orders |
| `hash` opaque public order token (R4 §R3.5) | guest order status surface exists; opaque token | **confirms** (verify token is opaque, not sequential id) | orders |
| Multi-axis status (payment ⟂ fulfilment) (R1 §R4) | `payment_outcome` is orthogonal to `order_status` already | **confirms** | orders |
| Polymorphic `locationables` menu-sharing (R4 §R3.1); admin-editable statuses (R4 §R4); Supabase Realtime `REPLICA IDENTITY FULL` (R2 §R5); float money (R4 §R8); JSONB blobs (R2 §R8) | direct `location_id` FK; typed 10-state machine; own `ws`; integer cents; relational | **reject (DOS already better)** — recorded so they are not re-introduced | all |

**Net:** the DeliveryOS schema is already aligned with the strongest external evidence (two-tier
modifiers, 86-toggle, snapshots, status_history table, integer money, FORCE-RLS). The actionable
**gaps** are small and additive: `modifier_groups.display_type`, `order_status_history.{comment,notify}`.
Everything richer (schedule engine, totals ledger, per-item price override, PSP seam, fulfilment-scope)
is **future**, explicitly outside the COD MVP.

---

## S4 — Components / tokens recommendations for `packages/ui`

Grounded in MERN (`repo-3 §R6` — confirms `cn` + `cva` + `hsl(var())`), Enatega (`repo-5 §R6/§R8`
UX shapes), and Bolt (`teardown §A–F`). **License-clean:** literal code only from MIT sources with
attribution; everything from R4 (PHP) and R5 (RN/proprietary backend) is **ideas only, no code**.

> **Token principle (all components):** colours come *only* from `var(--brand-*)` set per tenant at
> runtime on a root element, resolved through Tailwind's `hsl(var(--token))` indirection
> (`repo-3 §R6` validates the wiring; `repo-1 §R6`, `repo-2 §R6`, `repo-3 §R8` all confirm runtime CSS
> vars beat their static Tailwind-preset / hardcoded-hex tokens for white-label). **Never** add a
> compile-time brand preset. Component contrast must survive `derivePalette` (the auto-branding seam).

| Component (proposal) | Shape | Source grounding | Tokens |
|---|---|---|---|
| **CartButton (sticky, with total)** | Fixed bottom bar, item-count badge + running total, label "Order • €Y" / "Add • €X" | Enatega persistent cart bar (§R6,§R8); Bolt §F5 | `--brand-primary` bg, `--brand-on-primary` text; disabled state when below `min_order_value` |
| **ModifierSheet (bottom-sheet, live total)** | Radix Dialog/Sheet; required group = radio, optional = checkbox, governed by `min_select/max_select`; footer CTA recomputes live "Add • €X" | Bolt §A4; Enatega Variation+Addon min/max (§R3); R4 two-tier (§R3.3); DOS schema confirms | brand tokens; "required" + "sold out" use semantic `--brand-warning`/muted, never raw hex |
| **OrderStatusStepper** | Horizontal/vertical stepper; filled steps from per-transition timestamps; two branch variants (delivery `…IN_DELIVERY→DELIVERED`, pickup `READY→PICKED_UP`); terminal styling for `REJECTED/CANCELLED` | Enatega timestamp-driven steps (§R4,§R8); Bolt §A7,§F1; **real DOS 10-state machine** | `--brand-primary` filled, muted unfilled, `--brand-error` for rejected |
| **OwnerOrderTablet (audible alert)** | Live order queue card; **persistent audible alert** on new `PENDING` (Web Audio, iOS audio-context unlock on first tap); Accept/Reject + prep-time + Ready buttons wired to the machine | Enatega ringer+prep-limiter (§R6,§R8); Bolt §C,§F4 | brand tokens; alert affordance high-contrast |
| **CourierActionScreen (single action)** | One dominant primary button advancing the assignment (Accept offer → Picked up → Delivered) + map + tap-to-call venue/customer | Enatega rider single-action OrderDetail (§R6,§R8); Bolt §B,§F7 | `--brand-primary` for the one action; secondary tap-to-call |
| **StateChip (busy/closed/stop-list)** | Small status chip at venue + item level: open/closed/busy + sold-out(86); greyed-out, not removed | Bolt §A3,§F3; Enatega grey-out-not-delete (§R8); R4 86-toggle (§R3.4) | semantic tokens derived from brand; sold-out = muted/disabled |

**Already aligned (verify-only, `repo-3 §R6/§R9`):** `cn` util, `cva` variants, `hsl(var())` mapping
exist in `packages/ui`. **App-level (not `packages/ui`) adoptable recipes (`repo-3 §R8`):**
RHF + `zodResolver` + shadcn `Form` for owner menu/profile editors (port to TanStack Query v5 array-keys,
`isPending`), and one-hook-file-per-resource — these live in `apps/web`, MIT-with-attribution.

---

## S5 — Decision log (ADR-style) — main output

**BORROW** = copy MIT code (carry attribution). **ADAPT** = reimplement the idea on our stack.
**SKIP** = reject / not for us. **Zero BORROW on incompatible licences** (R4 PHP, R5 RN+proprietary
backend, any no-license). Tag = MVP | later | N/A.

| # | Pattern | Source repo(s) + licence | DOS module | Tag | Verdict | Rationale (one line) |
|---|---|---|---|---|---|---|
| 1 | Two-tier modifier schema (group + values, per-item min/max/required) | R4 TastyIgniter (MIT, **PHP→ideas only**); R5 Enatega (MIT, **RN→ideas only**) | menu | MVP | **ADAPT** | DOS schema already matches; external evidence confirms the shape — keep, don't lift code. |
| 2 | `display_type` (radio/checkbox/quantity) on modifier group | R4 (MIT, PHP) | menu | MVP | **ADAPT** | Small additive column; makes the radio-vs-checkbox UI contract explicit instead of inferred. |
| 3 | 86 / out-of-stock boolean kill-switch (item + modifier) | R4 (MIT, PHP); R5 (MIT, RN) | menu | MVP | **ADAPT** | Already present (`is_available`/`available`); confirm + surface in owner UI. Highest ROI/cost (R4 §R3.4). |
| 4 | Mealtime/schedule availability engine (daily/period/recurring) | R4 (MIT, PHP) | menu | later | **ADAPT** | Powers breakfast-lunch + preorder with zero cron; outside COD MVP (also `SCHEDULED` is scaffold). |
| 5 | Per-item modifier price override (4th table) | R4 (MIT, PHP) | menu | later | **SKIP** | YAGNI until menus actually re-price shared options (R4 says skip). |
| 6 | Fulfilment-scope per item (delivery-only/pickup-only) | R4 (MIT, PHP) | menu | later | **ADAPT** | One nullable enum; only when mixed delivery/pickup menus appear. |
| 7 | Denormalised order-line snapshot (name/price at order time) | R4 (MIT, PHP); confirmed by R2 intent | orders | MVP | **ADAPT** | Already done (`name_snapshot/price_snapshot`); menu edits must not rewrite history. |
| 8 | Append-only `order_status_history` (actor + comment + notify-flag) | R4 (MIT, PHP) | orders/notif | MVP | **ADAPT** | Table exists with actor; add `comment`+`notify` to feed Telegram per-category prefs. |
| 9 | Per-transition `*_at` timestamps powering filled-step stepper | R2 (MIT); R5 (MIT, RN) | orders/ui | MVP | **ADAPT** | `confirmed_at/ready_at/delivered_at` exist; stepper reads them. Add missing states if needed. |
| 10 | Totals as composable ledger rows `{code,title,value,priority}` | R4 (MIT, PHP) | orders | later | **SKIP** (now) | Fixed integer columns suffice for COD; revisit only when coupons/multi-fee compose. |
| 11 | Integer-minor-unit money everywhere (reject float) | R2/R4 anti-pattern (their float = SKIP) | orders/menu | MVP | **SKIP their float** | DOS integer-cents is correct; record float as a do-not-introduce. |
| 12 | `AbstractPaymentProvider` / manual-provider seam → COD | R1 Medusa (**MIT — code liftable w/ attribution**) | checkout/orders | later | **ADAPT** | Capture the *interface shape* only when adding a real PSP; single COD path now (YAGNI). |
| 13 | COD merchant knobs (min-order, fee, landing status) + cash payment-event audit row | R4 (MIT, PHP) | locations/orders | MVP/later | **ADAPT** | Knobs largely exist (`min_order_value`,`delivery_fee_flat`); per-attempt cash audit row = later. |
| 14 | RLS: tenant FK + tenant-prefixed composite indexes + per-tenant uniques + public-by-flag / owner-by-subquery | R2 Supabase-SaaS (**MIT**) | locations/db | MVP | **ADAPT** | DOS already does this *stronger* (FORCE-RLS, own-JWT identity) — align/confirm, fix to our identity not `auth.uid()` (R2 §R8 P1). |
| 15 | Cancellation-ordering workflow (cancel fulfilment → settle → cancel order) | R1 Medusa (**MIT — liftable**) | orders/couriers | later | **ADAPT** | Correctness pattern for order-cancel; reimplement in pg-boss worker/handler. |
| 16 | Multi-axis status (payment ⟂ fulfilment) | R1 (MIT) | orders | MVP | **ADAPT** | `payment_outcome` already orthogonal to `order_status`; keep the discipline. |
| 17 | shadcn primitives + `cn` + `cva` + `hsl(var())` wiring | R3 MERN (MIT, shadcn MIT) | ui | MVP | **SKIP-copy / verify** | DOS vendors its own; confirm aligned, no file lift needed. |
| 18 | RHF + `zodResolver` + shadcn `Form` recipe; one-hook-file-per-resource | R3 MERN (**MIT — code liftable w/ attribution**, port v3→v5) | web (apps) | MVP | **BORROW** | The one clean code-liftable item: copy the form/hook recipe, attribute, port TanStack v5. |
| 19 | Cart→review→confirm UI scaffold (strip Stripe, wire COD) | R3 MERN (**MIT — liftable**) | web/checkout | MVP | **BORROW** | Liftable scaffold; remove payment-session mechanics, wire single COD line. |
| 20 | Order-status stepper/badge UI | R3 MERN (**MIT — liftable**), remap to 10-state | web/ui | MVP | **ADAPT** | Lift the badge/stepper shell; remap states to the real DOS machine (not R3's 5 states). |
| 21 | Persistent cart-bar-with-total + modifier bottom-sheet + live "Add €X" | R5 Enatega (RN→ideas); Bolt (teardown) | ui | MVP | **ADAPT** | Signature food-app UX; reimplement on shadcn (no RN code). |
| 22 | Owner kitchen-tablet: audible+persistent alert, accept/reject, prep-time, ready | R5 Enatega (RN→ideas); Bolt §C,§F4 | orders/ui | MVP | **ADAPT** | Owner's primary screen; Web Audio + iOS unlock; wire to machine + `ws`. |
| 23 | Courier single-action OrderDetail (one big advance button + map + tap-to-call) | R5 Enatega (RN→ideas); Bolt §B,§F7 | couriers/ui | MVP | **ADAPT** | One explicit action per screen; reimplement on React/MapLibre. |
| 24 | Live courier pin + ETA/distance on tracking | R5 Enatega (RN→ideas); Bolt §A7 | couriers/ui | MVP | **ADAPT** | Throttled `courier:{id}/loc` topic on own `ws` + MapLibre; client-side ETA. |
| 25 | busy/closed/stop-list/preorder states at venue+item | R4 (ideas); R5 (ideas); Bolt §F3 | menu/ui | MVP | **ADAPT** | Venue (open/closed/busy) + item (86) live; preorder=future; greyed-out-not-deleted. |
| 26 | Editable order statuses (admin-defined status rows) | R4 (MIT, PHP) | orders | N/A | **SKIP** | Breaks typed 10-state machine + exhaustive courier logic; keep the enum. |
| 27 | Polymorphic `locationables` menu-sharing | R4 (MIT, PHP) | menu/locations | N/A | **SKIP** | Direct `location_id` FK + RLS is leaner/safer for per-restaurant. |
| 28 | Supabase Realtime (`postgres_changes` + `REPLICA IDENTITY FULL`) | R2 (MIT) | ws | N/A | **SKIP** | Own `ws` pushes curated non-PII deltas; avoids CDC leakage + write amplification (R2 §R5). |
| 29 | JSONB order/menu blobs; client-set totals | R2 (MIT) | menu/orders/checkout | N/A | **SKIP** | Unqueryable/tamperable; keep relational + server-computed + Zod. |
| 30 | Polling-as-realtime (`refetchInterval`); Stripe-first machine; Auth0; Mongo models | R3 MERN (MIT); R1 Mikro/Redis runtime | infra/orders/auth | N/A | **SKIP** | Own `ws`, COD, own RS256 JWT, Supabase+pg-boss — all chosen alternatives. |
| 31 | Pusher / Laravel Broadcasting realtime | R4 (MIT, PHP) | ws | N/A | **SKIP** | Third-party push SaaS dependency; own `ws` is free + controlled. |
| 32 | Marketplace surfaces (search/collections, commissions, withdrawals, zones-as-tax, surge, ads, wallet, in-app chat) | R5 (RN); Bolt §A2,§C | — | N/A | **SKIP** | Single-restaurant COD; cutting them is a feature, not a gap (R5 §R8). |

**Zero-BORROW-on-incompatible-licence check:** every **BORROW** (#18, #19) is from R3 MERN (MIT, TS,
liftable with attribution). No BORROW touches R4 (PHP) or R5 (RN+proprietary backend) — both are
ADAPT/SKIP only. Medusa items (#12,#15) are MIT-liftable but tagged ADAPT (we reimplement trimmed-to-COD).

---

## License compliance summary

Per-repo `R1` verdicts. "Copyable" = literal code may be lifted **with attribution** (carry the MIT
notice in a NOTICE/third-party-licenses entry). "Patterns-only" = ideas/schema/UX free; code not
liftable (wrong stack or proprietary parts). "Avoid" = do not reuse code.

| Repo | Licence | Reuse verdict | Attribution required if code lifted? | Source |
|---|---|---|---|---|
| **R1 medusajs/medusa** | MIT | **Copyable** (MIT) — but mostly ADAPT (trim to COD) | **Yes** (MIT notice) | `repo-1 §R1` |
| **R2 roshanx0/restaurant-ordering-saas** | MIT | **Copyable** (MIT) — RLS/index/schema shapes; SKIP its Realtime/JSONB/client-totals | **Yes** (MIT notice) | `repo-2 §R1` |
| **R3 arnobt78/Restaurant-…-MERN** | MIT (`/LICENSE` valid; GitHub API mis-flags `NOASSERTION`) | **Copyable** (MIT) — the only practical BORROW source (Form recipe, cart scaffold, stepper shell) | **Yes** (MIT notice; shadcn primitives = patterns-only, already vendored) | `repo-3 §R1` |
| **R4 tastyigniter/TastyIgniter** | MIT | **Patterns-only** (PHP/Laravel — no TS code liftable despite permissive licence) | **No code lifted** → none (ideas/schema are free) | `repo-4 §R1` |
| **R5 enatega (multi/single-vendor)** | Frontend **MIT**; **backend PROPRIETARY (paid)** | **Patterns-only** (RN frontend = UX/IA ideas; backend strictly **avoid**) | **No code lifted** → none; backend out of scope absolutely | `repo-5 §R1` |
| **Bolt Food teardown** | observed UX (no code) | **Patterns-only** (conventions catalogue) | n/a | `teardown` |

**Bottom line on licences:** only **R3 (MIT, TypeScript)** yields literal copyable code, and only two
small items (RHF+zod `Form` recipe, cart→confirm scaffold) — both carry the MIT attribution. R1/R2 are
MIT and copyable but reduce to ADAPT (trim-to-COD / align-to-stronger-RLS). R4 (PHP) and R5 (RN +
proprietary backend) are ideas-only by stack, regardless of their permissive frontend licences. No
copyleft or no-licence code is anywhere in scope.
