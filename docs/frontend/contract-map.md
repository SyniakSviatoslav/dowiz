# Frontend to Backend Contract Map

This document serves as the Source of Truth for mapping UI pages/components to the actual Fastify REST endpoints and WebSocket channels present in `apps/api/src/routes`.

## 1. Client PWA (`/s/:slug`)

| Page / Component | Required Data / Action | Backend Contract (REST / WS) | Status |
| :--- | :--- | :--- | :--- |
| **Menu Page** (`01-menu-client.html`) | Fetch location menu & info | `GET /api/public/menu/:slug` | WIRED |
| | Fetch location status (open/closed) | `GET /api/public/menu/:slug/status` | WIRED |
| | Delivery Zone Check | `POST /api/public/geo/check-zone` | WIRED |
| | Theme / Brand config | `GET /api/public/theme/:slug` | WIRED |
| **Cart** (`02-cart.html`) | Validate Promo Code | `POST /api/public/promotions/validate` | MISSING |
| **Checkout** (`03-checkout.html`) | Reverse Geocoding | `POST /api/public/geocode/reverse` | MISSING |
| | Create Order (Idempotent) | `POST /api/customer/orders` | WIRED |
| | OTP Verification | `POST /api/customer/otp/send`, `POST /api/customer/otp/verify` | WIRED |
| **Order Status** (`04-order-status.html`) | Order Snapshot | `GET /api/customer/orders/:id/status` | WIRED |
| | Live Status Updates | WS: `order:{id}` | WIRED |
| | Live Courier Location | WS: `order:{id}` (embedded payload) | WIRED |
| | Push Subscription | `POST /api/customer/push/subscribe` | WIRED |

## 2. Owner Dashboard (`/admin/*`)

| Page / Component | Required Data / Action | Backend Contract (REST / WS) | Status |
| :--- | :--- | :--- | :--- |
| **Dashboard** (`05-admin-dashboard.html`) | Live Orders Snapshot | `GET /api/owner/locations/:id/dashboard/snapshot` | WIRED |
| | Live Kanban Updates | WS: `location:{id}` | WIRED |
| | Toggle Busy Mode | `PATCH /api/owner/locations/:id/status` | WIRED |
| | Fetch Alerts | `GET /api/owner/locations/:id/alerts` | WIRED |
| **Orders** (`06-admin-orders.html`) | Update Order Status | `PATCH /api/orders/:id/{confirm,assign-courier,reject,ready}` | WIRED |
| | Fetch Online Couriers | `GET /api/owner/couriers/online` | MISSING |
| **Menu** (`07-admin-menu.html`) | Fetch Menu | `GET /api/owner/categories`, `GET /api/owner/products` | WIRED |
| | Bulk Edit Availability | `PATCH /api/owner/products/bulk` | WIRED |
| | Import Menu (Wolt/Glovo) | `POST /api/owner/menu-import` | WIRED |
| | AI Translate | `POST /api/owner/menu-translate` | WIRED |
| | AI Describe | `POST /api/ai/describe-product` | MISSING |
| **Settings** (`11-admin-settings.html`) | Update Location Info | `PATCH /api/owner/locations/:id` | WIRED |
| **Branding** (`14-admin-branding.html`) | Fetch/Update Theme | `GET / PATCH /api/owner/themes/:id` | WIRED |
| **CRM / GDPR** (`10-admin-crm.html`) | Customer DB | `GET /api/owner/locations/:id/signals/customers` | WIRED |
| | Reveal Contact | `POST /api/owner/locations/:id/reveal-contact` | WIRED |
| | GDPR Erasure | `POST /api/owner/locations/:id/gdpr/erasure` | WIRED |

## 3. Courier PWA (`/courier/*`)

| Page / Component | Required Data / Action | Backend Contract (REST / WS) | Status |
| :--- | :--- | :--- | :--- |
| **Tasks** (`15-courier-tasks.html`) | Fetch Assigned Tasks | `GET /api/courier/me/tasks` | WIRED |
| | Live Task Updates | WS: `courier:{id}` | WIRED |
| | Toggle Online Status | `PATCH /api/courier/me/status` | WIRED |
| **Delivery** (`16-courier-delivery.html`)| Geo Stream | WS: `/couriers/:id/location` or REST fallback | WIRED |
| | Update Task Status | `PATCH /api/courier/assignments/:id/{pickup,deliver}` | WIRED |
| **Auth** (`33-courier-login.html`) | Courier Login | `POST /api/courier/auth/login` | WIRED |

## Notes & Rules
- `MISSING` endpoints will be stubbed with "Coming Soon" or fallback UI, as per `G1` and "Out of Scope" guidelines. We **must not** modify the backend.
- Price calculation is strictly a server-side responsibility.
- WS connections must use the unified `useWebSocket` hook with exp backoff and jitter.
