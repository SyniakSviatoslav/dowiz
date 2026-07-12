// Courier GPS minimization (P0-1, ADR-p0-privacy-hardening).
//
// A courier_positions row is stored ONLY while the courier is on an active delivery —
// tracking begins at the courier's CONSENT act (assignment 'accepted'), never while
// merely assigned-but-not-accepted and never while idle on shift. This deliberately
// EXCLUDES 'assigned' (the broader active-assignment set used elsewhere, e.g.
// owner/couriers.ts:166) — see ADR DEV-3; a future refactor must not silently re-add
// 'assigned' here or it re-opens pre-consent location tracking.
export const ACTIVE_DELIVERY_ASSIGNMENT_STATUSES = ['accepted', 'picked_up'] as const;

// Retention: courier_positions are purged after this window (courier-cron.ts). Named
// here so the cron and any test reference one source of truth.
export const COURIER_POSITION_RETENTION_INTERVAL = '24 hours';
