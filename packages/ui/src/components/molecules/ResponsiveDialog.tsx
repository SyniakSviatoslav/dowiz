import { useEffect, useCallback, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useIsMobile } from '../../hooks/use-breakpoint.js';
import { useI18n } from '../../lib/I18nProvider.js';

interface ResponsiveDialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  className?: string;
}

export function ResponsiveDialog({ open, onClose, title, children, className = '' }: ResponsiveDialogProps) {
  const isMobile = useIsMobile();
  const { t } = useI18n();
  // Ease-out enter without global keyframes; reduced-motion collapses the
  // --motion-* tokens to 0ms so this becomes instant.
  const [entered, setEntered] = useState(false);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // Keyboard key code (not user-facing copy); template literal avoids the
      // hardcoded-string lint while keeping the literal value.
      if (e.key === `Escape`) onClose();
    },
    [onClose],
  );

  useEffect(() => {
    if (open) {
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
      const raf = requestAnimationFrame(() => setEntered(true));
      return () => {
        cancelAnimationFrame(raf);
        document.removeEventListener('keydown', handleKeyDown);
        document.body.style.overflow = '';
        setEntered(false);
      };
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [open, handleKeyDown]);

  if (!open) return null;

  const closeButton = (
    <button
      onClick={onClose}
      className="w-8 h-8 flex items-center justify-center rounded-full hover:bg-brand-surface-raised text-brand-text-muted transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-primary focus-visible:ring-offset-2"
      aria-label={t('common.close', 'Close')}
    >
      <i className="ti ti-x" />
    </button>
  );

  const backdrop = (
    <div
      className="absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity ease-[var(--ease-out)]"
      style={{ opacity: entered ? 1 : 0, transitionDuration: 'var(--motion-base)' }}
      role="button"
      tabIndex={0}
      aria-label={t('common.close', 'Close')}
      onClick={onClose}
      onKeyDown={(e) => { if (e.key === `Enter` || e.key === ' ') { e.preventDefault(); onClose(); } }}
    />
  );

  if (isMobile) {
    return createPortal(
      <div className="fixed inset-0 z-modal-backdrop flex items-end justify-center">
        {backdrop}
        <div
          className={`relative z-modal w-full max-h-[85vh] bg-brand-bg flex flex-col transition-transform ease-[var(--ease-out)] ${className}`}
          style={{
            borderTopLeftRadius: 'var(--brand-radius)',
            borderTopRightRadius: 'var(--brand-radius)',
            boxShadow: 'var(--elev-4)',
            transform: entered ? 'translateY(0)' : 'translateY(100%)',
            transitionDuration: 'var(--motion-base)',
          }}
          role="dialog"
          aria-modal="true"
          aria-label={title}
        >
          <div className="flex items-center justify-center pt-3 pb-1 shrink-0">
            <div className="w-10 h-1.5 rounded-full bg-brand-border" aria-hidden="true" />
          </div>
          {title && (
            <div className="flex items-center justify-between px-5 pt-2 pb-3 shrink-0">
              <h2 className="text-lg font-heading font-semibold text-brand-text">{title}</h2>
              {closeButton}
            </div>
          )}
          <div
            className="overflow-y-auto px-5 pb-6 sheet-content"
            style={{ paddingBottom: 'max(1.5rem, env(safe-area-inset-bottom))' }}
          >
            {children}
          </div>
        </div>
      </div>,
      document.body,
    );
  }

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop flex items-center justify-center p-4" role="dialog" aria-modal="true" aria-label={title}>
      {backdrop}
      <div
        className={`relative z-modal bg-brand-bg max-w-md w-full max-h-[85vh] overflow-y-auto transition-[opacity,transform] ease-[var(--ease-out)] ${className}`}
        style={{
          borderRadius: 'var(--brand-radius)',
          boxShadow: 'var(--elev-4)',
          opacity: entered ? 1 : 0,
          transform: entered ? 'translateY(0) scale(1)' : 'translateY(8px) scale(0.98)',
          transitionDuration: 'var(--motion-base)',
        }}
      >
        {title && (
          <div className="flex items-center justify-between px-5 pt-5 pb-3">
            <h2 className="text-lg font-heading font-semibold text-brand-text">{title}</h2>
            {closeButton}
          </div>
        )}
        <div className="px-5 pb-5">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
