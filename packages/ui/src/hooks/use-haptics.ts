import { useCallback } from 'react';

export type HapticType = 'tap' | 'success' | 'error';

export function useHaptics() {
  const trigger = useCallback((type: HapticType) => {
    if (typeof navigator === 'undefined' || !navigator.vibrate) return;
    const patterns: Record<HapticType, number[]> = {
      tap:    [10],
      success: [10, 30, 10],
      error:  [30, 20, 50],
    };
    const pattern = patterns[type];
    try {
      navigator.vibrate(pattern);
    } catch {
      // vibrate not supported — graceful no-op
    }
  }, []);

  return { trigger };
}
