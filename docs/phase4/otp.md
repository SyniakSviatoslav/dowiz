# OTP (One-Time Password) — Phase 4, Stage 26

## Overview

OTP is an **owner-toggle** phone verification seam. Disabled by default (`require_phone_otp = false`) — zero friction for new locations. When enabled, customers must verify their phone via 6-digit SMS code before ordering.

## Rate Limits

| Limit | Value | Period | Env Var |
|-------|-------|--------|---------|
| OTP sends | 3 | 15min per phone per location | `OTP_SEND_RATE_LIMIT` |
| OTP verifies | 5 | 15min per phone per location | `OTP_VERIFY_RATE_LIMIT` |
| Lockout | 1h | after 5 failed attempts | — |
| OTP code TTL | 5 min | — | `OTP_TTL_MS` |
| Verified token TTL | 15 min | single-use, order-scoped | — |

## Endpoints

### `POST /api/customer/locations/:slug/otp/send`
- **Auth**: None (customer-facing at checkout)
- **Rate**: 3/15min per phone
- **Body**: `{ phone: E.164, order_intent: { items: [{ product_id, quantity }], total, currency } }`
- **Validates**: `require_phone_otp = true` (else 400 OTP_NOT_REQUIRED)
- **Logic**: generate 6-digit code, argon2id hash, store in `phone_otp`, issue `otp_token` (opaque 32B base64url)
- **Response**: `{ otp_token, expires_in_ms: 300000 }`
- **SMS delivery**: P26 scaffold — logs masked phone. Real SMS gateway in Phase 5+.

### `POST /api/customer/locations/:slug/otp/verify`
- **Auth**: None
- **Rate**: 5/15min per phone (6th = 429 + 1h lockout)
- **Body**: `{ phone, code, otp_token, order_intent_hash }`
- **Validates**: argon2id verify. If attempts ≥ 5 → invalidate OTP, lockout.
- **Response**: `{ verified_token, expires_in_ms: 900000 }`

### `POST /orders` (extension)
- **Header**: `X-OTP-Verified: <verified_token>`
- **Logic**: If `require_phone_otp = true` AND valid token → metadata `otp_verified=true`. Token consumed (single-use).
- **Does NOT block**: If `require_phone_otp = true` AND no/invalid token → order proceeds (P26: not blocked; E27 may soft-confirm).

## Security

- OTP code: 6 digits, argon2id hashed. `code_hash` is immutable (DB trigger).
- `code_hash` never returned in API responses. SELECT returns `id, phone, expires_at, attempts, consumed_at` — no hash.
- `verified_token`: opaque 32B base64url (not JWT to avoid confusion). Stored as sha256 hash.
- 5 failed attempts → OTP invalidated + 1h lockout.
- No plaintext phone in events/logs. `maskPhone()` for display.

## Tables

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `phone_otp` | OTP code storage | `code_hash` (argon2id), `attempts`, `expires_at`, `consumed_at` |
| `customer_otp_sessions` | Token session management | `token_hash` (sha256), `purpose`, `order_intent_hash`, `expires_at`, `consumed_at` |

## Future (Phase 5+)

- Real SMS gateway integration (Twilio/etc.)
- Rate-limit counter via Redis (not DB COUNT)
- OTP for other operations (cancellation, high-value orders)
