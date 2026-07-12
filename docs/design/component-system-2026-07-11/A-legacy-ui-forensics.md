# A — Legacy dowiz browser UI: forensic inventory (2026-07-11)

Grounded in: real session recordings (Playwright traces of client personas, unzipped to
`/root/.claude/jobs/c6a4c73f/tmp/trace-r3/`), real screenshots (`e2e/artifacts/ui-audit/*.png`
of 2026-06-17 and `/root/dowiz/demo-*.png` of a later deployed build), and the FE code in this
working tree. Vintages differ — where a screenshot shows something the tree no longer has (or
vice-versa) it is flagged explicitly. No code was changed.

Evidence base:
- Code: `apps/web/src/pages/client/{MenuPage,CheckoutPage,OrderStatusPage}.tsx`,
  `apps/web/src/routes/ClientLayout.tsx`, `apps/web/src/lib/{CartProvider,cartReconcile,messenger}.ts(x)`,
  `apps/web/src/hooks/useMenuData.ts`, `apps/web/src/pages/admin/{MenuManagerPage,RecipeEditor}.tsx`,
  `packages/ui/src/components/**`, `apps/api/src/routes/public/menu.ts`,
  `packages/db/migrations/1790000000064_read-public-menu-perf.ts`.
- Screenshots (all inspected visually): `e2e/artifacts/ui-audit/01-menu-page-initial.png`,
  `FINAL-03-menu-products.png`, `FINAL-05-cart-drawer-open.png`, `FINAL-07-checkout-full.png`,
  `p2-06-checkout-initial.png`, `FINAL-11-order-status-full.png`; `/root/dowiz/demo-desktop.png`,
  `demo-product-detail.png`, `demo-cart.png`, `demo-checkout.png`, `demo-mobile.png`,
  `artepasta-desktop.png`, `currency-btn.png`.
- Traces: `e2e/findings/round-3/client-first-timer-impatient/trace.zip` (staging `/s/demo`, mobile),
  `round-1/client-price-skeptic/trace.zip`; finding JSONs + `e2e/findings/FINDINGS.md` triage.

---

## 1. Component inventory (ordering funnel)

### 1.1 Storefront shell & chrome

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| `ClientLayout` (shell) | `apps/web/src/routes/ClientLayout.tsx:22-237` | Tenant-themed app shell for all `/s/:slug` routes: fetches `/public/theme/:slug`, derives a full palette (`derivePalette`), wraps `CartProvider` + `ToastProvider`, owns the sticky header, sticky cart bar, and the cart dialog | `theme`, `isCartOpen`, `isBouncing` (listens to `dos:bounceCart` CustomEvent, :34-40); embed mode via `?embed=true` (:32); branding-preview via `postMessage` + `draft_*` URL params (:63-89) | Header brand text was double-escaped in older SSR builds — `01-menu-page-initial.png` shows literal “Dubin &amp; Sushi”; fix documented at `apps/api/src/lib/ssr-renderer.ts:53-56` |
| Header bar | `ClientLayout.tsx:115-127` | h-14 sticky bar: logo (hidden on load error :119), venue name, `SunlightToggle`, `CurrencySwitcher`, `LanguageSwitcher` | `LanguageSwitcher variant="full" allowed={supportedLocales}` | Two visible locale rows in the Jun-17 vintage (`01-menu-page-initial.png` shows header SQ/EN/UK **and** a second L/SQ/EN/UA row); the currency button crams icon+symbol+code (`currency-btn.png`) |
| Hero section | `MenuPage.tsx:551-599` | Gradient hero (160/200px) with venue name h1, Google rating stars + review count, OSRM-derived delivery ETA (`~N min`), `StateChip` for open/closed/busy | `locationInfo` from `/public/locations/:slug/info` (:303-315); geolocation + `router.project-osrm.org` fetch (:317-334) | Bottom scrim added because tenant palettes made the title illegible (:553-557 comment); trace frames confirm the low-contrast risk on the pink theme (`trace-r3/resources/page@…578915.jpeg`). Older vintage rendered a map hero + “mbyllet 23:59” closing-time chip instead (`demo-mobile.png`) |
| `StateChip` | `packages/ui/src/components/client/StateChip.tsx:67-91` | One pill for venue (`open/closed/busy`) and item (`available/sold_out`) states, brand-token native | `state`, `scope`, `detail` | `sold_out` scope is effectively unused on the storefront: the public menu only returns available items (`ProductCard.tsx:165-166` comment) |
| Venue banners | `MenuPage.tsx:701-724` | Closed banner (ordering discouraged) and distinct busy banner (“kitchen busy — orders may take longer”, ordering stays open) | `venueStatus` from `/info` `status` field with legacy `isOpen` fallback (:310) | Closed state does **not** block add-to-cart or checkout — informational only |
| Storefront footer | `MenuPage.tsx:1243-1271` | Venue name, address, Google-Maps/Instagram/Facebook round buttons | hidden in embed/activation contexts (:297) | — |

### 1.2 Category nav, search, sort, filter

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| Category tab nav | `MenuPage.tsx:602-642` | Horizontally scrolling tab row with per-category available counts `(N)`; scroll-spy via IntersectionObserver (:337-355); smooth scroll-to-section (:357-367); Chef’s Picks pseudo-tab `✦` first | `activeTab`; sticky offset computed from measured sticky height (`stickyRef`, :272-283) | Tabs are scroll anchors, not filters; the whole row disappears in sorted mode (see below). Right-edge fade hints overflow (:641) |
| `SearchInput` | `packages/ui/src/components/atoms/SearchInput.tsx`, used `MenuPage.tsx:650-657` | Compact search pill; filters name+description client-side (:188-191) | width animates 100→140px when non-empty (:649) | 100px collapsed width truncates the placeholder to “Kë” on Albanian (`demo-mobile.png`) |
| Sort control | `MenuPage.tsx:660-673` | 4 pill toggles: `default` (list icon), `price-asc` (“↑ $”), `price-desc` (“↓ $”), `name` (“A–Z”) | persisted per-slug in `localStorage` `dos_menu_prefs_<slug>` (:141-165) | Any non-default sort collapses ALL categories into one flat “All items” section and kills the category nav — deliberate monotonicity decision (:195-206). “$” glyphs on an ALL-denominated menu. Older deployed vintage had a single “Çmimi” toggle instead (`demo-desktop.png`) |
| Allergen filter | `MenuPage.tsx:675-691` | One pill per allergen present in the menu (union over BOM lines, :218-227); tap = show ONLY items **containing** that allergen; others dim to 0.4 | `filterAllergen` (single-select), persisted | **Inverted vs. user expectation**: it selects items *with* the allergen (a “what contains crustacean?” lens), not an exclusion filter — visible as `CRUSTACEAN`/`SOJE` pills in `01-menu-page-initial.png` and the trace frames |
| Comparison mode | — | **Does not exist.** `grep -ri compar apps/web/src packages/ui/src` yields only `localeCompare` (`MenuPage.tsx:204`) | — | The nearest analogs: global price sort (flat list), per-card kcal/macro badges, and the checkout `NutritionRing` aggregate. A real compare surface is greenfield for the new system |

### 1.3 Menu list & product card

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| Category section + grid | `MenuPage.tsx:789-867` | 2/3/4-col responsive grid per category, staggered entrance, `whileInView` | `displayCategories` memo re-groups after search/filter (:180-216) | Chef’s Picks section duplicates products already in their home category (both visible at once, `demo-desktop.png`); suppressed under non-default sort (:790-793) |
| `ProductCard` | `packages/ui/src/components/client/ProductCard.tsx:46-248` | The menu tile: 4:3 image (or crafted no-photo fallback :84-122), allergen corner badges (top-3 + “+N”, :123-139), kcal badge or Chef’s-Pick badge (:144-164), name (2-line clamp), quick-add `+` button, description, ingredient chips (top-4 + “+N”, :197-206), price + prep time + kcal/P/F macro line (:213-229), taste icon rows (:232-244) | `product{…}`, `onAdd`, `onClick`; hover lift gated on `(hover: hover)` (:42-44); `data-testid="menu-item"` / `"menu-item-add"` | Quick-add bypasses the detail modal only when the product has no modifier groups (`MenuPage.tsx:851-858`); unavailable = 0.55 opacity whole-card (:60-62); no sold-out chip by design (:165-166). Extremely dense: up to 7 data species on one tile (see `FINAL-03-menu-products.png`) |
| Loading skeleton | `MenuPage.tsx:728-744` | 6 card-shaped skeletons + 3 tab-pill skeletons (:606-611); 300ms minimum dwell (`MIN_SKELETON_DWELL`, :119, :260-263) | — | — |
| Empty/error states | `MenuPage.tsx:745-787` | Distinct: venue-not-found (404, home CTA), fetch-error (retry CTA), menu-unavailable, and no-results-for-filters (clear-filters CTA) | `notFound` vs `fetchError` deliberately separate (:125-127) | Older vintage of not-found visible in `artepasta-desktop.png` (“Restoranti nuk u gjet”) — it still rendered the map hero + a dead “Të gjitha (0)” tab above the empty state |

### 1.4 Product detail modal

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| Detail modal (bottom-sheet / centered) | `MenuPage.tsx:872-1237` | 16:9 hero image with shared-element `layoutId` morph (:907), lazy rich-media gallery/renderer/reveal (ADR-0002, :892-954), close button, kcal+macros chip on the photo (:965-975), name/price/prep/description, taste, nutrition, allergens, ingredients, modifiers, qty, add CTA | `detailProduct`, `detailMedia` (lazy `/products/:id/media`, 4s abort, :406-420); scroll-lock on `.app-shell-main` + body (:425-436) | `bomToNutrition(detailProduct)` is recomputed inline ~8× per render (:965-1108); real render in `demo-product-detail.png` |
| Taste section | `MenuPage.tsx:1026-1052` | Icon-repeated intensity per axis (spicy/sweet/salty/sour/richness, level 1-3) | axes without a mapped icon are dropped (:1030-1033) | Icon-only legend — meaning is guessable at best |
| Nutrition section | `MenuPage.tsx:1054-1075` | 4-up grid: kcal/protein/fat/carbs summed from BOM lines | only renders when kcal > 0 | Client-side derivation from `attributes.bom` — see §3 |
| Allergen section | `MenuPage.tsx:1077-1093` | Red-tinted card, uppercase chips via `getAllergenStyle` | i18n per allergen key | — |
| Ingredients section | `MenuPage.tsx:1095-1108` | Chips from BOM `supplyName`, excluding `packaging`/`utensil` kinds (:111) | — | — |
| Modifier groups | `MenuPage.tsx:1110-1178` | Groups with Required badge + “up to N”; options as toggle chips with `+<PriceDisplay>` deltas; `display_type` (radio/checkbox/select/quantity) resolved with legacy fallback `max_select===1 → radio` (:41-44) | selection in `modifierGroupSelection: Record<groupId, modifierId[]>`; min/max enforced in `toggleModifier` (:438-454); required groups pre-select first available option (:386-392) | `display_type` **only changes the header icon** (:1127) — `select` and `quantity` still render as chips; a “quantity” modifier cannot actually take a quantity |
| Selected summary | `MenuPage.tsx:1181-1190` | “Selected: X · Y” text line | — | — |
| Quantity stepper | `MenuPage.tsx:1194-1215` | −/qty/+ with 44px targets, min 1, no max | `quantity` | No stock-awareness (stock_count exists in admin but not enforced here) |
| Add to Cart CTA | `MenuPage.tsx:1216-1232` | Full-width bar: label left, live total right (`(price+Δ)×qty`); disabled until required groups satisfied (`canAdd`, :519-528) | `data-testid="product-detail-confirm"` | Unavailable product renders “Unavailable” CTA inside a modal the user could still open |

### 1.5 Cart

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| `CartProvider` | `apps/web/src/lib/CartProvider.tsx:61-116` | Per-venue cart in `localStorage` `dos_cart_<slug>` (versioned schema v1), cross-tab sync via `storage` events (:70-80), merge-on-add by `productId+options` (:89-99) | `addItem/updateQuantity/clearCart/bounceCart/reconcileToMenu` | Item identity: quick-add id `cart_<productId>` (`MenuPage.tsx:852`) vs modal hashed id (`MenuPage.tsx:470-475`) — merged correctly by the productId+options key, but two id schemes persist |
| Cart reconcile (F9) | `apps/web/src/lib/cartReconcile.ts:38-63`, applied `MenuPage.tsx:483-495` | On menu load: re-price modifier-free lines, drop items gone from the menu, stamp `menu_version`; toast summary | pure function; skips lines with modifiers (server backstops) | Good pattern to keep — prevents the checkout hard-block ambush |
| Sticky cart bar | `ClientLayout.tsx:131-158` | Fixed bottom pill: cart icon + count badge (99+ cap), “Cart · <AnimatedNumber total>”; bounce animation on add; hidden on `/checkout` | `StickyActionBar embedSticky` (visualViewport keyboard offset, `packages/ui/src/components/molecules/StickyActionBar.tsx:15-32`) | Bar covers the last grid row on some vintages (`demo-mobile.png`) |
| Cart dialog | `ClientLayout.tsx:159-223` (uses `ResponsiveDialog`) | Bottom-sheet “Shporta”: line rows (name + **unit** price), ±44px steppers (qty→0 removes), Total, Checkout CTA, Clear-cart (only when >1 line) | — | Shows unit price, not line total (`:180`) — a 3× line reads as its unit price while Total sums lines (`FINAL-05-cart-drawer-open.png`). No modifier summary on rows — two same-product lines with different options look identical. A **second, unused** `CartDrawer` + duplicate `CartItem` type lives at `packages/ui/src/components/client/CartDrawer.tsx:7-14` (and a third cart model at `packages/ui/src/hooks/use-cart.ts:4-10`) |

### 1.6 Checkout

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| `CheckoutPage` | `apps/web/src/pages/client/CheckoutPage.tsx:184-1151` | Single-page form (no steps): Contact → Address/Type → Payment → facts/nutrition → Summary → sticky Place-Order | Draft autosaved per-slug (`dos_checkout_draft_<slug>`, :343-351); last delivery pin/address reused (`dos_last_delivery_<slug>`, :307-324) | One long page, not a wizard — `FINAL-07-checkout-full.png`. Empty-cart state at :619-645 (“Browse menu” CTA; older vintage in `p2-06-checkout-initial.png`) |
| Contact fields | :667-699 | Name (required), Phone (required, Albanian-format coercion `normalizeAlbanianPhone` :15-24, E.164 validate :395-401) | `autoComplete`, `inputMode=tel` | Phone normalization is Albania-hardcoded |
| Messenger selector (tree version) | :684-698 | Optional `Select` of **3 kinds** — Telegram/WhatsApp/Viber — + handle input (`packages/db/migrations/1790000000038` CHECK enum; Zod `packages/shared-types/src/legacy.ts:48`) | handle required iff kind set; placeholder switches @username vs phone | **Diverges from the deployed G03 build**: `demo-checkout.png` shows a *required* “Komunikimi” channel select (“Zgjidh një kanal…”) + “Unë jam marrësi” receiver checkbox, per ADR-0016 (`docs/adr/ADR-checkout-communication.md`) whose ratified v1 kinds are **Phone, WhatsApp, Viber, Telegram, Signal, SimpleX (6 kinds)**. A 6-kind FE against the 3-kind DB CHECK/Zod enum is the “messenger 422/400” failure class; the widening migration exists only as a design artifact (`docs/design/checkout-communication/migration-1790000000074_checkout-communication.ts`), not in `packages/db/migrations/` (which ends at `…065`) |
| Entrance photo (UX-3) | :700-719 | Optional photo → `/public/entry-photo` (R2) before the order exists; preview thumb | `photoUploading`; silent failure by design (:214) | — |
| Delivery-type tabs | :729-733 | `delivery` / `pickup` segmented control; **scheduled hidden** in tree (:732 comment) | `role=tablist` | Jun-17 build still showed the dead third tab “Planifikuar” (`FINAL-07-checkout-full.png`); `deliveryType === 'scheduled'` branch remains at :837-844 |
| Map pin | `packages/ui/src/components/molecules/MapWithPin.tsx`, used :738-740 | MapLibre/OpenFreeMap drag-pin; falls back to venue center then Durrës `[19.456, 41.324]` (:225) | `onPinChange` | Hardcoded city default when everything fails (:434-435) |
| Address block | :741-813 | Street address, Entrance (required), Apartment (required), “How to find you” notes (required, :417-420), dropoff-instruction chips (Leave at door / Call on arrival / Ring bell / Hand to me / Text on arrival) + free-text | all folded into `delivery_instructions` string ≤500 chars (:473-476) — server delivery schema is `.strict()` | Heavy required-field load for a first order; entrance+apartment+notes all mandatory |
| Payment card | :852-917 | Cash-only method card; “Cash amount” input (min=total, red border under-total, change row :904-915); tip input (UX-4, 0–1,000,000) | `cash_pay_with`, `tip_amount` in POST body | `parseInt` on both inputs silently drops decimals (:876, :896); no card/crypto path in this page (crypto checkout shipped dark elsewhere) |
| Order summary | :945-1028 | Subtotal, delivery fee (or “Calculated at checkout” for distance-tiered venues — ADR-0005 mirror via `estimateOrderTotal`, :353-375), tax, tip line, ≈kcal line, Total, “Cash to courier (incl. tip)”, pre-order ETA | `feeKnown` gate — never invents a fee | Pre-order ETA is a **hardcoded 25–45 min** (:1020) |
| `NutritionRing` | :86-182, rendered :936-943 | SVG macro donut (protein/carbs/fat by kcal share) + kcal center | `--chart-*` tokens | **Dead on current add paths**: it needs `item.kcal` on cart items (:377-383) but neither `MenuPage.tsx:505-512` nor `:852` ever sets kcal — the ring can only appear for legacy carts |
| City fact card | :283-293, :920-934 | “Did you know?” Wikipedia REST summary of the venue’s city | — | Third-party fetch on the money path; pure novelty |
| Error surfaces | :1031-1078 | Location-load-failed banner (BUG-1 fix, disables submit + retry, :222-224); order-error banner with scroll-into-view (:237-239), designed 422 messages (MIN_ORDER, NOT_DELIVERABLE, CASH_AMOUNT_TOO_LOW, hard_block “review your cart”, :523-553), “Call the restaurant” fallback CTA for non-422 (:554-559, phone pre-cached on mount :300-306) | `PreflightResponse` 200-body outcomes (:33-37) | Good error taxonomy worth carrying forward |
| OTP flow | :574-617, `packages/ui/src/components/client/OTPModal.tsx` | Soft-confirm `requiresOtp` → send code → modal verify → resubmit with `x-otp-verified` header; intent-hash binds OTP to cart (:565-571) | — | — |
| Sticky Place Order | :1094-1116 | `StickyActionBar` with spinner state, “Place order • <total>”; disabled while placing or `!locationId` | form=checkout-form submit | — |
| Success overlay | :1127-1147 | Animated check + “Order placed!”, 1.5s then navigate to `/s/:slug/order/:id` | — | — |

### 1.7 Order status / tracking

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| `OrderStatusPage` | `apps/web/src/pages/client/OrderStatusPage.tsx:65-799` | Live tracking: WS room `order:<id>` + 30s watchdog refetch (:218-225); tracking-link `?t=` grant→JWT exchange (:172-190) | `STATUS_LABELS/VARIANTS/ACCENT/MESSAGE` maps (:19-63) | Real render: `FINAL-11-order-status-full.png` |
| WS banner | :490-509 | “Live updates paused” + call-restaurant CTA | `WSStatusDot` on the map (:523-525) | — |
| `CourierLiveMap` | `packages/ui`, used :512-527 | Courier pin (only after a real fix, :372-385), route polyline (G1/G2), destination pin | terminal-frame lock prevents status regressions (:273-277) | Delivery only; default dest pin hardcodes Tirana `[19.817, 41.331]` (:400-402) |
| ETA headline | :549-591 | HONEST server range `{low}–{high} min` or reassuring overdue line — never a single number (:406-414); pickup branch has its own copy | `etaRange.phase` pre_assign/assigned refinement subline | Jun-17 build showed the mock single “15-25 min” (`FINAL-11…png`; mock still in `packages/shared-types/src/utils.ts:56-60`) |
| `OrderProgress` stepper | `packages/ui/src/components/client/OrderProgress.tsx:58-99`, used :594-607 | Received→Confirmed→Preparing→Ready→On the way→Delivered (pickup variant: Picked up); terminal Rejected/Cancelled appended; `*At` timestamps light steps | `status`, `type`, per-step `at` | — |
| Terminal exits | :611-633 | “Order again” + call-restaurant on REJECTED/CANCELLED; non-blaming copy | — | Good pattern |
| Share-location / contact courier | :635-691 | Geolocation watch → WS `client_location`; Call courier (`tel:`); Message courier via `messengerLink` deep link (t.me / wa.me / viber://, `apps/web/src/lib/messenger.ts:18-38`) | auto-stop on DELIVERED (:317-324) | — |
| Order details card | :694-716 | Line rows `qty× name` + **line totals** (unit×qty fix with comment, :700-702), Total, tip line | `nameSnapshot`/`priceSnapshot` fallbacks | Jun-17 build shows the pre-fix bug: “5x ” with **“NaN ALL”** and no item name (`FINAL-11-order-status-full.png`) |
| Rating block | :719-757 | 5-star tap-to-submit + optional comment; Google review invite for all delivered orders (:748-756) | `canRate` | — |
| Nutrition snapshot | :760-782 | 4-up estimate grid from order fields | — | Fields are `protein_mg_total`/`fat_mg_total`/`carb_mg_total` rendered with unit **“g”** (:770-777) — either misnamed columns or a 1000× display error; kcal uses `kcal_total` |
| `MessageThread` | `packages/ui`, used :786-793 | Preset-key customer↔restaurant messages over REST + WS merge (:291-296) | `preset_key` + params only (no free text from customer side) | 404 order → fabricates a fake PENDING order client-side (:197-204) |

### 1.8 Admin authoring (Menu Manager)

| Component | Where | What it does | Notable props/state | Legacy quirks |
|---|---|---|---|---|
| `MenuManagerPage` | `apps/web/src/pages/admin/MenuManagerPage.tsx:208-1244` | Category CRUD (add :405-431), expandable category lists, product CRUD in a form sheet, search/sort/availability filters (:262-267, MobilePicker on mobile :578-596), product preview | via `useMenuData` (owner endpoints) | 99.7th-percentile churn hotspot per repo intel — it accreted every menu feature |
| Product form | :919-1046 | Photo upload (≤5MB image, :370-377), Name*, Price (ALL)*, Prep time*, Description, `RecipeEditor`, stock (“Available today (pieces)”, empty=unlimited), 5-axis Taste Profile picker (Low/Med/High, :994-1020), “Available for order” toggle | `formTaste`, `formRecipeLines` | Price is a bare text input in ALL; no modifier-group editor in this form (groups exist only via API/import) |
| `RecipeEditor` (BOM) | `apps/web/src/pages/admin/RecipeEditor.tsx:31-266` | Picks supplies from `SupplyLibraryPage` store by kind (food_ingredient/condiment/packaging/utensil), qty+unit per line; live kcal/P/F/C sum + completeness flag + BOM-allergen union (:52-67) | `onBomAllergensChange` bubbles allergens up | This is where storefront nutrition/ingredients/allergens actually come from |
| `KitchenBusyToggle` | `MenuManagerPage.tsx:60-100` | 30-min “kitchen busy” window PATCH; storefront busy banner is its mirror | honest initial state from public `/info` | — |
| `MenuScheduleEditor` | `MenuManagerPage.tsx:102-206` | Daily category availability windows (e.g. breakfast 07:00–11:00) → `menu_schedules`; storefront filters via `read_public_menu` | collapsed by default | — |
| AI PDF import | :1049-1233 | Upload PDF/image → AI parse → preview counts (categories/products/issues) → commit with merge/add_only/replace modes; privacy notice about AI egress (:1130) | `importStep` upload→preview→done | — |
| `MediaManager` | `components/admin/MediaManager.js`, gated `MEDIA_RICH_ENABLED` (:25-28) | Rich product media authoring (ADR-0002) | dark by default | — |

### 1.9 Shared atoms the funnel leans on

`PriceDisplay` (`packages/ui/src/components/atoms/PriceDisplay.tsx:20-31` — wraps `formatMoney`,
context currency + EUR rate), `CurrencySwitcher` + `CurrencyProvider` (`/v1/rates` fetch,
`packages/ui/src/lib/CurrencyProvider.tsx:35`), `AnimatedNumber`, `Pressable`, `ResponsiveDialog`
(`packages/ui/src/components/molecules/ResponsiveDialog.tsx`), `StickyActionBar` (keyboard-aware,
`StickyActionBar.tsx:15-32`), `EmptyState`, `SkeletonBase`, toast system (`useToast`),
`getAllergenStyle`, `useI18n` (sq/en/uk), `derivePalette` theming, `SunlightToggle`.

---

## 2. The ordering-flow map (as recorded)

Persona traces: round-3 `client-first-timer-impatient` ran against `https://dowiz-staging.fly.dev/s/demo`
(mobile 390px): goto → click search `input` → 5× click `[data-testid="menu-item-add"]` → done
(`e2e/findings/round-3/client-first-timer-impatient/transcript.md`; actions confirmed in
`trace-r3/trace.trace`). Frames `trace-r3/resources/page@…578915.jpeg` (cart bar “Shporta · 2100 ALL”,
badge 2) → `…591671.jpeg` (badge 5, “5250 ALL”) show the loop working with toast feedback each add.

```
/s/:slug (SSR shell → SPA hydrate)
 │  data: GET /public/locations/:slug/menu?locale  (menu_version, currency, categories→products→modifier_groups)
 │        GET /public/locations/:slug/info          (status, rating, geo, fee inputs, socials)
 │        GET /public/theme/:slug                   (palette, logo, locales)
 │  cart reconcile: localStorage cart ⇄ menu_version (reprice/drop + toast)
 ▼
BROWSE  hero → category tabs (scroll-spy) → grid of ProductCards
 │  optional: search (name/desc) · allergen lens · sort (flattens categories!)
 ▼
PRODUCT DETAIL (tap card; quick-add “+” skips this iff no modifier groups)
 │  data: lazy GET /products/:id/media (only if primary_media_id)
 │  select modifiers (min/max/required) → qty → Add (price = (base+Σdelta)×qty)
 ▼
CART (sticky bar → ResponsiveDialog sheet)
 │  qty ± / remove / clear → “Checkout”  → /s/:slug/checkout
 ▼
CHECKOUT (single long form)
 │  needs: name, phone(+355 coerce), [messenger kind+handle], delivery|pickup,
 │         map pin, address, entrance*, apartment*, how-to-find-you*, dropoff chip,
 │         cash amount (≥ total), tip, [entry photo]
 │  POST /orders {locationId, type, items[{product_id,quantity,modifier_ids[]}],
 │                customer{phone,name,messenger_*}, delivery{pin,address_text},
 │                payment{method:'cash'}, cash_pay_with, tip_amount, prefs.dropoff,
 │                delivery_instructions, idempotency_key}
 │  outcomes: clean → order id (+authToken) | soft_confirm+requiresOtp → OTP modal loop
 │            | hard_block → “review your cart” | 422 codes → designed messages
 ▼
ORDER STATUS /s/:slug/order/:id (?t= grant exchange for fresh devices)
 │  data: GET /customer/orders/:id/status (+etaRange, courierPosition, route)
 │        WS order:<id> (status, courier pos, route, messages) + 30s watchdog
 │  live map · ETA range · stepper · share-location · call/message courier
 │  DELIVERED → rating + Google review · REJECTED/CANCELLED → order-again exits
```

Friction observed in the recordings/screenshots:
- **Hydration gap**: round-2 finding `F-30841` — persona saw “no actionable elements” on `/s/demo`
  right after load (triaged as observe-before-hydrate in `e2e/findings/FINDINGS.md`, but it maps to a
  real SSR-shell→hydration window where nothing is tappable for an impatient user).
- **Selector-hallucination triage** (round-1 `F-21311/F-41412/F-74853`): NOT_A_BUG, but the fix note
  confirms the only stable affordances are `menu-item`/`menu-item-add` testids — the tab/menu chrome
  offered nothing else recognizable to act on.
- Round-3 clean run: search + 5 quick-adds with zero dead-ends — the storefront happy path is genuinely
  low-friction (`FINDINGS.md` R3 note).
- Visual friction: toast covers header controls while adding (`…591671.jpeg`); hero title contrast on
  light tenant palettes; the checkout is one long scroll with 5+ required fields for delivery
  (`FINAL-07-checkout-full.png` shows only ~2 sections fit a viewport).

---

## 3. The menu data model as the UI sees it

`GET /public/locations/:slug/menu` (served by `read_public_menu` SQL,
`packages/db/migrations/1790000000064_read-public-menu-perf.ts:74-140`; route caching/shaping
`apps/api/src/routes/public/menu.ts:60-176`):

```jsonc
{
  "menu_version": 42,                    // drives cart reconcile + cache headers
  "default_locale": "sq", "supported_locales": ["sq","en","uk"],
  "currency": { "code": "ALL", "minor_unit": 0 },   // FE reads only .code (MenuPage.tsx:76-80)
  "location_id": "…", "location_name": "Dubin & Sushi",
  "categories": [{
    "id": "…", "name": "…(localized)", "sort_order": 1,
    "products": [{
      "id": "…", "name": "…", "description": "…",
      "price": 1400,                     // integer, MAJOR units of venue currency (lek)
      "available": true,                 // only true rows are ever returned
      "prep_time_minutes": 15,           // migration …065
      "image_key": "…", "imageUrl": "…(resolved server-side)",
      "primary_media_id": null,          // non-null ⇒ FE lazily fetches /products/:id/media
      "attributes": {                    // UNTYPED jsonb bag — the de-facto extension point
        "chef_pick": true,
        "taste": { "spicy": 2, "sweet": 1, … },   // levels 1-3
        "bom": [{ "supplyId","supplyName","qty","unit","kind",
                   "kcal","proteinG","fatG","carbsG","allergens":[…] }],
        "stock_count": 12                // authored, not enforced in the funnel
      },
      "modifier_groups": [{
        "id","name","min_select","max_select","required","sort_order",
        "display_type": "radio|checkbox|select|quantity|null",   // migration …060
        "modifiers": [{ "id","name","price_delta","available","sort_order" }]
      }]
    }]
  }]
}
```

Derivations the FE performs (any new component system must either replicate or move server-side):
- **Nutrition/ingredients/allergens are not fields — they are folds over `attributes.bom`**
  (`MenuPage.tsx:99-114 bomToNutrition`): kcal/P/F/C summed, allergens unioned, ingredients =
  supplyNames minus packaging/utensils. Authored in admin via `RecipeEditor` supply lines.
- Chef’s Picks = synthetic category over `attributes.chef_pick` (`MenuPage.tsx:169-178`).
- Allergen filter values = union over all BOM lines (`MenuPage.tsx:218-227`).
- Price math: `unit = price + Σ selected modifier price_delta`; cart line total = unit × qty; all
  integer lek; EUR is a display-only conversion (`formatMoney`, `packages/shared-types/src/utils.ts:20-41`).
- Companion endpoints: `/public/locations/:slug/info` (open/closed/busy status, rating, lat/lng,
  `deliveryFeeFlat`/`freeDeliveryThreshold`/`minOrderValue`/`taxRate`/`hasDistanceTiers` — feeds the
  ADR-0005 total mirror, `CheckoutPage.tsx:273-280`), `/public/theme/:slug`, `/v1/rates` (EUR).
- Owner-side model (`useMenuData.ts:9-28`) is a *different* camelCase shape (`categoryId`,
  `recipeLines`, `stockCount`) from the public snake_case shape — two vocabularies for one domain.

---

## 4. Legacy issues NOT to carry forward

1. **Money display fragility.** The “NaN ALL / missing item name” order line rendered in production
   (`FINAL-11-order-status-full.png`), patched by fallback-chains at `OrderStatusPage.tsx:697-704`;
   the unit-vs-line-total confusion had to be fixed twice (cart dialog still shows unit price,
   `ClientLayout.tsx:180`). `formatMoney` hard-assumes amounts are ALL and ignores
   `currency.minor_unit` entirely (`utils.ts:20-41`; `MenuPage.tsx:76-80`); `parseInt` on cash/tip
   inputs drops decimals (`CheckoutPage.tsx:876,896`). New system: one Money type
   (amount+currency+minor_unit) rendered by one component, everywhere.
2. **The 3-kind vs 6-kind messenger split (G03).** DB CHECK + Zod allow only
   telegram/whatsapp/viber (`migrations/1790000000038:8`, `shared-types/src/legacy.ts:48`) while the
   deployed checkout already shows the ADR-0016 “Communication” selector (6 kinds incl. Phone,
   Signal, SimpleX + receiver checkbox — `demo-checkout.png`); the widening migration `…074` exists
   only under `docs/design/checkout-communication/`. Any FE/BE skew here 422/400s the order POST.
   New system: enum owned in one shared contract, FE options generated from it.
3. **Fabricated/hardcoded data on money-adjacent surfaces.** 404 order → client fabricates a fake
   PENDING order (`OrderStatusPage.tsx:197-204`); checkout pre-order ETA hardcoded 25–45
   (`CheckoutPage.tsx:1020`); `calcETA` mock returning “15-25 min” still exported
   (`utils.ts:56-60`); Durrës/Tirana fallback coordinates (`CheckoutPage.tsx:225`,
   `OrderStatusPage.tsx:400-402`).
4. **Nutrition has three inconsistent representations**: `attributes.bom` folds (storefront),
   `recipeLines` (admin), and order `*_mg_total` columns rendered with unit “g”
   (`OrderStatusPage.tsx:770-777`). The checkout `NutritionRing` is dead code — cart items never
   carry kcal (`CheckoutPage.tsx:377` vs `MenuPage.tsx:505-512,852`).
5. **`attributes` as an untyped any-bag** (`MenuPage.tsx:57,88-91`) — chef_pick/taste/bom/stock all
   live there with no schema; `attrEntries` even renders “whatever else is in the bag”
   (`MenuPage.tsx:542-545`).
6. **Sort destroys IA**: any non-default sort collapses categories into one flat list and removes
   the category nav (`MenuPage.tsx:195-206`) — defensible math, hostile UX; and the allergen filter
   *includes* rather than excludes (`MenuPage.tsx:192-194`), the opposite of the safety use-case.
7. **No comparison mode exists** despite being remembered as a feature — don’t “port” it; design it
   fresh (§1.2).
8. **`display_type` is cosmetic** — select/quantity modifier groups render as chips with only a
   different header icon (`MenuPage.tsx:1117-1130`).
9. **Page-component monoliths**: MenuPage 1276 lines, CheckoutPage 1152, MenuManagerPage 1244 —
   inline styles + Tailwind + framer-motion variants re-declared per page; duplicated no-photo
   fallback (ProductCard vs modal), duplicated CartItem types (×3: `CartDrawer.tsx:7`,
   `use-cart.ts:4`, provider usage), dead `CartDrawer` component.
10. **Double-escaped brand text** (“Dubin &amp; Sushi”) shipped to customers
    (`01-menu-page-initial.png`); fixed with a “do NOT hand-escape” landmine comment
    (`ssr-renderer.ts:53-56`) — escaping ownership must be structural in the new SSR story.
11. **Third-party calls woven into the funnel**: OSRM public router from the menu hero
    (`MenuPage.tsx:324`), Wikipedia on checkout (`CheckoutPage.tsx:286`) — latency/privacy liabilities
    on the two highest-value pages.
12. **Dead/vestigial UI shipped**: “Planifikuar” scheduled tab (Jun-17 build; branch still at
    `CheckoutPage.tsx:837-844`), “Të gjitha (0)” tab on the not-found page (`artepasta-desktop.png`),
    dev mock-order escape hatch (now DEV-gated, `CheckoutPage.tsx:518-522`).

**Worth keeping** (patterns the new system should preserve): menu_version cart reconcile (F9),
server-authoritative totals with the honest “fee at checkout” degrade (ADR-0005), honest ETA ranges,
designed 422 error taxonomy + call-restaurant fallback, terminal-state exits, OTP-bound-to-cart flow,
per-tenant `derivePalette` theming, reduced-motion discipline, 44px tap targets, and the
StateChip venue/item state language.
