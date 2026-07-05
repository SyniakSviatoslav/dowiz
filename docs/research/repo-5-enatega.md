# Repo 5 — Enatega (food-delivery-multivendor + food-delivery-singlevendor)

> Research dossier for DeliveryOS (white-label PER-RESTAURANT ordering, `/s/:slug`, NOT a marketplace).
> **SCOPE LOCK:** Enatega's **frontend apps are OSS (MIT)**; the **backend/API is PROPRIETARY (paid license) → STRICTLY OUT OF SCOPE.** This dossier analyzes **only the frontend UX + information architecture** of the four client surfaces. No backend code, schema, or resolver logic is examined or reused.
> Enatega stack = React Native (Expo) + Apollo/GraphQL. DeliveryOS = React 18 web PWA + own ws + REST/Zod. So **transport and runtime differ fundamentally** — every takeaway below is a **UX/IA convention**, never code.

Sources:
- https://github.com/enatega/food-delivery-multivendor (README, LICENSE, dir tree)
- https://github.com/enatega/food-delivery-singlevendor (README, LICENSE, dir tree)
- https://enatega-1.gitbook.io/enatega-multivendor (+ `/llms-full.txt`) — official docs
- `enatega-multivendor-app/src/screens/*` (customer screen inventory)
- `enatega-multivendor-app/src/apollo/{queries,subscriptions}.js` (entity field shapes, real-time)

---

## R1 — Identity & License (license-first)

| Item | Finding | Citation |
|------|---------|----------|
| Frontend license — multivendor | **MIT** | `food-delivery-multivendor/LICENSE` + README |
| Frontend license — singlevendor | **MIT** | `food-delivery-singlevendor/LICENSE` + README |
| Backend/API | **PROPRIETARY, paid license** — README (both): *"The frontend source code … is completely open source. However, the API and backend is proprietary and can be accessed via a paid license."* | both READMEs |

**Verdict:** Frontend is MIT → **permissive**. **Reuse verdict: UX/IA/flow PATTERNS ONLY.** We do **not** lift RN code into a web PWA (runtime mismatch makes code-reuse a non-goal anyway), and we **never** touch the proprietary backend. Value of this repo = *how a real 3-sided + admin food-delivery UX is structured* (screens, flows, state machine, real-time UX expectations).

**Backend exclusion is absolute** for the rest of this dossier: R3 reads only the entity shapes the **frontend GraphQL documents consume** (client-visible types), not backend models/resolvers.

---

## R2 — Stack & Topology (note divergence from DeliveryOS)

**Multivendor monorepo (5 apps):**
| App | Folder | Tech | DeliveryOS analog |
|-----|--------|------|-------------------|
| Customer mobile | `enatega-multivendor-app` | React Native + Expo | **Client** (`/s/:slug`) |
| Customer web | `enatega-multivendor-web` | React.js | Client (web is our only surface) |
| Rider | `enatega-multivendor-rider` | React Native + Expo | **Courier** |
| Restaurant/vendor | `enatega-multivendor-store` | React Native + Expo | **Owner** (order-ops side) |
| Admin dashboard | `enatega-multivendor-admin` | Next.js (`protected`/`unprotected` route split) | **Owner** (config/menu side) |

**Singlevendor monorepo (3 apps):** `CustomerApp` (RN/Expo), `RiderApp` (RN/Expo), `Admin Dashboard` (ReactJS). No separate vendor app — the single restaurant *is* the admin. **Singlevendor's IA is the closer analog to DeliveryOS** (one restaurant, admin = owner).

**Shared transport:** Apollo Client + GraphQL (queries/mutations/**subscriptions**), MongoDB backend (proprietary), Firebase (push/auth).

**Divergences from DeliveryOS (record, don't copy):**
- **RN/Expo ≠ web PWA.** Navigation = React Navigation stacks; ours = React Router routes. Screen *boundaries* transfer; navigation mechanics do not.
- **Apollo/GraphQL subscriptions ≠ our own ws + REST/Zod.** Capture the *UX expectation* (live pin, ETA, status push), not the transport.
- **5-app split ≠ our 1-codebase-3-routes.** We collapse customer/owner/courier into one PWA with route-gated roles. Enatega's per-app screen sets become our **per-route screen sets**.

---

## R3 — Data Model (frontend-visible GraphQL types ONLY)

Entity field shapes as consumed by `enatega-multivendor-app/src/apollo/queries.js` (client-side selection sets — **NOT backend schema**):

| Entity | Client-visible fields (selection set) | Maps to DeliveryOS |
|--------|----------------------------------------|--------------------|
| **Restaurant** | `_id, name, image, address, location, deliveryTime, minimumOrder, tax, rating, isAvailable, shopType, categories[], options[], addons[], openingTimes` | `location`/tenant (our `/s/:slug` storefront) |
| **Category** | `_id, title, foods[]` | `menu_category` |
| **Food** | `_id, title, image, description, subCategory, variations[], isOutOfStock` | `menu_item` |
| **Variation** | `_id, title, price, discounted, addons[], isOutOfStock` | item **variant / size** (price tier) |
| **Addon / Option** | addon: `_id, title, description, options[], quantityMinimum, quantityMaximum`; option: `_id, title, price` | **modifier group → modifier option** (min/max selection rules) |
| **Order** | `_id, orderId, restaurant, items[], user, rider, deliveryAddress, orderStatus, paymentStatus, orderAmount, createdAt, deliveredAt` | `order` |
| **Rider** | `_id, name, phone, location, available, currentWalletAmount` | `courier` (drop wallet — N/A for COD-only) |
| **Zone** | `_id, title, location, tax, description, isActive` | delivery area / `location` bounds |

**Key UX-relevant modeling lessons (HONOR):**
- **Two-level modifiers:** Food → **Variation** (required, single-select size/price tier) → **Addons** (multi-select, each addon group carries `quantityMinimum`/`quantityMaximum`). DeliveryOS should mirror this **min/max-per-group** rule in the modifier sheet — it's the cleanest UX contract for "pick 1 size, then 0–N extras." → Owner **menu module** + Client **item sheet**.
- **`isOutOfStock` / `isAvailable` at item AND restaurant level** drives greyed-out UI without deleting data. → HONOR (we have item `available` + storefront open/closed).
- **MAY-DEVIATE:** `currentWalletAmount`, `paymentStatus`, `discounted`, `shopType`, `tax`-per-zone — Enatega is marketplace/multi-payment. DeliveryOS is single-restaurant, **cash-on-delivery**, flat per-tenant tax → drop wallet, online-payment status, zone-tax.

---

## R4 — Order State Machine

**Canonical Enatega order statuses** (confirmed via `subscriptions.js` timestamp fields `acceptedAt/pickedAt/deliveredAt` + docs):

`PENDING → ACCEPTED → ASSIGNED → PICKED → DELIVERED` (+ `CANCELLED` terminal)

| Enatega status | Who drives it | UI surface |
|----------------|---------------|------------|
| PENDING | customer placed | customer stepper step 1; restaurant gets ringer/push |
| ACCEPTED | restaurant accepts (within prep-time limiter) | restaurant order card → Accept; customer stepper advances |
| ASSIGNED | rider assigned | rider gets order; customer sees rider info |
| PICKED | rider marks picked up | rider OrderDetail status button; `isPickedUp`/`pickedAt` |
| DELIVERED | rider marks delivered | rider status button → terminal; `deliveredAt` |
| CANCELLED | restaurant/admin reject | terminal |

**Map onto DeliveryOS 10-state COD machine (HONOR the shape, expand granularity):**
Enatega's 6 states collapse prep + courier-search. DeliveryOS 10-state proposal:
`PLACED → ACCEPTED → PREPARING → READY → COURIER_ASSIGNED → PICKED_UP → EN_ROUTE → DELIVERED` plus `REJECTED` and `CANCELLED` (terminals).
- Enatega `ACCEPTED` → split into our `ACCEPTED` + `PREPARING` + `READY` (we surface kitchen progress; Enatega hides it behind a prep-time timer).
- Enatega `ASSIGNED`/`PICKED` → our `COURIER_ASSIGNED`/`PICKED_UP`/`EN_ROUTE`.
- **HONOR:** each transition is owned by exactly one role and is **timestamped** (`acceptedAt/pickedAt/deliveredAt`) — gives the customer stepper its filled/unfilled steps for free. Adopt timestamp-per-transition.
- **N/A:** no `paymentStatus` branch (COD only — money settles at DELIVERED, off-machine).

---

## R5 — Real-time

From `enatega-multivendor-app/src/apollo/subscriptions.js` — **four** Apollo subscriptions:
1. `orderStatusChanged` — full order context on every transition (`orderStatus`, `acceptedAt/pickedAt/deliveredAt`, addresses, items, user, rider). Powers the customer **status stepper** + restaurant/rider live lists.
2. `subscriptionRiderLocation` — streams `location { coordinates }` → **live courier pin on map**.
3. `subscriptionOrder` — single-order watch (`orderStatus, completionTime, preparationTime, rider`).
4. `subscriptionNewMessage` — customer↔rider chat.

**UX expectations to honor (transport differs — our own ws, not Apollo):**
- **Live courier pin** that moves as `coordinates` stream in (R6 "live map") → DeliveryOS Client **Status** screen + Courier active-delivery.
- **ETA / distance-to-destination** computed client-side from rider coords + restaurant/delivery addresses (docs: "Distance to destination with estimated time").
- **Push on every status change** even when app backgrounded (Firebase) → our PWA push / ws + service-worker notification.
- **MAY-DEVIATE:** customer↔rider in-app chat is a heavier surface; COD storefront can defer chat to a phone-tap CTA.

**Map to our ws:** one order channel carrying status transitions + a courier-location channel (throttled). Don't replicate 4 separate subscription docs — collapse to `order:{id}` (status) and `courier:{id}/loc` (geo).

---

## R6 — Component / UX System (THE MAIN PAYOFF)

### Customer app — 41 screens (`enatega-multivendor-app/src/screens`)
Full inventory: About, Account, Addresses, **Cart**, CartAddress, ChatWithRider, **Checkout**, Collection, CreateAccount, CurrentLocation, EditAddress, Favourite, ForgotPassword, FullMap, Help, HelpBrowser, **Home**, Hyp, **ItemDetail**, Login, **Main**, MapSection, **Menu**, **MyOrders**, NewAddress, **OrderDetail**, Otp, Payment, Paypal, PhoneNumber, Profile, RateAndReview, Register, Reorder, **Restaurant**, Reviews, SaveAddress, Search, SelectLocation, Settings, Stripe.

**Core ordering flow (our Client analog):** Home/Main → **Restaurant** (menu by category) → **ItemDetail** (variation + addon modifier sheet) → **Cart** → **Checkout** → **OrderDetail** (live tracking). Maps almost 1:1 to DeliveryOS **`/s/:slug` → Menu → Item → Cart → Checkout → Status**.

**Reusable UX patterns:**
- **Persistent cart button with running total** (bottom bar, item-count + price) — the signature food-app pattern.
- **Modifier bottom-sheet** on ItemDetail: required Variation radio + Addon checkboxes governed by `quantityMinimum`/`quantityMaximum`, with a live "Add to cart — $X" button reflecting selections.
- **Order-status stepper** on OrderDetail (filled steps driven by transition timestamps) + live rider pin + ETA.
- **Reorder** screen (one-tap repeat of a past order).
- **Search + Collection** (curated groupings) for discovery — *marketplace-only, mostly N/A for single-restaurant.*

### Rider app (`enatega-multivendor-rider`)
Screens: Login/ForgotPassword, tutorial (unauth), **Orders list**, **OrderDetail** (`src/screens/OrderDetail/OrderDetail.js` — "status of the order can be changed by the rider"), Profile, wallet.
**Patterns:** order cards in a list; **OrderDetail = the single action surface** where the rider advances status (mark **PICKED**, mark **DELIVERED**) — a big primary status button + map + customer/restaurant addresses + tap-to-call. Maps to DeliveryOS **Courier: Tasks → Active delivery → Delivered.**

### Restaurant/store app (`enatega-multivendor-store`)
**Order-tablet UX:** **ringer + push** on incoming order; **prep-time limiter** (accept within a countdown); **order detail** with customer + items; **Accept / Reject**; **Print Invoice**; **toggle restaurant online/offline**; **delivered-orders history**. Maps to DeliveryOS **Owner: Dashboard → Orders** (the "kitchen tablet" live queue).

### Admin dashboard (`enatega-multivendor-admin`, Next.js, `protected`/`unprotected` split)
Pages (from docs): role-based Admin/Vendor; Restaurants/Vendors/Sections; **Zones + Rider assignment**; Withdrawals/Commissions; **Configuration + order-status oversight**; restaurant order **statistics**; **Menu management with discounts**; Users/Ratings/**Coupons**/Tipping. Maps to DeliveryOS **Owner: Menu + Couriers + config** (drop marketplace-only: commissions, withdrawals, multi-vendor, zones-as-tax).

---

## R7 — Checkout & Payments UX

**Checkout flow** (customer): Cart → **Checkout** screen = delivery address (CartAddress/SelectLocation) + tip + payment method selector + order summary → place order → **OrderDetail** tracking.
**Cash-on-delivery:** Enatega's payment selector lists **COD alongside** Stripe/PayPal (dedicated `Payment`, `Stripe`, `Paypal` screens). COD = a radio option that **skips the payment gateway** and goes straight to order placement; settlement happens at delivery.
**Map to DeliveryOS (COD-only):** **HONOR** the single-screen checkout (address + tip + summary + place) but **collapse the payment selector** — COD is the only method, so present it as a confirmed line ("Pay cash on delivery"), **drop** `Payment/Stripe/Paypal` screens entirely. → Client **Checkout**.

---

## R8 — UX Patterns to Adopt vs Anti-patterns

**Adopt (HONOR):**
- Persistent **cart bar w/ live total + count**. → Client.
- **Modifier sheet** with required-variation + min/max-addon rules and a live total-reflecting CTA. → Client item / Owner menu.
- **Status stepper driven by per-transition timestamps**. → Client Status.
- **Restaurant order-tablet**: ringer + prep-time countdown + Accept/Reject + online/offline toggle. → Owner Orders.
- **Rider single-action OrderDetail** (one big "advance status" button + map + tap-to-call). → Courier.
- **Reorder** one-tap. → Client.
- **Live rider pin + ETA/distance** on tracking. → Client/Courier.

**Anti-patterns / avoid (don't carry over):**
- **5 separate apps** → context-fragmentation; DeliveryOS keeps **1 PWA, role-routed**.
- **Marketplace bloat** in admin (commissions, withdrawals, vendor onboarding, zone-tax, collections, tipping, coupons) → out of scope for single-restaurant COD; cutting them is a feature, not a gap.
- **41 customer screens** → many are auth/address/payment variants (Otp, Stripe, Paypal, Hyp, Payment, CreateAccount vs Register vs Login). Consolidate; our COD + magic-link flow needs a fraction of these.
- **In-app chat** as a first-class screen → defer to a tap-to-call CTA for v1.
- **Wallet** on rider → N/A for COD.

---

## R9 — Liftable (UX/IA/flow IDEAS ONLY — no code, no backend) vs DeliveryOS plan

| # | Liftable UX idea | Verdict | DeliveryOS role/module |
|---|------------------|---------|------------------------|
| 1 | Core ordering IA: Storefront → Item(modifier sheet) → Cart(persistent total) → Checkout → Status(stepper) | **HONOR** | Client (`/s/:slug` → … → Status) |
| 2 | Two-level modifiers: required Variation + multi Addon groups with min/max | **HONOR** | Owner menu schema + Client item sheet |
| 3 | Per-transition timestamps powering a filled-step status stepper | **HONOR** | Client Status + state machine |
| 4 | Restaurant order-tablet: ringer + prep-countdown + Accept/Reject + online/offline | **HONOR** | Owner Dashboard/Orders |
| 5 | Rider OrderDetail = one big status-advance button + map + tap-to-call | **HONOR** | Courier Active-delivery |
| 6 | Live rider pin (streamed coords) + ETA/distance | **HONOR** (our ws, MapLibre) | Client Status + Courier |
| 7 | Reorder (one-tap repeat) | **HONOR** | Client MyOrders |
| 8 | `isOutOfStock`/`isAvailable` → greyed-out, not deleted | **HONOR** | Owner menu + Client |
| 9 | COD as a payment option that skips the gateway | **MAY-DEVIATE** → COD-only, drop selector | Client Checkout |
| 10 | Customer↔rider in-app chat screen | **MAY-DEVIATE** → tap-to-call CTA v1 | Client/Courier |
| 11 | Marketplace admin (commissions, withdrawals, zones-as-tax, coupons, tipping, vendor onboarding) | **N/A** (single-restaurant) | — drop from Owner |
| 12 | Rider wallet / online-payment status | **N/A** (COD) | — drop |
| 13 | Singlevendor IA (admin = the one restaurant; no vendor app) | **HONOR** as closest analog | Owner = admin |

**Bottom line:** Enatega's payoff for DeliveryOS is the **information architecture of a proven 3-sided + admin food-delivery UX** — screen boundaries, the order state machine shape, and the real-time tracking expectations. Lift the **flows and IA**, collapse 5 apps → 1 role-routed PWA, strip marketplace/online-payment surfaces for our single-restaurant COD model, and re-implement everything on our React/Tailwind/own-ws/REST stack (no RN code, no proprietary backend).
