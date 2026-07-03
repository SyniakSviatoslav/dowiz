// deliver v2 offer-sweep-cancel addendum (ADR-deliver-v2-cash-as-proof §Addendum, 2026-07-02).
//
// The order state machine (packages/domain/src/order-machine.ts) now PERMITS
// CONFIRMED/PREPARING/READY → CANCELLED so the SYSTEM dispatch-grace path can express a
// no-courier terminal for a pre-IN_DELIVERY order. Those edges are SYSTEM-only: an OWNER must
// not be able to drive them by piping a request-supplied `newStatus` straight into
// updateOrderStatus. The machine says what is POSSIBLE; this route-layer guard says who is ALLOWED.
//
// Existing owner cancels are preserved: PENDING→CANCELLED (pre-confirm reject/cancel) and
// IN_DELIVERY→CANCELLED (no-show, via owner/signals.ts) remain permitted.
const OWNER_FORBIDDEN_CANCEL_FROM: ReadonlySet<string> = new Set(['CONFIRMED', 'PREPARING', 'READY']);

/**
 * Route-layer authz for owner-driven order status transitions. Throws the app's standard
 * `{ statusCode, error, code }` error shape (caught by the route handlers → HTTP 403) when an owner
 * requests a SYSTEM-only cancel. Call it AFTER reading the order's current status and BEFORE handing
 * the transition to updateOrderStatus.
 */
export function assertOwnerTargetAllowed(from: string, to: string): void {
  if (to === 'CANCELLED' && OWNER_FORBIDDEN_CANCEL_FROM.has(from)) {
    throw {
      statusCode: 403,
      error: 'Cancelling an order in preparation is not available',
      code: 'CANCEL_NOT_PERMITTED',
    };
  }
}
