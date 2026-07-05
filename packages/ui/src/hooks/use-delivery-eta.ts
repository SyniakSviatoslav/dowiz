import { useEffect, useRef, useState } from 'react';
import {
  emaNext,
  etaSeconds as rawEtaSeconds,
  isArriving,
  polylineLengthMeters,
  progressAlongRoute,
  type LatLng,
} from '../lib/geo-anim.js';

export interface DeliveryEta {
  /** EMA-smoothed seconds to destination (null until computable). */
  etaSeconds: number | null;
  /** Distance left along the route, metres. */
  remainingMeters: number | null;
  /** True once within the proximity threshold (~150 m) — flip the UI to "arriving". */
  arriving: boolean;
}

const EMPTY: DeliveryEta = { etaSeconds: null, remainingMeters: null, arriving: false };

/**
 * Local ETA from the real route polyline + the live courier position. Zero provider
 * calls per ping: it projects the position onto the polyline (progressAlongRoute),
 * paces by the route's own average speed, and EMA-smooths across pings so the number
 * is stable and near-monotonic instead of jittering each update.
 */
export function useDeliveryEta(
  polyline: LatLng[] | null,
  baselineDurationS: number | null,
  pos: LatLng | null,
  baselineDistanceMeters?: number | null,
  alpha = 0.3,
): DeliveryEta {
  const [eta, setEta] = useState<DeliveryEta>(EMPTY);
  const smoothedRef = useRef<number | null>(null);

  useEffect(() => {
    if (!polyline || polyline.length === 0 || !pos) {
      smoothedRef.current = null;
      setEta(EMPTY);
      return;
    }
    const { remainingMeters } = progressAlongRoute(polyline, pos);
    const total = baselineDistanceMeters && baselineDistanceMeters > 0
      ? baselineDistanceMeters
      : polylineLengthMeters(polyline);
    const raw = rawEtaSeconds(remainingMeters, total, baselineDurationS ?? 0);
    const smoothed = Math.round(emaNext(smoothedRef.current, raw, alpha));
    smoothedRef.current = smoothed;
    setEta({ etaSeconds: smoothed, remainingMeters, arriving: isArriving(remainingMeters) });
  }, [polyline, baselineDurationS, baselineDistanceMeters, pos?.lat, pos?.lng]);

  return eta;
}
