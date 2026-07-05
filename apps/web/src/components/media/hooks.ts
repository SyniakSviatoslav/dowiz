// SSR-safe environment hooks for the media renderers. No deps.

import { useEffect, useState } from 'react';

const RM_QUERY = '(prefers-reduced-motion: reduce)';

/**
 * True when the user asked for reduced motion. SSR-safe (returns false on the
 * server / before hydration) and live — re-renders if the OS setting changes.
 */
export function useReducedMotion(): boolean {
  const [reduced, setReduced] = useState<boolean>(() =>
    typeof window !== 'undefined' && typeof window.matchMedia === 'function'
      ? window.matchMedia(RM_QUERY).matches
      : false,
  );

  useEffect(() => {
    if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return;
    const mql = window.matchMedia(RM_QUERY);
    const onChange = () => setReduced(mql.matches);
    onChange(); // sync in case it changed between first render and effect
    // addEventListener is the modern API; older Safari only has addListener.
    if (typeof mql.addEventListener === 'function') {
      mql.addEventListener('change', onChange);
      return () => mql.removeEventListener('change', onChange);
    }
    mql.addListener(onChange);
    return () => mql.removeListener(onChange);
  }, []);

  return reduced;
}

const SLOW_EFFECTIVE_TYPES = new Set(['slow-2g', '2g', '3g']);

type NetworkInformationLike = {
  saveData?: boolean;
  effectiveType?: string;
  addEventListener?: (type: 'change', listener: () => void) => void;
  removeEventListener?: (type: 'change', listener: () => void) => void;
};

function readSaveData(): boolean {
  if (typeof navigator === 'undefined') return false;
  const conn = (navigator as Navigator & { connection?: NetworkInformationLike }).connection;
  if (!conn) return false;
  if (conn.saveData) return true;
  return conn.effectiveType ? SLOW_EFFECTIVE_TYPES.has(conn.effectiveType) : false;
}

/**
 * True when the client signalled a desire to save data — navigator.connection
 * saveData flag, or an effectiveType of slow-2g/2g/3g. SSR-safe; live where the
 * Network Information API supports change events.
 */
export function useSaveData(): boolean {
  const [save, setSave] = useState<boolean>(readSaveData);

  useEffect(() => {
    if (typeof navigator === 'undefined') return;
    const conn = (navigator as Navigator & { connection?: NetworkInformationLike }).connection;
    setSave(readSaveData());
    if (!conn?.addEventListener) return;
    const onChange = () => setSave(readSaveData());
    conn.addEventListener('change', onChange);
    return () => conn.removeEventListener?.('change', onChange);
  }, []);

  return save;
}
