import type { OrderStatus } from './order-machine.js';

export class IllegalTransitionError extends Error {
  constructor(readonly from: OrderStatus, readonly to: OrderStatus) {
    super(`Illegal transition: ${from} → ${to}`);
    this.name = 'IllegalTransitionError';
  }
}

export class ScaffoldDisabledError extends Error {
  constructor(readonly from: OrderStatus, readonly to: OrderStatus) {
    super(`Scaffold transition disabled: ${from} → ${to}`);
    this.name = 'ScaffoldDisabledError';
  }
}

export class SameStatusError extends Error {
  constructor(readonly status: OrderStatus) {
    super(`Cannot transition to same status: ${status}`);
    this.name = 'SameStatusError';
  }
}

export class ConflictError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ConflictError';
  }
}
