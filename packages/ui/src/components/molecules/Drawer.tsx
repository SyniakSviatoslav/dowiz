import { useEffect, useCallback, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

type DrawerSide = 'left' | 'right';

interface DrawerProps {
  open: boolean;
  onClose: () => void;
  side?: DrawerSide;
  title?: string;
  children: ReactNode;
  className?: string;
}

export function Drawer({
  open,
  onClose,
  side = 'right',
  title,
  children,
  className = '',
}: DrawerProps) {
  // Ease-out slide-in enter without global keyframes; reduced-motion collapses
  // the --motion-* tokens to 0ms so this becomes instant.
  const [entered, setEntered] = useState(false);

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

  const sideClasses = side === 'right' ? 'right-0' : 'left-0';
  const hiddenTransform = side === 'right' ? 'translateX(100%)' : 'translateX(-100%)';

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop">
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm transition-opacity ease-[var(--ease-out)]"
        style={{ opacity: entered ? 1 : 0, transitionDuration: 'var(--motion-base)' }}
        role="button"
        tabIndex={0}
        aria-label="Close"
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
      />
      <div
        className={`absolute top-0 bottom-0 w-80 max-w-[85vw] bg-[var(--brand-surface)] flex flex-col transition-transform ease-[var(--ease-out)] ${sideClasses} ${className}`}
        style={{
          boxShadow: 'var(--elev-4)',
          transform: entered ? 'translateX(0)' : hiddenTransform,
          transitionDuration: 'var(--motion-base)',
        }}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        {title && (
          <div className="flex items-center justify-between px-4 py-4 border-b border-[var(--brand-border)]">
            <h2 className="text-lg font-heading font-semibold text-[var(--brand-text)]">{title}</h2>
            <button
              onClick={onClose}
              className="w-8 h-8 flex items-center justify-center rounded-full hover:bg-[var(--brand-surface-raised)] text-[var(--brand-text-muted)] transition-[background-color,transform] duration-[var(--motion-fast)] ease-[var(--ease-soft)] active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--brand-primary)] focus-visible:ring-offset-2"
              aria-label="Close"
            >
              <i className="ti ti-x" />
            </button>
          </div>
        )}
        <div className="flex-1 overflow-y-auto p-4">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
