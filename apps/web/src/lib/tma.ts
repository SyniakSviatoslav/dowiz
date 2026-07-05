// Telegram Mini App (TMA) — detection + theme-attribute mapping + back-button wiring for the
// existing /s/:slug storefront opened inside Telegram's WebView (2026-07-04
// customer-distribution-channels research §3). Flag: VITE_TMA_ENABLED (default false/unset).
//
// Scope + honest gap (see docs/design/channel-hub/TMA-VALIDATION.md): this module does NOT
// load `telegram-web-app.js` itself — the storefront's CSP script-src (apps/api/src/lib/
// spa-shell.ts) does not currently whitelist telegram.org, and the shared index.html serves
// every route (admin/courier/storefront alike), so unconditionally injecting a third-party
// script there would load it on every page, not just Telegram-opened ones. Until a future
// phase adds a scoped CSP allowance + conditional script injection, `window.Telegram?.WebApp`
// will only exist if something else on the page loaded that script — this module is honest,
// tested groundwork that activates the instant that happens, not a promise that it does today.
//
// We do NOT restyle anything here — only set data-tma-* attributes on the root element so
// CSS can opt in later (per lane scope: "do NOT restyle").

export interface TelegramThemeParams {
  bg_color?: string;
  text_color?: string;
  hint_color?: string;
  link_color?: string;
  button_color?: string;
  button_text_color?: string;
  secondary_bg_color?: string;
  [key: string]: string | undefined;
}

export interface TelegramBackButton {
  show: () => void;
  hide: () => void;
  onClick: (cb: () => void) => void;
  offClick: (cb: () => void) => void;
}

export interface TelegramWebApp {
  initData?: string;
  initDataUnsafe?: Record<string, unknown>;
  colorScheme?: 'light' | 'dark';
  themeParams?: TelegramThemeParams;
  ready: () => void;
  expand: () => void;
  BackButton?: TelegramBackButton;
}

interface RootLike {
  setAttribute: (k: string, v: string) => void;
}

function readViteEnv(): Record<string, string | undefined> {
  // import.meta.env is Vite-injected at build time; guard so the module is also importable
  // in a plain Node test context (mirrors src/lib/tileConfig.ts's guard).
  return (typeof import.meta !== 'undefined' && (import.meta as any).env) || {};
}

export function isTmaFlagEnabled(env: Record<string, string | undefined> = readViteEnv()): boolean {
  return env.VITE_TMA_ENABLED === 'true';
}

/**
 * Detect the Telegram WebApp bridge object. Returns undefined outside Telegram, or when
 * `telegram-web-app.js` hasn't loaded (see the honest CSP gap in the file header). Accepts
 * an injectable `win` so this is unit-testable without a DOM (node --test).
 */
export function detectTelegramWebApp(win: any = typeof window !== 'undefined' ? window : undefined): TelegramWebApp | undefined {
  return win?.Telegram?.WebApp;
}

// Theme params come from an UNTRUSTED bridge object (window.Telegram.WebApp) — only these
// documented keys map to attributes, and only hex-color values pass. Anything else is dropped,
// so a hostile page context can never mint arbitrary data-* attribute names/values on <html>.
const TMA_THEME_KEY_ALLOWLIST = [
  'bg_color',
  'text_color',
  'hint_color',
  'link_color',
  'button_color',
  'button_text_color',
  'secondary_bg_color',
] as const;
const TMA_HEX_COLOR_RE = /^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/;

/**
 * Map Telegram themeParams → data-tma-* attributes on `root` WITHOUT restyling anything —
 * CSS can opt in later. Returns the applied attribute map (useful for assertions in tests).
 * Keys are allowlisted and values must be hex colors (see TMA_THEME_KEY_ALLOWLIST).
 */
export function applyTmaThemeAttributes(webApp: TelegramWebApp | undefined, root?: RootLike): Record<string, string> {
  const attrs: Record<string, string> = {};
  if (!webApp) return attrs;
  attrs['data-tma'] = 'true';
  if (webApp.colorScheme === 'light' || webApp.colorScheme === 'dark') attrs['data-tma-scheme'] = webApp.colorScheme;
  const theme = webApp.themeParams || {};
  for (const key of TMA_THEME_KEY_ALLOWLIST) {
    const value = theme[key];
    if (typeof value === 'string' && TMA_HEX_COLOR_RE.test(value)) {
      attrs[`data-tma-${key.replace(/_/g, '-')}`] = value;
    }
  }
  if (root) {
    for (const [k, v] of Object.entries(attrs)) root.setAttribute(k, v);
  }
  return attrs;
}

export interface InitTmaOpts {
  win?: any;
  root?: RootLike;
  /** Override flag detection (tests only); defaults to isTmaFlagEnabled(). */
  enabled?: boolean;
}

/**
 * Full init: ready() + expand() (best-effort — swallow errors from a WebView that doesn't
 * implement a method) + theme-attribute mapping. No-op when the flag is off or the WebApp
 * bridge isn't present. Returns true iff Telegram WebApp init actually ran.
 */
export function initTelegramMiniApp(opts: InitTmaOpts = {}): boolean {
  const enabled = opts.enabled ?? isTmaFlagEnabled();
  if (!enabled) return false;
  const webApp = detectTelegramWebApp(opts.win);
  if (!webApp) return false;
  try { webApp.ready(); } catch { /* best-effort — some hosts may lag the full API */ }
  try { webApp.expand(); } catch { /* best-effort */ }
  applyTmaThemeAttributes(webApp, opts.root);
  return true;
}

/**
 * Wires the Telegram BackButton to `onBack` (e.g. closing an in-app sheet instead of
 * navigating away) for in-app SPA navigation. Returns a cleanup function. No-op (returns a
 * no-op cleanup) when the WebApp/BackButton isn't present — trivial, additive, never breaks
 * normal browser back behavior outside Telegram.
 */
export function setupTmaBackButton(onBack: () => void, win?: any): () => void {
  const backButton = detectTelegramWebApp(win)?.BackButton;
  if (!backButton) return () => {};
  backButton.show();
  backButton.onClick(onBack);
  return () => {
    try { backButton.offClick(onBack); } catch { /* best-effort */ }
    try { backButton.hide(); } catch { /* best-effort */ }
  };
}
