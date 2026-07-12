import { useEffect, useCallback, useRef, useState, type ReactNode } from 'react';
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
  // Ease-out slide-up enter without global keyframes; reduced-motion collapses
  // the --motion-* tokens to 0ms so this becomes instant.
  const [entered, setEntered] = useState(false);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
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

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop">
      <div
        className={`absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity ease-[var(--ease-out)]`}
        style={{ opacity: entered ? 1 : 0, transitionDuration: `transitionDuration` }}
        role="button"
        tabIndex={0}
        aria-label="Close"
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === `Enter` || e.key === ' ') { e.preventDefault(); onClose(); } }}
      />
      <div
        ref={sheetRef}
        className={`absolute bottom-0 left-0 right-0 max-h-[85vh] bg-brand-surface flex flex-col transition-transform ease-[var(--ease-out)] ${className}`}
        style={{
          borderTopLeftRadius: 'var(--brand-radius)',
          borderTopRightRadius: 'var(--brand-radius)',
          boxShadow: `boxShadow`,
          transform: entered ? 'translateY(0)' : 'translateY(100%)',
          transitionDuration: `transitionDuration`,
        }}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="flex items-center justify-center pt-3 pb-1">
          <div className="w-10 h-1.5 rounded-full bg-brand-border" aria-hidden="true" />
        </div>
        {title && (
          <div className="flex items-center justify-between px-5 pt-2 pb-3">
            <h2 className="text-lg font-heading font-semibold text-brand-text">{title}</h2>
            <button
              onClick={onClose}
              className={`w-8 h-8 flex items-center justify-center rounded-full hover:bg-brand-surface-raised text-brand-text-muted transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand-primary focus-visible:ring-offset-2`}
              aria-label="Close"
            >
              <i className="ti ti-x" />
            </button>
          </div>
        )}
        <div
          className="flex-1 overflow-y-auto px-5 pb-6"
          style={{ paddingBottom: 'max(1.5rem, env(safe-area-inset-bottom))' }}
        >
          {children}
        </div>
      </div>
    </div>,
    document.body,
  );
}
