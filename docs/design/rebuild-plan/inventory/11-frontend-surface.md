# 11 — Frontend Surface Inventory & Rebuild Map (Lane B)

- **Date:** 2026-07-04 · **Lane:** B (FE surface → Astro 5 shell + Svelte 5 islands) · **Docs-only** — no code changed.
- **Stack authority:** `docs/design/rebuild-plan/06-complete-rebuild-stack.md` (Astro 5.x shell + Svelte 5 runes islands, route groups `/s/[slug]` + `/admin` + `/courier`, OpenAPI-generated TS client, Tailwind, i18n decided **in this doc §5**).
- **Method:** every census table carries its extraction command + count; rows reconcile to the count. Sources: direct reads of `apps/web/src` + `packages/ui/src` + 5 read-only census sweeps (storefront / admin / courier / ui-package / cross-cutting), all citations `file:line` against the working tree at `fix/audit-remediation`.
- 🔴 = money / auth / RLS / PII-adjacent → **council-before-port + Playwright red→green proof mandatory** (per 06 §, unchanged gates).

## 0. Machine-verifiable counts (reconciliation header)

| # | Census | Extraction command (run from `/root/dowiz`) | Count |
|---|--------|---------------------------------------------|-------|
| C1 | Route elements (react-router `<Route>`) | `grep -h '<Route ' apps/web/src/main.tsx apps/web/src/routes/*.tsx \| wc -l` | **40** |
| C2 | Page components | `find apps/web/src/pages -name '*.tsx' \| wc -l` | **35** |
| C3 | packages/ui components (.tsx) | `find packages/ui/src -name '*.tsx' \| wc -l` | **56** |
| C4 | apps/web non-page components (.tsx) | `find apps/web/src/components -name '*.tsx' \| wc -l` | **11** |
| C5 | apps/web total .tsx | `find apps/web/src -name '*.tsx' \| wc -l` | **52** (35 pages + 11 components + 4 routes + main.tsx + lib/CartProvider.tsx) |
| C6 | i18n catalog keys | `grep -cE "^  '[^']+':" packages/ui/src/lib/i18n-catalog.ts` | **1,445** (× 3 locales sq/en/uk) |
| C7 | i18n catalog weight | `gzip -c packages/ui/src/lib/i18n-catalog.ts \| wc -c` | **61,358 B gz** (219,160 B raw) |
| C8 | VITE_* flags referenced in FE | `grep -rhoE 'VITE_[A-Z0-9_]+' apps/web/src packages/ui/src \| sort -u \| wc -l` | **19** |
| C9 | E2E specs (main tree, parity oracle) | `find e2e apps/api/e2e -name '*.spec.ts' -not -path '*/node_modules/*' \| wc -l` | **175** |
| C10 | Dynamic i18n key families | `grep -rhoE 't\(\`[a-z_.]+\$' apps/web/src packages/ui/src \| sort -u \| wc -l` | **13** families (§5.2) |

Visual regression net: `e2e/visual/{client,owner,courier}-path.visual.spec.ts` + `harness.ts` — baselines are React-DOM screenshots → **status NEEDS-REBASE for every mapped row** (Astro/Svelte DOM will differ; the *assertion intents* port, the PNG baselines do not).

---

## 1. Route / page census (C1 = 40 route elements → 27 addressable paths)

Extraction: C1 above. All top-level surfaces are `React.lazy` route-split (`apps/web/src/main.tsx:20-28`); app shell = BrowserRouter + MotionConfig + I18nProvider + CurrencyProvider + ThemeProvider + ErrorBoundary + InstallPrompt (`main.tsx:69-88`).

### 1.1 Top level (`apps/web/src/main.tsx:49-62`) — 12 route elements

| Path | Component | Guard | Lazy | Flag | → Rebuild target | Proof |
|---|---|---|---|---|---|---|
| `/` | `Navigate → /start` | — | — | — | Astro redirect (`Astro.redirect`) | `e2e/tests/flow-start-hero.spec.ts` |
| `/start` | `StartPage` (MenuFirstOnboarding, anonymous mode) | none | ✓ | `VITE_ACCESS_GATE_PUBLIC_ENABLED` (gate sub-component) | Astro page + **OnboardingWizard island** `client:load` | `flow-onboarding-parsing.spec.ts`, `flow-start-hero.spec.ts` |
| `/login` | admin `LoginPage` | none (is the gate) 🔴 | ✓ | `VITE_GOOGLE_OAUTH_ENABLED` | Astro page + **AuthLogin island** `client:load` 🔴 | `flow-onboarding-auth.spec.ts`, `simple-auth.spec.ts` |
| `/privacy` | `PrivacyPage` (static, CI content-hash `PRIVACY_NOTICE_VERSION` `PrivacyPage.tsx:9`) | none | ✓ | — | **Astro static** (zero JS) | new: static-content hash check ports as CI gate |
| `/auth/callback` | `AuthCallback` (OAuth `#code=` exchange) 🔴 | none | ✓ | — | Astro page + tiny island `client:load` 🔴 | `flow-onboarding-auth.spec.ts` |
| `/claim` | `ClaimPage` (fragment `#token=`, scrubbed via `history.replaceState`, `ClaimPage.tsx:14-43`) 🔴 | claim token | ✓ | — | Astro page + **ClaimFlow island** `client:load` 🔴 | `flow-simpl-s6-claim.spec.ts` |
| `/s/:slug/*` | `ClientRoutes` | public | ✓ | — | **Astro SSR route group** `/s/[slug]` (§1.2, §4) | `client/*.spec.ts` (10 specs) |
| `/branding-preview/:slug/*` | `ClientRoutes` (iframe target of BrandingPage, postMessage themed) | public | ✓ | — | same Astro route group, `?preview=1` variant | `flow-ui-admin-branding.spec.ts` |
| `/admin/*` | `AdminRoutes` | `dos_access_token` guard in layout → redirect `/login` (`AdminRoutes.tsx:64-69`) 🔴 | ✓ | — | Astro route group `/admin` (§1.3) | `admin/*.spec.ts` (6) |
| `/courier/*` | `CourierRoutes` | per-page token 🔴 | ✓ | — | Astro route group `/courier` (§1.4) | `courier/*.spec.ts` (3) |
| `/courier-invite/:inviteId` | `CourierInvitePage` 🔴 | invite token + 16-char code | ✓ | — | Astro page + **InviteRedeem island** `client:load` 🔴 | `flow-ui-courier-invite.spec.ts` |
| `*` | `NotFound` (inline `main.tsx:90-104`, paper skin) | — | — | — | **Astro static 404** | `fe-polish-batch.spec.ts` (404 case) |

### 1.2 Client route group (`apps/web/src/routes/ClientRoutes.tsx:15-25`) — 4 route elements

| Path | Component | Behavior | → Rebuild target | Proof |
|---|---|---|---|---|
| `/s/:slug` (index) | `MenuPage` under `ClientLayout` | full storefront (§2.1) | Astro **SSR page** + islands (§7) | `client/menu.spec.ts`, `client/menu-interaction.spec.ts` |
| `/s/:slug/checkout` | `CheckoutRedirect` → `/s/:slug?checkout=1` | **redirect seam** — checkout is a bottom-sheet over the menu, never a bare page (`ClientRoutes.tsx:7-13`) 🔴 | Astro redirect; sheet opens from `?checkout=1` in CartCheckout island 🔴 | `flow-simpl-s1-sheet-checkout.spec.ts` |
| `/s/:slug/order/:id` | `OrderStatusPage` | live tracking (§2.3) | Astro page + **OrderTracker island** `client:load` | `client/status.spec.ts`, `client/status-live.spec.ts`, `client/order-stepper.spec.ts` |
| layout route `/` | `ClientLayout` | cart provider + sheets host | island-shared Svelte store (cart) + sheet host inside CartCheckout island | `client/cart.spec.ts` |

### 1.3 Admin route group (`apps/web/src/routes/AdminRoutes.tsx:257-283`) — 16 route elements (incl. layout route)

Layout: desktop sidebar (collapsible) + mobile sticky topbar + BottomTabBar (4 primary + More-sheet) + `AdminCommandCenter` (⌘K palette, g-sequences, `?` help — nav-only, `AdminRoutes.tsx:222-224`) + Sunlight/Currency/Language switchers. Auth guard at layout. `data-skin=paper`, `data-surface=dark`.

| Path | Page | Notes | → Target island (all `client:load` unless noted) | Proof |
|---|---|---|---|---|
| `/admin/login` | `Navigate → /login` | legacy alias | Astro redirect | — |
| `/admin` (index) | `AdminHome` triage → onboarding / activation / `DashboardPage` (`AdminRoutes.tsx:232-255`) | entry-flow O3: draft venue → activation tool | server-side triage in Astro (fetch settings server-side) → render matching island | `flow-admin-deep.spec.ts` |
| `/admin/orders` | `DashboardPage` | deep-link alias of index | **OrdersBoard island** 🔴 (status PATCH) | `admin/dashboard.spec.ts`, `admin/orders.spec.ts` |
| `/admin/menu` | `MenuManagerPage` | §2.5 | **MenuManager island** 🔴 (price writes) | `admin/menu-manager.spec.ts`, `undo-redo.spec.ts` |
| `/admin/supplies` | `SupplyLibraryPage` | localStorage-only (§2.6) | **SupplyLibrary island** | `admin/supplies.spec.ts` |
| `/admin/promotions` | `PromotionsPage` | 🔴 discounts | **Promotions island** 🔴 | `admin/promotions.spec.ts`, `flow-modifiers-promotions.spec.ts` |
| `/admin/branding` | `BrandingPage` | derivePalette + fonts + iframe preview | **Branding island** | `flow-ui-admin-branding.spec.ts`, `storefront-fonts-owner.spec.ts` |
| `/admin/couriers` | `CouriersPage` | 🔴 invites + earnings | **Couriers island** 🔴 | `flow-ui-courier-invite.spec.ts` |
| `/admin/analytics` | `AnalyticsPage` (only page ALSO React.lazy inside group, `AdminRoutes.tsx:19`) | charts + geo map + export | **Analytics island** `client:visible` (below-fold charts) | `flow-ui-analytics-supplies.spec.ts` |
| `/admin/crm` | `CRMPage` | 🔴 PII (masked-only) | **CRM island** 🔴 | `flow-admin-deep.spec.ts` |
| `/admin/settings` | `SettingsPage` | 🔴 fees/minOrder/pause | **Settings island** 🔴 | `flow-ui-admin-settings.spec.ts` |
| `/admin/onboarding` | `OnboardingPage` → `MenuFirstOnboarding mode="authed"` | 13-LOC wrapper | reuse OnboardingWizard island (authed prop) | `flow-onboarding-parsing.spec.ts` |
| `/admin/activation` | `ActivationPage` | 🔴 publish gate + iframe live-preview | **Activation island** 🔴 | `flow-onboarding-auth.spec.ts` |
| `/admin/_flow-test` | `FlowTestPage` | DEV-only, tree-shaken from prod (`AdminRoutes.tsx:24-26`) | **DROP from prod build** — keep as dev-only Astro page or move into Playwright fixtures | n/a (dev tool) |
| `*` | `Navigate → /admin` | — | Astro redirect | — |

### 1.4 Courier route group (`apps/web/src/routes/CourierRoutes.tsx:83-103`) — 8 route elements

Layout: header (Sunlight/Currency/Language) + BottomTabBar (tasks/earnings/history/shift); chrome hidden on delivery + login views; paper grain forced off on delivery for sunlight contrast (`CourierRoutes.tsx:41-55`); ToastProvider wraps all.

| Path | Page | → Target island | Proof |
|---|---|---|---|
| `/courier` | `TasksPage` (offers + 30 s countdown + WS `courier:{id}` + pull-to-refresh) | **CourierTasks island** `client:load` | `courier/tasks.spec.ts`, `courier/offer-timer.spec.ts` |
| `/courier/delivery/:id` | `DeliveryPage` 🔴 (cash-as-proof) | **Delivery island** `client:load` 🔴 (maplibre lazy-imported inside) | `flow-ui-courier-actions.spec.ts`, `capture-delivery.spec.ts`, `deliver-v2-cancel-revert.spec.ts` |
| `/courier/login` | courier `LoginPage` 🔴 | **CourierLogin island** `client:load` 🔴 | `courier/full-coverage.spec.ts` |
| `/courier/earnings` | `EarningsPage` 🔴 money display | **Earnings island** `client:visible` 🔴 | `flow-ui-courier-full.spec.ts` |
| `/courier/history` | `HistoryPage` | **History island** `client:visible` | `flow-ui-courier-full.spec.ts` |
| `/courier/shift` | `ShiftPage` | **Shift island** `client:load` | `flow-courier-deep.spec.ts` |
| `*` | `Navigate → /courier` | Astro redirect | — |

Reconciliation: 12 (main) + 4 (client) + 16 (admin) + 8 (courier) = **40 `<Route>` elements = C1 exact** (layout `<Route path="/">` wrappers and index routes counted; `/admin/_flow-test` DEV-conditional included; hand-verified against the four files — no route exists outside them).

---

## 2. Per-page UI element & state census

Format per row: elements · states (Task-Exit dimensions: loading/empty/error/success + rare) · mutations · flags · file:line · → target + proof.

### 2.1 `routes/ClientLayout.tsx` — storefront shell (cart + checkout sheets) 🔴

| Section | Elements | States | Mutations | file:line |
|---|---|---|---|---|
| Header chrome | sticky logo (hides onError), venue name, CurrencySwitcher, LanguageSwitcher (compact mobile / full desktop, `allowed={supportedLocales}`) | — | GET `/public/theme/:slug` (Zod, nullable-tolerant) | 131, 175-189 |
| Theme/branding | `derivePalette` + Google-Fonts injection (allowlist, egress-safe); postMessage live-preview protocol (`branding_preview_logo/theme/ready`); `draft_primary/bg/text` query params | — | GET `/public/locations/:slug/info` | 31-41, 101-153 |
| 🔴 Sticky cart bar | StickyActionBar + AnimatedNumber total, spring count badge (99+ cap), `cart-bounce` on `dos:bounceCart` | hidden when count=0 or checkout path | — | 193-220 |
| 🔴 Cart sheet | ResponsiveDialog: item rows, ±qty (44 px, aria-label), PriceDisplay, **free-delivery nudge** (role=progressbar / "unlocked"), Checkout btn, Clear (only >1 item) | empty state | `updateQuantity`/`clearCart` (CartProvider) | 221-304 |
| 🔴 Checkout bottom-sheet | ResponsiveDialog wrapping `<CheckoutPage onClose>` headerless; `?checkout=1` seam opens it, param stripped via replace-nav, closes on `/order/` | — | — | 74-86, 305-310 |
| Embed mode | `?embed=true` → body class `embed-mode`, sticky bar embed variant | — | — | 54, 63-67 |

→ Rebuild: cart = shared Svelte-runes store (module state) consumed by MenuBrowser + CartCheckout islands; sheets live in the **CartCheckout island** (`client:idle` — upgraded to eager on first add-to-cart). Proof: `client/cart.spec.ts`, `flow-simpl-s1-sheet-checkout.spec.ts`, `embed-mode.spec.ts`.

### 2.2 `pages/client/MenuPage.tsx` (1,811 LOC) — god-page, section-by-section

| Section | Elements | States | Mutations / IO | Flags | file:line |
|---|---|---|---|---|---|
| Hero / venue header | `vendor-info` band; backdrop precedence **hero image → self-hosted video → StylizedMap**, each degrading onError; dark scrim; Google rating stars + review count + external reviews link; StateChip venue state; `vendor-closes-at`; delivery ETA "~N min" | heroImg loading/ok/none; heroVideoOk; geoStatus unknown/granted/denied | GET `/public/locations/:slug/info` (:460); `/media/:id/hero/cover.webp` + `video.mp4`; geolocation (8 s timeout) → **OSRM** `router.project-osrm.org` ETA (:485) | — | 744-823, 458-491 |
| Closed/preview gates | preview banner (honest "demo, not live" + pitch list + hidden claim CTA `SHOW_CLAIM_CTA=false` :454); closed banner `venue-closed-banner`; busy banner (busy ≠ closed, ordering stays open) | `isClosed` (:446), `isPreview` (:450), `orderingDisabled` (:451) | `POST /api/claim/request` (:661) | — | 970-1049 |
| Sold-out / blocked add | card `isAvailable` sold-out state; add blocked: preview → toast `preview.cannot_order`, closed → toast `client.closed_cannot_order`; detail button disabled + relabeled | — | — | — | 1210-1211, 1703-1714 |
| Category chips + scroll-spy | `category-nav` chips (All + Chef's Picks ✦ + categories, available-counts); tap = **jump-nav anchor** (NOT filter) w/ double-rAF scroll; IntersectionObserver scroll-spy (rootMargin `-{offset+4}px 0px -62%`), disabled in flat/lens view; sticky height via ResizeObserver | active chip aria-pressed | — | — | 493-533, 826-868, 410-418 |
| Search / sort / lenses | SearchInput (persisted); price-sort tri-cycle; macro lenses ("Most protein", "Calories low→high"); allergen chips (**FROZEN** `ALLERGENS_ENABLED=false` hard const :42, operator freeze 2026-06-30); prefs persisted `dos_menu_prefs_{slug}` | search→flat filter; lens→ranked + explicit no-data bucket; empty → clear-filters state | — | `VITE_MENU_CHARACTERISTICS_FILTER`, `VITE_MENU_ALLERGEN_FILTER` (both OFF) | 252-338, 871-964 |
| Product cards | ProductCard (ui pkg): image precedence chain (:717-732), price, prep, kcal/macros from BOM (`bomToNutrition` :134-147), ingredient chips, taste, chefPick badge, sold-out, `hideAdd` in preview; compare toggle per card (max 2) | — | onAdd: direct `addItem` + `dos:bounceCart` + haptic `vibrate(12)` (:686) or open detail | `VITE_MENU_CHARACTERISTICS_COMPARISON` (OFF) | 1152-1223 |
| Product-detail sheet | `role=dialog aria-modal` bottom-sheet; sticky 56 px Close + grabber; Escape close (:605-610); dual scroll-lock (:590-601); rich-media hero (MediaGallery/MediaRenderer via Suspense) → image → gradient fallback; taste axes + level dots; "What's inside" macro tiles + ingredient chips; frozen allergen surface; **modifiers** radio/checkbox/select/quantity via `resolveDisplayType`, min/max enforcement (:612-628), required badges; qty stepper; `canAdd` validates required groups (:706-715) | — | add computes price+delta (:690-704) | media server-gated | 1273-1720 |
| Activation embed | `?activation=1` in iframe → postMessage `dos_activation_edit_product` instead of detail | — | — | — | 535-546 |
| Compare bar + panel | floating 1/2-count bar; `MenuComparePanel` | — | — | COMPARISON flag | 1231-1269 |
| Footer | name, address + Maps link, opening-hours table (today bolded), tap-to-call, socials (WhatsApp/IG/FB conditional); hidden in embed/activation | — | — | — | 1723-1807 |
| Load states | skeleton (300 ms min dwell); **404 notFound (no retry) vs fetchError (retry)**; empty-menu; soft refetch on locale change (no skeleton) | 4 distinct | GET `/public/locations/:slug/menu?locale=` (:375) | — | 368-405, 1053-1114 |
| 🔴 Cart reconciliation | `reconcileToMenu` on menu_version → toasts `cart.prices_updated` / `cart.items_removed` | — | — | — | 670-682 |

→ Rebuild: menu grid + hero + footer render **server-side in Astro** (SEO parity with today's bot SSR, §4); **MenuBrowser island** (`client:load`) hydrates chips/scroll-spy/search/detail-sheet/add-to-cart over the SSR HTML. OSRM ETA + geolocation stay client-side in the island. 🔴 cart-reconcile logic ports 1:1 (money-adjacent). Proof: `client/menu.spec.ts`, `client/menu-interaction.spec.ts`, `client/venue-state.spec.ts`, `client/modifier-display-type.spec.ts`, `storefront-characteristics.spec.ts`, `flow-productcard-declutter.spec.ts`.

### 2.3 Checkout flow 🔴 — `CheckoutPage.tsx` (787) + `checkout/*` (5 files)

| Section | Elements / fields | States | Mutations | file:line |
|---|---|---|---|---|
| Empty cart | icon + "Browse menu" | items=0 | — | CheckoutPage 516-542 |
| OrderSummaryAccordion | collapsed items + photos + combined price + DishStats nutrition | summaryOpen | GET menu (useOrderMenuMap) | OrderSummaryAccordion 22-70 |
| ContactInfoSection | name; **Communication `Select` (required)** — **6 messenger kinds** (`lib/messenger.ts:6-8`): **phone, whatsapp, viber, telegram, signal, simplex** (simplex = text-only w/ lock icon; `PHONE_KINDS={phone,whatsapp,viber,signal}` drive phone/OTP/dedup :11); per-kind handle input; "deliver to someone else" receiver block; entry-photo upload | commError/phoneError; sameReceiver | 🔴 `POST /public/entry-photo` multipart 60 s | ContactInfoSection 64-155 |
| DeliveryDetailsSection | MapWithPin; street (required); entrance/apartment (**required only if pin null/low-confidence** — §3 rule CheckoutPage:266-276); "how to find you" notes; 5 dropoff-instruction chips + custom | pickup/scheduled branches (scheduled = "coming soon") | — | DeliveryDetailsSection 56-165 |
| PaymentSection 🔴 | cash card; **cash amount** (min=total, red if <, change calc); courier tip 0–1,000,000 | cash<total warning | — | PaymentSection 44-95 |
| Crypto method 🔴 | radiogroup cash/crypto + irreversibility disclosure (USDT/USDC, 3-day refund) | paymentMethod | — | CheckoutPage 639-666 (`VITE_PAYMENTS_CRYPTO_ENABLED` OFF) |
| OrderSummarySection 🔴 | subtotal; delivery fee or "Calculated at checkout" (`feeKnown`); VAT inclusive-parenthetical vs exclusive-addend; tip line; "cash to courier incl tip"; nutrition; pre-order ETA 25–45 min | feeKnown | — | OrderSummarySection 40-127 |
| Errors | locationLoadFailed alert + refresh; orderError `role=alert` + scrollIntoView + **"Call restaurant" fallback**; privacy notice | placing; showPhoneFallback | — | CheckoutPage 670-728 |
| Submit 🔴 | `order-confirm-button`; success checkmark overlay → nav to `/order/:id` | placing | **`POST /orders`** — body: items, customer{phone (phone-kinds only), name, messenger_kind/handle}, receiver?, delivery_photo_key, tip_amount, delivery{pin,address_text}, payment.method, cash_pay_with, **`idempotency_key` (crypto.randomUUID)**, acknowledged_codes, prefs.dropoff, instructions ≤500 | 226-459, 730-783, body 311-359 |
| 🔴 Preflight/OTP | 200 `PreflightResponse`: `soft_confirm`+`requiresOtp` → OTPModal (`POST /customer/locations/:slug/otp/send` + `/verify` w/ `order_intent_hash`); non-OTP soft_confirm → auto-ack + single silent retry; `hard_block` → "review your cart". Error codes: `MIN_ORDER_NOT_MET`, `NOT_DELIVERABLE`, `CASH_AMOUNT_TOO_LOW`, `item_unavailable` (422/409) | otp states | above | 293-514 |
| Post-success | save `dos_last_delivery_{slug}`, clear draft `dos_checkout_draft_{slug}`, `requestPushPermission` (VAPID subscribe), `clearCart`; crypto → `redirectUrl` | — | GET `/push/vapid-public-key`; `POST /customer/push/subscribe` | 131-177, push.ts |

→ Rebuild: **CartCheckout island** (single island containing cart sheet + checkout sheet + OTP modal), `client:idle`; **entire section is 🔴 council-before-port**; money errors stay INLINE (never toast). Proof: `client/checkout.spec.ts`, `client/client-checkout-happy-path.spec.ts`, `flow-ui-client-checkout.spec.ts`, `flow-order-creation.spec.ts`, `ux2-messenger-deeplink.spec.ts`, `ux4-tips.spec.ts`.

### 2.3b `pages/client/OrderStatusPage.tsx` (798 LOC) 🔴

| Section | Elements | States | Mutations | file:line |
|---|---|---|---|---|
| Load/handoff | layout-matched skeleton; not-found/session-expired EmptyState + back-to-menu + call-restaurant; `?t=` track-token exchange → JWT, param stripped | loading/error | GET `/customer/orders/:id/status`; `POST /customer/track/exchange`; GET `/api/public/locations/:slug/fallback-config` | 152-214, 415-467 |
| WS live | room `order:{id}`; handles `order.route`/`order.courier_updated`/`order.status`/`order.message`; **terminal-lock** vs reordered frames; 30 s watchdog refetch | wsStatus → offline banner + call CTA | WS send `client_location`/`client_location_stop` | 216-300, 489-508 |
| Stepper | OrderProgress (pickup/delivery-aware), timestamps light steps | 8 statuses PENDING→DELIVERED + REJECTED/CANCELLED (label/variant/accent/message maps) | — | 20-64, 593-606 |
| Map/ETA | CourierLiveMap (route polyline, courier pin only after real fix, dest pin), WSStatusDot; **honest ETA always a range, never 0/single**; overdue copy | isPickup (no map); hasCourierFix | — | 371-413, 510-526 |
| Live actions | Share-my-location (`watchPosition`, auto-stop on DELIVERED); call courier; message courier (messengerLink) | IN_DELIVERY only; sharingLocation | WS `client_location` | 316-359, 634-691 |
| Terminal | "not charged" reassurance, order-again, call | terminal statuses | — | 608-632 |
| Details 🔴 | line totals, Total, tip line | — | — | 693-715 |
| Rating | 5-star tap-submit + comment ≤1000; Google-review invite on delivered | ratingBusy | `POST /orders/:id/rating` | 106-118, 717-757 |
| Nutrition snapshot | 4-tile est-only | kcal_total≠null | — | 759-781 |
| Messages | MessageThread presets | — | GET/`POST /orders/:id/messages` + `/read` | 120-150, 785-792 |

a11y: root `role=region aria-live=polite`, sr-only status announcer, cold-open silent toast adopt. No pull-to-refresh (WS watchdog instead). → **OrderTracker island** `client:load` 🔴 (money display + track-token). Proof: `client/status.spec.ts`, `client/status-live.spec.ts`, `client/order-stepper.spec.ts`, `flow-customer-track-link.spec.ts`, `flow-ui-order-status.spec.ts`.

### 2.3c Storefront supporting components (apps/web)

| Component | Purpose / behaviors | → Target | Proof |
|---|---|---|---|
| `client/MenuComparePanel.tsx` (138) | 2-col compare; neutral markers ("cheaper"/"faster" only); Escape+backdrop close, scroll-lock, **no full Tab focus-trap (documented gap :30-54)** | Svelte component inside MenuBrowser island (flag OFF) — fix focus-trap at port | `storefront-characteristics.spec.ts` |
| `client/DishStats.tsx` | SVG calorie ring + macro/ingredient bars; `compact` used (compare + checkout accordion); **`full` variant unused → DROP** | Svelte component (compact only) | visual NEEDS-REBASE |
| `client/StylizedMap.tsx` | decorative SVG map+pin, aria-hidden, zero network | **Astro static** (inline SVG in SSR) | visual NEEDS-REBASE |
| `AccessRequestForm.tsx` + `accessRequestOutcome.ts` | email+consent+honeypot → `POST /api/access-requests`; success = sent-consent AND 2xx; 429 rate error; mounted ONLY in MenuFirstOnboarding (flag OFF) | Svelte component inside OnboardingWizard island | `soft-access-gate.spec.ts` + unit test ports |
| `media/*` (5: MediaGallery, MediaRenderer, VideoClip, SpinViewer, RevealOverlay) | ADR-0002 rich media, all lazy + server-gated (endpoint returns `[]` when off): kind-dispatch renderer w/ error boundary; zero-CLS carousel (role=tablist, no auto-advance, neighbour prefetch); video useSaveData→poster + WCAG 2.2.2 pause; spin drag-scrub + 4 s timeout→poster; RevealOverlay particle dissolve **not mounted anywhere → DROP-CANDIDATE** | Svelte components lazy-imported inside MenuBrowser island (dark) | `flow-ui-images.spec.ts`, `flow-client-product-images.spec.ts` |
| `pwa/InstallPrompt.tsx` | beforeinstallprompt banner (Android) + iOS share-hint after 1.5 s; self-hides standalone/dismissed (`localStorage dowiz-pwa-install`); role=dialog + focus trap + Escape | **InstallPrompt island** `client:idle`, app-wide | `pwa-install.spec.ts` |
| `lib/voice/*` (gate.ts, handlers.ts, menuContext.ts, types.ts) | ADR-0015 client-only voice: handlers (setSort/lens/category/search/compare/readOrder/navigateCheckout + **confirm-gated addToCart only write**), `ConfirmationGate` single write sink. **FULLY DARK: MicFab exists in `packages/ui/src/voice/` but is mounted NOWHERE in apps/web (PR-3 mount unbuilt), `createVoiceGate` has zero app callers, `VITE_VOICE_ENABLED` declared but never read** (`vite-env.d.ts:9`, `gate.ts:9`; suite census §3.1) | port as dark lib into MenuBrowser island's module scope; MicFab stays unmounted (do not invent UI during port) | unit `voiceAdapter.test.ts` ports; no E2E (dark) |

### 2.4 `admin/DashboardPage.tsx` (839 LOC) — orders board 🔴

| Section | Elements | States | Mutations | file:line |
|---|---|---|---|---|
| Orders board | OrderCard 2-col grid, status filter chips, sort popover/MobilePicker, live/history tabs, SearchInput, quick-stats strip (AnimatedNumber) | loading = 4 shimmer cards; empty ×3 variants (no-active / no-past / no-match); error EmptyState + retry; success | 🔴 `PATCH /orders/:id/status` (:290) — REJECT/CANCEL behind `useConfirm`; honest-dispatch reconcile (`dispatched:false` / `no_courier`) | 271-321, 616-714 |
| New-order alert | persistent banner fallback, armed/muted/blocked pill, sound ping loop + haptics | armed/muted/blocked tri-state | sound unlock local-only | 200-229, 400-421 |
| Order messages | expand row + preset send | — | `POST /orders/:id/messages` (:241) | 231-249 |
| Courier live map | CourierLiveMap seeded from `/couriers/live`, WS position/shift updates | — | reads only | 106-182, 731-739 |
| Readiness checklist | 7-item progress (menu/phone/address/couriers/branding/placeOrder/telegram) | — | reads 5 owner endpoints | 41-101, 766-806 |
| Order detail | ResponsiveDialog + PriceDisplay + items | — | — | 809-835 |
| CSV export | `exportCSV(filteredOrders)` | — | — | 718-728 |

WS rooms `location:{id}:dashboard` + `location:{id}:couriers`; realtime = claim-check + debounced authed refetch (no PII on bus). Filters out synthetic `E2E …` orders display-only (:348). → **OrdersBoard island**, `client:load`; 🔴 status mutation ports under council. Proof: `admin/dashboard.spec.ts`, `dashboard-courier-pins.spec.ts`, `real-notifications.spec.ts`.

### 2.5 `admin/MenuManagerPage.tsx` (1,405 LOC) — god-page, section-by-section 🔴

| Section | Elements | States | Mutations | Flags | file:line |
|---|---|---|---|---|---|
| KitchenBusyToggle | toggle btn, 30-min window | loading; toast on fail | `PATCH /owner/locations/:id/kitchen-busy {busy_until}` (:126) | — | 105-149 |
| MenuScheduleEditor | collapsible, category Select, time inputs | best-effort | `POST /owner/locations/:id/menu-schedules` (:181); `DELETE …/:id` (:195) | — | 151-258 |
| Category CRUD | inline add input, delete | toast | `POST /owner/menu/categories` (:627); `DELETE …/:id` (:640) | — | 314-341, 623-646 |
| Filter toolbar | SearchInput, FilterMenu ×2 (sort, availability) popover/MobilePicker | — | — | — | 790-820 |
| Category tabs | horizontal snap-scroll w/ counts | — | lazy `loadProducts` | — | 822-836 |
| Product grid | cards: img + onError fallback, 🔴 PriceDisplay, availability `role=switch`, stock chip, allergen chips, edit/delete | shimmer loading; productsLoading; empty-category CTA; no-match | 🔴 `PATCH …/products/:id {available}` optimistic + rollback (:612); `DELETE …/products/:id` confirm (:597) | — | 838-1003 |
| Product preview | ResponsiveDialog, edge-bleed image, stop-listed overlay | — | — | — | 1009-1070 |
| Add/Edit form | ResponsiveDialog; UndoRedoButtons; photo picker (5 MB jpg/png/webp); MediaManager; Name/🔴Price/PrepTime/Desc; RecipeEditor (BOM); AllergenEditor; stock; 5×3 taste grid; available toggle | saving; validation (price>0, prep 1–1440) | 🔴 `POST /owner/menu/products` (:558) / `PATCH …/:id` (:555); `POST …/:id/image` multipart (:565) | `VITE_UNDO_REDO_ENABLED` (default ON), `VITE_MEDIA_RICH_ENABLED` (default OFF) | 1072-1218 |
| Undo/redo | `useHistoryStack` limit 100; ⌘/Ctrl+Z, Shift+Z, Ctrl+Y; snapshots exclude image fields | canUndo/canRedo | client-draft only (no API) | UNDO_REDO | 416-460 |
| PDF/photo import | 3-step wizard (upload→preview→done); merge/add_only/replace modes; drag-drop .pdf/image 10 MB; AI-privacy notice; preview counts + issues | importLoading, importError, done | `POST /owner/menu/import/preview` multipart (:664); `POST /owner/menu/import/commit {force:true}` (:680) | — | 648-705, 1220-1401 |

No product drag-reorder (only MediaManager has up/down). No bulk-edit beyond availability. → **MenuManager island** `client:load` 🔴 (price/product writes). Undo/redo ports as a Svelte-runes history store (same snapshot semantics); keyboard bindings port into the island. Proof: `admin/menu-manager.spec.ts`, `undo-redo.spec.ts`, `flow-ui-admin-menumanager.spec.ts`, `flow-ui-admin-product-bom.spec.ts`, `flow-onboarding-parsing.spec.ts` (import).

### 2.6 Remaining admin pages (compact census)

| Page | Key sections (elements) | States | Mutations (🔴 flagged) | file:line anchors | Proof |
|---|---|---|---|---|---|
| `SettingsPage.tsx` (769) 🔴 | store details (name/phone-E164/address); 🔴 deliveryFee + minOrder + MapWithRadius; 7-day hours; language sync→TG targets; Telegram connect (deep-link + QR + test send); notif categories (transactional always-on, flag `VITE_TG_CATEGORY_GATING` OFF); 🔴 fallback phone; 🔴 pause/resume delivery | shimmer loading, error EmptyState, justSaved, tgLoading/tgTesting, togglingDelivery | 🔴 `PUT /owner/settings` (:318, :733); `PUT …/notifications/targets/:id` (:176/:263/:274); `POST …/telegram/connect-init` (:236); `POST …/notifications/test` (:250); `PUT …/settings/fallback` (:207) | 394-743 | `flow-ui-admin-settings.spec.ts` |
| `CouriersPage.tsx` (586) 🔴 | header (active count, CSV export); 🔴 invite form (email + role, result = link + 16-char code); expandable courier rows (NO presence dot per ADR-0006, 🔴 earnings today/week/month, deliveries, shifts); all-neutral live map; order-detail dialog (🔴 total/cash, PII) | skeleton, error+retry, empty, detailsLoading, inviteError/inviteResult | 🔴 `POST /owner/locations/:id/courier-invites {role,email,ttl_hours:48}` (:144) | 144-583 | `flow-ui-courier-invite.spec.ts` |
| `BrandingPage.tsx` (513) | auto-generate from website URL; 3× ColorInput + AA contrast warnings (text<4.5, primary<3); font Selects from `FONT_ALLOWLIST`/`fontIdsForRole` w/ live `googleFontsHref` preview; logo upload 2 MB; Google/social fields; sandboxed iframe `/branding-preview/:slug` + postMessage live theme; UrlRow copy | generating, contrastWarnings role=status, logoUploading, previewLoaded skeleton | `POST /owner/brand/generate` (:149); `PUT /owner/brand` (:115); `POST …/theme/logo` multipart (:96) | 96-509 | `flow-ui-admin-branding.spec.ts`, `storefront-fonts-owner.spec.ts` |
| `AnalyticsPage.tsx` (512) | 4 KPI cards (AnimatedNumber, 7d/30d); CSV+JSON export per section; SimpleBar revenue chart; top-products drill-down; ingredient consumption = honest "not available" EmptyState; day×hour heatmap; geo heatmap (react-map-gl/maplibre, hardcoded Carto style :499 — **inconsistency, ignores VITE_TILE_STYLE_URL**) | skeleton, error+retry, no-data period-aware, productOrdersLoading | reads only (`/owner/analytics*`) | 129-509 | `flow-ui-analytics-supplies.spec.ts` |
| `PromotionsPage.tsx` (469) 🔴 | form: code, type (percentage/fixed/free_delivery), 🔴 discount_value, min_order, validity window, max_uses; list cards w/ 🔴 formatALL discount, expired/usage chips, active Toggle, edit dialog | saving, saveError, field errors, skeleton, error+retry, empty CTA | 🔴 `POST /owner/promotions` (:271); `PATCH …/:id` (:284/:296 non-optimistic); `DELETE …/:id` (:318) | 31-465 | `admin/promotions.spec.ts` |
| `SupplyLibraryPage.tsx` (425) | **entirely localStorage** (`dos_supplies`, :25; 15 seeded supplies :27-45 — NO server persistence); form (kind/unit/nutrition-per-100/allergen chips/reorder threshold); search + sort + SegmentedControl filter | loading, empty ×2 | localStorage only | 25-425 | `admin/supplies.spec.ts` |
| `CRMPage.tsx` (417) 🔴 | count + search + sort + CSV export; desktop table / mobile cards; 🔴 masked phone ONLY (reveal is order-scoped by design :62-67); 🔴 LTV PriceDisplay; customer detail (prefs, orders w/ PII, heatmap) — never fabricates on error. **No GDPR-erase action exists on this page** (gap: erase is operator-side SQL, see audit lane) | skeleton, error+retry, empty/no-match, detail loading/error/null | reads only | 62-417 | `flow-admin-deep.spec.ts` |
| `ActivationPage.tsx` (371) 🔴 | 3-gate publish trinity (menu/notifications/fulfillment), pickup toggle, test-order row; inline product-edit modal (hand-rolled `role=dialog`, NOT ResponsiveDialog :333-367) triggered by iframe postMessage `dos_activation_edit_product`; iframe preview `/s/:slug?embed=true&activation=1` + 8 s draft poll | skeleton, settingsError, published/draft, savingProduct | 🔴 `POST /owner/activation/:id/pickup` (:105); 🔴 `POST …/publish` (:120); 🔴 `PATCH /owner/menu/products/:id {name,price}` (:86) | 86-368 | `flow-onboarding-auth.spec.ts` |
| `RecipeEditor.tsx` (266) | BOM: supply picker (kind tabs, search, multi-select from `loadSupplies`), qty steppers, auto nutrition rollup (complete/incomplete), BOM-allergen chips → AllergenEditor | — | none (local via onChange) | 31-266 | `flow-ui-admin-product-bom.spec.ts`, `flow-ingredients.spec.ts` |
| `AllergenEditor.tsx` (158) | tri-state `unset/none/listed` attestation (unset blocks publish), EU_ALLERGENS chips, BOM cross-check advisory | — | none | 13-117 | `flow-allergen-authoring.spec.ts`, `storefront-allergen-step0.spec.ts` |
| `LoginPage.tsx` (196) 🔴 | email/password; Telegram start+poll (5-min deadline, 410 handling); Google OAuth link (flag OFF); dev mock-auth (`?dev=true` only); sessionExpired banner | loading, tgWaiting, error 401 | 🔴 `POST /auth/local/login` (:76); `POST /auth/telegram/start` (:42) + poll (:50); `POST /dev/mock-auth` (:96); tokens → sessionStorage + safeStorage | 38-196 | `flow-onboarding-auth.spec.ts` |
| `AuthCallback.tsx` (38) 🔴 | reads `#code=` → exchange → store tokens → `/admin` | error text | 🔴 `POST /auth/exchange {code}` (:18) | 12-38 | `flow-onboarding-auth.spec.ts` |
| `OnboardingPage.tsx` (13) | wrapper `<MenuFirstOnboarding mode="authed" />` | — | — | 11-13 | `flow-onboarding-parsing.spec.ts` |
| `FlowTestPage.tsx` (351, DEV-only) | E2E lifecycle harness (create→transition→assign→deliver) | — | dev-only order mutations | — | n/a — DROP from prod (already tree-shaken) |
| `components/admin/MediaManager.tsx` | rich-media gallery: presign→raw PUT→confirm; set-primary; hide w/ confirm; reorder via up/down (no dnd-kit); lists via public endpoint | loading, uploading progress, empty, busyId; 413=budget/400=rejected | `POST …/media/presign` (:121); PUT presigned (:143); `POST …/media/confirm` (:155); `…/set-primary` (:174); `PATCH …/media/:id` (:198); `POST …/media/reorder` (:223) | 69-340 | `flow-ui-images.spec.ts` (flag-on env) |
| `components/admin/AdminCommandCenter.tsx` | CommandPalette (fuzzy nav, no mutations) + ShortcutsHelp sheet; `mod+k`, `?`, g-sequences `g o/m/c/a/s` (:22-28) | returns null if flag off | none | 36-116 | new test (keyboard nav) — no existing spec |

Admin cross-cutting: ResponsiveDialog (shared focus-trap/Escape/backdrop/scroll-lock) used by Dashboard/MenuManager/Couriers/Promotions/SupplyLibrary dialogs; ActivationPage's hand-rolled dialog is the one divergence → normalize to the shared Svelte sheet primitive in rebuild. a11y baseline everywhere: `role=switch/tab/dialog/alert/status`, `aria-busy` skeletons, `aria-live=polite`, `min-h 44px` tap targets, `motion-reduce` branches, `focus-visible` rings — **this is the porting bar, not an aspiration.**

### 2.14 Courier pages census

| Page | Key sections | States (incl. rare) | Mutations | file:line | Proof |
|---|---|---|---|---|---|
| `TasksPage.tsx` (226) | online/offline pulse badge (shift-derived); TaskCard list w/ accept/decline + 30 s offer countdown; accept-error `role=alert` banner (survives list emptying); pull-to-refresh (`VITE_PULL_TO_REFRESH_ENABLED`); WS room `courier:{courierId}` → `task_assigned` prepend + ping sound; ⚠️ courierId parsed client-side from JWT via `atob` w/ `'c1'` fallback (:30-40) 🔴 | skeleton; error+retry; empty ×2 (online/offline w/ CTA); success staggered; 410 expired-offer | GET `/courier/me/assignments` (:45); `POST /courier/assignments/:id/accept` (:116); `…/reject` (:132, optimistic + restore) | 26-223 | `courier/tasks.spec.ts`, `courier/offer-timer.spec.ts`, `pull-to-refresh.spec.ts` |
| `DeliveryPage.tsx` (560) 🔴 | full-screen CourierLiveMap (me-marker + dest pin + client pin + route line); GPS-denied banner; WS+GPS status dots; no-customer-coords honest banner (no fake pin); drop-off card (ETA, note, entry-photo modal, tip); 🔴 cash panel Total/Pays/Change; contact (tel:, messenger deep-link, MessageThread presets); order-closed banner; pickup btn; 🔴 deliver = cash input + SwipeToComplete; celebration → auto-nav | loading, task-not-found shell, geoError, wsStatus ×6, orderClosed, deliverError (409/422 → closed + auto-nav), offline→toast on pickup | `POST /courier/assignments/:id/picked-up` (:214); 🔴 `POST …/delivered {payment_outcome:'paid_full', cash_amount: task.total}` (:239-244); GPS `POST /courier/shifts/ping` every 12 s (:191-209); `POST /orders/:orderId/messages` (:72) | 14-556 | `capture-delivery.spec.ts`, `deliver-v2-cancel-revert.spec.ts`, `flow-geo-tracking.spec.ts`, `flow-offline-phone-fallback.spec.ts` |
| `ShiftPage.tsx` (303) | on-shift pulse; live elapsed timer (1 s, client-computed); Start/End shift; 🔴 today stats (deliveries/earnings/distance/online); messenger prefs (telegram/whatsapp/viber + handle) | skeleton, error+retry, active/offline, msgSaving/msgSaved | `POST /courier/me/shift/start` (:122) / `end` (:140); `PATCH /courier/me/messenger` (:54) | 51-300 | `flow-courier-deep.spec.ts` |
| `EarningsPage.tsx` (231) 🔴 | Today/Week/Month CountUp PriceDisplay cards + tips subline; payout history rows (reference/date/amount/StatusBadge) | skeleton, error+retry, unexpected-response error, empty payouts | GET only (`/courier/me/earnings`, Zod-parsed) | 47-226 | `flow-ui-courier-full.spec.ts` |
| `HistoryPage.tsx` (159) | delivery cards (StatusBadge, amount, locale dates sq-AL/en-GB/uk-UA, 5-star, feedback) | skeleton, error+retry, empty | GET only | 40-157 | `flow-ui-courier-full.spec.ts` |
| `LoginPage.tsx` (121) 🔴 | email/password, shake-on-error, wordmark, LanguageSwitcher | loading, 401 vs generic error | 🔴 `POST /courier/auth/login` → JWT → safeStorage (:24-34) | 18-116 | `courier/full-coverage.spec.ts` |
| `CourierInvitePage.tsx` (245) 🔴 | invite validation (distinct expired/used/revoked/not-found copy); onboarding form (name/email/phone-E164/password≥12/16-char code) | loadingInvite, inviteError branches, submitting, 410/401 branches | GET `/courier/auth/invites/:inviteId` (:54); 🔴 `POST …/redeem` → JWT (:85-99) | 44-244 | `flow-ui-courier-invite.spec.ts` |

Courier rebuild risks recorded: (a) **no wake-lock / background GPS** — tracking dies on screen-sleep (zero `wakeLock` hits repo-wide); rebuild should add Screen Wake Lock API to the Delivery island (new capability, flag-gated). (b) `cashCollected` input value is **not transmitted** — delivered body always sends `task.total` (`DeliveryPage.tsx:239-244` vs :49,:536-544) — port-as-is + flag to money lane, do NOT silently "fix" during port 🔴. (c) JWT `atob` client parse w/ `'c1'` fallback → replace with server-echoed identity in OpenAPI client 🔴 (council). (d) two map stacks (MapLibreBase vs react-map-gl in Analytics) → converge on one Svelte maplibre wrapper.

### 2.21 Misc top-level pages

| Page | Sections | States | Mutations | file:line | Proof |
|---|---|---|---|---|---|
| `MenuFirstOnboarding.tsx` (380) | phases `choose→parsing→review/blank→submitting` (:33); hero + dropzone (`upload-menu-cta`) + "start without menu" + account link; AccessRequestGate (flag OFF); parse spinner; review card (item/category counts) + name/phone/slug (`.dowiz.org`, reserved-words + ë/ç slugify :23-31); 🔴 anonymous Telegram claim (deep-link + 5-min poll → dual token persist). **No three.js/PaperScene — WebGL landing was rolled back** (:18-19), CSS animations only | choose/dragActive/parsing/review/blank/submitting; UNSUPPORTED_TYPE vs generic parse error; slugError; 409 SLUG_TAKEN; tgWaiting/timeout/410 | `POST /owner/menu/import/anonymous` multipart 120 s timeout (:95); `POST /owner/onboarding/start` (:119); 🔴 `POST /auth/telegram/start` (:143) + poll (:151) | 21-378 | `flow-onboarding-parsing.spec.ts`, `flow-start-hero.spec.ts` |
| `ClaimPage.tsx` (159) 🔴 | token from URL **fragment** `#token=` + session stash + immediate `history.replaceState` scrub, held in ref only (:14-43); accept (authed) / sign-in-to-claim (stash → `/login`) / decline; humane error per code (`INVALID_OR_EXPIRED_TOKEN`/`ALREADY_CLAIMED`/`CONTACT_MISMATCH`/`CONTACT_REQUIRED`); claim binds ownership only, never publishes | idle/working/claimed/declined/error | 🔴 `POST /claim/accept {token}` (:59); `POST /claim/decline` (:73) | 14-156 | `flow-simpl-s6-claim.spec.ts` |
| `PrivacyPage.tsx` (51) | static GDPR notice (legal basis, data, purpose, 12-mo retention, rights, `mailto:privacy@dowiz.org` erasure contact), CI content-hash bound `PRIVACY_NOTICE_VERSION` (:9) | — | — | 9-51 | port CI hash gate |

---

## 3. Component census — packages/ui (C3 = 56 .tsx) + apps/web (C4 = 11)

Extraction: C3/C4 commands (§0). Used-by = `grep -rl "\b<Name>\b" apps/web/src packages/ui/src` excluding self + index barrels; aggregate files re-checked by exported symbol (Base→Button/Input/FormField/StatusBadge, Status→EmptyState/SkeletonBase/WSStatusDot, etc.). Targets: **SvC** = Svelte component usable inside any island (and SSR-rendered by Astro where static), **store** = Svelte-runes module store/context, **static** = Astro-only, **DROP** = dead.

### 3.1 packages/ui components (all 56 accounted)

| Component | Purpose | Used-by (files) | Target | Notes |
|---|---|---|---|---|
| `components/Base.tsx` | Button/Input/FormField/StatusBadge primitives | Button in 18 apps/web files; StatusBadge 2 | SvC | foundation set |
| `components/Status.tsx` | SkeletonBase / EmptyState / WSStatusDot | EmptyState 15, WSStatusDot 3 | SvC | EmptyState embeds ArtNouveauDivider |
| `components/ErrorBoundary.tsx` | app-level render-error catch | main.tsx | `<svelte:boundary>` (Svelte 5) per island + Astro error page | behavior parity: chunk-load failures surface, never blank page |
| `components/NomadicScene.tsx` | `ArtNouveauDivider` ornament (15 LOC — remnant of rolled-back WebGL landing) | Status.tsx:3,27 | SvC (tiny) | alive via EmptyState; rest of scene already deleted |
| `components/PaperIllustration.tsx` | named paper illustrations (empty states) | courier TasksPage | SvC/static | |
| `admin/AdminUI.tsx`, `client/ClientUI.tsx`, `courier/CourierUI.tsx` | pure re-export barrels (4/16/3 LOC; ClientUI also `CartItem` type) | — | **DROP (dissolve)** | barrels don't port |
| `admin/Toggle.tsx` | switch w/ `role=switch` | 4 | SvC | |
| `admin/ColorInput.tsx` | color picker + hex | 2 (Branding) | SvC | |
| `admin/OrderCard.tsx` | dashboard order card; `VITE_OWNER_TWO_TAP` two-tap confirm (:14,:234) 🔴 | 3 | SvC 🔴 | status actions |
| `atoms/PriceDisplay.tsx` | single money-render authority 🔴 | **20** | SvC 🔴 | ports with formatMoney; no client math |
| `atoms/CurrencySwitcher.tsx` | ALL/EUR switch | 3 layouts | SvC | |
| `atoms/SearchInput.tsx` | debounced search | 6 | SvC | |
| `atoms/SegmentedControl.tsx` | segmented filter | 1 | SvC | |
| `atoms/Select.tsx` | styled select | 10 | SvC | |
| `atoms/SunlightToggle.tsx` | high-contrast sunlight mode (pre-paint script in index.html) | 2 layouts | SvC + inline head script ports to Astro | `sunlight-mode.spec.ts` |
| `atoms/Textarea.tsx` | textarea | 3 | SvC | |
| `client/OrderProgress.tsx` | status stepper (pickup/delivery-aware) | 2 | SvC | `client/order-stepper.spec.ts` |
| `client/OTPModal.tsx` 🔴 | checkout OTP dialog | 4 | SvC 🔴 | part of CartCheckout island |
| `client/ProductCard.tsx` | storefront product card (photo/price/macros/badges/sold-out) | 3 | **SvC, SSR-rendered by Astro + hydrated** | dual server/client render is the SEO seam |
| `client/StateChip.tsx` | venue open/closed/busy chip | 2 | SvC | `client/venue-state.spec.ts` |
| `courier/SwipeToComplete.tsx` 🔴 | swipe-to-deliver gesture | 2 | SvC 🔴 | pointer-gesture port; money proof |
| `courier/TaskCard.tsx` | offer card + 30 s countdown | 2 | SvC | `courier/offer-timer.spec.ts` |
| `molecules/ResponsiveDialog.tsx` | THE sheet/dialog primitive: focus-trap, Escape, backdrop, scroll-lock, mobile bottom-sheet / desktop modal | **19** | SvC — highest-leverage port; build FIRST | normalize ActivationPage's hand-rolled dialog onto it |
| `molecules/Toast.tsx` | ToastProvider/useToast | 3 route groups + pages | store + SvC | S4 rule: every owner mutation failure → toast |
| `molecules/ConfirmDialog.tsx` | useConfirm destructive gate | 1 | SvC | REJECT/CANCEL/delete flows |
| `molecules/MobilePicker.tsx` | mobile wheel-picker for sorts/filters | 3 | SvC | |
| `molecules/BottomTabBar.tsx` | mobile tab bar (admin/courier) | 2 layouts | SvC in shell islands | |
| `molecules/StickyActionBar.tsx` | storefront sticky cart bar (embed variant) | 2 | SvC | |
| `molecules/AnimatedNumber.tsx` / `AnimatedCheck.tsx` | count-up number / success check | 3 / 1 | SvC (Svelte tween) | reduced-motion collapse |
| `molecules/LiveDot.tsx` | pulsing live indicator | 3 | SvC | |
| `molecules/MessageThread.tsx` | order chat w/ presets (status-keyed) | 3 (owner/courier/customer) | SvC | `cr8-order-messages.spec.ts` |
| `molecules/Pressable.tsx` | tap-scale wrapper | 1 | SvC or CSS-only | consider CSS `:active` — YAGNI check at port |
| `molecules/PullToRefreshIndicator.tsx` | touch pull-to-refresh | 1 (courier Tasks) | SvC + attachment | flag ON |
| `molecules/UndoRedoButtons.tsx` | undo/redo toolbar | 2 (MenuManager) | SvC | pairs w/ history store |
| `molecules/TourHint.tsx` (`HintCard`) | onboarding hint card | 2 | SvC | |
| `molecules/MapLibreBase.tsx` | shared imperative maplibre wrapper: dynamic `import('maplibre-gl')` (:71), `VITE_TILE_STYLE_URL` (:17-19), DOM markers, courier arrow w/ bearing, routeLine, radiusCircle, flyTo, error text | 4 | SvC w/ Svelte attachment; **the ONLY map stack** (Analytics converges here) | lazy import preserved |
| `molecules/CourierLiveMap.tsx` | live map (courier/dest/client pins, honest placeholder when nothing to plot) | 4 | SvC | `dashboard-courier-pins.spec.ts` |
| `molecules/MapWithPin.tsx` | checkout pin picker 🔴 (address confidence drives required fields) | 1 | SvC 🔴 | |
| `molecules/MapWithRadius.tsx` | settings delivery-radius editor 🔴 | 1 | SvC 🔴 | |
| `organisms/CommandPalette.tsx` / `ShortcutsHelp.tsx` | ⌘K fuzzy nav / shortcuts sheet | 1 each (AdminCommandCenter) | SvC in AdminShell island | flag ON |
| `lib/I18nProvider.tsx` | locale context (`useI18n`) | **23** | Paraglide runtime + tiny locale store (§5) | |
| `lib/CurrencyProvider.tsx` | currency context + formatMoney 🔴 | 3 | store 🔴 | money-render authority |
| `lib/sound-prefs.tsx` | SoundPrefsProvider/useSoundPrefs (owner alert prefs) | 1 (Dashboard) | store | |
| `theme/ThemeProvider.tsx` | per-tenant CSS-var injection | 4 | Astro inline `<style>` vars at SSR + tiny store for live preview | §3.3 |
| `voice/*` — 8 components: MicFab, ConfirmChip, DisambiguationChips, DisclosureSheet, ErrorPill, PartialTranscriptPill, ReadBackPanel, VoiceSettingToggle (+ state-machine.ts, useVoiceControl.ts, layout.ts, types.ts) | ADR-0015 voice UI suite — **entirely dark: zero apps/web mounts** (only `lib/voice` type refs); dir is untracked-new (preserved from sandbox worktrees, commit a43485d0) | ConfirmChip 3 / MicFab 9 / Disambiguation+Disclosure+ErrorPill 1 (all intra-suite); PartialTranscriptPill, ReadBackPanel, VoiceSettingToggle **0 refs even intra-suite** | port dark as Svelte suite WITH the voice-fe-mount plan (`docs/design/voice-fe-mount/`), or defer whole suite to that lane | do NOT wire during port; 3 zero-ref components = DROP-or-wire decision at mount time |

Reconciliation: 44 rows above cover 56 files (voice row = 8 files; barrel row = 3; Base/Status export-sets = 2; AnimatedNumber/Check row = 2; CommandPalette/ShortcutsHelp row = 2). Dead findings: **3 barrels dissolve; 3 voice components zero-ref; RevealOverlay (apps/web) unmounted; DishStats `full` variant unused; tileConfig.ts dead seam** — everything else has live consumers.

### 3.2 packages/ui hooks (13 files, `packages/ui/src/hooks/`)

`use-breakpoint` (useIsMobile), `use-cart`, `use-courier-marker`, `use-delivery-eta`, `use-embed`, `use-geolocation` (watchPosition, 5-state GeoStatus), `use-geo-stream`, `use-haptics`, `use-history-stack` (+test; undo/redo, limit 100), `use-keyboard-shortcuts` (mod+k, sequences), `use-online`, `use-pull-to-refresh`, `use-sound`. → All become Svelte-runes utilities (`$state`/`$effect` modules) or attachments; `use-history-stack` and `use-keyboard-shortcuts` port with their unit tests.

### 3.3 Theme / tokens (SoT preserved per 06 §1)

- `theme/tokens.css` — 506 lines, **251 custom properties**; families: `--status-*` (61), `--brand-*` (44), `--color-*` (22), `--paper-*` (12), `--motion-*` (8), `--radius-*` (6), `--ink-*` (6), `--action-*` (6), `--z-*` (5), `--text-*` (5), `--weight-*` (4), `--safe-*` (4) + tail. Ports verbatim as the Tailwind theme bridge (CSS vars stay the runtime authority for per-tenant theming).
- **Paper skin:** `data-skin="paper"` via `paperSkinAttr()` — `VITE_PAPER_SKIN==='on'` OR runtime `localStorage dos_paper_skin` (`theme/paperSkin.ts:10-13`); internal admin/courier only; grain disabled on courier delivery view. `data-surface="dark"` marks dark-context sections.
- **derivePalette** (`theme/palette.ts:105`): `{primary,bg,text}` → full ThemeConfig; enforces AA (`ensureContrast(text,bg,4.5)`, `readableOn` fg poles, surfaces stepped toward fg). Guardrail test `palette.contrast.test.ts` ports to the new package (red→green re-proof).
- **Fonts** (`theme/fonts.ts:39`): **13-font FONT_ALLOWLIST** (ids = stored/transmitted values; roles heading/body/both; `base:true` self-hosted vs Google-loaded; `CYRILLIC_SAFE_FALLBACK` for Latin-only faces); `fontIdsForRole`, `googleFontsHref` egress-safe (base→null, junk dropped — `fonts.test.ts:84-93` ports). Owner picker in Branding; per-tenant application at SSR.
- **ThemeProvider:** injects per-tenant CSS vars; in rebuild the *initial* vars render server-side in Astro (`<html style="--brand-…">` — kills theme-flash), the store only handles live branding-preview postMessage updates.

### 3.4 apps/web components (C4 = 11)

Censused in context above: `AccessRequestForm` (§2.3c), `admin/AdminCommandCenter` + `admin/MediaManager` (§2.6), `client/DishStats` + `client/StylizedMap` (§2.3c), `media/*` ×5 (§2.3c), `pwa/InstallPrompt` (§2.3c). Plus `lib/CartProvider.tsx` (52nd tsx): cart state + persistence + reconcile seam → **cart runes store** 🔴 (shared by MenuBrowser + CartCheckout islands; unit tests `cartReconcile` port).

---

## 4. Storefront SSR/SEO + PWA behaviors (parity contract for the Astro shell)

> This section is the **behavioral contract** the Astro `/s/[slug]` route must reproduce natively (today it is hand-rolled server HTML; in the rebuild it becomes Astro's actual output — bot/human split disappears for the menu page).

### 4.1 Bot-UA SSR (today)

- Dispatch: `apps/api/src/routes/public/ssr.ts:18-52` — `GET /s/:slug`; bot regex `spa-shell.ts:15-19` (google/bing/yandex/facebook/whatsapp/telegram/slack/discord/twitter/linkedin/applebot/petalbot/semrush/ahrefs/…).
- **Rendered for bots** (`ssr-renderer.ts:267-442`, server LRU 50 entries/60 s): full category→product grid (name, description, `/images/{key}`, formatted price, add button); `<h1>` venue name; empty → "Menu not available yet."
- **JSON-LD** (`buildJsonLd` :66-158): `Restaurant` (servesCuisine Albanian, priceRange, PostalAddress, GeoCoordinates, openingHoursSpecification from `hours_json`) + `Menu`/`hasMenuItem` (first 30 items as `MenuItem`→`Offer`, `toMajorUnits`, EUR|ALL) + `BreadcrumbList` + `FAQPage` (>2 items); `</script>`-breakout escaping (:154-157).
- **Meta/OG** (:249-265, :346-354): `og:title "{name} — Order Online | Dowiz"`, description, url, type, site_name, `og:locale sq_AL`, twitter summary_large_image, `og:image /og-image.png`, canonical, theme-color `#ea4f16`; **hreflang** per locale + x-default (:240-247).
- **Shadow tenants** (unconsented demos): `read_preview_menu` → `X-Robots-Tag: noindex,nofollow`, generic title, no real name (ssr.ts:33-40, renderer :294-296). This privacy behavior is a **hard parity requirement**.
- Human shell: `serveSpaShell` (`spa-shell.ts:116-190`) injects per-tenant title+OG into built index.html; bespoke CSP (plausible.io, openfreemap tiles, OSRM, wikipedia; R2 widened only when voice live :154-157); `Cache-Control: no-cache`.
- **sitemap/robots** (`apps/api/src/routes/public/seo.ts`): `robots.txt` (:45-87, allow `/s/`, disallow cart/checkout/order/admin/courier/api; explicit AI-crawler allowlist GPTBot/ClaudeBot/PerplexityBot/…); `sitemap.xml` index sharded 50k (:90-118) + per-shard hreflang URLs (:121-163), excludes shadow tenants + product-less locations.

→ Rebuild mapping: Astro SSR renders the SAME HTML for bots and humans (islands hydrate over it); JSON-LD/OG/hreflang become an Astro component; robots/sitemap stay API-side (Rust) or move to Astro endpoints — Lane C decision; shadow-noindex is 🔴-adjacent (privacy) → carry the E2E. Proof: `seed-public.spec.ts`, `storefront-smoke.spec.ts`, new JSON-LD snapshot test.

### 4.2 PWA + offline semantics

- **SW** `apps/api/public/sw.js`: cache `dowiz-shell-v1`; install→skipWaiting; activate→purge old versions + claim; **cache-first GET, same-origin, basic only; explicitly NEVER intercepts `/api/*` or `/ws/*`** or sw.js/webmanifest.
- **Money red-line CONFIRMED HOLDING:** zero hits for background-sync/SyncManager/periodicSync/outbox/offline-queue across `apps/web/src` + `apps/api/public`. Order POSTs go straight through `apiClient` with body `idempotency_key`; **never cached, never queued, never replayed**. Offline = previously-cached GET shell only; no offline fallback page. → Rebuild MUST preserve: SW scope excludes all API POSTs; add a CI guardrail grepping the new SW for sync/queue primitives (ports regression intent).
- Manifest `apps/api/public/manifest.json`: standalone, portrait, 192/512 maskable, shortcuts (`/s/dubin-sushi`, `/orders`); **inconsistency:** index.html links `/manifest.json` while ssr-renderer references per-tenant `/s/:slug/manifest.webmanifest` (:357) — resolve at rebuild (per-tenant manifest wins for installed storefronts).
- index.html head: `lang="sq"`, theme-color, apple-web-app metas, Google-Fonts preconnect, pre-paint Sunlight-Mode inline script, SW registration, Plausible analytics.

## 6. FE cross-cutting infrastructure census

*(File order note: §6 precedes §5 on the page; section numbers track the Lane-B mission areas, not page order.)*

### 6.1 apiClient (`apps/web/src/lib/apiClient.ts`) → OpenAPI-generated client + thin wrapper

| Semantic | Behavior (today) | file:line | Rebuild disposition |
|---|---|---|---|
| Base URL | `VITE_API_BASE_URL \|\| '/api'` | :4, :138 | env of Astro build |
| Headers | JSON unless FormData; `Authorization: Bearer` from safeStorage; `X-Idempotency-Key` on POST/PUT/PATCH when caller passes key (separate from checkout's body `idempotency_key`) | :126-135, :146, :131-133 | wrapper around generated client 🔴 (auth) |
| 401 → refresh | single transparent refresh + ONE retry; no general retry/backoff | :152-158 | port 1:1 🔴 |
| Refresh single-flight | in-tab `inflightRefresh` promise + **cross-tab `navigator.locks.request('dos-token-refresh')`**; loser re-reads winner's rotated token inside the lock; 409 concurrent-rotation → use stored token; preserves `active_location_id` from expired JWT | :12, :52-66, :25-50, :39-42 | port 1:1 🔴 — this is hard-won multi-tab correctness |
| 401 bounce | admin paths only (`pathname.startsWith('/admin')`), `authRedirectInFlight` guard, sets `dos_auth_expired`, → `/login`; customer/courier deliberately never bounce | :71, :174-194 | port; per-route-group behavior |
| Error envelope | `ApiError{status,message,data}`, `.code` (SCREAMING_SNAKE), `.correlationId` (on-screen support code); A1 message → legacy `error` → statusText; timeout AbortController 10 s → ApiError 408; 204 → undefined; optional Zod parse | :73-97, :117-122, :166, :211-228 | codes come from OpenAPI enums; `mapApiError` matrix (ADR-0010 A2) is **documented-future, not implemented** — error UX today: **money = inline `role=alert` (never toast)** (CheckoutPage:698-703), **owner mutations = toast** (guard test `s4-toast-on-error.test.ts`) |
| Public bypass | `publicApi.ts:41-45` root-fetch `/public/...` (no auth/refresh/bounce) | — | Astro server-side fetches at SSR time |

### 6.2 WS client (`apps/web/src/lib/useWebSocket.ts`) → Svelte store/attachment

- URL: `VITE_WS_BASE_URL || wss://{host}/ws` (:4-6). Status union `connecting|connected|disconnected|reconnecting|error|disabled` (:8).
- Reconnect forever: backoff `min(2000·1.5^n, 15000)` + ≤1 s jitter (:23-24, :94-102); clean close 1000/1005 → no reconnect (:89-91); prompt resume on online/focus/visibilitychange resets attempts (:128-141). No client heartbeat — server pings 30 s + zombie terminate (`apps/api/src/websocket.ts:287-297`).
- Auth: token sent as `?token=` (DEPRECATED, server logs usage-to-zero, never logs token value — websocket.ts:179-189, :347-351) AND post-open `{type:'auth',token}` (:49-61). Rebuild: **drop the URL-token path** (deprecation completes at port) 🔴 council.
- Subscribe: `{type:'subscribe',room}` after `auth_success`; unsubscribe + close 1000 on unmount (:70-77, :155-160). Server room authz tri-state (ADR-0013) unchanged.
- **"_truncated" is not a frame field** — it is the PII claim-check convention: bus deltas carry no PII; `DashboardPage.tsx:124-159` paints instant non-PII card then debounced 800 ms authed `/owner/orders` refetch. `onReconnect` → full refetch except first connect (:155-158). Consumers: OrderStatusPage:226, DashboardPage:135+:162 (two rooms), TasksPage:85, DeliveryPage:152.

### 6.3 VITE_* flag census (C8 = 19) — all read at build time, no central flags.ts

| Flag | Default | Gates | Read at |
|---|---|---|---|
| `VITE_UNDO_REDO_ENABLED` | **ON** (`!== 'false'`) | menu-manager undo/redo | MenuManagerPage.tsx:36 |
| `VITE_KEYBOARD_SHORTCUTS_ENABLED` | **ON** | ⌘K palette + g-nav + ? help | AdminCommandCenter.tsx:12 |
| `VITE_PULL_TO_REFRESH_ENABLED` | **ON** | courier tasks pull-to-refresh | TasksPage.tsx:14 |
| `VITE_TILE_STYLE_URL` | `openfreemap.org/styles/liberty` | maplibre tile style | tileConfig.ts:12 + MapLibreBase.tsx:17-19 |
| `VITE_TILE_PROVIDER` | `'free'` | **DEAD SEAM** — `tileConfig.ts` never imported | tileConfig.ts:27-28 |
| `VITE_TG_CATEGORY_GATING` | OFF | notif category preference centre | SettingsPage.tsx:11 |
| `VITE_PAYMENTS_CRYPTO_ENABLED` 🔴 | OFF (ADR-0017) | crypto payment method | CheckoutPage.tsx:58 |
| `VITE_PAPER_SKIN` | OFF (`=== 'on'`; runtime opt-in `localStorage dos_paper_skin`) | internal admin/courier paper skin | packages/ui/src/theme/paperSkin.ts:10-13 |
| `VITE_MENU_CHARACTERISTICS_ENABLED` | OFF | L2 descriptive band (dormant) | MenuPage.tsx:35 |
| `VITE_MENU_CHARACTERISTICS_FILTER` | OFF | macro sort lenses | MenuPage.tsx:37 |
| `VITE_MENU_CHARACTERISTICS_COMPARISON` | OFF | compare toggle/bar/panel | MenuPage.tsx:36 |
| `VITE_MENU_ALLERGEN_FILTER` | OFF (+ hard const `ALLERGENS_ENABLED=false` freeze) | allergen filter chips | MenuPage.tsx:27, :42 |
| `VITE_MEDIA_RICH_ENABLED` | OFF | rich-media manager (server flag is the real gate) | MenuManagerPage.tsx:30, MediaManager.tsx:24 |
| `VITE_GOOGLE_OAUTH_ENABLED` 🔴 | OFF | Google OAuth login button | LoginPage.tsx:11 |
| `VITE_ACCESS_GATE_PUBLIC_ENABLED` | OFF | public access-request gate | AccessRequestForm.tsx:13 |
| `VITE_OWNER_TWO_TAP` | OFF | two-tap owner confirm on OrderCard | packages/ui OrderCard.tsx:14, :234 |
| `VITE_VOICE_ENABLED` | **declared, never read** (vite-env.d.ts:9); live gate is server `VOICE_CONTROL_ENABLED && !VOICE_KILL` (api voice-flag.ts:11) | storefront voice (dark) | — |
| `VITE_API_BASE_URL` | `/api` | API base | apiClient.ts:4 (also MenuPage.tsx:729 default `''`) |
| `VITE_WS_BASE_URL` | derived `wss://host/ws` | WS base | useWebSocket.ts:6 |

→ Rebuild: flags port to Astro `import.meta.env` (same names); **carry the staging-deploy lesson** — flags default OFF bake at build time, deploys must pass build-args (memory: staging-deploy-flags). Kill `VITE_TILE_PROVIDER`/tileConfig dead seam; converge Analytics map onto the flag-aware wrapper.

### 6.4 GDPR / export / import UI

- **Export:** owner-side client-generated CSV/JSON only (`lib/exportCSV.ts`; consumers Analytics :244/:308, Couriers :245, Dashboard :722). No customer self-service export/erase UI exists (GDPR erase is operator-side SQL per audit lane; FE surface = PrivacyPage `mailto:` contact only). → port as-is; customer-facing GDPR self-service = explicitly out of rebuild parity scope (flag as product gap, not a port gap).
- **PDF/photo menu import:** front door `/start` dropzone (PDF/image ≤10 MB) → `POST /owner/menu/import/anonymous` (120 s timeout) → phase machine choose→parsing→review/blank; error split UNSUPPORTED_TYPE vs parse-failed; authed 3-step wizard in MenuManager (preview→commit, merge/add_only/replace). Server 413 `FILE_TOO_LARGE`. Proof: `flow-onboarding-parsing.spec.ts`.
- **Analytics (product):** Plausible script in index.html:34 (cookieless) — ports to Astro head; CSP already allows plausible.io. Additionally `apps/web/src/lib/analytics.ts` defines the **7-event funnel taxonomy contract** (`menu_view → item_add → cart_open → checkout_start → order_placed → courier_assigned → delivered`, :18-26) with transport deliberately unwired (PostHog seam, no key) — the taxonomy ports verbatim as a typed module; wiring stays a separate act.
- **Misc lib:** `lib/hooks.ts` (`useOnlineStatus` + geolocation re-export), `lib/cartReconcile.ts` (🔴 menu-version reconcile, unit-tested), `lib/dishNutrition.ts` (+test), `lib/tileConfig.ts` (dead seam, §6.3), `api/devBootstrap.ts` + `api/mockData.ts` (DEV-only, tree-shaken — DROP from prod build).

---

## 5. i18n census + DECISION

### 5.1 Census

- **SSOT:** `packages/ui/src/lib/i18n-catalog.ts` — key-major `Record<key, {sq,en,uk}>` (`i18n-catalog.ts:1-9`); **1,445 keys** (C6) × 3 locales (`sq` default, `en`, `uk` — display SQ/EN/UA, `packages/ui/src/lib/I18nProvider.tsx:14,55`). Catalog weight: **219 kB raw / 61.4 kB gz** (C7).
- **Parity gate:** `scripts/i18n-parity.ts` — every key must carry en+sq+uk, no TODO drafts (enforced per catalog header comment); helper `scripts/i18n-add.ts`.
- **Runtime:** `t(key, fallback)` everywhere; no per-locale code-splitting today — all three locales ship in the main bundle.
- **Dynamic-key families (C10 = 13):** `allergen.$` (9 call sites), `dos_menu_prefs_$` (5), `admin.days.$` (4), `dos_last_delivery_$` (2), `dos_checkout_draft_$` (2), `admin.taste_$` (2), `voice.err.$`, `order.$`, `message.location.$`, `message.action.$`, `courier.$`, `client.day_$`, `admin.courier_status_$` (1 each). Extraction: C10 command.

### 5.2 DECISION: **adopt Paraglide-JS 2 (inlang)** — not a direct catalog port

**Why (grounded):**
1. **Budget arithmetic.** The catalog alone is 61.4 kB gz (C7) — a direct port that imports the whole catalog into any storefront island consumes the *entire* ≤60–90 kB gz storefront JS budget before a single component ships. A hand-rolled per-surface catalog split would re-implement, badly, what a compiler does.
2. **Paraglide compiles each message to a tree-shakable function** — each Svelte island bundles only the keys it references (vendor claim: up to ~70 % smaller i18n bundles; bundle size stays flat as the catalog grows — [paraglidejs.com](https://paraglidejs.com/), [github.com/opral/paraglide-js](https://github.com/opral/paraglide-js)). With ~3 locales the all-locales-per-message tradeoff is explicitly favorable (< ~20 locales per [issue #22](https://github.com/opral/paraglide-js/issues/22)); this **supersedes** the old "per-locale splitting" requirement with a stronger guarantee: per-*message* splitting. If strict per-locale emission is later required, Paraglide 2's per-locale output structure / server locale-splitting options cover it — VERIFY-ON-ADOPT during the Phase A spike.
3. **First-class Astro + Svelte support** via `paraglideVitePlugin` (official docs; works in Astro islands and SvelteKit alike — keeps the SvelteKit-extraction contingency of 06 §2 open).
4. **Type-safety:** message functions are typed TS — missing keys become compile errors, replacing a whole class of runtime-fallback bugs.

**Migration mechanics (mechanical, reversible):**
- One-shot script converts `i18n-catalog.ts` → `messages/{sq,en,uk}.json` (inlang format). The catalog stays SSOT until cutover; the converter runs in CI so both worlds stay in sync during the strangler phase.
- **Parity gate ports:** re-target `scripts/i18n-parity.ts` at `messages/*.json` (same rule: every key in all 3 locales, no drafts). CI check preserved per 06 §1.
- **Dynamic keys:** Paraglide cannot construct keys at runtime (compiler needs static analysis — [message-keys docs](https://paraglidejs.com/message-keys), [issue #440](https://github.com/opral/paraglide-js/issues/440)). Our 13 families (~31 call sites) each become an explicit `const MAP = { code: m.key_fn }` lookup — bounded, type-checked work; families are enumerable (allergens = EU-14 list, days = 7, taste axes = 5, messenger kinds = 6, courier statuses, order statuses).
- **Locale routing:** locale persisted as today (storage + `<html lang>`); storefront SSR pages render in the tenant's default locale server-side, switcher hydrates in the shell island.

**Fallback (recorded):** if the Paraglide/Astro adapter fights the islands setup in the Phase A spike, the contingency is a direct catalog port **split per surface** (storefront keys ≈ small subset; admin/courier keys stay behind auth where budget is looser). The generated `messages/*.json` convert back losslessly. Trigger: storefront island i18n overhead measured > ~8 kB gz for MenuBrowser in the spike.

---

## 7. Island architecture summary

**Principle:** islands = smallest interactive units; the Astro shell owns everything SEO/static (menu HTML, hero, hours, footer, JSON-LD, 404, privacy). Storefront JS budget target **≤60–90 kB gz** total hydrated JS on `/s/[slug]` (excl. lazily-loaded maplibre on the order page).

### 7.1 Island roster — 27 islands

| # | Island | Route(s) | Hydration | Rationale |
|---|---|---|---|---|
| 1 | **MenuBrowser** | `/s/[slug]` | `client:load` | chips/scroll-spy/search/detail-sheet/add-to-cart over SSR HTML; largest storefront island |
| 2 | **CartCheckout** 🔴 | `/s/[slug]` | `client:idle` (eager-upgraded on first add) | cart bar + cart sheet + checkout sheet + OTP modal; shares cart runes store with #1 |
| 3 | **OrderTracker** 🔴 | `/s/[slug]/order/[id]` | `client:load` | WS live status; maplibre dynamic-imported only when delivery has coords |
| 4 | StorefrontShellControls | `/s/[slug]` layout | `client:idle` | language/currency switchers (tiny) |
| 5 | InstallPrompt | all | `client:idle` | PWA nudge |
| 6 | AccessRequestGate | `/start` | `client:visible` | flag OFF |
| 7 | OnboardingWizard | `/start`, `/admin/onboarding` | `client:load` | upload→parse→claim phases 🔴 (TG claim) |
| 8 | AuthLogin 🔴 | `/login` | `client:load` | 3 auth methods |
| 9 | AuthCallback 🔴 | `/auth/callback` | `client:load` | code exchange |
| 10 | ClaimFlow 🔴 | `/claim` | `client:load` | fragment-token handling |
| 11 | AdminShell | `/admin/*` layout | `client:load` | sidebar/tabbar/more-sheet + CommandCenter (⌘K, g-nav) |
| 12 | OrdersBoard 🔴 | `/admin`, `/admin/orders` | `client:load` | WS ×2 rooms, status mutations |
| 13 | MenuManager 🔴 | `/admin/menu` | `client:load` | biggest admin island (CRUD+import+undo/redo+BOM+allergen) |
| 14 | Settings 🔴 | `/admin/settings` | `client:load` | fees/hours/TG |
| 15 | Couriers 🔴 | `/admin/couriers` | `client:load` | invites+earnings |
| 16 | Branding | `/admin/branding` | `client:load` | palette/fonts/logo + iframe preview |
| 17 | Analytics | `/admin/analytics` | `client:visible` | charts + geo map (maplibre lazy) |
| 18 | Promotions 🔴 | `/admin/promotions` | `client:load` | discounts |
| 19 | SupplyLibrary | `/admin/supplies` | `client:load` | localStorage-only |
| 20 | CRM 🔴 | `/admin/crm` | `client:load` | PII-masked list |
| 21 | Activation 🔴 | `/admin/activation` | `client:load` | publish gate + iframe postMessage |
| 22 | CourierTasks | `/courier` | `client:load` | WS offers + timer + pull-to-refresh |
| 23 | Delivery 🔴 | `/courier/delivery/[id]` | `client:load` | maplibre + GPS + cash-as-proof |
| 24 | Shift | `/courier/shift` | `client:load` | timer + messenger prefs |
| 25 | Earnings 🔴 / History | `/courier/earnings`, `/courier/history` | `client:visible` | read-only money display |
| 26 | CourierLogin 🔴 / InviteRedeem 🔴 | `/courier/login`, `/courier-invite/[id]` | `client:load` | auth |
| 27 | VoiceControl (dark) | `/s/[slug]` | `client:idle`, flag OFF | lib ports, MicFab still unbuilt |

**Biggest 5 by ported scope:** MenuManager (1,405 LOC + MediaManager 340 + RecipeEditor 266 + AllergenEditor 158), MenuBrowser (bulk of MenuPage 1,811), CartCheckout (CheckoutPage 787 + 5 sections + ClientLayout sheets), OrdersBoard (DashboardPage 839), Delivery (560 + maplibre wrapper).

**Astro-static (zero JS):** privacy, 404, redirects, menu/hero/hours/footer HTML on `/s/[slug]`, JSON-LD/OG/hreflang head component, StylizedMap SVG.

**Budget notes:** storefront critical path = islands 1+2+4+5 + runtime; Svelte 5 compiled output + Paraglide per-key messages are the levers; maplibre (~65 kB gz) never loads on `/s/[slug]` itself (only order page with active delivery, dynamic import — same as today's `MapLibreBase` dynamic import). Framer-motion equivalents: Svelte transitions (built-in, ~0 kB extra); framer-motion is NOT ported.

## 8. 🔴 Red-line register (council-before-port + Playwright red→green mandatory)

**23 🔴 rows** across this doc: CartCheckout (order POST, preflight/OTP, crypto method, cash/tip, VAT/fee display, cart reconcile), OrderTracker (totals/tip, track-token exchange), OrdersBoard status PATCH, MenuManager price/product writes + kitchen-busy, Settings (fee/minOrder/pause/fallback), Promotions CRUD, Couriers invites + earnings display, Activation publish/pickup/product-patch, CRM PII, admin Login/AuthCallback, courier Login/InviteRedeem, courier Delivery (delivered POST w/ cash_amount, GPS ping), courier Earnings/Shift stats, ClaimFlow, OnboardingWizard TG claim, apiClient auth/refresh/locks, WS auth (URL-token retirement), entry-photo upload (PII), shadow-tenant noindex (privacy).
Known money-lane facts to carry (do NOT silently fix during port): Delivery `cashCollected` input not transmitted (`DeliveryPage.tsx:239-244`); courierId `atob` JWT parse w/ `'c1'` fallback (`TasksPage.tsx:30-40`); OSRM third-party ETA call from client (`MenuPage.tsx:485` — also a CSP/privacy consideration for rebuild).

## 9. Gaps — what could NOT be fully enumerated

1. **Per-key i18n usage map** (which of the 1,445 keys each island needs) — deferred to the Paraglide compiler itself; not hand-enumerable at acceptable cost.
2. **packages/ui internal-only helpers** below component granularity (utils/constants bodies) — censused at file level in §3, not per-export.
3. **Visual-net baselines** — all 3 path specs exist (`e2e/visual/*`) but every baseline is React-DOM-derived: blanket status **NEEDS-REBASE**; intents (states × 3 breakpoints × 2 langs) re-recorded per ported surface.
4. **Dev-only surfaces** (FlowTestPage, devBootstrap mock) — inventoried as DROP-from-prod; their internal UI not censused.
5. **Worktree copies** (`.claude/worktrees/*`) contain divergent FE files (e.g. voice FE mount) — this census covers the main tree only; the voice-fe-mount plan (`docs/design/voice-fe-mount/`) must be reconciled before its rows are ported.
