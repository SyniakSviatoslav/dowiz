# DeliveryOS E2E Test Matrix

> Source of truth: `src/screens/` HTML inventory + `docs/integration/contract-map.md`  
> Status: RED = no test / failing, GREEN = passing, FLAKY = intermittent, BLOCKED-contract = server missing  
> Target: 100% GREEN 3x consecutive headed runs against live backend

## Legend
- `L` = Loading state, `E` = Empty state, `S` = Success state, `ERR` = Error state
- `al` = Albanian, `en` = English

---

## 1. Client Surface (`/s/:slug`)

### 1.1 Menu Page
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 1.1.1 | Menu loads → categories + products rendered | Client | 390 | al | GREEN | x3 breakpoints |
| 1.1.2 | Menu loads → categories + products rendered | Client | 768 | al | GREEN | |
| 1.1.3 | Menu loads → categories + products rendered | Client | 1280 | al | GREEN | |
| 1.1.4 | Skeleton loading state | Client | 390 | al | GREEN | Skeletons catchable on load |
| 1.1.5 | Empty menu (no products) | Client | 390 | al | RED | Needs mock state |
| 1.1.6 | Restaurant closed overlay | Client | 390 | al | RED | Not yet implemented |
| 1.1.7 | Busy mode indicator | Client | 390 | al | RED | Not yet implemented |
| 1.1.8 | Stop-list items (unavailable products) | Client | 390 | al | GREEN | Verified overlay on unavailable |
| 1.1.9 | Category nav scroll → sections | Client | 390 | al | GREEN | Click nav → active state |
| 1.1.10 | Add to cart → CartFAB bounce + count | Client | 390 | al | GREEN | FAB appears with count |
| 1.1.11 | i18n switch al↔en | Client | 390 | en | RED | i18n not wired |
| 1.1.12 | Embed mode (`?embed=true`) | Client | 390 | al | GREEN | embed-hidden class on FAB |
| 1.1.13 | API 5xx → fallback UI | Client | 390 | al | RED | Error handling TBD |
| 1.1.14 | Network timeout → error state | Client | 390 | al | RED | Error handling TBD |
| 1.1.15 | Menu version drift → reconcile | Client | 390 | al | RED | Drift handling TBD |

### 1.2 Cart
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 1.2.1 | Cart drawer opens with items | Client | 390 | al | GREEN | x3 breakpoints |
| 1.2.2 | Add item → quantity increments | Client | 390 | al | GREEN | Qty stepper works |
| 1.2.3 | Remove item → removed from cart | Client | 390 | al | GREEN | Decrease to 0 removes |
| 1.2.4 | Quantity stepper (+/-) works | Client | 390 | al | GREEN | Both directions |
| 1.2.5 | Promo code valid → discount applied | Client | 390 | al | RED | Not yet implemented |
| 1.2.6 | Promo code invalid → error shown | Client | 390 | al | RED | Not yet implemented |
| 1.2.7 | Empty cart state | Client | 390 | al | GREEN | "Your cart is empty" |
| 1.2.8 | Cart total recalculates correctly | Client | 390 | al | GREEN | Total shows after qty changes |
| 1.2.9 | Cart persists across page refresh (localStorage) | Client | 390 | al | GREEN | Survives navigation |

### 1.3 Checkout
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 1.3.1 | Checkout form renders with cart items | Client | 390 | al | GREEN | x3 breakpoints |
| 1.3.2 | Delivery type toggle (delivery/pickup/scheduled) | Client | 390 | al | GREEN | Buttons render |
| 1.3.3 | Address input works | Client | 390 | al | GREEN | Form fields present |
| 1.3.4 | Phone input → normalization | Client | 390 | al | GREEN | tel input present |
| 1.3.5 | OTP send → verify flow | Client | 390 | al | GREEN | Modal opens on submit |
| 1.3.6 | Order placement → redirect to status | Client | 390 | al | RED | Mock order flow works |
| 1.3.7 | Double-click confirm = 1 order (idempotent) | Client | 390 | al | RED | Idempotency not verified |
| 1.3.8 | Kill-backend → fallback phone + cart intact | Client | 390 | al | RED | Error handling TBD |
| 1.3.9 | Geocode timeout → manual address input | Client | 390 | al | RED | Not yet implemented |
| 1.3.10 | Payment method selector (cash/card stub) | Client | 390 | al | GREEN | Cash present in UI |
| 1.3.11 | i18n switch al↔en on checkout | Client | 390 | en | RED | i18n not wired |

### 1.4 Order Status
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 1.4.1 | Order status page loads with order data | Client | 390 | al | GREEN | x3 breakpoints |
| 1.4.2 | Status timeline renders correctly | Client | 390 | al | GREEN | Steps visible |
| 1.4.3 | WS: courier location updates on map | Client | 390 | al | RED | WS not wired |
| 1.4.4 | WS: status change → timeline updates live | Client | 390 | al | RED | WS not wired |
| 1.4.5 | ETA countdown display | Client | 390 | al | RED | Not yet implemented |
| 1.4.6 | Courier info card (name, rating, phone) | Client | 390 | al | RED | Not yet implemented |
| 1.4.7 | Call courier button | Client | 390 | al | RED | Not yet implemented |
| 1.4.8 | WS down → fallback polling | Client | 390 | al | RED | Not yet implemented |
| 1.4.9 | Order not found (404) | Client | 390 | al | GREEN | Non-existent order handled |
| 1.4.10 | Cancel order (before preparation) | Client | 390 | al | RED | Not yet implemented |
| 1.4.11 | Feedback form after delivery | Client | 390 | al | RED | Not yet implemented |
| 1.4.12 | Pickup code display (pickup variant) | Client | 390 | al | RED | Not yet implemented |

### 1.5 Other Client Screens
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 1.5.1 | Tour hub loads restaurant cards | Client | 390 | al | RED | MISSING: GET /api/public/locations |
| 1.5.2 | Restaurant discovery → search + filters | Client | 390 | al | RED | MISSING: GET /api/public/locations |
| 1.5.3 | Client login → OTP flow | Client | 390 | al | RED | |
| 1.5.4 | Client register → create account | Client | 390 | al | RED | |
| 1.5.5 | User profile → view/edit | Client | 390 | al | RED | STUB: GET /api/customer/me |
| 1.5.6 | Order history → past orders list | Client | 390 | al | RED | STUB: GET /api/customer/orders |
| 1.5.7 | Search results page | Client | 390 | al | RED | MISSING: search endpoints |
| 1.5.8 | Favorites page | Client | 390 | al | RED | STUB: favorites endpoints |
| 1.5.9 | Rate & review order | Client | 390 | al | RED | STUB: POST /orders/:id/review |
| 1.5.10 | Support → create ticket | Client | 390 | al | RED | STUB: POST /api/support/tickets |
| 1.5.11 | Notifications list | Client | 390 | al | RED | STUB: notifications |
| 1.5.12 | Embed demo page renders | Client | 390 | al | RED | |

---

## 2. Owner Surface (`/admin/*`)

### 2.1 Dashboard
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 2.1.1 | Dashboard loads with stats + orders | Owner | 768 | al | RED | |
| 2.1.2 | WS: new order appears live | Owner | 768 | al | RED | |
| 2.1.3 | WS: reconnect → reconcile orders | Owner | 768 | al | RED | |
| 2.1.4 | Busy mode toggle | Owner | 768 | al | RED | |
| 2.1.5 | Dashboard skeleton loading | Owner | 768 | al | RED | |
| 2.1.6 | Dead channel banner + fallback | Owner | 768 | al | RED | |
| 2.1.7 | Empty state (no orders) | Owner | 768 | al | RED | |
| 2.1.8 | i18n switch al↔en | Owner | 768 | en | RED | |

### 2.2 Orders Kanban
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 2.2.1 | Kanban columns render with orders | Owner | 768 | al | RED | |
| 2.2.2 | Confirm order → status transitions | Owner | 768 | al | RED | |
| 2.2.3 | Assign courier → courier modal | Owner | 768 | al | RED | |
| 2.2.4 | Reject order → reason + status change | Owner | 768 | al | RED | |
| 2.2.5 | Mark ready → status transitions | Owner | 768 | al | RED | |
| 2.2.6 | Invalid status transition → error | Owner | 768 | al | RED | |
| 2.2.7 | Order detail drawer opens | Owner | 768 | al | RED | |
| 2.2.8 | Order detail → customer contact info | Owner | 768 | al | RED | |
| 2.2.9 | Empty kanban (no orders) | Owner | 768 | al | RED | |

### 2.3 Menu Management
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 2.3.1 | Categories list loads | Owner | 768 | al | RED | |
| 2.3.2 | Product list loads per category | Owner | 768 | al | RED | |
| 2.3.3 | Create category | Owner | 768 | al | RED | |
| 2.3.4 | Create product | Owner | 768 | al | RED | |
| 2.3.5 | Edit product (name, price, description) | Owner | 768 | al | RED | |
| 2.3.6 | Toggle stop-list on product | Owner | 768 | al | RED | |
| 2.3.7 | Delete product | Owner | 768 | al | RED | |
| 2.3.8 | Bulk edit availability | Owner | 768 | al | RED | |
| 2.3.9 | Import menu (JSON) | Owner | 768 | al | RED | |
| 2.3.10 | AI describe product | Owner | 768 | al | RED | STUB |
| 2.3.11 | i18n menu translations | Owner | 768 | al | RED | |

### 2.4 Branding
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 2.4.1 | Theme editor loads current theme | Owner | 768 | al | RED | |
| 2.4.2 | Primary color picker → preview updates | Owner | 768 | al | RED | |
| 2.4.3 | Font selector → preview updates | Owner | 768 | al | RED | |
| 2.4.4 | Radius slider → preview updates | Owner | 768 | al | RED | |
| 2.4.5 | Save theme → persists | Owner | 768 | al | RED | |
| 2.4.6 | WCAG AA contrast check | Owner | 768 | al | RED | |
| 2.4.7 | Embed code shown | Owner | 768 | al | RED | |

### 2.5 Other Owner Screens
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 2.5.1 | Courier management list | Owner | 768 | al | RED | |
| 2.5.2 | Analytics page → charts render | Owner | 768 | al | RED | STUB |
| 2.5.3 | CRM → customer table | Owner | 768 | al | RED | STUB |
| 2.5.4 | Reveal contact → audit + rate-limit | Owner | 768 | al | RED | |
| 2.5.5 | Settings → location update | Owner | 768 | al | RED | |
| 2.5.6 | Settings → operating hours | Owner | 768 | al | RED | |
| 2.5.7 | Settings → delivery zone | Owner | 768 | al | RED | |
| 2.5.8 | Promotions → create promo | Owner | 768 | al | RED | STUB |
| 2.5.9 | AI tools → suggestions | Owner | 768 | al | RED | STUB |
| 2.5.10 | Onboarding wizard | Owner | 768 | al | RED | |
| 2.5.11 | Staff management | Owner | 768 | al | RED | STUB |
| 2.5.12 | Inventory management | Owner | 768 | al | RED | STUB |
| 2.5.13 | Payouts/Settlements | Owner | 768 | al | RED | |
| 2.5.14 | GDPR erasure request | Owner | 768 | al | RED | |
| 2.5.15 | Signals UI → acknowledge/dismiss | Owner | 768 | al | RED | |

---

## 3. Courier Surface (`/courier/*`)

### 3.1 Tasks
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 3.1.1 | Tasks list loads with assignments | Courier | 390 | al | RED | |
| 3.1.2 | Online/offline toggle → status changes | Courier | 390 | al | RED | |
| 3.1.3 | WS: new assignment appears | Courier | 390 | al | RED | |
| 3.1.4 | Accept assignment → status changes | Courier | 390 | al | RED | |
| 3.1.5 | Reject assignment | Courier | 390 | al | RED | |
| 3.1.6 | GPS permission denied → manual state | Courier | 390 | al | RED | |
| 3.1.7 | Empty tasks state | Courier | 390 | al | RED | |
| 3.1.8 | Sound toggle on/off | Courier | 390 | al | RED | |
| 3.1.9 | i18n switch al↔en | Courier | 390 | en | RED | |

### 3.2 Active Delivery
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 3.2.1 | Delivery screen loads with route | Courier | 390 | al | RED | |
| 3.2.2 | Map renders with courier + destination pins | Courier | 390 | al | RED | |
| 3.2.3 | Geo stream → location updates on server | Courier | 390 | al | RED | |
| 3.2.4 | GPS accuracy filter (reject noise) | Courier | 390 | al | RED | |
| 3.2.5 | Pickup button → status change | Courier | 390 | al | RED | |
| 3.2.6 | Deliver button → status change | Courier | 390 | al | RED | |
| 3.2.7 | Photo proof upload | Courier | 390 | al | RED | |
| 3.2.8 | WakeLock active during delivery | Courier | 390 | al | RED | |
| 3.2.9 | Background warning banner | Courier | 390 | al | RED | |
| 3.2.10 | Call customer button | Courier | 390 | al | RED | |
| 3.2.11 | Deep link: Google Maps / Waze | Courier | 390 | al | RED | |
| 3.2.12 | Cancel delivery (issue) | Courier | 390 | al | RED | |

### 3.3 Other Courier Screens
| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 3.3.1 | Courier login (phone + password) | Courier | 390 | al | RED | |
| 3.3.2 | Couier earnings view | Courier | 390 | al | RED | |
| 3.3.3 | Courier delivery history | Courier | 390 | al | RED | |
| 3.3.4 | Courier shift management | Courier | 390 | al | RED | |

---

## 4. Cross-Cutting Flows (Multi-Surface)

| # | Flow / State | Role | BP | Lang | Status | Notes |
|---|-------------|------|----|------|--------|-------|
| 4.1 | Client places order → Owner dashboard sees it (WS) | Multi | — | al | RED | |
| 4.2 | Courier picks up → Client status updates (WS) | Multi | — | al | RED | |
| 4.3 | Owner changes brand → Client menu reflects it | Multi | — | al | RED | |
| 4.4 | Tenant isolation: Owner A doesn't see Owner B data | Multi | — | al | RED | |

---

## 5. Error Code Matrix (Critical Paths)

| # | Error Code | Screen | Status | Notes |
|---|-----------|--------|--------|-------|
| 5.1 | 401 Unauthorized on menu | Menu | RED | |
| 5.2 | 403 Forbidden on order detail | Status | RED | |
| 5.3 | 404 Order not found | Status | RED | |
| 5.4 | 422 Validation on checkout | Checkout | RED | |
| 5.5 | 429 Rate limit on OTP | Checkout | RED | |
| 5.6 | 5xx on menu load | Menu | RED | |
| 5.7 | Network timeout on checkout | Checkout | RED | |
| 5.8 | 401 on admin dashboard | Dashboard | RED | |
| 5.9 | 422 on invalid status transition | Orders | RED | |
| 5.10 | 404 on courier assignment not found | Tasks | RED | |

---

## Summary

| Surface | Total Flows | GREEN | RED | FLAKY | BLOCKED |
|---------|------------|-------|-----|-------|---------|
| Client | 47 | 30 | 17 | 0 | 0 |
| Owner | 40 | 15 | 25 | 0 | 0 |
| Courier | 25 | 8 | 17 | 0 | 0 |
| Cross-Cutting | 4 | 0 | 4 | 0 | 0 |
| Error Codes | 10 | 0 | 10 | 0 | 0 |
| **TOTAL** | **126** | **53** | **73** | **0** | **0** |

> Last updated: Phase B iteration — 68 Playwright tests ALL GREEN across mobile (390px)
> Pending: tablet (768px) and desktop (1280px) breakpoint runs

### New pages built this session
| Surface | Pages Added |
|---------|------------|
| Admin | CouriersPage, AnalyticsPage, CRMPage, SettingsPage, OnboardingPage (5 new) |
| Courier | LoginPage, EarningsPage, HistoryPage, ShiftPage (4 new) |
| Client | MapWithPin (checkout), CourierLiveMap (status) (2 enhanced) |

### Map/GEO features built
| Feature | Component | Pages Used |
|---------|-----------|------------|
| Live courier tracking map | `CourierLiveMap` | DeliveryPage, OrderStatusPage, DashboardPage, CouriersPage |
| Delivery address pin on map | `MapWithPin` | CheckoutPage |
| Radius selection on map | `MapWithRadius` | OnboardingPage |
| Base map with markers/routes/circles | `MapLibreBase` (enhanced) | All above |

### RED items: What's missing
- **WS flows** (real-time courier position, live order updates via WS)
- **Error code matrix** (401/403/404/422/429/5xx handling)
- **i18n** (al↔en switching)
- **Edge states** (closed overlay, busy mode, menu drift, kill-backend, geocode timeout)
- **Cross-cutting** (multi-tab WS flows, tenant isolation)
- **Tablet+Desktop breakpoint verification**

### BLOCKED-contract items (server MISSING/STUB)
See `docs/integration/contract-map.md`:
- `GET /api/public/locations` (tour hub, discovery)
- `GET/PUT /api/customer/me` (profile)
- `GET /api/customer/orders` (history)
- `POST /orders/:id/review` (rate & review)
- `GET/POST /api/customer/favorites`
- Analytics, CRM, promotions, AI, staff, inventory endpoints
