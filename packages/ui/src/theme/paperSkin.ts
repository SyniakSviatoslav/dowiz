// PAPER / MOEBIUS internal skin — feature gate.
//
// The skin is OFF by default (prod stays on the dark teal identity). It turns on when
// either the build sets VITE_PAPER_SKIN=on, or a session opts in via
// localStorage('dos_paper_skin')='on' (used for live preview / the iteration loop).
// Internal-only: callers gate /admin and /courier roots, never the client storefront.
export function isPaperSkinEnabled(): boolean {
  // Build-time flag (statically replaced by Vite; safe in SSR — no global access).
  const env = (import.meta as unknown as { env?: Record<string, string | undefined> }).env;
  if (env?.VITE_PAPER_SKIN === 'on') return true;

  // Runtime opt-in for preview without a rebuild.
  if (typeof window !== 'undefined') {
    try {
      return window.localStorage?.getItem('dos_paper_skin') === 'on';
    } catch {
      // private mode / storage disabled — feature simply stays off
      void 0;
    }
  }
  return false;
}

/** The value to spread onto an internal shell root: `{...paperSkinAttr()}`. */
export function paperSkinAttr(): { 'data-skin'?: 'paper' } {
  return isPaperSkinEnabled() ? { 'data-skin': 'paper' } : {};
}
