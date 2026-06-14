# ADR: Notification Consolidation (H-4)

**Date:** 2026-06-12  
**Status:** Accepted  
**PR: (part of NX-1 / H-series)**

## Context

The notification system had two parallel mechanisms for detecting and alerting on pending/dwelling orders:

1. **Dwell-monitor** (`workers/dwell-monitor.ts`): Runs every 60s. Checks all non-terminal order statuses (PENDING, CONFIRMED, PREPARING, IN_DELIVERY). Creates `location_alerts`. Had 3 bugs preventing Telegram delivery:
   - Sent `targetId: null` to dispatch → targets never found
   - Used `eventType: 'dwell.alert'` not in `NotificationEventType` → rendered as "Unknown"
   - Not in quiet-hours allowlist → silently dropped

2. **Pending-aging** (`notifications/workers/index.ts:escalatePendingAging()`): Runs every 5 minutes. Only checks PENDING orders. Correctly resolves targets and uses valid event type `'order.pending_aging'`. Produced the observed ~5-minute delay.

## Decision

**Fix dwell-monitor, remove pending-aging.**

## Rationale

- Dwell-monitor has better granularity (60s vs 5min)
- Dwell-monitor handles all order statuses (not just PENDING)
- Dwell-monitor already has tiered escalation infrastructure
- The bugs were straightforward to fix: resolve real targets + use `'order.dwell_escalation'` (already in the event type union)
- Removing pending-aging eliminates the duplicate-poller problem and dead code

## Changes

### Dwell-monitor fixes

1. `scheduleEscalation()`: Replaced `targetId: null` with real target query (`SELECT id FROM owner_notification_targets WHERE location_id = $1 AND status = 'active' AND channel = 'telegram'`)
2. `eventType`: Changed from `'dwell.alert'` (not in union) to `'order.dwell_escalation'` (in union, has render case, has locale templates)
3. Removed tier 3 (SMS scaffold — was always off via `DWELL_TIER3_ENABLED`)

### Removed

- `escalatePendingAging()` method — entire function deleted
- `queue.boss.work('order.pending_aging', ...)` in server.ts
- `queue.boss.schedule('order.pending_aging', ...)` in server.ts
- Alert resolution code for `kind = 'pending_aging'` in dispatch handler
- `ORDER_PENDING_AGING` queue constant (no longer needed)
- `dwell.escalate` queue worker (already dead — no sender existed)

### Dashboard status changes

- `transitionOrder()` in `dashboard.ts` replaced raw SQL + ad-hoc publish with `updateOrderStatus()` from `orderStatusService.ts`
- This ensures confirm/reject transitions go through the canonical path and trigger Telegram lifecycle events (`order.confirmed`, `order.rejected`)

## Consequences

- **Positive**: Pending notifications now fire within 60-120s instead of ~5-10min
- **Positive**: All status changes through dashboard trigger Telegram notifications
- **Positive**: One canonical status-change path confirmed (with rejection_reason handled separately)
- **Neutral**: `'order.pending_aging'` event type still exists in the type union (backward compat for existing tests), but is no longer actively produced
- **Risk**: If dwell-monitor has a bug, no fallback mechanism exists. Mitigated by: advisory lock, singletonKey, per-minute cron, and existing alert lifecycle

## Verification

- `pnpm typecheck` — all 12 packages green
- `npx tsx apps/api/scripts/verify-orphans.ts` — zero raw string leaks, zero critical queue orphans
- Visual proof: `order.pending_aging` no longer appears as "worker registered but no sender"
