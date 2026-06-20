// Pure geo + animation math for the live courier marker and ETA (G2).
//
// All decision logic lives here as pure functions so it can be unit-tested without
// a DOM/React harness; the hooks (useCourierMarker, useDeliveryEta) are thin rAF/
// state glue over these. Coordinates use { lat, lng } to match the backend
// RouteResult; map components convert to [lng, lat] at their boundary.

export interface LatLng { lat: number; lng: number }

/** Distance between two coordinates in metres. */
export function haversineMeters(a: LatLng, b: LatLng): number {
  const R = 6_371_000;
  const dLat = ((b.lat - a.lat) * Math.PI) / 180;
  const dLng = ((b.lng - a.lng) * Math.PI) / 180;
  const s =
    Math.sin(dLat / 2) ** 2 +
    Math.cos((a.lat * Math.PI) / 180) * Math.cos((b.lat * Math.PI) / 180) * Math.sin(dLng / 2) ** 2;
  return 2 * R * Math.atan2(Math.sqrt(s), Math.sqrt(1 - s));
}

export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/** Linear interpolation between two coordinates (movement only — no extrapolation). */
export function lerpLatLng(a: LatLng, b: LatLng, t: number): LatLng {
  const c = Math.max(0, Math.min(1, t));
  return { lat: lerp(a.lat, b.lat, c), lng: lerp(a.lng, b.lng, c) };
}

/** Initial bearing (azimuth) from→to, degrees clockwise from north, 0..360. */
export function bearingDeg(from: LatLng, to: LatLng): number {
  const φ1 = (from.lat * Math.PI) / 180;
  const φ2 = (to.lat * Math.PI) / 180;
  const dλ = ((to.lng - from.lng) * Math.PI) / 180;
  const y = Math.sin(dλ) * Math.cos(φ2);
  const x = Math.cos(φ1) * Math.sin(φ2) - Math.sin(φ1) * Math.cos(φ2) * Math.cos(dλ);
  return ((Math.atan2(y, x) * 180) / Math.PI + 360) % 360;
}

/** Exponential moving average step. alpha∈(0,1]: higher = snappier, lower = smoother. */
export function emaNext(prev: number | null, sample: number, alpha: number): number {
  if (prev === null || !Number.isFinite(prev)) return sample;
  return alpha * sample + (1 - alpha) * prev;
}

export interface RouteProgress {
  /** Metres remaining from the snapped point to the end of the polyline. */
  remainingMeters: number;
  /** Closest point on the polyline to `pos`. */
  snapped: LatLng;
  /** Index of the segment [i, i+1] the snapped point lies on. */
  segmentIndex: number;
}

/**
 * Project a live position onto the polyline and measure the distance left to the
 * destination. Local equirectangular projection (accurate at city scale, cheap
 * enough per-ping). Zero provider calls.
 */
export function progressAlongRoute(polyline: LatLng[], pos: LatLng): RouteProgress {
  if (polyline.length === 0) return { remainingMeters: 0, snapped: pos, segmentIndex: 0 };
  if (polyline.length === 1) return { remainingMeters: haversineMeters(pos, polyline[0]!), snapped: polyline[0]!, segmentIndex: 0 };

  const latRad = (pos.lat * Math.PI) / 180;
  const mPerDegLat = 111_320;
  const mPerDegLng = 111_320 * Math.cos(latRad);
  const toXY = (p: LatLng) => ({ x: (p.lng - pos.lng) * mPerDegLng, y: (p.lat - pos.lat) * mPerDegLat });

  let best = { d: Infinity, i: 0, t: 0 };
  for (let i = 0; i < polyline.length - 1; i++) {
    const A = toXY(polyline[i]!);
    const B = toXY(polyline[i + 1]!);
    const dx = B.x - A.x;
    const dy = B.y - A.y;
    const lenSq = dx * dx + dy * dy;
    let t = lenSq === 0 ? 0 : ((-A.x) * dx + (-A.y) * dy) / lenSq; // P is origin
    t = Math.max(0, Math.min(1, t));
    const cx = A.x + t * dx;
    const cy = A.y + t * dy;
    const d = Math.hypot(cx, cy);
    if (d < best.d) best = { d, i, t };
  }

  const a = polyline[best.i]!;
  const b = polyline[best.i + 1]!;
  const snapped = lerpLatLng(a, b, best.t);

  // remaining = rest of the current segment + all subsequent segments
  let remaining = haversineMeters(snapped, b);
  for (let j = best.i + 1; j < polyline.length - 1; j++) {
    remaining += haversineMeters(polyline[j]!, polyline[j + 1]!);
  }
  return { remainingMeters: remaining, snapped, segmentIndex: best.i };
}

/** Total length of a polyline in metres. */
export function polylineLengthMeters(polyline: LatLng[]): number {
  let total = 0;
  for (let i = 0; i < polyline.length - 1; i++) total += haversineMeters(polyline[i]!, polyline[i + 1]!);
  return total;
}

/**
 * ETA seconds from remaining distance, pacing by the route's own average speed
 * (totalDistance / baselineDuration). Falls back to a sane urban speed if the
 * baseline is missing/degenerate.
 */
export function etaSeconds(remainingMeters: number, totalDistanceMeters: number, baselineDurationS: number): number {
  const speed = totalDistanceMeters > 0 && baselineDurationS > 0
    ? totalDistanceMeters / baselineDurationS // m/s along this route
    : (18 * 1000) / 3600; // 18 km/h fallback
  return Math.round(remainingMeters / speed);
}

/** A ping is out of order if its timestamp predates the last one we showed. */
export function isOutOfOrder(lastTimestamp: number | null, timestamp: number): boolean {
  return lastTimestamp !== null && timestamp < lastTimestamp;
}

/**
 * Snap (instant move) instead of tween when there's no previous fix, or the jump is
 * large (reconnect / GPS leap) — so the marker never "drives" across the city.
 */
export function shouldSnap(prev: LatLng | null, next: LatLng, thresholdMeters = 500): boolean {
  return prev === null || haversineMeters(prev, next) > thresholdMeters;
}

/** Proximity at which we flip to "arriving" (matches the deliver-contract threshold). */
export const ARRIVE_THRESHOLD_M = 150;

export function isArriving(remainingMeters: number, thresholdMeters = ARRIVE_THRESHOLD_M): boolean {
  return remainingMeters <= thresholdMeters;
}
