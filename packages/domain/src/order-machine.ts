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
  CONFIRMED: ['PREPARING', 'IN_DELIVERY'],
  PREPARING: ['READY'],
  READY: ['IN_DELIVERY', 'PICKED_UP'],
  // deliver v2 (ADR-deliver-v2-cash-as-proof): CANCELLED = the no-cash-tail terminal (refused/cancelled-on-door)
  // so the customer never sees "Delivered" for refused food; READY = the revert target for courier
  // cancel/abort/owner-reassign of an order force-driven to IN_DELIVERY (no new order_status enum value added).
  // Both are downgrades to terminal/assignable states the machine already owns; the central updateOrderStatus
  // fold terminalizes the active assignment on either edge so no order leaves IN_DELIVERY stranded.
  IN_DELIVERY: ['DELIVERED', 'CANCELLED', 'READY'],
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
