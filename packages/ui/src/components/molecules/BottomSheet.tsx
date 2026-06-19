import { useEffect, useCallback, useRef, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

interface BottomSheetProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  children: ReactNode;
  className?: string;
  snapPoints?: number[];
}

export function BottomSheet({
  open,
  onClose,
  title,
  children,
  className = '',
  snapPoints,
}: BottomSheetProps) {
  const sheetRef = useRef<HTMLDivElement>(null);

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

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop">
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        role="button"
        tabIndex={0}
        aria-label="Close"
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
      />
      <div
        ref={sheetRef}
        className={`absolute bottom-0 left-0 right-0 max-h-[85vh] bg-[var(--brand-surface)] rounded-t-2xl shadow-elevation-4 flex flex-col animate-slide-up ${className}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="flex items-center justify-center pt-2 pb-1">
          <div className="w-10 h-1 rounded-full bg-[var(--brand-border)]" />
        </div>
        {title && (
          <div className="flex items-center justify-between px-5 pt-2 pb-3">
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
        <div className="flex-1 overflow-y-auto px-5 pb-6">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
