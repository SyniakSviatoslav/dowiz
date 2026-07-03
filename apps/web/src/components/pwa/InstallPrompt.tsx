import { useCallback, useEffect, useRef, useState } from 'react';
import { AnimatePresence, motion } from 'framer-motion';
import { useI18n } from '@deliveryos/ui';

// ─────────────────────────────────────────────────────────────────────────────
// InstallPrompt — first-visit "Add to Home Screen" nudge for the storefront PWA.
//
// Two paths, one component:
//   • Android / Chromium / desktop → capture the `beforeinstallprompt` event,
//     preventDefault it, stash the deferred event, and show a dismissible banner.
//     "Install" calls deferredPrompt.prompt() and resolves userChoice.
//   • iOS Safari → NEVER fires beforeinstallprompt, so we feature-detect
//     iOS + Safari + not-standalone and show a manual "Share → Add to Home
//     Screen" hint instead.
//
// Self-hiding: never shows when already installed (display-mode: standalone /
// navigator.standalone) or after `appinstalled`. Dismissal + installed state
// persist in localStorage so it doesn't nag on every visit. No new deps — native
// APIs (beforeinstallprompt, matchMedia) + the repo's framer-motion + i18n.
// ─────────────────────────────────────────────────────────────────────────────

const STORAGE_KEY = 'dowiz-pwa-install';
const IOS_HINT_DELAY_MS = 1500; // let the page settle before the manual iOS nudge

type BannerMode = 'hidden' | 'native' | 'ios';

// The non-standard install event. Typed locally because lib.dom.d.ts omits it.
type BeforeInstallPromptEvent = Event & {
  readonly platforms: string[];
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>;
};

// Guarded localStorage — raw access THROWS in sandboxed iframes / privacy modes.
// (Mirrors packages/ui safeStorage, kept inline to avoid a new package export.)
function storageGet(key: string): string | null {
  try {
    return typeof window !== 'undefined' ? window.localStorage.getItem(key) : null;
  } catch {
    return null;
  }
}
function storageSet(key: string, value: string): void {
  try {
    if (typeof window !== 'undefined') window.localStorage.setItem(key, value);
  } catch {
    /* storage unavailable — ignore */
  }
}

function isStandalone(): boolean {
  if (typeof window === 'undefined') return false;
  try {
    return (
      window.matchMedia?.('(display-mode: standalone)').matches === true ||
      // iOS Safari exposes standalone on navigator, not via display-mode media query.
      (window.navigator as unknown as { standalone?: boolean }).standalone === true
    );
  } catch {
    return false;
  }
}

function isIOSSafari(): boolean {
  if (typeof navigator === 'undefined') return false;
  const ua = navigator.userAgent || '';
  const iOS =
    /iphone|ipad|ipod/i.test(ua) ||
    // iPadOS 13+ reports as "Macintosh" but is a touch device.
    (/macintosh/i.test(ua) && typeof document !== 'undefined' && 'ontouchend' in document);
  // On iOS every engine is WebKit; only real Safari can Add-to-Home-Screen.
  const safari = /safari/i.test(ua) && !/crios|fxios|edgios|opios|opr/i.test(ua);
  return iOS && safari;
}

export function InstallPrompt() {
  const { t } = useI18n();
  const [mode, setMode] = useState<BannerMode>('hidden');
  const deferredRef = useRef<BeforeInstallPromptEvent | null>(null);
  const cardRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (typeof window === 'undefined') return;

    // Already installed, or previously dismissed/installed → stay out of the way.
    if (isStandalone()) {
      storageSet(STORAGE_KEY, 'installed');
      return;
    }
    if (storageGet(STORAGE_KEY)) return; // 'dismissed' | 'installed'

    const onBeforeInstall = (e: Event) => {
      e.preventDefault(); // suppress the mini-infobar; we drive our own UI
      deferredRef.current = e as BeforeInstallPromptEvent;
      setMode('native');
    };
    const onInstalled = () => {
      storageSet(STORAGE_KEY, 'installed');
      deferredRef.current = null;
      setMode('hidden');
    };

    window.addEventListener('beforeinstallprompt', onBeforeInstall);
    window.addEventListener('appinstalled', onInstalled);

    // iOS never fires beforeinstallprompt — show the manual hint after a beat.
    let iosTimer: ReturnType<typeof setTimeout> | undefined;
    if (isIOSSafari()) {
      iosTimer = setTimeout(() => {
        // Re-check: user may have installed / dismissed in the interim.
        if (!isStandalone() && !storageGet(STORAGE_KEY)) setMode('ios');
      }, IOS_HINT_DELAY_MS);
    }

    return () => {
      window.removeEventListener('beforeinstallprompt', onBeforeInstall);
      window.removeEventListener('appinstalled', onInstalled);
      if (iosTimer) clearTimeout(iosTimer);
    };
  }, []);

  // Move focus into the banner when it appears (keyboard + SR users).
  useEffect(() => {
    if (mode !== 'hidden') cardRef.current?.focus();
  }, [mode]);

  const dismiss = useCallback(() => {
    storageSet(STORAGE_KEY, 'dismissed');
    setMode('hidden');
  }, []);

  const install = useCallback(async () => {
    const deferred = deferredRef.current;
    if (!deferred) {
      dismiss();
      return;
    }
    try {
      await deferred.prompt();
      const choice = await deferred.userChoice;
      if (choice.outcome === 'accepted') {
        storageSet(STORAGE_KEY, 'installed');
      } else {
        storageSet(STORAGE_KEY, 'dismissed');
      }
    } catch {
      // prompt() can reject if already consumed — treat as dismissed, don't nag.
      storageSet(STORAGE_KEY, 'dismissed');
    } finally {
      deferredRef.current = null;
      setMode('hidden');
    }
  }, [dismiss]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Escape') dismiss();
    },
    [dismiss],
  );

  return (
    <AnimatePresence>
      {mode !== 'hidden' && (
        <motion.div
          ref={cardRef}
          data-testid="pwa-install-prompt"
          role="dialog"
          aria-label={t('pwa.install_title', 'Install dowiz')}
          tabIndex={-1}
          onKeyDown={onKeyDown}
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 24 }}
          transition={{ duration: 0.24, ease: [0.16, 1, 0.3, 1] }}
          className="fixed inset-x-0 bottom-0 outline-none"
          style={{ zIndex: 'var(--z-toast)' as unknown as number }}
        >
          <div
            className="mx-auto flex max-w-md items-start gap-3 rounded-t-2xl px-4 py-4"
            style={{
              background: 'var(--brand-surface-raised)',
              color: 'var(--brand-text)',
              border: '1px solid var(--brand-border)',
              borderBottom: 'none',
              boxShadow: '0 -8px 24px rgba(0,0,0,.12), 0 -2px 6px rgba(0,0,0,.06)',
              paddingBottom: 'calc(1rem + var(--safe-bottom))',
            }}
          >
            <div
              className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl"
              style={{ background: 'var(--brand-primary-light)', color: 'var(--brand-primary)' }}
              aria-hidden="true"
            >
              <i className="ti ti-download text-xl" />
            </div>

            <div className="min-w-0 flex-1">
              <p
                className="text-sm font-semibold"
                style={{ fontFamily: 'var(--brand-font-heading)' }}
              >
                {t('pwa.install_title', 'Install dowiz')}
              </p>

              {mode === 'ios' ? (
                <p
                  className="mt-0.5 flex flex-wrap items-center gap-1 text-xs"
                  style={{ color: 'var(--brand-text-muted)' }}
                >
                  <span>{t('pwa.ios_hint', "Tap Share, then 'Add to Home Screen'.")}</span>
                  <i className="ti ti-share text-sm" aria-hidden="true" />
                </p>
              ) : (
                <p className="mt-0.5 text-xs" style={{ color: 'var(--brand-text-muted)' }}>
                  {t('pwa.install_body', 'Add dowiz to your home screen for faster ordering.')}
                </p>
              )}

              <div className="mt-3 flex items-center gap-2">
                {mode === 'native' && (
                  <button
                    type="button"
                    data-testid="pwa-install-cta"
                    onClick={install}
                    className="inline-flex min-h-9 items-center rounded-lg px-3.5 text-sm font-semibold transition-transform active:scale-95"
                    style={{ background: 'var(--brand-primary)', color: 'var(--color-on-primary)' }}
                  >
                    {t('pwa.install_cta', 'Install')}
                  </button>
                )}
                <button
                  type="button"
                  data-testid="pwa-install-dismiss"
                  onClick={dismiss}
                  className="inline-flex min-h-9 items-center rounded-lg px-3.5 text-sm font-medium transition-transform active:scale-95"
                  style={{ background: 'transparent', color: 'var(--brand-text-muted)' }}
                >
                  {t('pwa.dismiss', 'Not now')}
                </button>
              </div>
            </div>

            <button
              type="button"
              aria-label={t('pwa.close_label', 'Dismiss install prompt')}
              onClick={dismiss}
              className="shrink-0 rounded-lg p-1 transition-transform active:scale-90"
              style={{ color: 'var(--brand-text-muted)' }}
            >
              <i className="ti ti-x text-lg" aria-hidden="true" />
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

export default InstallPrompt;
