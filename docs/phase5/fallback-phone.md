# Fallback Phone & Degradation (Stage 33)

## Overview
Graceful degradation when external services fail. Every customer-facing error path shows a fallback phone banner instead of a blank page or silent failure.

## Architecture

### Migration: `fallback_config` JSONB
- **Table**: `locations`
- **Column**: `fallback_config` (JSONB, NOT NULL DEFAULT '{}')
- **Keys**:
  - `phone` (text) — override for `locations.public_phone`
  - `show_phone_on_error` (bool, default true)
  - `show_phone_on_offline` (bool, default true)
  - `ws_retry_max` (int, default 10)
  - `ws_retry_base_ms` (int, default 2000)

### Customer Error Paths
| Path | Fallback Trigger | Mechanism |
|------|-----------------|-----------|
| JWT expired | WS close code 1008 | `fallback:needed` event → `showFallbackBanner` |
| WS offline | Max reconnects exceeded | `fallback:needed` event → `showFallbackBanner` |
| POST /api/orders 5xx | Server error response | `fallback:needed` event → `showFallbackBanner` |
| POST /api/orders network error | Fetch throws | `fallback:needed` event → `showFallbackBanner` |
| Geolocation fail | Permission denied / timeout | `fallback:needed` event → `showFallbackBanner` |
| Any unhandled error | `window.onerror` / `unhandledrejection` | Error boundary → `showFallbackBanner` |

### Owner Dead-Channel Detection
- Query `owner_notification_targets` for active targets with `last_error IS NOT NULL`
- Dead channels = push + telegram both have errors → dashboard banner
- Re-enable via PUT `/api/owner/locations/:id/notifications/targets/:targetId` with `{ status: 'active' }`

### Customer Contact Reveal
- `POST /api/owner/locations/:locationId/orders/:orderId/reveal-customer-contact`
- Rate limited: 10/min per owner
- Inserts audit record in `customer_contact_reveals` table
- Returns unmasked `name` and `phone`
- Emits PII-free `customer.contact_revealed` MessageBus event

### Resilience Library
- `withTimeout(promise, ms)` — rejects with `TimeoutError` after ms
- `withTimeoutFallback(promise, ms, fallback, label?)` — returns fallback on timeout
- `retryWithBackoff(fn, opts)` — max 3 attempts, exponential backoff + jitter

## API Endpoints

### Public
- `GET /api/public/locations/:slug/fallback-config` — no auth, returns `{ phone, showPhoneOnError, showPhoneOnOffline }`

### Owner (location-scoped)
- `GET /:locationId/settings/fallback` — read fallback config
- `PUT /:locationId/settings/fallback` — update fallback config
- `GET /:locationId/degradation` — current degradation status
- `POST /:locationId/orders/:orderId/reveal-customer-contact` — reveal customer phone

### Admin
- `GET /api/admin/fallback/health` — overview of all locations' fallback config
- `POST /api/admin/fallback/r2-check` — coverage stats

## UI Pages
- `/admin/settings-fallback.html` — owner settings for fallback phone + toggles + WS retry
- Dashboard dead-channel banner in `dashboard.html`
- Customer fallback banner injected by `fallback-phone.ts` via `fallback:needed` CustomEvent

## Key Decisions
- `show_phone_on_error=false` → GDPR-friendly generic message, no phone shown
- Fallback phone in `localStorage` cached from server fetch
- No PII in MessageBus events, audit logs, or error messages
- Dead-channel detection is advisory only — does not prevent operation
- Re-enabling a channel resets `last_error` and `disabled_at`
