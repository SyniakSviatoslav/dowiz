import { useEffect, useCallback, type ReactNode } from 'react';
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

  const sideClasses = side === 'right' ? 'right-0 translate-x-0' : 'left-0 translate-x-0';

  return createPortal(
    <div className="fixed inset-0 z-modal-backdrop">
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" onClick={onClose} />
      <div
        className={`absolute top-0 bottom-0 w-80 max-w-[85vw] bg-[var(--brand-surface)] shadow-elevation-4 flex flex-col ${sideClasses} ${className}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        {title && (
          <div className="flex items-center justify-between px-4 py-4 border-b border-[var(--brand-border)]">
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
        <div className="flex-1 overflow-y-auto p-4">{children}</div>
      </div>
    </div>,
    document.body,
  );
}
