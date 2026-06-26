import type { ReactNode } from 'react';
import { useI18n } from '../lib/I18nProvider.js';
import { PaperIllustration } from './PaperIllustration.js';
import { ArtNouveauDivider } from './NomadicScene.js';
import { isPaperSkinEnabled } from '../theme/paperSkin.js';

// --- SkeletonBase ---
export function SkeletonBase({ className = '' }: { className?: string }) {
  return (
    <div className={`animate-pulse rounded bg-[var(--brand-surface-raised)] ${className}`} />
  );
}

// --- EmptyState ---
export function EmptyState({ title, description, icon, action, fullPage = false }: { title: string; description: string; icon?: ReactNode; action?: ReactNode; fullPage?: boolean }) {
  // Paper skin: when no icon is supplied, fall back to a Moebius line illustration so
  // internal empty states feel hand-drawn rather than blank. Opt-in via the global skin
  // flag; off everywhere else (incl. the white-label client storefront).
  const paperFallback = !icon && isPaperSkinEnabled()
    ? <PaperIllustration name="island" className="mx-auto max-w-[200px]" />
    : null;
  const card = (
    <div className="flex flex-col items-center justify-center p-8 text-center rounded-[var(--brand-radius)] border border-dashed border-[var(--brand-border)] bg-[var(--brand-surface)]">
      {icon && <div className="mb-4 text-4xl text-[var(--brand-text-muted)]">{icon}</div>}
      {paperFallback && (
        <div className="mb-4 w-full flex flex-col items-center">
          {paperFallback}
          <div className="w-40 mt-3"><ArtNouveauDivider /></div>
        </div>
      )}
      <h3 className="mb-2 text-lg font-semibold text-[var(--brand-text)]">{title}</h3>
      <p className="mb-6 text-sm text-[var(--brand-text-muted)] max-w-sm">{description}</p>
      {action}
    </div>
  );
  // fullPage: this empty/error state IS the whole screen → vertically centre it so it doesn't
  // strand at the top with a large dead void below (esp. on mobile). See mobile-polish #10.
  return fullPage ? <div className="min-h-[68dvh] flex flex-col items-center justify-center">{card}</div> : card;
}

// --- OfflineBanner ---
export function OfflineBanner({ fallbackPhone }: { fallbackPhone?: string }) {
  const { t } = useI18n();
  return (
    <div className="fixed top-0 left-0 right-0 z-50 bg-[var(--color-warning-strong)] text-[var(--brand-text)] px-4 py-2 text-sm font-medium text-center shadow-md">
      {t('common.offline', 'You are currently offline. Please check your internet connection.')}
      {fallbackPhone && (
        <span className="block mt-1">{t('common.call_us', 'Need help? Call us:')} <a href={`tel:${fallbackPhone}`} className="underline font-bold">{fallbackPhone}</a></span>
      )}
    </div>
  );
}

// --- WSStatusDot ---
export function WSStatusDot({ status }: { status: 'connecting' | 'connected' | 'disconnected' | 'reconnecting' | 'error' }) {
  const colors = {
    connected: 'bg-[var(--color-success)]',
    connecting: 'bg-[var(--color-warning)] animate-pulse',
    reconnecting: 'bg-[var(--color-warning)] animate-pulse',
    disconnected: 'bg-[var(--brand-text-muted)]',
    error: 'bg-[var(--color-danger)]',
  };

  return (
    <div className="flex items-center gap-2" data-connected={status === 'connected' ? 'true' : 'false'} title={`WebSocket Status: ${status}`}>
      <span className="relative flex h-3 w-3">
        {(status === 'connecting' || status === 'reconnecting') && (
          <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--color-warning)] opacity-75"></span>
        )}
        <span className={`relative inline-flex rounded-full h-3 w-3 ${colors[status]}`}></span>
      </span>
    </div>
  );
}

// --- (ErrorBoundary moved to canonical ErrorBoundary.tsx) ---
