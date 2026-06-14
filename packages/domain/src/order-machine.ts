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
  CONFIRMED: ['PREPARING'],
  PREPARING: ['READY'],
  READY: ['IN_DELIVERY'],
  IN_DELIVERY: ['DELIVERED'],
  DELIVERED: [],
  REJECTED: [],
  CANCELLED: [],
  SCHEDULED: [],
  PICKED_UP: [],
};

const SCAFFOLD_STATUSES: ReadonlySet<OrderStatus> = new Set(['SCHEDULED', 'PICKED_UP']);

export function assertTransition(from: OrderStatus, to: OrderStatus): void {
  if (from === to) throw new SameStatusError(from);
  if (SCAFFOLD_STATUSES.has(to)) throw new ScaffoldDisabledError(from, to);
  if (SCAFFOLD_STATUSES.has(from)) throw new ScaffoldDisabledError(from, to);

  const allowed = TRANSITIONS[from];
  if (!allowed || !allowed.includes(to)) throw new IllegalTransitionError(from, to);
}

export function isTerminal(status: OrderStatus): boolean {
  return ['DELIVERED', 'REJECTED', 'CANCELLED'].includes(status);
}
