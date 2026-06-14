# packages/ui — Coverage Table

## Screens

| Screen | Module | Status | Embed Safe | Server Wired |
|--------|--------|--------|-----------|-------------|
| MenuPage | `screens/MenuPage.tsx` | ✅ DONE | ✅ | ✅ |
| CartPage | `screens/CartPage.tsx` | ✅ DONE | ✅ | ✅ |
| CheckoutPage | `screens/CheckoutPage.tsx` | ✅ DONE | ✅ | ✅ |
| OrderStatusPage | `screens/OrderStatusPage.tsx` | ✅ DONE | ✅ | ✅ |
| AdminDashboardPage | `screens/AdminDashboardPage.tsx` | ✅ DONE | ✅ | ✅ |
| AdminOrdersPage | `screens/AdminOrdersPage.tsx` | ✅ DONE | ✅ | ✅ |
| AdminMenuPage | `screens/AdminMenuPage.tsx` | ✅ DONE | ✅ | ✅ |
| AdminCouriersPage | `screens/AdminCouriersPage.tsx` | ✅ DONE | ✅ | ✅ |
| CourierTasksPage | `screens/CourierTasksPage.tsx` | ✅ DONE | ✅ | ✅ |
| CourierDeliveryPage | `screens/CourierDeliveryPage.tsx` | ✅ DONE | ✅ | ✅ |

## Components

### ClientComponents (12)

| Component | Module | Embed Safe |
|-----------|--------|-----------|
| ProductCard | `ClientComponents.tsx` | ✅ |
| CategoryNav | `ClientComponents.tsx` | ✅ |
| CartFab | `ClientComponents.tsx` | ✅ (hidden in embed) |
| SkeletonCard | `ClientComponents.tsx` | ✅ |
| EmptyState | `ClientComponents.tsx` | ✅ |
| Header | `ClientComponents.tsx` | ✅ |
| StatusTimeline | `ClientComponents.tsx` | ✅ |
| CourierCard | `ClientComponents.tsx` | ✅ |
| MiniMap | `ClientComponents.tsx` | ✅ |
| QuantityStepper | `ClientComponents.tsx` | ✅ |
| StepIndicator | `ClientComponents.tsx` | ✅ |

### AdminComponents (14)

| Component | Module | Embed Safe |
|-----------|--------|-----------|
| Sidebar | `AdminComponents.tsx` | ✅ (no fixed) |
| StatCard | `AdminComponents.tsx` | ✅ |
| OrderKanbanCard | `AdminComponents.tsx` | ✅ |
| AlertBanner | `AdminComponents.tsx` | ✅ |
| Topbar | `AdminComponents.tsx` | ✅ |
| Drawer | `AdminComponents.tsx` | ✅ (absolute in embed) |
| Modal | `AdminComponents.tsx` | ✅ (absolute in embed) |
| Table | `AdminComponents.tsx` | ✅ |
| CouriersMap | `AdminComponents.tsx` | ✅ |
| AdminShell | `AdminComponents.tsx` | ✅ |
| MobileTabBar | `AdminComponents.tsx` | ✅ (hidden in embed) |
| getStatusColor | `AdminComponents.tsx` | ✅ (pure function) |
| getStatusLabel | `AdminComponents.tsx` | ✅ (pure function) |

### CourierComponents (5)

| Component | Module | Embed Safe |
|-----------|--------|-----------|
| CourierShell | `CourierComponents.tsx` | ✅ |
| TaskCard | `CourierComponents.tsx` | ✅ |
| OnlineToggle | `CourierComponents.tsx` | ✅ |
| DeliveryDetailModal | `CourierComponents.tsx` | ✅ (absolute in embed) |

### Fallback (2)

| Component | Module | Embed Safe |
|-----------|--------|-----------|
| OfflineBanner | `Fallback.tsx` | ✅ (sticky in embed) |
| ErrorBoundary | `Fallback.tsx` | ✅ |

## Hooks (3)

| Hook | Module | Tests |
|------|--------|-------|
| useOnlineStatus | `hooks/use-online.ts` | ❌ None |
| useCart | `hooks/use-cart.ts` | ❌ None |
| useEmbed | `hooks/use-embed.ts` | ❌ None |

## Lib (6)

| Module | Purpose | Tests |
|--------|---------|-------|
| `lib/api-client.ts` | HTTP client with idempotency | ❌ None |
| `lib/websocket.ts` | WS client with reconnect | ❌ None |
| `lib/auth.ts` | Auth state + localStorage | ❌ None |
| `lib/theme.ts` | Brand presets + SSR inject | ❌ None |
| `lib/i18n.ts` | Locale strings (sq/en) | ❌ None |

## Utils (2)

| Module | Purpose | Tests |
|--------|---------|-------|
| `utils/index.ts` | Format, parse, calc helpers | ❌ None |
| `utils/delivery-zone.ts` | Geo delivery zone check | ❌ None |
| `utils/embed.ts` | isEmbedMode, posFixed | ❌ None |

## HTML Screens (41)

All 41 `.html` screens in `src/screens/` have embed mode support (`?embed=true`).

- 15 already had it before Stage 5
- 22 were updated in Stage 5
- 4 (18-embed-demo.html) is the embed demo itself

## Server Contract

See `docs/integration/contract-map.md` for detailed endpoint coverage:
- 109 endpoints WIRED
- 30 endpoints STUB (post-MVP)
- 4 endpoints MISSING (needs server work)
