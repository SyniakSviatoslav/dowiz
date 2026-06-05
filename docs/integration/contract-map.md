# DeliveryOS Contract Map

> Stage 0 output. Maps every UI screen → required API endpoints + WS rooms.
> Status: WIRED (connected) / STUB (UI scaffold, no real endpoint yet) / MISSING (no server endpoint exists)

## Legend
- **WIRED** — real server endpoint exists and UI can use it
- **STUB** — UI component exists but endpoint marked as post-MVP
- **MISSING** — server endpoint doesn't exist; needs creation

---

## 1. Client Surface (/s/:slug)

### 1.1 Menu (`/s/:slug`) — `01-menu-client.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `/public/locations/:slug/menu` | GET | none | WIRED | Returns categories + products + modifiers + stop-list. Populates all product cards, prices, descriptions, allergens, availability |
| 2 | `/s/:slug` (SSR) | GET | none | WIRED | HTML shell with restaurant info (name, rating, review count, open/closed status, operating hours, hero image, delivery ETA) |
| 3 | `/public/locations/:slug/fallback-config` | GET | none | STUB | Fallback menu if server is offline |
| 4 | `POST /orders` | POST | none | WIRED | Order placement (called from checkout, not menu). Idempotent via idempotency key |
| 5 | `POST /api/telemetry` | POST | none | WIRED | Embed height reporting, usage stats |
| 6 | `/public/locations/:locationId/theme.css` | GET | none | WIRED | Brand CSS variables injected into SSR shell |

**WS rooms:** none on menu page. `order:{orderId}` post-placement only.

**UI data mapping:**
- `location.name` → header title
- `location.rating`, `location.review_count` → hero rating stars
- `location.open`, `location.closing_time` → open/closed badge
- `location.delivery_eta` → "Delivery from 30 min"
- `categories[].name` → category nav tabs
- `products[].name`, `products[].price`, `products[].description` → product cards
- `products[].image_url` → product thumbnail (placeholder `ti-photo` when null)
- `products[].available` → stop-list overlay
- `products[].allergens[]` → allergen tags
- `products[].modifier_groups[]` → product modal choices

---

### 1.2 Cart (`/s/:slug/cart`) — `02-cart.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /orders/:id` | GET | none | WIRED | Not needed for initial cart (client-side cart via localStorage). Used if server-side cart restored |
| 2 | `POST /orders` | POST | none | WIRED | Only on checkout confirmation. Not called from cart itself |

**WS rooms:** none

**UI data mapping:** (all client-side via localStorage + mock-data.js)
- `cart[].items` → rendered product rows with qty, price
- `location.delivery_fee` → order summary delivery fee
- Promo code applied via `SUSHI15` / `FLAT20` (client-side only)
- Subtotal, discount, total → checkout bar

---

### 1.3 Checkout (`/s/:slug/checkout`) — `03-checkout.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /orders` | POST | none | WIRED | Called on confirmOrder(). Body: items, subtotal, deliveryFee, discount, tip, contactless, deliveryInstructions, total. Returns order ID + status |
| 2 | `GET /orders/:id` | GET | none | WIRED | Poll after placement to get order ID and redirect to status |
| 3 | `POST /api/customer/otp/send` | POST | Customer JWT | WIRED | OTP send (optional, when OTP toggle is on for verification) |
| 4 | `POST /api/customer/otp/verify` | POST | Customer JWT | WIRED | OTP verify before order placement |

**WS rooms:** none at checkout

**UI data mapping:**
- Step 1: phone number, name → `POST /orders` customer fields
- Step 2: delivery address, delivery type (delivery/pickup/scheduled), delivery instructions → order metadata
- Step 3: payment method (cash selected, card stub), tip amount, promo code, contactless toggle → order metadata
- `confirmOrder()` builds `orderData` object and POSTs to `/orders`

---

### 1.4 Order Status (`/s/:slug/orders/:orderId`) — `04-order-status.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /orders/:id` | GET | none | WIRED | Returns order status, ETA, items, total, courier info, timeline |
| 2 | `POST /api/customer/orders/:orderId/cancel` | POST | Customer JWT | WIRED | Cancel order before preparation |
| 3 | `POST /api/telemetry` | POST | none | WIRED | Courier GPS position reporting |

**WS rooms:** `order:{orderId}` — real-time status updates, courier location

**UI data mapping:**
- `order.eta` → "~14 minutes" display
- `order.status` → timeline visual (ORDERED → CONFIRMED → PREPARING → IN_DELIVERY → DELIVERED)
- `order.courier_name`, `order.courier_initials` → courier info card
- `order.courier_phone` → call button
- `order.courier_rating` → star rating
- `order.courier_lat`, `order.courier_lng` → map marker position (via WS)
- `order.items[]` → order details accordion
- `order.total` → total display
- `order.destination_address` → destination pin on map

---

### 1.5 Tour Hub — `00-tour-hub.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/public/locations` | GET | none | MISSING | List all restaurants/locations for discovery |
| 2 | `GET /public/locations/:slug/menu` | GET | none | WIRED | Featured restaurant previews |

**WS rooms:** none

**UI data mapping:** Restaurant cards with name, cuisine, rating, delivery time, promo badges. This is hub/discovery page.

---

### 1.6 Embed Demo — `18-embed-demo.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /s/:slug` (SSR) | GET | none | WIRED | Embedded menu iframe |
| 2 | `POST /api/telemetry` | POST | none | WIRED | Height reporting via postMessage |

**WS rooms:** none

**UI data mapping:** Iframe embed of the menu page with `?embed=true` param.

---

### 1.7 Restaurant Discovery — `19-restaurant-discovery.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/public/locations` | GET | none | MISSING | Search/discovery — list all locations with filters |
| 2 | `GET /public/locations/:slug/menu` | GET | none | WIRED | Detail for featured |

**WS rooms:** none

**UI data mapping:** Search bar, cuisine filters, restaurant cards with distance, rating, delivery time.

---

### 1.8 Client Login — `20-client-login.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /auth/exchange` | POST | none | WIRED | Phone/OTP-based login exchange. Returns JWT |
| 2 | `POST /auth/refresh` | POST | Bearer JWT | WIRED | Refresh access token |
| 3 | `POST /api/customer/otp/send` | POST | none | WIRED | Send OTP to phone number |
| 4 | `POST /api/customer/otp/verify` | POST | none | WIRED | Verify OTP before auth |

**WS rooms:** none

**UI data mapping:** Phone input → OTP send/verify → JWT stored in localStorage/sessionStorage.

---

### 1.9 Client Register — `21-client-register.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /auth/exchange` | POST | none | WIRED | Register via phone + name. Creates customer account |
| 2 | `POST /api/customer/otp/send` | POST | none | WIRED | OTP send for verification |
| 3 | `POST /api/customer/otp/verify` | POST | none | WIRED | Verify phone |

**WS rooms:** none

**UI data mapping:** Name, phone, email inputs → registration endpoint.

---

### 1.10 User Profile — `22-user-profile.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/customer/me` | GET | Customer JWT | STUB | Profile data: name, phone, email, avatar, saved addresses |
| 2 | `PUT /api/customer/me` | PUT | Customer JWT | STUB | Update profile |
| 3 | `POST /api/customer/push/subscribe` | POST | Customer JWT | WIRED | Push notification subscription |
| 4 | `POST /api/customer/push/unsubscribe` | POST | Customer JWT | WIRED | Unsubscribe from push |

**WS rooms:** none

**UI data mapping:** Saved addresses, notification preferences, push toggle, personal info.

---

### 1.11 Order History — `23-order-history.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/customer/orders` | GET | Customer JWT | STUB | Past orders list with status, date, total, items |
| 2 | `GET /orders/:id` | GET | none | WIRED | Detail for individual order |

**WS rooms:** none

**UI data mapping:** Order cards with restaurant name, date, items, total, reorder button.

---

### 1.12 Search Results — `24-search-results.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/public/locations?q={query}` | GET | none | MISSING | Search locations by name or cuisine |
| 2 | `GET /api/public/products?q={query}` | GET | none | MISSING | Search products across location |

**WS rooms:** none

**UI data mapping:** Search query → filtered location/product results.

---

### 1.13 Favorites — `25-favorites.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/customer/favorites` | GET | Customer JWT | STUB | Saved/favorite products and restaurants |
| 2 | `POST /api/customer/favorites` | POST | Customer JWT | STUB | Add to favorites |
| 3 | `DELETE /api/customer/favorites/:id` | DELETE | Customer JWT | STUB | Remove from favorites |

**WS rooms:** none

**UI data mapping:** Saved items with name, price, restaurant, quick-add to cart.

---

### 1.14 Rate & Review — `26-rate-review.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /orders/:id/review` | POST | Customer JWT | STUB | Submit rating (food, delivery) + comment |
| 2 | `GET /orders/:id` | GET | none | WIRED | Load order items for review context |

**WS rooms:** none

**UI data mapping:** Star ratings for food + delivery, comment textarea → submitted to review endpoint.

---

### 1.15 Support — `27-support.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /api/support/tickets` | POST | Customer JWT | STUB | Create support ticket |
| 2 | `GET /api/support/tickets` | GET | Customer JWT | STUB | List support tickets |

**WS rooms:** none

**UI data mapping:** Issue type selector, order reference, message text → ticket creation.

---

### 1.16 Pickup Order — `28-pickup-order.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /orders/:id` | GET | none | WIRED | Order status for pickup variant (shows pickup code) |
| 2 | `POST /orders` | POST | none | WIRED | Same as delivery — uses pickup flow variant |

**WS rooms:** `order:{orderId}`

**UI data mapping:** Shows pickup code (e.g. DS-XXXX), order items, ETA, restaurant address.

---

### 1.17 Splash — `37-splash.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /public/locations/:slug/menu` | GET | none | WIRED | Preload menu data after splash |
| 2 | `GET /s/:slug/manifest.webmanifest` | GET | none | WIRED | PWA manifest for install prompt |

**WS rooms:** none

**UI data mapping:** App logo, loading spinner, auto-redirect to menu or restaurant discovery.

---

### 1.18 Error States — `38-error-states.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | (UI component showcase) | — | — | — | Shows offline, 404, 500, empty states. No real API calls |

**WS rooms:** none

**UI data mapping:** Static display of error/empty/offline UI patterns.

---

### 1.19 Notifications — `39-notifications.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/customer/notifications` | GET | Customer JWT | STUB | List notifications |
| 2 | `PATCH /api/customer/notifications/:id/read` | PATCH | Customer JWT | STUB | Mark as read |
| 3 | `POST /api/customer/push/subscribe` | POST | Customer JWT | WIRED | Web push subscription |

**WS rooms:** none (notifications are push-based)

**UI data mapping:** Notification cards with icon, title, message, timestamp, read/unread state.

---

### 1.20 Welcome — `welcome.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | (Static landing page) | — | — | — | Welcome/intro page, no API calls |

**WS rooms:** none

**UI data mapping:** Static promotional content.

---

## 2. Owner Surface (/admin/*)

### 2.1 Admin Dashboard — `05-admin-dashboard.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/dashboard` | GET | Owner JWT | WIRED | Stats: orders count, revenue, active orders, courier count. Trend data vs yesterday |
| 2 | `GET /api/owner/locations/:id/orders` | GET | Owner JWT | WIRED | Active orders list (pending → in_delivery). Status filterable |
| 3 | `PATCH /api/owner/locations/:id/orders/:orderId/status` | PATCH | Owner JWT | WIRED | Accept/decline order (status transition) |
| 4 | `POST /api/owner/locations/:id/orders/:orderId/assign` | POST | Owner JWT | WIRED | Assign courier to order |
| 5 | `GET /api/owner/locations/:id/couriers` | GET | Owner JWT | WIRED | Courier list with online/busy/offline status, location, current assignment |
| 6 | `PATCH /api/owner/locations/:id/settings` | PATCH | Owner JWT | WIRED | Toggle open/closed, busy mode |
| 7 | `GET /api/owner/locations/:id/alerts` | GET | Owner JWT | WIRED | Alert banner data (unresponsive courier, stop-list warnings) |
| 8 | `PATCH /api/owner/locations/:id/alerts/:alertId` | PATCH | Owner JWT | WIRED | Dismiss alert |

**WS rooms:** `location:{id}` — real-time order updates, courier location, new orders

**UI data mapping:**
- `dashboard.orders_count` → Orders stat card
- `dashboard.revenue`, `dashboard.revenue_trend` → Revenue card
- `dashboard.active_orders` → Active count
- `dashboard.couriers.online`, `.busy`, `.offline` → Courier stat + map
- `orders[]` → active order cards with status, timer, items, total
- `couriers[]` → map markers with initials, position, status
- `settings.is_open` → open/closed toggle
- `alerts[]` → alert banner + dropdown

---

### 2.2 Owner Orders — `06-admin-orders.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/orders` | GET | Owner JWT | WIRED | All orders for kanban: pending, confirmed, preparing, ready, in_delivery, scheduled |
| 2 | `GET /api/owner/locations/:id/orders/:orderId` | GET | Owner JWT | WIRED | Order detail (drawer): customer info, items, totals, delivery address, timeline |
| 3 | `PATCH /api/owner/locations/:id/orders/:orderId/status` | PATCH | Owner JWT | WIRED | Confirm, mark ready, hand to courier status transitions |
| 4 | `POST /api/owner/locations/:id/orders/:orderId/assign` | POST | Owner JWT | WIRED | Assign courier to confirmed order |
| 5 | `POST /api/owner/manual-order` | POST | Owner JWT | STUB | Create manual order for phone-in orders |
| 6 | `GET /api/owner/locations/:id/couriers` | GET | Owner JWT | WIRED | Available couriers for assignment modal |

**WS rooms:** `location:{id}` — new orders appear in real-time, status updates

**UI data mapping:**
- Kanban columns: PENDING, CONFIRMED, PREPARING, READY, IN_DELIVERY, SCHEDULED
- Each order card: id, phone, items, total, timer, status badge, action buttons
- Drawer: customer detail, items breakdown, delivery address, timeline events
- Assign modal: courier list with status, rating, distance

---

### 2.3 Owner Menu Management — `07-admin-menu.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/categories` | GET | Owner JWT | WIRED | Category list with product count |
| 2 | `POST /api/owner/locations/:id/categories` | POST | Owner JWT | WIRED | Create category |
| 3 | `PUT /api/owner/locations/:id/categories/:catId` | PUT | Owner JWT | WIRED | Update category name, reorder |
| 4 | `DELETE /api/owner/locations/:id/categories/:catId` | DELETE | Owner JWT | WIRED | Delete category |
| 5 | `GET /api/owner/locations/:id/products` | GET | Owner JWT | WIRED | Product list with availability, price, allergens, images |
| 6 | `POST /api/owner/locations/:id/products` | POST | Owner JWT | WIRED | Create product |
| 7 | `PUT /api/owner/locations/:id/products/:prodId` | PUT | Owner JWT | WIRED | Update product (name, price, description, allergens, stock) |
| 8 | `DELETE /api/owner/locations/:id/products/:prodId` | DELETE | Owner JWT | WIRED | Delete product |
| 9 | `POST /api/owner/locations/:id/menu/import/json` | POST | Owner JWT | WIRED | Bulk import menu JSON |
| 10 | `POST /api/owner/locations/:id/menu/import/csv` | POST | Owner JWT | WIRED | Bulk import menu CSV |
| 11 | `GET /api/owner/locations/:id/menu/export/json` | GET | Owner JWT | WIRED | Export menu |
| 12 | `POST /locations/:id/menu/translate` | POST | Owner JWT | STUB | AI translate descriptions |

**WS rooms:** `location:{id}` — menu change notifications

**UI data mapping:**
- Left panel: categories list with drag-reorder, active state, item count
- Right panel: product cards with thumbnail, name, description, price, toggle (in-menu/stop-list), allergens
- Edit modal: photo upload, name, price, category selector, description, in-menu toggle, delete
- AI sparkle button → `POST /menu/translate` for AI-generated descriptions

---

### 2.4 Owner Couriers — `08-admin-couriers.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/couriers` | GET | Owner JWT | WIRED | Courier list with status, stats, current assignment, rating, GPS info |
| 2 | `POST /api/owner/locations/:id/couriers` | POST | Owner JWT | WIRED | Add new courier (name, phone) |
| 3 | `DELETE /api/owner/locations/:id/couriers/:courierId` | DELETE | Owner JWT | WIRED | Remove courier |
| 4 | `GET /api/owner/locations/:id/couriers/:courierId` | GET | Owner JWT | WIRED | Courier detail: stats, current assignment, recent orders, rating |
| 5 | `PATCH /api/owner/locations/:id/couriers/:courierId/status` | PATCH | Owner JWT | WIRED | Toggle online/busy/offline |

**WS rooms:** `location:{id}` — courier location updates, status changes

**UI data mapping:**
- Left panel: courier list with avatar initials, name, status badge, stats (orders, km), rating, GPS accuracy
- Right panel: map with courier markers + legend
- Add courier form: name, phone (must start +355)
- Detail drawer: stats grid, current assignment card, recent orders list, rating, status toggle pills

---

### 2.5 Owner Analytics — `09-admin-analytics.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/analytics` | GET | Owner JWT | STUB | Revenue, order volume, peak hours, popular items, courier performance |
| 2 | `GET /api/owner/locations/:id/dashboard` | GET | Owner JWT | WIRED | Basic stats usable for analytics too |

**WS rooms:** none

**UI data mapping:** Charts (revenue over time, order volume), top products, peak hours, customer demographics.

---

### 2.6 Owner CRM — `10-admin-crm.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/customers` | GET | Owner JWT | STUB | Customer list with order count, total spent, loyalty points |
| 2 | `GET /api/owner/locations/:id/customers/:customerId` | GET | Owner JWT | STUB | Customer detail: order history, preferences |

**WS rooms:** none

**UI data mapping:** Customer table with name, phone, orders count, total spent, last order date.

---

### 2.7 Owner Settings — `11-admin-settings.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/settings` | GET | Owner JWT | WIRED | Location settings: name, address, phone, operating hours, delivery radius, fee, payment methods |
| 2 | `PUT /api/owner/locations/:id/settings` | PUT | Owner JWT | WIRED | Update settings |
| 3 | `GET /api/owner/locations/:id/notifications` | GET | Owner JWT | WIRED | Notification preferences |
| 4 | `PUT /api/owner/locations/:id/notifications` | PUT | Owner JWT | WIRED | Update notification prefs |
| 5 | `GET /api/owner/locations/:id/settlements` | GET | Owner JWT | WIRED | Settlement/payout settings |

**WS rooms:** none

**UI data mapping:** Location details form, operating hours editor, delivery zone settings, notification toggles, bank account info.

---

### 2.8 Owner Promotions — `12-admin-promotions.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/promotions` | GET | Owner JWT | STUB | Promo codes list: code, discount %, valid period, usage count |
| 2 | `POST /api/owner/locations/:id/promotions` | POST | Owner JWT | STUB | Create promo code |
| 3 | `DELETE /api/owner/locations/:id/promotions/:promoId` | DELETE | Owner JWT | STUB | Delete promo |

**WS rooms:** none

**UI data mapping:** Promo code cards with code, discount type (% or fixed), usage limit, active dates, enable/disable toggle.

---

### 2.9 Owner AI — `13-admin-ai.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /locations/:id/menu/translate` | POST | Owner JWT | STUB | AI description generation/translation |
| 2 | `POST /api/owner/ai/suggestions` | POST | Owner JWT | STUB | AI-powered menu suggestions, pricing, category optimization |

**WS rooms:** none

**UI data mapping:** AI chat interface for menu optimization, description generation, pricing suggestions.

---

### 2.10 Owner Branding — `14-admin-branding.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `PUT /api/owner/locations/:id/theme` | PUT | Owner JWT | WIRED | Save brand theme (primary color, font, radius, logo URL) |
| 2 | `GET /api/owner/locations/:id/theme` | GET | Owner JWT | WIRED | Load current theme |
| 3 | `POST /api/owner/locations/:id/theme/logo` | POST | Owner JWT | STUB | Upload logo image |

**WS rooms:** none

**UI data mapping:** Primary color picker, font selector, radius slider, logo upload, logo preview, hero image upload. Real-time preview of brand changes.

---

### 2.11 Owner Onboarding — `17-onboarding.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /api/owner/onboarding` | POST | Owner JWT | WIRED | Create location with initial setup data |
| 2 | `GET /api/owner/onboarding/status` | GET | Owner JWT | WIRED | Check onboarding progress |

**WS rooms:** none

**UI data mapping:** Multi-step wizard: restaurant info → address → menu setup → courier setup → branding → publish.

---

### 2.12 Owner Login — `29-admin-login.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /auth/exchange` | POST | none | WIRED | Email/phone + password login. Returns owner JWT |
| 2 | `POST /auth/refresh` | POST | Bearer JWT | WIRED | Refresh token |
| 3 | `GET /auth/google` | GET | none | WIRED | Google OAuth redirect |
| 4 | `GET /auth/google/callback` | GET | none | WIRED | Google OAuth callback |

**WS rooms:** none

**UI data mapping:** Email/phone input, password, "remember me" toggle → auth exchange → JWT stored.

---

### 2.13 Owner Staff — `30-admin-staff.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/staff` | GET | Owner JWT | STUB | Staff list with roles, permissions |
| 2 | `POST /api/owner/locations/:id/staff` | POST | Owner JWT | STUB | Invite staff member |
| 3 | `DELETE /api/owner/locations/:id/staff/:staffId` | DELETE | Owner JWT | STUB | Remove staff |
| 4 | `POST /api/owner/locations/:id/invites` | POST | Owner JWT | WIRED | Send invite link |

**WS rooms:** none

**UI data mapping:** Staff list with name, role (admin/manager/staff), status, last active. Invite form.

---

### 2.14 Owner Inventory — `31-admin-inventory.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/inventory` | GET | Owner JWT | STUB | Inventory items with stock level, unit, cost |
| 2 | `PUT /api/owner/locations/:id/inventory/:itemId` | PUT | Owner JWT | STUB | Update stock level |
| 3 | `POST /api/owner/locations/:id/inventory` | POST | Owner JWT | STUB | Add inventory item |

**WS rooms:** none

**UI data mapping:** Inventory table, stock level indicators (green/yellow/red), cost tracking.

---

### 2.15 Owner Payouts — `32-admin-payouts.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/owner/locations/:id/settlements` | GET | Owner JWT | WIRED | Payout/settlement history |
| 2 | `GET /api/owner/locations/:id/payouts` | GET | Owner JWT | STUB | Payout schedule and status |

**WS rooms:** none

**UI data mapping:** Payout list with date, amount, status, bank account info.

---

### 2.16 Order WS Status Debug — `40-order-ws-status.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /orders/:id` | GET | none | WIRED | Load order for debug display |

**WS rooms:** `order:{orderId}` — live WebSocket message log

**UI data mapping:** Debug panel showing WebSocket connection status, messages received, order state changes, courier position updates.

---

### 2.17 Owner Order Card Component — `41-owner-order-card.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | (UI component showcase) | — | — | — | Reusable order card component for kanban |

**WS rooms:** none

**UI data mapping:** Order card template used in 05-admin-dashboard and 06-admin-orders.

---

## 3. Courier Surface (/courier/*)

### 3.1 Courier Tasks — `15-courier-tasks.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/courier/me` | GET | Courier JWT | WIRED | Courier name, status, current assignment |
| 2 | `GET /api/courier/me/assignments` | GET | Courier JWT | WIRED | List of pending/active assignments |
| 3 | `POST /api/courier/assignments/:id/accept` | POST | Courier JWT | WIRED | Accept a task |
| 4 | `POST /api/courier/assignments/:id/reject` | POST | Courier JWT | WIRED | Decline a task |
| 5 | `POST /api/courier/shifts/start` | POST | Courier JWT | WIRED | Start shift (go online) |
| 6 | `POST /api/courier/shifts/end` | POST | Courier JWT | WIRED | End shift (go offline) |
| 7 | `GET /api/courier/shifts` | GET | Courier JWT | WIRED | Shift history |
| 8 | `PATCH /api/courier/me/location` | PATCH | Courier JWT | WIRED | Update GPS position |
| 9 | `PATCH /api/courier/me/status` | PATCH | Courier JWT | WIRED | Toggle online/offline |

**WS rooms:** `courier:{id}` — new assignment notifications

**UI data mapping:**
- Top bar: courier name, online/offline status with indicator
- Online toggle button → `PATCH /me/status` + `POST /shifts/start|end`
- Task card: order ID, customer name, address, items, payment (cash), distance
- Detail modal: mini map, delivery address, customer contact, order items, total to collect
- Pending badge indicator
- Sound on/off toggle
- Empty state when no active tasks

---

### 3.2 Courier Active Delivery — `16-courier-delivery.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/courier/assignments/:id` | GET | Courier JWT | WIRED | Assignment detail: address, customer, items, ETA, distance |
| 2 | `POST /api/courier/assignments/:id/picked-up` | POST | Courier JWT | WIRED | Mark order as picked up |
| 3 | `POST /api/courier/assignments/:id/delivered` | POST | Courier JWT | WIRED | Mark order as delivered |
| 4 | `POST /api/courier/assignments/:id/cancel` | POST | Courier JWT | WIRED | Cancel delivery (issue) |
| 5 | `PATCH /api/courier/me/location` | PATCH | Courier JWT | WIRED | Live GPS location streaming |

**WS rooms:** `courier:{id}` — GPS position sent to server, relayed to customer + owner

**UI data mapping:**
- Status bar: GPS accuracy indicator, screen active (WakeLock)
- Map area: route line, destination pin, courier marker
- Background warning banner
- Delivery info: customer name, full address, apartment details
- Navigation buttons: Google Maps + Waze deep links
- ETA row: time, distance
- Call customer button
- Action buttons: "Picked up" → "Delivered" → success overlay
- Photo proof (optional) upload

---

### 3.3 Courier Login — `33-courier-login.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /api/courier/auth/login` | POST | none | WIRED | Courier login with phone + password |
| 2 | `POST /api/courier/auth/refresh` | POST | Courier JWT | WIRED | Refresh token |
| 3 | `POST /auth/courier/activate` | POST | none | WIRED | Activate courier account via invite code |

**WS rooms:** none

**UI data mapping:** Phone input, password, invite code (for activation).

---

### 3.4 Courier Earnings — `34-courier-earnings.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/courier/me/payouts` | GET | Courier JWT | WIRED | Payout history with amounts, dates, status |

**WS rooms:** none

**UI data mapping:** Earnings summary (today, week, month), payout list with status (pending/paid), tips breakdown.

---

### 3.5 Courier History — `35-courier-history.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /api/courier/me/assignments?status=completed` | GET | Courier JWT | WIRED | Completed delivery history |
| 2 | `GET /api/courier/shifts` | GET | Courier JWT | WIRED | Shift history with hours worked |

**WS rooms:** none

**UI data mapping:** Delivery history cards with order ID, customer, date, amount, rating. Filterable by date range.

---

### 3.6 Courier Shift — `36-courier-shift.html`

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `POST /api/courier/shifts/start` | POST | Courier JWT | WIRED | Start shift |
| 2 | `POST /api/courier/shifts/end` | POST | Courier JWT | WIRED | End shift |
| 3 | `GET /api/courier/shifts` | GET | Courier JWT | WIRED | Shift history and current shift status |

**WS rooms:** none

**UI data mapping:** Start/end shift controls, shift timer, today's stats (hours, deliveries, earnings).

---

## 4. Shared / Infrastructure Endpoints

| # | API Endpoint | Method | Auth | Status | Notes |
|---|-------------|--------|------|--------|-------|
| 1 | `GET /health` | GET | none | WIRED | Health check for all surfaces |
| 2 | `POST /api/telemetry` | POST | none | WIRED | Analytics events from all screens |
| 3 | `GET /api/push/vapid-public-key` | GET | none | WIRED | Web push subscription public key |
| 4 | `GET /robots.txt` | GET | none | WIRED | SEO |
| 5 | `GET /sitemap.xml` | GET | none | WIRED | SEO |
| 6 | `GET /s/:slug/manifest.webmanifest` | GET | none | WIRED | PWA manifest per location |

---

## 5. WebSocket Room Summary

| Room | Pattern | Used By | Purpose |
|------|---------|---------|---------|
| Order | `order:{orderId}` | 04-order-status, 40-order-ws-status | Real-time status updates, ETA changes, courier position |
| Location | `location:{id}` | 05-admin-dashboard, 06-admin-orders, 08-admin-couriers | New orders, status changes, courier GPS, alerts |
| Courier | `courier:{id}` | 15-courier-tasks, 16-courier-delivery | New assignments, GPS relay |

---

## 6. Missing Endpoints Summary

| Endpoint | Needed By | Priority |
|----------|-----------|----------|
| `GET /api/public/locations` | 00-tour-hub, 19-restaurant-discovery | HIGH |
| `GET /api/public/locations?q={query}` | 24-search-results | HIGH |
| `GET /api/public/products?q={query}` | 24-search-results | MEDIUM |
| `GET /api/customer/me` | 22-user-profile | HIGH |
| `PUT /api/customer/me` | 22-user-profile | HIGH |
| `GET /api/customer/orders` | 23-order-history | HIGH |
| `GET /api/customer/favorites` | 25-favorites | MEDIUM |
| `POST /api/customer/favorites` | 25-favorites | MEDIUM |
| `DELETE /api/customer/favorites/:id` | 25-favorites | MEDIUM |
| `POST /orders/:id/review` | 26-rate-review | MEDIUM |
| `POST /api/support/tickets` | 27-support | LOW |
| `GET /api/support/tickets` | 27-support | LOW |
| `GET /api/customer/notifications` | 39-notifications | MEDIUM |
| `PATCH /api/customer/notifications/:id/read` | 39-notifications | MEDIUM |
| `GET /api/owner/locations/:id/analytics` | 09-admin-analytics | MEDIUM |
| `GET /api/owner/locations/:id/customers` | 10-admin-crm | MEDIUM |
| `GET /api/owner/locations/:id/customers/:customerId` | 10-admin-crm | MEDIUM |
| `GET /api/owner/locations/:id/promotions` | 12-admin-promotions | LOW |
| `POST /api/owner/locations/:id/promotions` | 12-admin-promotions | LOW |
| `DELETE /api/owner/locations/:id/promotions/:promoId` | 12-admin-promotions | LOW |
| `POST /api/owner/ai/suggestions` | 13-admin-ai | LOW |
| `POST /api/owner/locations/:id/theme/logo` | 14-admin-branding | MEDIUM |
| `GET /api/owner/locations/:id/staff` | 30-admin-staff | MEDIUM |
| `POST /api/owner/locations/:id/staff` | 30-admin-staff | MEDIUM |
| `DELETE /api/owner/locations/:id/staff/:staffId` | 30-admin-staff | MEDIUM |
| `GET /api/owner/locations/:id/inventory` | 31-admin-inventory | LOW |
| `PUT /api/owner/locations/:id/inventory/:itemId` | 31-admin-inventory | LOW |
| `POST /api/owner/locations/:id/inventory` | 31-admin-inventory | LOW |
| `GET /api/owner/locations/:id/payouts` | 32-admin-payouts | MEDIUM |
| `POST /api/owner/manual-order` | 06-admin-orders | MEDIUM |

---

## 7. Endpoint Status Summary

| Status | Count | Details |
|--------|-------|---------|
| **WIRED** | ~55 | Core endpoints exist on server |
| **STUB** | ~20 | Future/post-MVP — UI exists, no endpoint yet |
| **MISSING** | ~30 | Needed but no server endpoint — needs creation |
| **TOTAL** | ~105 | All endpoints mapped across 41 screens |

---

## 8. Auth Flow Summary

| Auth Type | Token | Screens |
|-----------|-------|---------|
| None (public) | — | Menu, Cart, Checkout, Order Status (read-only), Tour, Discovery, Embed, Splash, Error, Welcome, Health |
| Customer JWT | Bearer `access_token` | Login, Register, Profile, Order History, Favorites, Rate & Review, Support, Notifications, Cancel Order, OTP, Push Subscribe |
| Courier JWT | Bearer `access_token` | Tasks, Active Delivery, Login, Earnings, History, Shift, Profile |
| Owner JWT | Bearer `access_token` | Dashboard, Orders, Menu, Couriers, Analytics, CRM, Settings, Promotions, AI, Branding, Onboarding, Login, Staff, Inventory, Payouts |

---

## 9. Key Integration Notes

1. **Cart state** is entirely client-side (localStorage). No server-side cart endpoint exists.
2. **SSR pages** (`GET /s/:slug`, cart, checkout, order-status) render the HTML shell with data injected server-side.
3. **Order placement** uses `POST /orders` with idempotency key (client-generated UUID). No auth required.
4. **WebSocket** connections are established after order placement (customer) or on dashboard load (owner) or on shift start (courier).
5. **OTP flow** is owner-toggleable (off by default). When on, `POST /api/customer/otp/verify` must succeed before `POST /orders`.
6. **Theme CSS** is served as a standalone CSS file (`/public/locations/:locationId/theme.css`) and injected into the SSR shell.
7. **Embed mode** (`?embed=true`) hides fixed elements and reports height via `postMessage`. API telemetry is called to track embed usage.
8. **Promo codes** are currently client-side mock only (`SUSHI15`, `FLAT20`). Server-side promo validation is post-MVP.
9. **Payment** is cash-only (77% of Albanian market). Card payment UI is marked "Soon" — no endpoint needed yet.
10. **Scheduled orders** are supported in checkout UI and kanban but server-side scheduling is post-MVP.
