# MVP Sensor-Seams — Runtime Status

> Living status of the council-APPROVED `mvp-sensor-seams` runtime (ADR-0007/0008/0009 v4).
> Migrations 066/067 applied+verified on staging. Branch `feat/mvp-sensor-seams` (not prod).
> Updated 2026-06-25.

## ✅ SHIPPED + VALIDATED ON STAGING (backend runtime)

| § | What | Where | Proof |
|---|------|-------|-------|
| **§1.1** | Geofence capture (`courier_geofence_enter`, exactly-once, own-order, dual-context RLS) + frozen `promised_window` (set-once) + mutable `live_eta` (per-stage, width-floor, cap-last) | `courier/shifts.ts` ping · `etaGather.ts` (`clampWindow`, `synthesizeAndPersistEtaWindow`) · `orderStatusService.ts` (best-effort SAVEPOINT) · `customer/orders.ts` (exposes `promisedWindow`/`liveEta`) | `eta-synthesis.test.ts` 7/7 · `flow-sensor-eta-window.spec.ts` · `flow-sensor-geofence.spec.ts` · psql DB-state (1 geofence row, exactly-once) · ledger #18 |
| **§1.2** | Normalised delivery baseline (`route_distance_m` + `expected_delivery_min`, haversine×road-factor, no router) at DELIVERED | `courier/assignments.ts` delivered handler → `delivery_trace` upsert | `flow-sensor-delivery-baseline.spec.ts` · psql: route_distance_m=2603, expected_delivery_min=7 |
| **§1.3** | Anonymous storefront-funnel ingest (`POST /api/funnel`, per-IP 60/min, uniform 204, app.current_tenant RLS arm) + 90-day batched retention sweep + `FUNNEL_INGEST_ENABLED` kill-switch | `routes/public/funnel.ts` · `server.ts` · `workers/anonymizer-retention.ts` · `config` flag | `flow-sensor-funnel.spec.ts` 3/3 · psql: valid event landed, unknown-location FK-dropped |
| **§1.4** | `eta_cap` "not silent" — owner-readable `eta_cap_exceeded` sensor event when the honest estimate exceeds the cap (chose `order_sensor_events` dual-context RLS so it survives courier-confirm context, unlike member-only `location_alerts`) | `etaGather.ts` (nested SAVEPOINT) | psql: cap=1 → confirm → `eta_cap_exceeded {requested_hi_min:35,cap_min:1}`, window clamped 1–2 |
| **§4 (M5)** | IP velocity gate — close the unused `clientIpHash`; ≥20 orders/(location,IP)/15min → 429 IP_THROTTLE | `routes/orders.ts` | psql-seed integration (below) |

**§4.2 owner abort** — already exists at the API layer (`PATCH /api/orders/:id/status` → CANCELLED/REJECTED, owner-auth, guarded state machine). Only the one-tap FE button is outstanding (FE).

## ⏳ REMAINING — FE / product layer (the proposal §8 sequences these as "mostly FE")

These need the design-system + visual-regression-net discipline and/or product decisions, so they are
a distinct effort from the backend seam runtime above. Each is unblocked by the shipped backend:

| § | What | Needs | Backend ready? |
|---|------|-------|----------------|
| **§2.1** | Courier prep countdown + dispatch nudge (advisory, override; non-compliance NOT an owner signal — Counsel #3) | Courier-app FE timer off `prep_time_minutes` + `dispatch_margin_min`; expose `dispatch_margin_min` in the courier task payload | Knobs exist on `locations`; prep on `products` |
| **§2.2** | BackgroundWarning + heartbeat-flag → owner notify (NO auto-reassign) | Courier-app background detection → existing `last_heartbeat_at`/dwell infra; owner surface | Heartbeat + dwell-monitor exist |
| **§2.3** | Daily availability checklist (binary `is_available`, NO stock decrement) | Owner-app checklist UI over the existing `is_available` toggle | `is_available` exists |
| **§2.4** | Proactive WISMO: notify customer when `live_eta` shifts > `material_shift_min`; humane cause-hint on confirm-time rejection | Notification-pipeline integration (telegram/push) keyed on per-stage `live_eta` delta; customer-facing rejection reason (careful not to leak internal notes) | `live_eta` per-stage + `material_shift_min` knob exist |
| **§5** | Prep presets by category (default `prep_time_minutes` per category) | A category-preset store (small migration) + apply-on-create + owner UI; product decision on preset values | `prep_time_minutes` exists (defaults 15) |
| **§1.x FE surfacing** | Show `liveEta`/`promisedWindow` band, geofence "arriving" beat, and `eta_cap_exceeded` in the owner alerts UI; emit funnel events from the storefront | FE wiring of the already-exposed/recorded backend signals | All signals exposed/recorded |

## Notes
- **Stock decrement/restock runtime** remains the named DEFERRED follow-up (Option B) — ships with its
  SECURITY-DEFINER restock fix + anti-cheat-green DoD. The inert `stock_remaining` column-seam is live.
- All backend pieces are non-blocking/observe-don't-control; an order with no sensor data behaves
  byte-identically to before. Prod migrates 066/067 + this runtime on merge to `main`.
