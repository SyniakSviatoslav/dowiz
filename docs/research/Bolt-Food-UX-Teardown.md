# Bolt Food — UX teardown (companion input, provided by user 2026-06-22)

> Inventory of user actions/flows across Bolt's 3 apps (client/courier/partner), mapped to
> DeliveryOS roles. Observed-behaviour + app-store-listing analysis — patterns/flows, not code.
> Framing: **Bolt = marketplace aggregator; DeliveryOS = white-label per-restaurant** (`/s/:slug`).
> Marketplace flows (aggregated search, cross-restaurant collections, Bolt Plus, surge, ads) are
> N/A or trimmed; the single-venue order flow (menu→item→cart→checkout→tracking→rating) and the
> operational states (busy/closed/stop-list/preorder) are the 1:1 zone — where Bolt is most polished.

## A. Client app
- **A1 Onboarding/registration**: phone/email/social + OTP; profile (name, phone); permissions
  (geo foreground+background, push); delivery address (GPS auto, search, drop-pin, apt/entrance/
  floor/courier note, multiple saved addresses home/work/other).
- **A2 Discovery/home** *(mostly N/A for DeliveryOS)*: address switcher; tabs Food/Market/DineOut;
  Delivery↔Pickup toggle; cuisine categories, search, filters, sort; promo banners, collections;
  venue card (name, rating, ETA, delivery fee, min order, distance, promo badge, open/busy/closed,
  favourite); favourites, recent orders, quick reorder.
- **A3 Venue/menu page** *(CORE)*: header (cover, logo, rating+reviews, ETA, fee/min, hours, about,
  allergens); menu categories + sticky category nav; in-menu search; dish card (photo, name, desc,
  price, badge popular/spicy/new); states: venue **closed** (preorder/"opens at…"), **busy mode**
  (raised ETA), **stop-list** (item sold out — dimmed, not clickable).
- **A4 Item/customization** *(CORE)*: modifier groups required(radio)/optional(checkbox) with
  select limits; quantity, add-ons, upsell ("add a drink"); special-request note; live price
  recompute → "Add to cart" (price ON the button).
- **A5 Cart** *(CORE)*: line items w/ modifiers; edit qty, delete, duplicate; price breakdown
  (items, delivery, service fee, small-order fee, discounts/promo, tip, **total**); min-order
  control (block checkout + "add €X more"); switch to pickup; time ASAP vs scheduled slot; promo/
  voucher; subscription benefits; upsell/"frequently added".
- **A6 Checkout** *(CORE)*: confirm address + courier instructions; **contactless/leave-at-door**;
  time ASAP/scheduled; payment card/Apple-Google-Pay/**cash (where available)**/credits; promo,
  tip, note; final breakdown → "Order" → 3DS/payment confirm.
- **A7 Order tracking (real-time)** *(CORE — main loop)*: states ≈ accepted→venue-confirmed→
  preparing→ready→courier-assigned→picked-up→on-the-way→approaching→delivered; live map (venue,
  courier, dropoff) w/ ETA; courier card (name, photo, vehicle, rating); **masked-number call/
  chat**; order details, support; **cancellation** (window before confirm/prep); pickup variant
  (preparing→ready-for-pickup→pickup code); delivery confirmation (sometimes photo/PIN).
- **A8 Post-order**: 3-way rating (courier+venue+dishes); tip after delivery; text review; receipt,
  reorder, "report a problem"/refund, help; order history.
- **A9 Profile/account**: personal data, saved addresses, payment methods, subscription, promo/
  credits, language, notifications, history, support, privacy, logout/delete, referral.
- **A10 Cross-cutting (client)**: push (order statuses + promo); in-app chat/support; states
  loading(skeleton)/empty/error+retry/offline; busy/surge banners, ETA recompute, "just closed".

## B. Courier app
- Onboarding/verification (docs, vehicle, check, training); availability (go online/offline, zone,
  slot/shift booking); **offer** accept/reject w/ timer (payout, distance, pickup/dropoff); to-venue
  (nav→arrived→pickup confirm: item check / venue code); to-client (nav→arrived→handoff→delivery
  confirm PIN/photo/signature); batching (stacked orders); earnings (per-order, tips, bonuses,
  surge, cash/change handling); problems (venue closed, client unreachable timer+photo, reassign,
  cancel); demand heatmap, quests/incentives; ratings, stats, account, support.

## C. Partner side (restaurant/store)
- Onboarding (KYC, menu setup, hours, delivery zones, bank); **order tablet/dashboard**: **audible
  alert** new order → accept/reject → prep time → "ready"; menu management (items, categories,
  modifiers, photos, prices, availability/**stop-list**, schedules breakfast/lunch, "86" items);
  operational modes (busy mode ↑time, pause intake, early close); analytics/money (history, refunds/
  disputes, sales, ratings, top items, payouts); marketing (discounts/promo, sponsored/ads);
  reviews (view + reply); support.

## D. Cross-cutting patterns worth conceptually adopting
- One cart = one venue; transparent fees; ETA everywhere. Real-time status as the main retention
  loop (most important). Masked call, contactless, leave-at-door. Scheduling, pickup, reorder,
  favourites. 3-way rating + tips. busy/closed/stop-list/preorder states at EVERY level (venue,
  category, item) — Bolt is the reference for how to display them.

## E. Map to DeliveryOS (MVP / later / skip) — per the user's table
CORE-MVP: menu→item→cart→checkout→status (copy 1:1 conceptually); customization; real-time tracking
+ live courier map (MapLibre+WS already present); venue/item states busy/closed/stop-list
(mandatory); cash payment; cancellation-light (before confirm); owner **audible new-order alert**
(critical, iOS audio-context fix planned); owner accept/reject+prep-time+ready; menu mgmt + stop-list;
busy-mode-light; courier offer accept/decline w/ timer (base); courier nav→pickup(code)→delivery.
LATER: Apple/Google Pay+card (2Checkout/Verifone roadmap); 3-way rating (basic soon)+tips(later);
reorder/favourites/history; scheduled time/pickup; promo/vouchers (promo-builder is planned); courier
PIN/photo confirm; courier batching; owner analytics/payouts/disputes; masked call (MVP-consider, else
fallback phone). SKIP (marketplace mechanics): aggregated search/cross-restaurant collections; Bolt
Plus/surge/venue ads; Bolt Market/DineOut; courier earnings/incentives/heatmap/slots.

## F. Top-7 to consciously "borrow as an idea" in MVP
1. Order state machine + live map (main retention screen).
2. Transparent price breakdown on cart/checkout (zero surprises in total).
3. busy/closed/stop-list states at 3 levels.
4. Audible + persistent new-order alert for the owner.
5. Button with the total price ("Add • €X", "Order • €Y").
6. Masked client↔courier comms (or a clear fallback to the venue phone).
7. Simple courier offer accept/decline with timer + explicit action on each button.
