import { useState, useEffect, useRef } from 'react';

export interface GeoPosition {
  lat: number;
  lng: number;
  accuracy: number;
  heading: number | null;
  speed: number | null;
}

export interface GeoError {
  code: number;
  message: string;
}

export type GeoStatus = 'idle' | 'requesting' | 'granted' | 'denied' | 'unavailable';

export function useGeolocation(options?: PositionOptions) {
  const [position, setPosition] = useState<GeoPosition | null>(null);
  const [error, setError] = useState<GeoError | null>(null);
  const [status, setStatus] = useState<GeoStatus>('idle');
  const watchId = useRef<number | null>(null);

  useEffect(() => {
    if (typeof navigator === 'undefined' || !navigator.geolocation) {
      setStatus('unavailable');
      return;
    }

    setStatus('requesting');

    const id = navigator.geolocation.watchPosition(
      (pos) => {
        setPosition({
          lat: pos.coords.latitude,
          lng: pos.coords.longitude,
          accuracy: pos.coords.accuracy,
          heading: pos.coords.heading,
          speed: pos.coords.speed,
        });
        setError(null);
        setStatus('granted');
      },
      (err) => {
        setError({ code: err.code, message: err.message });
        setStatus(err.code === err.PERMISSION_DENIED ? 'denied' : 'unavailable');
      },
      { enableHighAccuracy: true, timeout: 10000, maximumAge: 30000, ...options },
    );

    watchId.current = id;

    return () => {
      navigator.geolocation.clearWatch(id);
    };
  }, []);

  return { position, error, status } as const;
}
