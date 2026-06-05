import { useState, useEffect, useRef } from 'react';

// useOnlineStatus.ts
export function useOnlineStatus() {
  const [isOnline, setIsOnline] = useState(typeof navigator !== 'undefined' ? navigator.onLine : true);

  useEffect(() => {
    if (typeof window === 'undefined') return;

    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return isOnline;
}

// useSound.ts
export function useSound(url: string) {
  const audioRef = useRef<HTMLAudioElement | null>(null);

  const play = () => {
    if (typeof window === 'undefined') return;
    if (!audioRef.current) {
      audioRef.current = new Audio(url);
    }
    audioRef.current.currentTime = 0;
    audioRef.current.play().catch(() => {
      // Audio playback requires user interaction (esp. iOS)
    });
  };

  return { play };
}

// useEmbedMode.ts
export function useEmbedMode() {
  const [isEmbed, setIsEmbed] = useState(false);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      const urlParams = new URLSearchParams(window.location.search);
      setIsEmbed(urlParams.get('embed') === 'true');
    }
  }, []);

  return isEmbed;
}

// useGeolocation.ts
export function useGeolocation(options: PositionOptions = { enableHighAccuracy: true }) {
  const [position, setPosition] = useState<GeolocationPosition | null>(null);
  const [error, setError] = useState<GeolocationPositionError | null>(null);

  useEffect(() => {
    if (typeof navigator === 'undefined' || !navigator.geolocation) {
      return;
    }

    const watcher = navigator.geolocation.watchPosition(
      (pos) => setPosition(pos),
      (err) => setError(err),
      options
    );

    return () => navigator.geolocation.clearWatch(watcher);
  }, [options.enableHighAccuracy, options.maximumAge, options.timeout]);

  return { position, error };
}

// useGeoStream.ts
export function useGeoStream(courierId: string, enabled: boolean) {
  const { position } = useGeolocation();

  useEffect(() => {
    if (!enabled || !position) return;

    // Filter accuracy and speed (Phase 3 R1: accuracy>100m, speed>150m/s -> drop)
    if (position.coords.accuracy > 100) return;
    if (position.coords.speed && position.coords.speed > 150) return;

    // TODO: Send via WebSocket or API
    // apiClient(`/couriers/${courierId}/location`, { method: 'POST', body: { lat: position.coords.latitude, lng: position.coords.longitude } })
  }, [position, enabled, courierId]);
}
