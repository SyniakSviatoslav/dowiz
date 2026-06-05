import { useState, useRef, useCallback } from 'react';

export interface GeoPoint {
  lat: number;
  lng: number;
  accuracy: number;
  heading: number | null;
  speed: number | null;
  timestamp: number;
}

export type GeoStreamStatus = 'idle' | 'active' | 'error' | 'unsupported';

export function useGeoStream(options?: PositionOptions) {
  const [points, setPoints] = useState<GeoPoint[]>([]);
  const [status, setStatus] = useState<GeoStreamStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const watchId = useRef<number | null>(null);

  const start = useCallback(() => {
    if (typeof navigator === 'undefined' || !navigator.geolocation) {
      setStatus('unsupported');
      setError('Geolocation not supported');
      return;
    }
    setStatus('active');
    const id = navigator.geolocation.watchPosition(
      (pos) => {
        const point: GeoPoint = {
          lat: pos.coords.latitude,
          lng: pos.coords.longitude,
          accuracy: pos.coords.accuracy,
          heading: pos.coords.heading,
          speed: pos.coords.speed,
          timestamp: pos.timestamp,
        };
        setPoints((prev) => [...prev, point]);
        setError(null);
      },
      (err) => {
        setStatus('error');
        setError(err.message);
      },
      { enableHighAccuracy: true, timeout: 10000, maximumAge: 5000, ...options },
    );
    watchId.current = id;
  }, []);

  const stop = useCallback(() => {
    if (watchId.current !== null) {
      navigator.geolocation.clearWatch(watchId.current);
      watchId.current = null;
    }
    setStatus('idle');
  }, []);

  const clear = useCallback(() => {
    setPoints([]);
  }, []);

  return { points, status, error, start, stop, clear, isActive: status === 'active' } as const;
}
