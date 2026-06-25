# Repo Dossier 4 — `tastyigniter/TastyIgniter`

> **Research scope.** DeliveryOS is a white-label, **per-restaurant** food-ordering app
> (`/s/:slug`, NOT a marketplace) on Fastify + Postgres + pg-boss + React 18 PWA, Zod,
> own WebSocket; schema `organizations → locations → (menu/category/item/modifier/order/customer/courier)`
> keyed on `location_id`; 10 order states; cash-on-delivery; RLS tenant isolation; Albania mobile-first.
>
> TastyIgniter is **PHP/Laravel** — a *different stack*. **Verdict: PATTERNS-ONLY, no code lift**
> (confirmed below even though license is MIT). Its value to us is **domain completeness** —
> it is the single richest open reference for **menu / modifier / schedule / COD data modelling**.

---

## R1 — Identity & License

| Field | Value | Source |
|---|---|---|
| Slug | `tastyigniter/TastyIgniter` | github.com/tastyigniter/TastyIgniter |
| Description | "Powerful, yet easy to use, open-source online ordering, table reservation and management system for restaurants" | repo header |
| Language | PHP 100% (PHP 8.3–8.4) | repo sidebar |
| Stars / Forks | ~3.6k / ~1.2k | repo sidebar |
| Recency | **v4.2.5, 2026-06-14** — actively maintained | Releases |
| License | **MIT** ("The MIT License (MIT)") | `LICENSE.md` |

**License verdict:** MIT is fully permissive — code reuse *would* be legally allowed.
**But the stack is PHP/Laravel and DeliveryOS is TypeScript/Fastify**, so there is *nothing liftable
as code*. **R1 disposition: PATTERNS-ONLY regardless of license.** We borrow **data-model shapes and
domain concepts**, never source.

---

## R2 — Stack & topology

- **Framework:** Laravel 12 (`laravel/framework: ^12.0`) — `composer.json` of the app skeleton.
- **The app repo is a thin skeleton.** All domain logic lives in the composer package
  **`tastyigniter/core: ^4.0`** plus a constellation of **`ti-ext-*` extension packages**
  (`composer.json require`). This is the single most important architectural fact: TastyIgniter is
  **modular by extension**, not monolithic.
- **Extension architecture** (each is its own repo / composer package, auto-discovered via
  `package:discover`; an extension's `database/migrations/` is auto-loaded on enable):
  - `ti-ext-local` — **Locations**, areas, working hours, reviews (`src/Models/Location.php`,
    `LocationArea.php`, `WorkingHour.php`, `LocationSettings.php`).
  - `ti-ext-cart` — **the entire menu + order domain** (see R3/R4). Models: `Menu`, `Category`,
    `MenuCategory`, `MenuOption`, `MenuOptionValue`, `MenuItemOption`, `MenuItemOptionValue`,
    `Mealtime`, `MenuSpecial`, `Stock`, `StockHistory`, `Order`, `OrderMenu`,
    `OrderMenuOptionValue`, `OrderTotal`, `Cart`.
  - `ti-ext-reservation` — table booking (`Reservation`, `DiningArea/Section/Table`).
  - `ti-ext-payregister` — **payment gateways incl. COD** (`src/Models/Payment.php`,
    `PaymentLog.php`, `PaymentProfile.php`; gateway drivers under `src/Payments/`).
  - `ti-ext-broadcast` — real-time notifications via Laravel Broadcasting/Pusher (R5).
  - `ti-ext-api` — REST API resources (its `docs/menus.md` / `docs/orders.md` are excellent
    schema references and were used heavily below).
  - Core module split inside `tastyigniter/core/src`: `Admin`, `Main`, `System`, `Flame`.
    `Status` / `StatusHistory` models live in `core/src/Admin/Models/`.

**Map to DeliveryOS:** TastyIgniter's "extension package" boundary ≈ our `packages/*` + per-domain API
route modules. Our equivalents: `menu`, `orders`, `locations` modules. We do **not** want their
runtime plugin/discovery machinery — too heavy for a single white-label tenant app.

---

## R3 — Data model (DEEP — the core value)

### 3.1 Locations & multi-location (`ti-ext-local`, `Locationable` trait)

- `Location` (`src/Models/Location.php`) is the tenant unit. `IGNITER_LOCATION_MODE` env =
  `single` | `multiple` toggles single vs multi-restaurant.
- **Multi-location is modelled with a polymorphic pivot `locationables`** (`location_id`,
  `locationable_id`, `locationable_type` ∈ {menus, categories, mealtimes, …}, `options`). Menus,
  categories, and mealtimes are **`Locationable`** — i.e. *the same menu row can be shared across, or
  scoped to, specific locations* via this pivot.
  - **DeliveryOS divergence (we do it differently, arguably better for white-label):** we key
    `menu/category/item/modifier` **directly on `location_id`** (FK column), enforced by **RLS**.
    TastyIgniter's polymorphic any-to-any pivot supports *menu sharing across locations*, which a
    per-restaurant app does not need. **Disposition: SKIP the polymorphic pivot; KEEP the idea that
    "shared vs per-location" is a product decision** — if we ever add chains, a `location_menu` join
    is the migration path. Module: `locations` / `menu`.

### 3.2 Menu & categories (`ti-ext-cart`)

- `Menu` (`menus`, pk `menu_id`): `menu_name`, `menu_description` (nullable since 2025-05-22),
  `menu_price` (float), `minimum_qty`, `menu_status` (bool), `menu_priority` (int),
  **`order_restriction`** (array → `delivery` | `collection` | both). Relations:
  - `categories` → belongsToMany via pivot **`menu_categories`**
  - `mealtimes` → belongsToMany via pivot **`menu_mealtimes`** (schedule, see 3.4)
  - `menu_options` → hasMany `MenuItemOption` (modifiers, see 3.3)
  - `locations` → polymorphic `locationable`
  - `special` → hasOne `MenuSpecial` (time-boxed promo price)
  - `ingredients` / `allergens` → polymorphic `ingredientable`
- `Menu::isAvailable($datetime)` composes **mealtime availability AND ingredient/stock status** — a
  single domain method answering "can a customer order this right now?".

**Map → DeliveryOS `menu` module:**
| TastyIgniter | DeliveryOS | Disposition |
|---|---|---|
| `menu_name/description/price/status/priority` | `item.name/description/price_cents/is_active/sort` | **borrow-idea** (we already have) |
| `order_restriction` (delivery/collection/both) | (likely absent) | **adapt** — add `fulfillment_restriction` enum to `item` so an item can be delivery-only/pickup-only. Cheap, real-world need. |
| `minimum_qty` per item | (check) | **adapt** — useful for combos/bulk; one nullable int column |
| `menu_categories` M:N pivot | `category_id` FK on item | our 1:N is simpler; TastyIgniter allows an item in many categories. **borrow-idea only if** merchandising needs multi-category; otherwise **skip**. |

### 3.3 Modifiers — the modelling to study (`ti-ext-cart`)

TastyIgniter splits modifiers into a **reusable option library** + **per-item attachment** +
**per-item value overrides**. This is the key pattern:

1. **`MenuOption`** (`menu_options`, pk `option_id`) — a *reusable* option group ("Size", "Toppings"):
   `option_name`, `display_type` (`checkbox`/`radio`/`select`/`quantity`).
2. **`MenuOptionValue`** (`menu_option_values`, pk `option_value_id`) — the *catalog* choices for a
   group: `option_id`, `name`, **`price`** (float, nullable = no surcharge), `priority`. Polymorphic
   `ingredientable` to track stock impact.
3. **`MenuItemOption`** (the `menu_options` *relation on a Menu*) — **attaches** an option group to a
   specific menu item with per-attachment rules: `menu_id`, `option_id`, **`required`** (bool),
   **`min_selected`**, **`max_selected`**.
4. **`MenuItemOptionValue`** — **per-item override** of a catalog value: lets one item charge a
   different `new_price`/`quantity`/`subtract_stock` for the same shared option value
   (per `ti-ext-api/docs/menus.md`: `menu_option_value_id`, `new_price`, `quantity`,
   `subtract_stock`).

So the chain is **`MenuOption → MenuOptionValue` (shared library)**, attached to items via
**`MenuItemOption → MenuItemOptionValue` (per-item rules + price override)**.

**Map → DeliveryOS `menu` module (HIGH VALUE — adapt):**
- DeliveryOS today: `modifier` keyed on `location_id` (likely a flatter item↔modifier model).
- **borrow-idea:** the **two-tier separation** — a *reusable modifier group + values* catalog,
  attached to items with **per-attachment `required` / `min_select` / `max_select`** constraints,
  and an optional **per-item price override**. The selection-constraint triple
  (`required`, `min_selected`, `max_selected`) + `display_type` (radio vs checkbox vs quantity) is
  exactly the Zod/validation contract a robust cart needs and is worth replicating in our schema and
  `Zod` order-validation. **Adapt, don't lift.**
- **MAY-DEVIATE (simplify):** the 4-table fan-out (`MenuItemOptionValue` price overrides) is heavy.
  For Albania mobile-first single-restaurant, a **3-table** shape
  (`modifier_group → modifier_option`, attached to `item` with constraint columns, single price on
  the option) is usually enough. Add per-item overrides **only if** menus actually re-price shared
  options. **skip the 4th table until proven needed.**

### 3.4 Menu schedules / availability — the "86 / stop-list" + breakfast-lunch (`Mealtime`)

`Mealtime` (`mealtimes`, pk `mealtime_id`) is the **time-window availability engine**:
- Fields: `mealtime_name`, `start_time`/`end_time` (daily window), `mealtime_status` (bool on/off),
  `start_at`/`end_at` (datetime period), `recurring_every` (array of weekday names Sun–Sat),
  `recurring_from`/`recurring_to` (time bounds). The `start_date/end_date` columns were added
  2025-07-08 (`add_start_end_date_mealtimes_table`).
- **`isAvailable($dateTime)` has three modes:** **Daily** (between `start_time`/`end_time`),
  **Period** (datetime within `start_at`/`end_at`), **Recurring** (weekday ∈ `recurring_every` AND
  time within `recurring_from`/`recurring_to`, with overnight-window handling).
  `isAvailableNow()` = `isAvailable(CarbonImmutable::now())`.
- Attached to items via **`menu_mealtimes`** pivot. `Menu::isAvailable()` AND-combines mealtime +
  stock/ingredient → **this is the "breakfast/lunch schedule"**.
- **Stop-list / "86" (out of stock):** modelled via **`Stock` / `StockHistory`** + the
  `Stockable` trait; 2026-02-27 migration adds **`out_of_stock_override`** to `stocks` — i.e. a
  manual flag to instantly 86 an item independent of quantity. `MenuOptionValue.subtract_stock`
  decrements stock when a modifier is chosen.

**Map → DeliveryOS `menu` module (HIGH VALUE — adapt):**
- We almost certainly lack a first-class **availability/schedule** concept. **borrow-idea:** a
  `schedule` (mealtime) entity with `{mode: daily|period|recurring, start/end, weekdays}` attached to
  items, plus a derived `is_available_now` the storefront and API both call. This powers
  "breakfast menu hides at 11am" with zero cron — pure time comparison at query time.
- **borrow-idea (the 86 button):** a manual **`out_of_stock` / `available` override** on item (and
  modifier-option), separate from any quantity tracking. Cheap (one bool/timestamp), high operator
  value, mobile-first ("sold out for today"). Module: `menu`. **This is the single most actionable
  steal in the dossier.**
- **MAY-DEVIATE:** TastyIgniter's full `Stock`/`StockHistory` inventory ledger is more than a
  COD-cash Albania MVP needs. **skip quantitative stock; keep the boolean 86 override + the schedule
  engine.**

### 3.5 Orders, order_menus, order_totals, customers (`ti-ext-cart` + api docs)

- **`Order`** (`orders`, pk `order_id`): `customer_id`, `location_id`, `address_id`,
  `first_name/last_name/email/telephone`, `comment`, `notify`, **`order_type`** (`delivery` |
  `collection`), `order_date`/`order_time` (scheduled fulfilment), `processed` (bool — payment
  done), `total_items`, `order_total` (float), **`payment`** (gateway code, e.g. `"cod"`),
  `invoice_prefix`/`invoice_date`, **`status_id`**, **`status_updated_at`**, `assignee_id`,
  `assignee_group_id`, `hash` (public token), `ip_address`, `user_agent`. `cart` column cast as
  serialized snapshot. Relations: belongsTo `customer/location/address/payment_method`; hasMany
  `payment_logs`, `menus` (OrderMenu), `menu_options` (OrderMenuOptionValue), `totals` (OrderTotal).
- **`OrderMenu`** — line items: `id/name/qty/price/subtotal/comment/options` — a **denormalised
  snapshot** of what was ordered (name/price copied at order time, not just FK).
- **`OrderMenuOptionValue`** — the modifier choices captured per line (snapshot).
- **`OrderTotal`** (`order_totals`) — **a totals *ledger*, not columns**: rows of
  `{code, title, value, priority}` (e.g. `subtotal`, `delivery`, `tax`, `coupon`, `total`). The
  order's money breakdown is *append-only rows ordered by priority*, not a fixed set of columns.
- **`Customer`** (`ti-ext-user`) — `customer_id`, name/email/telephone, addresses (1:N), guest vs
  registered.

**Map → DeliveryOS `orders` module:**
| TastyIgniter | DeliveryOS | Disposition |
|---|---|---|
| `order_type` delivery/collection | our order has fulfilment type | **HONOR** (already) |
| **`OrderTotal` ledger rows** `{code,title,value,priority}` | (likely fixed columns) | **borrow-idea / adapt** — a totals-row table makes tax + delivery fee + future coupons composable and auditable without schema churn. **Strong candidate**, but our integer-cents tax rule must apply (store `value_cents` int, never float — TastyIgniter uses float, **anti-pattern for us**). |
| **`OrderMenu` snapshot** (name/price copied) | (verify) | **HONOR** — denormalising name/price at order time is correct; menu edits must not mutate history. If we don't, **adopt**. |
| `hash` public order token | our public order link | **borrow-idea** — opaque token for guest order status page |
| `order_date`/`order_time` scheduled | scheduled orders | **adapt** if we support pre-orders |
| `processed` bool (payment done) | order state | maps to our COD flow (R7) |
| Customer guest+registered + addresses 1:N | customer module | **HONOR** |

---

## R4 — Order state machine

TastyIgniter does **NOT** hardcode a fixed enum of states. Instead:

- **`Status`** (`core/src/Admin/Models/Status.php`, table `statuses`, pk `status_id`):
  `status_name`, `status_color`, **`notify_customer`** (bool), `status_comment`, **`status_for`**
  (`order` | `reservation`). Statuses are **data, admin-editable**, and partitioned by `status_for`
  (scopes `isForOrder()` / `isForReservation()`). Default seeded order statuses are roughly
  **Received → Pending → Confirmed → Preparation → (Out for delivery) → Completed / Cancelled**
  (per docs; the exact set is seeded data, editable by the restaurant).
- **`StatusHistory`** (`core/src/Admin/Models/StatusHistory.php`, table `status_history`): the audit
  log — `status_history_id`, **polymorphic `object_id`/`object_type`** (shared by orders AND
  reservations), `status_id`, `user_id` (staff who changed it), `comment`, `notify` (bool),
  `created_at`. Static **`createHistory()`** resolves the status, inserts a history row, fires
  `admin.statusHistory.beforeAddStatus`, then **atomically updates the parent's `status_id` +
  `status_updated_at`**.
- On the `Order` model: `updateOrderStatus($id, $options)` → delegates to `addStatusHistory()` (via
  `LogsStatusHistory` trait) → writes history + sets `status_id`. Helpers `isCompleted()` (processed
  AND status == configured completed id), `isCanceled()`, `markAsCanceled()` (fires
  `OrderCanceledEvent`). The "completed"/"canceled" status ids are **config-driven**, not constants.

**Map → DeliveryOS 10-state COD machine (`orders` module):**
- DeliveryOS uses a **fixed 10-state machine in code** (typed, Zod-validated transitions). This is
  **better for a single white-label product** than TastyIgniter's admin-editable status soup —
  keep our typed enum. **Disposition: KEEP our approach; do NOT adopt editable statuses** (anti-pattern
  for a contract-driven app: editable states break exhaustive switch + courier logic).
- **borrow-idea (strong):** the **separate append-only `status_history` audit table** keyed to the
  order, recording `{to_status, actor (staff/courier/system), comment, notify_flag, created_at}`.
  TastyIgniter logs *who* changed status and *whether the customer was notified* on every transition
  — exactly what a COD courier flow + dispute trail needs. If DeliveryOS only mutates a single
  `order.status` column, **adopt an order_status_history table** as the source of truth, with the
  current `status` denormalised on `order` for fast reads (their `status_id` + `status_updated_at`
  mirror pattern).
- **borrow-idea:** the **`notify_customer` flag *per status* + `notify` per transition** — decouples
  "this state notifies by default" from "suppress notification on this particular change". Maps
  cleanly onto our Telegram-notifications design (per-category prefs). Module: `orders` + notif.
- The polymorphic history (orders + reservations share one table) is **N/A** — we have no
  reservations; a dedicated `order_status_history` is simpler.

---

## R5 — Real-time

- **Default frontend: POLLING.** TastyIgniter's storefront/admin order list refreshes by AJAX poll;
  real-time push is an **opt-in extension**.
- **`ti-ext-broadcast`** adds push via **Laravel Broadcasting → Pusher** (or other Laravel broadcast
  drivers), emitting order-update / reservation notifications to admin/kitchen browsers. It is **not**
  installed by default and depends on a third-party (Pusher) account.
- **DeliveryOS divergence (we do it better):** we run **our own WebSocket** server (no Pusher
  dependency, no per-message SaaS cost), pushing order/courier events directly. **Disposition: N/A /
  keep ours.** Only takeaway: TastyIgniter's *event taxonomy* (new-order, status-change,
  assignment-change as distinct broadcast events) is a reasonable channel design to mirror on our ws.

---

## R6 — Component / theme system

- TastyIgniter uses **Laravel Blade + a `themes/` directory** (server-rendered, Bootstrap 5) with a
  theme/partial override system (e.g. `igniter.payregister::_partials.cod.payment_form`).
- **N/A to DeliveryOS `packages/ui` (React 18).** No component lift possible across Blade→React.
  **Only IA/structure ideas transfer:** the partial-per-payment-method and partial-per-cart-section
  decomposition is a sensible *information-architecture* template for our checkout components, nothing
  more. **Disposition: N/A (UX), borrow IA framing only.**

---

## R7 — Checkout & payments (COD first-class — capture the shape)

- **Payment-gateway abstraction (`ti-ext-payregister`):** every gateway extends
  **`BasePaymentGateway`** and implements `processPaymentForm($data, $host, $order)`,
  `defineFieldsConfig()`, a payment-form partial, plus `isApplicable()` / `completesPaymentOnCheckout()`
  hooks. `Payment` model = a configured gateway instance per location; `PaymentLog` = per-attempt
  audit; `PaymentProfile` = saved cards (N/A for COD).
- **COD is a first-class gateway** — `ti-ext-payregister/src/Payments/Cod.php` extends
  `BasePaymentGateway`. Its `processPaymentForm()`:
  1. `validateApplicableFee($order, $host)` — COD can carry a surcharge / min-order rule.
  2. **`$order->updateOrderStatus($host->order_status, ['notify' => false])`** — moves the order to
     the **admin-configured post-COD status** (no external auth step).
  3. **`$order->markAsPaymentProcessed()`** — sets `processed = true`, fires payment events, saves
     quietly. COD is marked "processed" **at checkout submit** even though cash is collected on
     delivery (post-payment model).
- So the **COD data shape** = `order.payment = "cod"` + `order.processed = true` +
  a `status_history` transition to the configured COD status + a `payment_logs` row.
  COD config carries: `order_status` (which state COD orders land in), `order_total` minimum, and an
  applicable fee/surcharge.

**Map → DeliveryOS `orders` / checkout (we already center on COD — confirm the shape):**
- **borrow-idea / adapt:**
  - **COD-specific config on the location/checkout:** `cod_min_order`, `cod_fee`,
    `cod_lands_in_status` — TastyIgniter proves these are real merchant knobs. Our integer-cents rule
    applies. Module: `orders` / `locations`.
  - **`payment_logs` audit row even for cash** — a per-attempt/transition payment record (not just an
    order column) gives us a clean dispute/recon trail for cash collection. **adopt** as a thin
    `order_payment_event` table.
  - The **gateway-driver abstraction** (`processPaymentForm` + `completesPaymentOnCheckout` +
    `defineFieldsConfig`) is the right seam for the day we add Stripe/local PSPs — but **MAY-DEVIATE /
    don't pre-build:** per `/ponytail` YAGNI, keep a single COD path now; capture the *seam shape*
    (a `PaymentMethod` interface with `process()` + `completesOnCheckout()`) so adding a gateway later
    is additive, not a refactor.
- **HONOR:** COD marked "processed/placed at submit, cash settled on delivery" — that is exactly our
  cash-on-delivery semantics; our 10-state machine already encodes "placed → … → delivered (cash
  collected)".

---

## R8 — Patterns to adopt vs anti-patterns

**Adopt (domain / data-model):**
1. **Two-tier modifier model** — reusable `option group + values` library, attached per-item with
   `required / min_select / max_select / display_type` constraints (R3.3). *(menu)*
2. **Mealtime/schedule availability engine** — `daily | period | recurring` modes, AND-combined into
   a single `isAvailableNow()` the storefront + API share (R3.4). *(menu)*
3. **First-class 86 / out-of-stock override** — a manual boolean kill-switch on item & modifier-option,
   separate from quantity (R3.4). **Highest ROI, lowest cost.** *(menu)*
4. **`order_totals` ledger rows** `{code,title,value,priority}` — composable money breakdown instead of
   fixed columns (R3.5) — **but store integer cents**. *(orders)*
5. **Denormalised order line snapshots** (`OrderMenu` copies name/price) so menu edits never rewrite
   history (R3.5). *(orders)*
6. **Append-only `status_history`** with `actor + comment + notify_flag` per transition; current
   status denormalised on the order (R4). *(orders)*
7. **Per-status `notify_customer` + per-transition `notify`** decoupling → feeds our Telegram prefs (R4).
8. **COD config knobs** (`cod_min_order`, `cod_fee`, landing status) + a payment-event audit row even
   for cash (R7). *(orders/locations)*
9. **`order_restriction` / fulfilment-scope on items** (delivery-only / pickup-only) (R3.2). *(menu)*

**Anti-patterns (do NOT copy):**
- **Float money** everywhere (`menu_price`, `order_total`, `OrderTotal.value` as float) — DeliveryOS
  rule is **integer cents**. Hard avoid.
- **Admin-editable order statuses** (`Status` rows) — breaks a typed, contract-driven 10-state machine
  and exhaustive courier logic. Keep our enum.
- **Polymorphic `locationables` any-to-any** menu↔location sharing — overkill for per-restaurant;
  our direct `location_id` FK + RLS is leaner and safer.
- **Pusher-dependent realtime** — we own our ws; no third-party push SaaS.
- **Runtime extension/plugin discovery** machinery — heavy for a single white-label tenant; our
  `packages/*` build-time modularity is enough.
- **Full quantitative `Stock`/`StockHistory` inventory ledger** — more than a COD-cash MVP needs;
  take only the boolean 86.

---

## R9 — Liftable (IDEAS/schema only — PHP code NOT liftable) vs what DeliveryOS already does better

**Liftable as schema shapes / domain ideas (no code):**
- Modifier two-tier shape + selection-constraint triple (group→values, item attach with
  required/min/max/display_type). → `menu`
- Mealtime schedule entity (`mode`, windows, weekday recurrence) + derived `is_available_now`. → `menu`
- 86 / out-of-stock boolean override on item & option. → `menu`
- `order_totals` ledger-row table (integer-cents). → `orders`
- `order_status_history` audit table (actor/comment/notify, polymorphic-collapsed to orders-only). → `orders`
- COD knobs + cash payment-event audit row + the `PaymentMethod` *seam* (interface only). → `orders`
- Order line-item snapshot discipline (copy name/price at order time). → `orders`
- Item `fulfillment_restriction` (delivery/collection/both). → `menu`

**What DeliveryOS already does better (keep as-is):**
- **Typed 10-state machine** (vs editable status rows) — more robust, courier-aware.
- **Integer-cents money** (vs float) — correctness.
- **Direct `location_id` FK + RLS tenant isolation** (vs polymorphic locationables) — simpler, safer
  for white-label per-restaurant.
- **Own WebSocket** (vs Pusher dependency) — cost + control.
- **Build-time `packages/*` modularity** (vs runtime plugin discovery) — appropriate weight.
- **React 18 PWA** (vs Blade SSR) — mobile-first Albania target.

---

### Sources
- Repo / license / stack: github.com/tastyigniter/TastyIgniter (`composer.json`, `LICENSE.md`), v4.2.5 release 2026-06-14.
- Menu/modifier/order schema: `tastyigniter/ti-ext-cart/src/Models/` (`Menu.php`, `MenuOption.php`,
  `MenuOptionValue.php`, `Mealtime.php`, `Order.php`, `OrderMenu.php`, `OrderTotal.php`) and
  `ti-ext-cart/database/migrations/` (mealtime start/end-date 2025-07-08; stock `out_of_stock_override` 2026-02-27).
- API field shapes: `tastyigniter/ti-ext-api/docs/menus.md`, `docs/orders.md`.
- Status machine: `tastyigniter/core/src/Admin/Models/Status.php`, `StatusHistory.php`.
- Locations / multi-location: `tastyigniter/ti-ext-local/src/Models/Location.php`; `locationables` pivot;
  `IGNITER_LOCATION_MODE` docs.
- COD / payments: `tastyigniter/ti-ext-payregister/src/Payments/Cod.php`, `src/Models/Payment.php`, `PaymentLog.php`.
- Realtime: `tastyigniter/ti-ext-broadcast` (Laravel Broadcasting / Pusher).
