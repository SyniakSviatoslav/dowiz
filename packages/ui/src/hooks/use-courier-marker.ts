import { useEffect, useRef, useState } from 'react';
import {
  bearingDeg,
  isOutOfOrder,
  lerpLatLng,
  shouldSnap,
  type LatLng,
} from '../lib/geo-anim.js';

export interface CourierTarget extends LatLng {
  /** Monotonic source timestamp (e.g. recorded_at ms) for the out-of-order guard. */
  recordedAt?: number;
}

export interface SmoothedMarker extends LatLng {
  /** Heading in degrees (0..360) for icon rotation. */
  bearing: number;
}

export interface UseCourierMarkerOptions {
  /** Expected gap between pings; the tween spans this so motion looks continuous. */
  pingIntervalMs?: number;
  /** Jumps larger than this snap instead of tweening (reconnect / GPS leap). */
  snapThresholdMeters?: number;
}

/**
 * rAF-tweens a courier marker between successive position pings:
 *   • linear interpolation only (movement, never forward extrapolation),
 *   • ignores out-of-order pings (older recorded_at than the last shown),
 *   • snaps (no tween) on the first fix or a large jump,
 *   • exposes bearing (prev→cur azimuth) for icon rotation,
 *   • pauses the rAF loop while the tab is hidden and snaps to the latest on resume.
 *
 * Returns the smoothed position+bearing, re-rendering per animation frame — use it
 * inside a small isolated component (e.g. the map wrapper), not a large page, so
 * the per-frame render stays cheap.
 */
export function useCourierMarker(
  target: CourierTarget | null,
  opts: UseCourierMarkerOptions = {},
): SmoothedMarker | null {
  const pingIntervalMs = opts.pingIntervalMs ?? 3000;
  const snapThresholdMeters = opts.snapThresholdMeters ?? 500;

  const [shown, setShown] = useState<SmoothedMarker | null>(null);

  const fromRef = useRef<LatLng | null>(null);
  const toRef = useRef<LatLng | null>(null);
  const startRef = useRef<number>(0);
  const bearingRef = useRef<number>(0);
  const lastTsRef = useRef<number | null>(null);
  const rafRef = useRef<number | null>(null);

  // Ingest a new target → set up a tween or snap.
  useEffect(() => {
    if (!target) return;
    const ts = target.recordedAt ?? Date.now();
    if (isOutOfOrder(lastTsRef.current, ts)) return; // stale ping — drop
    lastTsRef.current = ts;

    const next: LatLng = { lat: target.lat, lng: target.lng };
    const prev = shown ? { lat: shown.lat, lng: shown.lng } : fromRef.current;

    if (prev) bearingRef.current = bearingDeg(prev, next);

    if (shouldSnap(prev, next, snapThresholdMeters)) {
      fromRef.current = next;
      toRef.current = next;
      setShown({ ...next, bearing: bearingRef.current });
      return;
    }

    fromRef.current = prev ?? next;
    toRef.current = next;
    startRef.current = Date.now();
  }, [target?.lat, target?.lng, target?.recordedAt]);

  // rAF tween loop, paused while the tab is hidden.
  useEffect(() => {
    let stopped = false;

    const frame = () => {
      if (stopped) return;
      const from = fromRef.current;
      const to = toRef.current;
      if (from && to) {
        const t = Math.min(1, (Date.now() - startRef.current) / pingIntervalMs);
        const pos = lerpLatLng(from, to, t);
        setShown((cur) =>
          cur && cur.lat === pos.lat && cur.lng === pos.lng && cur.bearing === bearingRef.current
            ? cur
            : { ...pos, bearing: bearingRef.current },
        );
      }
      rafRef.current = requestAnimationFrame(frame);
    };

    const start = () => {
      if (rafRef.current == null) rafRef.current = requestAnimationFrame(frame);
    };
    const stop = () => {
      if (rafRef.current != null) {
        cancelAnimationFrame(rafRef.current);
        rafRef.current = null;
      }
    };

    const onVisibility = () => {
      if (document.hidden) {
        stop();
      } else {
        // Resume: snap straight to the latest target (don't "drive" the gap).
        if (toRef.current) {
          fromRef.current = toRef.current;
          setShown({ ...toRef.current, bearing: bearingRef.current });
        }
        start();
      }
    };

    if (!document.hidden) start();
    document.addEventListener('visibilitychange', onVisibility);
    return () => {
      stopped = true;
      stop();
      document.removeEventListener('visibilitychange', onVisibility);
    };
  }, [pingIntervalMs]);

  return shown;
}
