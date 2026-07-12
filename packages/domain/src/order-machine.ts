import { IllegalTransitionError, ScaffoldDisabledError, SameStatusError } from './errors.js';

export const ORDER_STATUSES = [
  'PENDING',
  'CONFIRMED',
  'PREPARING',
  'READY',
  'IN_DELIVERY',
  'DELIVERED',
  'REJECTED',
  'CANCELLED',
  'SCHEDULED',
  'PICKED_UP',
] as const;

export type OrderStatus = (typeof ORDER_STATUSES)[number];

const TRANSITIONS: Record<OrderStatus, ReadonlyArray<OrderStatus>> = {
  PENDING: ['CONFIRMED', 'REJECTED', 'CANCELLED'],
<<<<<<< Updated upstream
  CONFIRMED: ['PREPARING', 'IN_DELIVERY'],
  PREPARING: ['READY'],
  READY: ['IN_DELIVERY', 'PICKED_UP'],
  IN_DELIVERY: ['DELIVERED'],
=======
  // deliver v2 offer-sweep-cancel addendum (ADR-deliver-v2-cash-as-proof §Addendum, 2026-07-02):
  // CANCELLED added to CONFIRMED/PREPARING/READY as the SYSTEM-only dispatch-exhausted terminal edge —
  // a no-courier order the owner ignored for the full grace window must be able to reach the
  // customer-honest terminal even pre-IN_DELIVERY. The machine states what is POSSIBLE; owner exposure
  // is closed at the route layer (assertOwnerTargetAllowed → 403 CANCEL_NOT_PERMITTED). Pinned by the
  // exhaustive assertTransition test so this widening is conscious and any future drift fails red.
  CONFIRMED: ['PREPARING', 'IN_DELIVERY', 'CANCELLED'],
  PREPARING: ['READY', 'CANCELLED'],
  READY: ['IN_DELIVERY', 'PICKED_UP', 'CANCELLED'],
  // deliver v2 (ADR-deliver-v2-cash-as-proof): CANCELLED = the no-cash-tail terminal (refused/cancelled-on-door)
  // so the customer never sees "Delivered" for refused food; READY = the revert target for courier
  // cancel/abort/owner-reassign of an order force-driven to IN_DELIVERY (no new order_status enum value added).
  // Both are downgrades to terminal/assignable states the machine already owns; the central updateOrderStatus
  // fold terminalizes the active assignment on either edge so no order leaves IN_DELIVERY stranded.
  IN_DELIVERY: ['DELIVERED', 'CANCELLED', 'READY'],
>>>>>>> Stashed changes
  DELIVERED: [],
  REJECTED: [],
  CANCELLED: [],
  SCHEDULED: [],
  PICKED_UP: [],
};

// PICKED_UP is a live terminal state for pickup orders (READY → PICKED_UP).
// SCHEDULED remains scaffold (the scheduled flow isn't implemented yet).
const SCAFFOLD_STATUSES: ReadonlySet<OrderStatus> = new Set(['SCHEDULED']);

export function assertTransition(from: OrderStatus, to: OrderStatus): void {
  if (from === to) throw new SameStatusError(from);
  if (SCAFFOLD_STATUSES.has(to)) throw new ScaffoldDisabledError(from, to);
  if (SCAFFOLD_STATUSES.has(from)) throw new ScaffoldDisabledError(from, to);

  const allowed = TRANSITIONS[from];
  if (!allowed || !allowed.includes(to)) throw new IllegalTransitionError(from, to);
}

export function isTerminal(status: OrderStatus): boolean {
  return ['DELIVERED', 'PICKED_UP', 'REJECTED', 'CANCELLED'].includes(status);
}
