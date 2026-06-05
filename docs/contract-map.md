# DeliveryOS Contract Map

Complete mapping of all server API endpoints to their Zod contract schemas in `@deliveryos/shared-types`.

## Legend
- `→` = request body schema
- `←` = response body schema
- `⇄` = both request and response

---

## Public (no auth)

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/s/:slug` | `SSRQueryParams` | HTML | `public/ssr.ts` |
| GET | `/s/:slug/cart` | — | HTML shell | `public/client-flow.ts` |
| GET | `/s/:slug/checkout` | — | HTML shell | `public/client-flow.ts` |
| GET | `/s/:slug/orders/:orderId` | — | HTML shell | `public/client-flow.ts` |
| GET | `/s/:slug/manifest.webmanifest` | — | JSON manifest | `public/pwa.ts` |
| GET | `/public/locations/:locationIdOrSlug/menu` | `locale?` (qry) | `PublicMenuResponse` | `public/menu.ts` |
| GET | `/public/locations/:slug/fallback-config` | — | `FallbackConfig` | `public/fallback-config.ts` |
| GET | `/public/locations/:locationId/theme.css` | `hash?` (qry) | CSS | `public/theme.ts` |
| GET | `/api/push/vapid-public-key` | — | `{ publicKey: string }` | `public/vapid.ts` |
| POST | `/api/telemetry` | `TelemetryBody` | `TelemetryResponse` | `public/telemetry.ts` |
| POST | `/api/orders` | `CreateOrderInput` | `OrderResponse` | `orders.ts` |
| GET | `/api/orders/:id` | — | `OrderResponse` | `orders.ts` |

---

## Customer (prefix: `/api/customer`)

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| POST | `/:slug/otp/send` | `SendOTPBody` | `SendOTPResponse` | `customer/otp.ts` |
| POST | `/:slug/otp/verify` | `VerifyOTPBody` | `VerifyOTPResponse` | `customer/otp.ts` |
| POST | `/orders/:orderId/cancel` | `CancelOrderBody` | `OrderResponse` | `customer/orders.ts` |
| POST | `/push/subscribe` | `PushSubscriptionBody` | `PushResponse` | `customer/push.ts` |
| POST | `/push/unsubscribe` | — | `PushResponse` | `customer/push.ts` |

---

## Owner (prefix: `/api/owner/locations/:locationId`)

### Dashboard & Orders

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/dashboard/snapshot` | — | `DashboardSnapshotResponse` | `owner/dashboard.ts` |
| POST | `/orders/:orderId/confirm` | — | `ConfirmOrderResponse` | `owner/dashboard.ts` |
| POST | `/orders/:orderId/reject` | `RejectOrderBody` | `ConfirmOrderResponse` | `owner/dashboard.ts` |
| POST | `/orders/:orderId/assign-courier` | `AssignCourierBody` | `AssignCourierResponse` | `owner/dashboard.ts` |
| PATCH | `/orders/:orderId/metadata` | `UpdateMetadataBody` | — | `owner/order-meta.ts` |
| POST | `/orders/:orderId/reveal-customer-contact` | — | `RevealContactResponse` | `owner/reveal-contact.ts` |
| POST | `/orders/:orderId/mark-no-show` | — | `MarkNoShowResponse` | `owner/signals.ts` |

### Menu Management

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/categories` | — | `CategoryResponse[]` | `owner/categories.ts` |
| POST | `/categories` | `CreateCategoryBody` | `CategoryResponse` | `owner/categories.ts` |
| PATCH | `/categories/:id` | `UpdateCategoryBody` | `CategoryResponse` | `owner/categories.ts` |
| DELETE | `/categories/:id` | — | — | `owner/categories.ts` |
| GET | `/products` | `category_id?` (qry) | `ProductResponse[]` | `owner/products.ts` |
| POST | `/products` | `CreateProductBody` | `ProductResponse` | `owner/products.ts` |
| PATCH | `/products/:id` | `UpdateProductBody` | `ProductResponse` | `owner/products.ts` |
| DELETE | `/products/:id` | — | — | `owner/products.ts` |
| GET | `/products/:id/translations/:locale` | — | `ProductTranslationResponse` | `owner/products.ts` |
| PUT | `/products/:id/translations/:locale` | `ProductTranslationBody` | `ProductTranslationResponse` | `owner/products.ts` |
| DELETE | `/products/:id/translations/:locale` | — | — | `owner/products.ts` |
| GET | `/modifier-groups` | — | `ModifierGroupResponse[]` | `owner/modifiers.ts` |
| POST | `/modifier-groups` | `CreateModifierGroupBody` | `ModifierGroupResponse` | `owner/modifiers.ts` |
| PATCH | `/modifier-groups/:id` | `UpdateModifierGroupBody` | `ModifierGroupResponse` | `owner/modifiers.ts` |
| DELETE | `/modifier-groups/:id` | — | — | `owner/modifiers.ts` |
| POST | `/modifier-groups/:groupId/modifiers` | `CreateModifierBody` | `ModifierResponse` | `owner/modifiers.ts` |
| PATCH | `/modifiers/:id` | `UpdateModifierBody` | `ModifierResponse` | `owner/modifiers.ts` |
| DELETE | `/modifiers/:id` | — | — | `owner/modifiers.ts` |

### Couriers

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/couriers` | — | `CourierListResponse` | `owner/couriers.ts` |
| PATCH | `/couriers/:courierId` | `UpdateCourierBody` | — | `owner/couriers.ts` |
| GET | `/couriers/live` | — | `LiveCourierResponse` | `owner/couriers.ts` |
| POST | `/courier-invites` | — | — | `owner/courier-invites.ts` |
| GET | `/courier-invites` | — | — | `owner/courier-invites.ts` |
| DELETE | `/courier-invites/:inviteId` | — | — | `owner/courier-invites.ts` |

### Location & Settings

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| PATCH | `/` | `UpdateLocationBody` | `LocationResponse` | `owner/location.ts` |
| GET | `/settings/dwell` | — | `DwellSettingsResponse` | `owner/dwell-settings.ts` |
| PUT | `/settings/dwell` | `UpdateDwellSettingsBody` | `DwellSettingsResponse` | `owner/dwell-settings.ts` |
| GET | `/settings/fallback` | — | `FallbackSettingsResponse` | `owner/fallback.ts` |
| PUT | `/settings/fallback` | `UpdateFallbackSettingsBody` | `FallbackSettingsResponse` | `owner/fallback.ts` |
| GET | `/degradation` | — | `DegradationStatus` | `owner/fallback.ts` |
| GET | `/settings/retention` | — | `RetentionSettings` | `owner/gdpr.ts` |
| PUT | `/settings/retention` | `UpdateRetentionBody` | `RetentionSettings` | `owner/gdpr.ts` |

### Branding

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/theme` | — | `ThemeFullResponse` | `owner/branding.ts` |
| PUT | `/theme` | `UpdateThemeBody` | `UpdateThemeResponse` | `owner/branding.ts` |
| POST | `/theme/logo` | multipart | `LogoUploadResponse` | `owner/branding.ts` |

### Signals & Alerts

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/signals` | — | `SignalListResponse` | `owner/signals.ts` |
| GET | `/signals/compute` | `ComputeSignalQuery` | — | `owner/signals.ts` |
| POST | `/signals/:signalId/acknowledge` | — | `AcknowledgeSignalResponse` | `owner/signals.ts` |
| POST | `/signals/:signalId/dismiss` | `DismissSignalBody` | `DismissSignalResponse` | `owner/signals.ts` |
| GET | `/alerts` | — | `AlertListResponse` | `owner/alerts.ts` |
| POST | `/alerts/:alertId/acknowledge` | — | `AcknowledgeAlertResponse` | `owner/alerts.ts` |
| POST | `/alerts/acknowledge-all` | `AcknowledgeAllAlertsBody` | — | `owner/alerts.ts` |

### Settlements & GDPR

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/settlements` | — | `SettlementListResponse` | `owner/settlements.ts` |
| GET | `/settlements/:id` | — | — | `owner/settlements.ts` |
| POST | `/settlements/:id/approve` | — | — | `owner/settlements.ts` |
| POST | `/settlements/:id/pay` | `PaySettlementBody` | — | `owner/settlements.ts` |
| POST | `/settlements/:id/dispute` | `DisputeSettlementBody` | — | `owner/settlements.ts` |
| POST | `/settlements/:id/reopen` | `ReopenSettlementBody` | — | `owner/settlements.ts` |
| POST | `/settlements/regenerate` | `RegenerateSettlementsBody` | — | `owner/settlements.ts` |
| POST | `/gdpr-requests` | `CreateGDPRRequest` | `CreateGDPRResponse` | `owner/gdpr.ts` |
| GET | `/gdpr-requests` | — | `GDPRRequestListResponse` | `owner/gdpr.ts` |
| GET | `/gdpr-requests/:requestId` | — | — | `owner/gdpr.ts` |

### Notifications

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| GET | `/notifications/targets` | — | — | `owner/notifications.ts` |
| POST | `/telegram/connect-init` | — | — | `owner/notifications.ts` |
| POST | `/notifications/test` | — | — | `owner/notifications.ts` |
| PATCH | `/targets/:targetId` | — | — | `owner/notifications.ts` |
| POST | `/push/subscribe` | — | — | `owner/push.ts` |
| POST | `/push/unsubscribe` | — | — | `owner/push.ts` |
| GET | `/push/state` | — | — | `owner/push.ts` |

---

## Courier (prefix: `/api/courier`)

| Method | Path | Request | Response | Source File |
|--------|------|---------|----------|------------|
| POST | `/invites/:inviteId/redeem` | `CourierInviteRedeemBody` | `CourierInviteRedeemResponse` | `courier/auth.ts` |
| POST | `/login` | `CourierLoginBody` | `CourierLoginResponse` | `courier/auth.ts` |
| POST | `/refresh` | `CourierRefreshBody` | `CourierRefreshResponse` | `courier/auth.ts` |
| POST | `/logout` | `CourierLogoutBody` | — | `courier/auth.ts` |
| GET | `/me` | — | `CourierProfile` | `courier/tasks.ts` |
| GET | `/me/assignments` | — | `AssignmentListResponse` | `courier/tasks.ts` |
| POST | `/assignments/:id/accept` | — | `AcceptRejectResponse` | `courier/tasks.ts` |
| POST | `/assignments/:id/reject` | — | `AcceptRejectResponse` | `courier/tasks.ts` |
| POST | `/assignments/:id/picked-up` | — | `PickupDeliverResponse` | `courier/delivery.ts` |
| POST | `/assignments/:id/delivered` | `{ cash_collected, cash_amount? }` | `PickupDeliverResponse` | `courier/delivery.ts` |
| POST | `/assignments/:id/cancel` | `{ reason }` | — | `courier/delivery.ts` |
| POST | `/shifts/transition` | `CourierStatusBody` | — | `courier/delivery.ts` |
| POST | `/shifts/ping` | `CourierLocationBody` | — | `courier/delivery.ts` |
| GET | `/me/payouts` | `status?` (qry) | `CourierEarningsResponse` | `courier/delivery.ts` |
| GET | `/me/payouts/:id` | — | — | `courier/delivery.ts` |
| GET | `/me/audit-log` | — | `CourierAuditLogResponse` | `courier/tasks.ts` |
| PATCH | `/me/password` | `ChangePasswordBody` | — | `courier/tasks.ts` |

---

## Common Schemas (reused across endpoints)

| Schema | Used By | Source |
|--------|---------|--------|
| `GeoPoint` | delivery, location, OTP | `common/geo.ts` |
| `DeliveryPolygon` | location settings | `common/geo.ts` |
| `BusinessHours` | location settings | `common/geo.ts` |
| `CursorPagination` | list endpoints | `common/pagination.ts` |
| `CursorResponse` | paginated responses | `common/pagination.ts` |
| `ErrorResponse` | all error responses | `common/pagination.ts` |
| `FallbackConfig` | public + owner | `common/geo.ts` |
| `OrderStatusEnum` | orders, assignments | `legacy.ts` |
| `OrderResponse` | order creation, retrieval | `legacy.ts` |
| `CreateOrderInput` | order creation | `legacy.ts` |
| `AuthToken` | JWT payload (discriminated union) | `legacy.ts` |

---

## Contract Schema File Index

| File | Exports |
|------|---------|
| `common/geo.ts` | `GeoPoint`, `DeliveryPolygon`, `BusinessHours`, `CursorPagination`, `FallbackConfig` |
| `common/pagination.ts` | `CursorResponse`, `ErrorResponse` |
| `public/menu.ts` | `PublicProduct`, `PublicCategory`, `PublicLocation`, `PublicMenuResponse` |
| `public/ssr.ts` | `SSRQueryParams` |
| `public/telemetry.ts` | `TELEMETRY_ACTIONS`, `TelemetryBody`, `TelemetryResponse` |
| `customer/otp.ts` | `SendOTPBody`, `SendOTPResponse`, `VerifyOTPBody`, `VerifyOTPResponse` |
| `customer/orders.ts` | `CancelOrderBody` |
| `customer/push.ts` | `PushSubscriptionBody`, `PushResponse` |
| `owner/dashboard.ts` | `DashboardCounts`, `ActiveOrderSummary`, `DashboardSnapshotResponse` |
| `owner/orders.ts` | `ConfirmOrderResponse`, `RejectOrderBody`, `AssignCourierBody`, `AssignCourierResponse`, `UpdateMetadataBody`, `RevealContactResponse`, `MarkNoShowResponse` |
| `owner/categories.ts` | `CreateCategoryBody`, `UpdateCategoryBody`, `CategoryResponse` |
| `owner/products.ts` | `CreateProductBody`, `UpdateProductBody`, `ProductResponse`, `ProductTranslationBody`, `ProductTranslationResponse` |
| `owner/modifiers.ts` | `CreateModifierGroupBody`, `UpdateModifierGroupBody`, `ModifierGroupResponse`, `CreateModifierBody`, `UpdateModifierBody`, `ModifierResponse` |
| `owner/couriers.ts` | `CourierListItem`, `CourierListResponse`, `UpdateCourierBody`, `LiveCourier`, `LiveCourierResponse` |
| `owner/location.ts` | `UpdateLocationBody`, `LocationResponse` |
| `owner/settings.ts` | `DwellThresholds`, `DwellSettingsResponse`, `UpdateDwellSettingsBody`, `RetentionSettings`, `UpdateRetentionBody`, `FallbackSettingsResponse`, `UpdateFallbackSettingsBody` |
| `owner/branding.ts` | `UpdateThemeBody`, `ThemeResponse`, `ThemeFullResponse`, `UpdateThemeResponse`, `LogoUploadResponse` |
| `owner/signals.ts` | `SignalItem`, `SignalListResponse`, `ComputeSignalQuery`, `AcknowledgeSignalResponse`, `DismissSignalBody`, `DismissSignalResponse` |
| `owner/alerts.ts` | `AlertItem`, `AlertListResponse`, `AcknowledgeAlertResponse`, `AcknowledgeAllAlertsBody` |
| `owner/settlements.ts` | `SettlementItem`, `SettlementListResponse`, `PaySettlementBody`, `DisputeSettlementBody`, `ReopenSettlementBody`, `RegenerateSettlementsBody` |
| `owner/gdpr.ts` | `CreateGDPRRequest`, `GDPRRequestItem`, `GDPRRequestListResponse`, `CreateGDPRResponse` |
| `courier/auth.ts` | `CourierLoginBody`, `CourierLoginResponse`, `CourierRefreshBody`, `CourierRefreshResponse`, `CourierLogoutBody`, `CourierInviteRedeemBody`, `CourierInviteRedeemResponse` |
| `courier/tasks.ts` | `CourierProfile`, `CourierAssignment`, `AssignmentListResponse`, `AcceptRejectResponse`, `CourierShift`, `CourierShiftResponse`, `CourierAuditLogResponse`, `ChangePasswordBody` |
| `courier/delivery.ts` | `CourierLocationBody`, `CourierStatusBody`, `PickupDeliverResponse`, `CourierEarningsResponse` |
| `legacy.ts` | `CreateOrderInput`, `OrderItemInput`, `StatusUpdateInput`, `OrderResponse`, `OrderItemResponse`, `OrderStatusEnum`, `AuthToken` |
