import { useEffect, useCallback, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { useIsMobile } from '../../hooks/use-breakpoint.js';

interface ResponsiveDialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  className?: string;
}

export function ResponsiveDialog({ open, onClose, title, children, className = '' }: ResponsiveDialogProps) {
  const isMobile = useIsMobile();
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    },
    [onClose],
  );

  useEffect(() => {
    if (open) {
      document.addEventListener('keydown', handleKeyDown);
      document.body.style.overflow = 'hidden';
    }
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.body.style.overflow = '';
    };
  }, [open, handleKeyDown]);

  if (!open) return null;

  if (isMobile) {
    return createPortal(
      <div className="fixed inset-0 z-modal-backdrop flex items-end justify-center">
        <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
        <div
          className={`relative z-modal w-full max-h-[85vh] bg-[var(--brand-bg)] rounded-t-2xl shadow-elevation-4 flex flex-col animate-slide-up ${className}`}
          role="dialog"
          aria-modal="true"
          aria-label={title}
        >
          <div className="flex items-center justify-center pt-2 pb-1 shrink-0">
            <div className="w-10 h-1 rounded-full bg-[var(--brand-border)]" />
          </div>
          {title && (
            <div className="flex items-center justify-between px-5 pt-2 pb-3 shrink-0">
              <h2 className="text-lg font-heading font-semibold text-[var(--brand-text)]">{title}</h2>
              <button
                onClick={onClose}
                className="w-8 h-8 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] transition-colors"
                aria-label="Close"
              >
                <i className="ti ti-x" />
              </button>
            </div>
          )}
          <div className="overflow-y-auto px-5 pb-6 sheet-content">{children}</div>
        </div>
      </div>,
      document.body,
    );
  }

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop flex items-center justify-center p-4" role="dialog" aria-modal="true" aria-label={title}>
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div className={`relative z-modal bg-[var(--brand-bg)] rounded-xl shadow-elevation-4 max-w-md w-full max-h-[85vh] overflow-y-auto ${className}`}>
        {title && (
          <div className="flex items-center justify-between px-5 pt-5 pb-3">
            <h2 className="text-lg font-heading font-semibold text-[var(--brand-text)]">{title}</h2>
            <button
              onClick={onClose}
              className="w-8 h-8 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] transition-colors"
              aria-label="Close"
            >
              <i className="ti ti-x" />
            </button>
          </div>
        )}
        <div className="px-5 pb-5">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
