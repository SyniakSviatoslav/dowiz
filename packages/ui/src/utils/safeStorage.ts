// Guarded localStorage. Raw `localStorage` access THROWS in sandboxed iframes (opaque
// origin — e.g. embed mode), privacy modes, and storage-blocked browsers. Unguarded reads
// then crash the app — including render-time reads that take down a whole page. Always go
// through this wrapper; it degrades to a no-op / null instead of throwing.
/* eslint-disable local/no-empty-catch -- storage access is best-effort; failures are silently ignored by design */
export const safeStorage = {
  get(key: string): string | null {
    try {
      return typeof window !== 'undefined' ? window.localStorage.getItem(key) : null;
    } catch {
      return null;
    }
  },
  set(key: string, value: string): void {
    try {
      if (typeof window !== 'undefined') window.localStorage.setItem(key, value);
    } catch {
      /* storage unavailable — ignore; write is best-effort */
    }
  },
  remove(key: string): void {
    try {
      if (typeof window !== 'undefined') window.localStorage.removeItem(key);
    } catch {
      /* storage unavailable — ignore; remove is best-effort */
    }
  },
};
