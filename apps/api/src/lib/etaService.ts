// Prep-Time + Client ETA (v1) · P3 — the ETA range engine (PURE, unit-tested).
//
// Spec invariants (operator "Prep-Time + Клієнтський ETA v1"):
//  - ALWAYS a range [low, high], NEVER a single number, NEVER 0.
//  - Two phases: pre_assign (wide margin — delivery queue unknown) → assigned (narrow — queue real).
//  - Width reflects queue depth; values shrink as prep decays / the leg shortens.
//  - Near the end (in delivery, tiny core) → a humane floor band (~5–10 min), never "0".
//  - Monotonic: once narrowed (assigned), the shown top must not visibly re-widen.
//  - Honest under poor inputs: missing coords / NaN / negative never leak a single number, 0, NaN,
//    or low>high — they degrade to a wider, finite, floored range.
//
// This module does NO IO: callers gather the numeric inputs (kitchen queue, courier queue, the
// delivery leg, decayed prep-remaining) and pass them in, so every branch is deterministically
// testable. The DB-gathering wrapper lives in the event layer (P4).

import { distanceKm } from './geo.js';

export interface EtaConfig {
  mopedSpeedKmh: number;
  roadFactor: number;       // straight-line × this ≈ road distance (deliberately rough, v1)
  m1Low: number; m1High: number;   // Phase 1 (pre_assign) margins — WIDE
  m2Low: number; m2High: number;   // Phase 2 (assigned) margins — NARROW
  nearEndLow: number; nearEndHigh: number; // floor band when almost there
  fallbackDeliveryMin: number;     // used when the delivery leg can't be measured (no pin)
  minLowFloor: number;             // the low bound never drops below this → never "~0"
  minBandMin: number;              // high − low is always at least this → never a single number
}

export const ETA_DEFAULTS: EtaConfig = {
  mopedSpeedKmh: 22,
  roadFactor: 1.3,
  m1Low: 10, m1High: 20,
  m2Low: 5, m2High: 10,
  nearEndLow: 5, nearEndHigh: 10,
  fallbackDeliveryMin: 20,
  minLowFloor: 5,
  minBandMin: 5,
};

export type EtaPhase = 'pre_assign' | 'assigned';

const TERMINAL_STATUSES = ['DELIVERED', 'REJECTED', 'CANCELLED'];

export interface EtaRange {
  /** Lower bound in whole minutes (≥ minLowFloor). */
  lowMin: number;
  /** Upper bound in whole minutes (≥ lowMin + minBandMin). */
  highMin: number;
  phase: EtaPhase;
  /** True when the estimate has elapsed but the order isn't terminal — UI shows a humane note. */
  overdue: boolean;
}

export interface EtaInput {
  phase: EtaPhase;
  status: string; // order_status enum value
  /** This order's REMAINING kitchen time (minutes), already decayed by the caller from timestamps. */
  prepRemainingMinutes: number;
  /** Phase 1: Σ prep-remaining of orders preparing AHEAD on the same kitchen. */
  kitchenQueueAheadMinutes: number;
  /** Phase 2: Σ ahead of this order on the assigned courier (remaining prep + round-trip). */
  courierQueueAheadMinutes: number;
  /** Delivery leg in minutes; null/NaN ⇒ couldn't measure (missing coords) ⇒ fallback is used. */
  deliveryLegMinutes: number | null;
  /** Minutes since the order was placed — drives the overdue flag. */
  elapsedSincePlacedMinutes?: number;
  /** The previously-shown high bound — Phase 2 never re-widens above it (monotonic). */
  previousHighMin?: number;
  config?: Partial<EtaConfig>;
}

/** Coerce to a finite, non-negative number (guards NaN / null / negative / Infinity). */
function nonNeg(n: unknown): number {
  return typeof n === 'number' && Number.isFinite(n) && n > 0 ? n : 0;
}

/**
 * Delivery leg in minutes between an origin and the customer. Returns null when ANY coordinate is
 * missing/non-finite (so the range layer applies its fallback instead of producing NaN).
 */
export function deliveryLegMinutes(
  originLat: number | null | undefined,
  originLng: number | null | undefined,
  custLat: number | null | undefined,
  custLng: number | null | undefined,
  config: Partial<EtaConfig> = {},
): number | null {
  const cfg = { ...ETA_DEFAULTS, ...config };
  const coords = [originLat, originLng, custLat, custLng];
  if (coords.some((c) => typeof c !== 'number' || !Number.isFinite(c))) return null;
  const roadKm = distanceKm(originLat as number, originLng as number, custLat as number, custLng as number) * cfg.roadFactor;
  if (!Number.isFinite(roadKm) || roadKm < 0) return null;
  const speed = cfg.mopedSpeedKmh > 0 ? cfg.mopedSpeedKmh : ETA_DEFAULTS.mopedSpeedKmh;
  const min = (roadKm / speed) * 60;
  return Number.isFinite(min) ? Math.max(0, min) : null;
}

/**
 * The core of the feature: turn the gathered numerics into an honest, floored, never-zero range.
 * Pure & total — every degenerate input maps to a valid {lowMin<highMin, both finite, low≥floor}.
 */
export function computeEtaRange(input: EtaInput): EtaRange {
  const cfg = { ...ETA_DEFAULTS, ...input.config };

  const prep = nonNeg(input.prepRemainingMinutes);
  const kitchenAhead = nonNeg(input.kitchenQueueAheadMinutes);
  const courierAhead = nonNeg(input.courierQueueAheadMinutes);
  const leg =
    input.deliveryLegMinutes == null || !Number.isFinite(input.deliveryLegMinutes)
      ? cfg.fallbackDeliveryMin
      : Math.max(0, input.deliveryLegMinutes);

  let core: number;
  let mLow: number;
  let mHigh: number;
  if (input.phase === 'assigned') {
    core = prep + courierAhead + leg;
    mLow = cfg.m2Low;
    mHigh = cfg.m2High;
  } else {
    core = prep + kitchenAhead + leg;
    mLow = cfg.m1Low;
    mHigh = cfg.m1High;
  }

  let lowMin = Math.round(core - mLow);
  let highMin = Math.round(core + mHigh);

  // Near the door: in delivery with a tiny remaining core → a calm "~5–10 min", never a countdown to 0.
  if (input.status === 'IN_DELIVERY' && core <= cfg.nearEndHigh) {
    lowMin = cfg.nearEndLow;
    highMin = cfg.nearEndHigh;
  }

  // Invariants: never ~0 (floor the low), never a single number (enforce a min band), never low>high.
  lowMin = Math.max(cfg.minLowFloor, lowMin);
  if (highMin < lowMin + cfg.minBandMin) highMin = lowMin + cfg.minBandMin;

  // Monotonic: once we're in Phase 2 we never visibly push the top back up (anxiety-inducing).
  if (
    input.phase === 'assigned' &&
    typeof input.previousHighMin === 'number' &&
    Number.isFinite(input.previousHighMin)
  ) {
    highMin = Math.min(highMin, Math.max(input.previousHighMin, lowMin + cfg.minBandMin));
  }

  const elapsed = input.elapsedSincePlacedMinutes;
  const overdue =
    typeof elapsed === 'number' &&
    Number.isFinite(elapsed) &&
    elapsed > highMin &&
    !TERMINAL_STATUSES.includes(input.status);

  return { lowMin, highMin, phase: input.phase, overdue };
}
