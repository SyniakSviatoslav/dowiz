import { useState, useEffect, useRef } from 'react';
import { useGeolocation } from '@deliveryos/ui';

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

// useGeoStream.ts
export function useGeoStream(courierId: string, enabled: boolean) {
  const { position } = useGeolocation();

  useEffect(() => {
    if (!enabled || !position) return;

    if (position.accuracy > 100) return;
    if (position.speed && position.speed > 150) return;

    // TODO: Send via WebSocket or API
  }, [position, enabled, courierId]);
}
