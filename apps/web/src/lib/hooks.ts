import { useState, useEffect, useRef, useCallback } from 'react';
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
//
// The original implementation called `new Audio().play()` from a non-gesture
// context (a WebSocket message) and swallowed the autoplay rejection — silent
// on iOS Safari, with no signal to the user that the alert never fired. That is
// the "silent false promise of an alert", which the council ruled worse than no
// alert at all.
//
// This hook fixes that with three guarantees:
//  1. UNLOCK — primes the <audio> element on the first real user gesture
//     (the standard iOS Safari play()+pause() unlock). `armed` flips true only
//     once playback is genuinely permitted, so the UI can show an honest state.
//  2. PERSISTENCE — `start()` loops the ping every few seconds until `stop()`,
//     rather than a single fire-and-forget `.play()`.
//  3. HONEST FAILURE — a one-shot `play()` returns a Promise<boolean> resolving
//     to whether playback actually started, so callers can fall back to a
//     visible banner when audio is blocked. `armed` is the source of truth for
//     the indicator.
export function useSound(url: string, repeatMs = 4000) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const loopRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [armed, setArmed] = useState(false);

  const ensureEl = () => {
    if (typeof window === 'undefined') return null;
    if (!audioRef.current) {
      const el = new Audio(url);
      el.preload = 'auto';
      audioRef.current = el;
    }
    return audioRef.current;
  };

  // Standard iOS/Safari unlock: play then immediately pause inside a user
  // gesture. Resolves true if the browser allowed playback (→ armed).
  const unlock = useCallback(async (): Promise<boolean> => {
    const el = ensureEl();
    if (!el) return false;
    try {
      el.muted = true;
      await el.play();
      el.pause();
      el.currentTime = 0;
      el.muted = false;
      setArmed(true);
      return true;
    } catch {
      el.muted = false;
      setArmed(false);
      return false;
    }
  }, [url]);

  // Auto-unlock on the first user gesture anywhere in the app. We attach with
  // { once: true } per event and tear them all down as soon as one succeeds.
  useEffect(() => {
    if (typeof window === 'undefined') return;
    let done = false;
    const events: Array<keyof DocumentEventMap> = ['pointerdown', 'keydown', 'touchstart'];
    const handler = () => {
      if (done) return;
      done = true;
      void unlock();
      cleanup();
    };
    const cleanup = () => events.forEach(e => window.removeEventListener(e, handler));
    events.forEach(e => window.addEventListener(e, handler, { once: true, passive: true }));
    return cleanup;
  }, [unlock]);

  // Fire once. Resolves to whether playback actually started (false = blocked).
  const play = useCallback(async (): Promise<boolean> => {
    const el = ensureEl();
    if (!el) return false;
    try {
      el.currentTime = 0;
      await el.play();
      setArmed(true);
      return true;
    } catch {
      // Autoplay blocked (no gesture yet) — caller falls back to a visible cue.
      return false;
    }
  }, [url]);

  // Loop the ping until stop() — persistent alert for an un-acknowledged order.
  const start = useCallback(() => {
    if (loopRef.current) return; // already looping
    void play();
    loopRef.current = setInterval(() => { void play(); }, repeatMs);
  }, [play, repeatMs]);

  const stop = useCallback(() => {
    if (loopRef.current) {
      clearInterval(loopRef.current);
      loopRef.current = null;
    }
    const el = audioRef.current;
    if (el) { el.pause(); el.currentTime = 0; }
  }, []);

  useEffect(() => () => { if (loopRef.current) clearInterval(loopRef.current); }, []);

  return { play, start, stop, unlock, armed };
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
