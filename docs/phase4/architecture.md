# Phase 4 — Architecture

## Overview

Phase 4 adds five new capabilities on top of the Phase 0–3 foundation:
- **E24** Live dashboard (owner real-time view)
- **E25** Dwell monitor (order stuck detection via configurable thresholds)
- **E26** Anti-fake signals (velocity checks, reputation decay, OTP verification)
- **E27** Preflight engine (order validation pipeline)
- **E28** Native push delivery
- **E29** Self-serve owner onboarding

Stage 23 (P4-0) lays the **operational seams** — schema-only, no runtime logic, no endpoints.

---

## Seam Map

| Seam | Table/Column | Created In | Consumed By | Purpose |
|------|-------------|-----------|-------------|---------|
| Reputation counters | `customers.no_show_count`, `customers.completed_count`, `customers.last_no_show_at` | 024 | E26 | Soft signal for owner to identify problematic customers |
| IP hash | `orders.client_ip_hash` | 025 | E25, E26 | Velocity check per IP (hashed, not raw) |
| OTP toggle | `locations.require_phone_otp` | 025 | E26 | Owner-toggleable phone verification |
| OTP storage | `phone_otp` | 025 | E26 | OTP code hashes (argon2id), tenant-isolated |
| Onboarding state | `locations.onboarding_state` (JSONB) | 026 | E29 | Self-serve flow progress, 8 steps |
| Onboarding completion | `locations.onboarding_completed_at` | 026 | E29 | Go-live marker |
| Dwell thresholds | `locations.dwell_thresholds` (JSONB) | 026 | E25 | Per-state timeout config |

---

## Key Design Decisions

### JSONB over enum for onboarding
`onboarding_state` is JSONB (`{v, step, completedSteps[], data{}}`) to allow adding/removing steps without ALTER TABLE. Schema versioned via `v` field.

### JSONB over separate columns for dwell thresholds
`dwell_thresholds` is JSONB with `v` field so owners can add custom states in future without migrations. DB validates only that it's an object — per-key validation is in the app layer.

### Soft counters, not invariants
Reputation counters have `CHECK (>= 0)` but no cross-column constraints (e.g., `no_show_count <= completed_count`). Decay and signal weight are runtime concerns (E26).

### OTP off by default
`require_phone_otp` defaults to `false`. Zero friction for new locations. Owner explicitly opts in.

---

## Migration Order

```
M024 (customer-reputation)    →  adds to customers
M025 (anti-fake-seam)         →  adds to orders, locations, creates phone_otp
M026 (onboarding-alert-config) →  adds to locations
```

All three are forward-only. `down()` exists for staging rollback but is never applied on production.
