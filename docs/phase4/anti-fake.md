# Phase 4 — Anti-Fake Signals (Stage 26)

## Red Lines (Non-Negotiable)

### R1: No Auto-Ban 🔴
- No signal ever blocks `POST /orders`. Order with max signals → 200, not 4xx.
- No `customers.banned` column exists. Migration guard: `grep banned` → 0.
- No signal cancels existing orders. Signals are read-only advisory.

### R2: Reputation Decays 🔴
- Signal strength = `no_show_count * exp(-age_days / 30) / max(1, completed_count)`.
- `last_no_show_at = NULL` → strength = 0.
- Owner acknowledge shifts `last_no_show_at` by -7 days (forgive), does NOT reset counter.

### R3: OTP — Owner-Toggle, Off by Default 🔴
- `require_phone_otp = false` (default) → zero friction.
- Toggle on → OTP required at checkout.
- Rate-limits: 3 sends / 15min per phone, 5 verifies / 15min per phone, 1h lockout.
- OTP code: 6 digits, argon2id-hash, immutable. No plaintext storage.
- OTP token: opaque 32B base64url, short-lived (15min), order-scoped, single-use.

### R4: Velocity — Privacy-First 🔴
- Counters per `phone_hash` + `client_ip_hash` only. No raw PII.
- Windows: 1h (threshold=3) → `velocity_rapid`, 24h (threshold=10) → `velocity_high_volume`.
- 24h retention, tenant-scoped.

### R5: Human-in-Loop 🔴
- All signals visible to owner on dashboard + `/admin/signals` page.
- Acknowledge (forgive, shift decay) or Dismiss (mark reviewed, no shift).
- No auto-acknowledge after timeout.

### R6: Security 🔴
- 0 PII in events, jobs, audit logs.
- Cross-tenant signal query → 404, not 403.
- Zod `.strict()` on all endpoints. RS256 JWT only. 0 cookies.

## Architecture

```
POST /orders
  → location.require_phone_otp? check X-OTP-Verified header
  → async velocity_events INSERT (debounced, pg-boss)
  → E27 preflight consumes signals

SignalRaiserWorker (cron */5, singleton)
  → computeSignals() per active location
  → persist to customer_signals
  → WS: preflight.signal_raised

Owner:
  GET /signals — list with pagination
  GET /signals/compute — read-only what-if
  POST /signals/:id/acknowledge — manual forgive
  POST /signals/:id/dismiss — mark reviewed
  POST /orders/:id/mark-no-show — manual no-show counter

Customer:
  POST /otp/send — get OTP code (SMS scaffold, P5+ real gateway)
  POST /otp/verify — verify code, get verified_token
```

## Decay Function

```
strength = no_show_count * exp(-age_days / 30) / max(1, completed_count)
age_days = (now() - last_no_show_at) / 86400
severity: low (0.5–1.0), medium (1.0–2.0), high (>2.0)
```

## OTP Flow

1. Customer enters phone at checkout.
2. `POST /otp/send` → 6-digit code generated, argon2id hashed, stored in `phone_otp`.
3. `POST /otp/verify` → code verified, `verified_token` issued (15min, order-scoped).
4. `POST /orders` with `X-OTP-Verified: <token>` → token consumed, order metadata `otp_verified=true`.
5. If `require_phone_otp=true` and no valid token → order proceeds (P26: not blocked; E27 may soft-confirm).
