// §3 — no-progress circuit breaker. Pure: reads progressMetric deltas + caps and
// decides whether to stop the loop. "opossum guards the product; nothing guards
// the loop." This is the loop's guard.

import type { BreakerConfig, BreakerState, BreakerReason } from './types.js';

export const DEFAULT_BREAKER: BreakerConfig = {
  K: 3, // stall after 3 non-improving iterations
  maxIter: 25,
  budgetUsd: Number.POSITIVE_INFINITY,
  timeCapMs: Number.POSITIVE_INFINITY,
};

export function initBreaker(): BreakerState {
  return { stallCount: 0, tripped: false, reason: null };
}

export interface BreakerInput {
  /** Change in progressMetric vs previous iteration. <0 = progress (good). */
  delta: number;
  /** 1-based iteration index just completed. */
  iteration: number;
  cumulativeCostUsd: number;
  elapsedMs: number;
}

/**
 * Advance the breaker by one completed iteration. Trips on ANY of: stall
 * (delta >= 0 for K iters; reset on delta < 0), max_iter, budget, time_cap.
 * Pure — returns a fresh state; the first matching reason is reported.
 */
export function stepBreaker(
  prev: BreakerState,
  input: BreakerInput,
  cfg: BreakerConfig = DEFAULT_BREAKER,
): BreakerState {
  // A strictly-improving iteration resets the stall counter; otherwise it grows.
  const stallCount = input.delta < 0 ? 0 : prev.stallCount + 1;

  let reason: BreakerReason | null = null;
  if (stallCount >= cfg.K) reason = 'stall';
  else if (input.iteration >= cfg.maxIter) reason = 'max_iter';
  else if (input.cumulativeCostUsd >= cfg.budgetUsd) reason = 'budget';
  else if (input.elapsedMs >= cfg.timeCapMs) reason = 'time_cap';

  return { stallCount, tripped: reason !== null, reason };
}

export function breakerReasonText(reason: BreakerReason): string {
  switch (reason) {
    case 'stall': return 'no progress for K consecutive iterations';
    case 'max_iter': return 'hit MAX_ITER cap';
    case 'budget': return 'hit cost BUDGET cap';
    case 'time_cap': return 'hit wall-clock TIME_CAP';
  }
}
