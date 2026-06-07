import { useCallback, useRef } from 'react';

export type SoundType = 'notification' | 'success' | 'error' | 'order_update';

const DEFAULT_SOUNDS: Record<SoundType, string> = {
  notification: '/sounds/notification.mp3',
  success: '/sounds/success.mp3',
  error: '/sounds/error.mp3',
  order_update: '/sounds/order-update.mp3',
};

export function useSound(customSounds?: Partial<Record<SoundType, string>>) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const sounds = { ...DEFAULT_SOUNDS, ...customSounds };

  const play = useCallback(
    (type: SoundType) => {
      const src = sounds[type];
      if (!src) return;
      try {
        if (!audioRef.current) {
          audioRef.current = new Audio();
        }
        audioRef.current.src = src;
        audioRef.current.currentTime = 0;
        audioRef.current.play().catch(() => {});
    } catch {
      // Audio not supported or autoplay blocked
      console.debug('[use-sound] audio playback failed');
    }
    },
    [sounds],
  );

  const playNotification = useCallback(() => play('notification'), [play]);
  const playSuccess = useCallback(() => play('success'), [play]);
  const playError = useCallback(() => play('error'), [play]);
  const playOrderUpdate = useCallback(() => play('order_update'), [play]);

  return { play, playNotification, playSuccess, playError, playOrderUpdate };
}
